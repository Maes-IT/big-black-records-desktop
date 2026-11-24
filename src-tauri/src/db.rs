use rusqlite::{Connection, DatabaseName, OpenFlags};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub struct DbState(pub Mutex<Connection>);

/// Dossier des données de l'application (base + sauvegardes automatiques).
pub struct DataDir(pub PathBuf);

pub const DB_FILE: &str = "bigblack.sqlite";
const BACKUP_DIR: &str = "backups";
const AUTO_BACKUP_PREFIX: &str = "bigblack-auto-";
/// Nombre de sauvegardes automatiques (une par jour d'ouverture) conservées.
const AUTO_BACKUP_KEEP: usize = 14;

/// Migrations numérotées : la migration d'indice i fait passer `user_version` de i à i + 1.
///
/// RÈGLE : ne jamais modifier une migration déjà livrée. Pour changer le schéma,
/// AJOUTER une entrée (ex. `include_str!("../migrations/002_xxx.sql")`).
///
/// La v1 est le schéma d'origine. Il est écrit en `IF NOT EXISTS`, donc il s'applique
/// aussi bien à une base neuve qu'à une base créée avant le versionnage (user_version = 0).
const MIGRATIONS: &[&str] = &[include_str!("../migrations/schema.sql")];

pub fn schema_version() -> i64 {
    MIGRATIONS.len() as i64
}

fn user_version(conn: &Connection) -> Result<i64, String> {
    conn.query_row("PRAGMA user_version", [], |r| r.get(0))
        .map_err(|e| e.to_string())
}

/// Amène la base à la dernière version du schéma. Chaque migration est appliquée
/// dans sa propre transaction : en cas d'échec, la base reste dans l'état précédent.
pub fn migrate(conn: &mut Connection) -> Result<(), String> {
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .map_err(|e| e.to_string())?;
    let current = user_version(conn)?;
    let latest = schema_version();
    if current > latest {
        return Err(format!(
            "Cette base (version {current}) a été créée par une version plus récente de l'application \
             (qui gère la version {latest}). Mettez l'application à jour."
        ));
    }
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(current as usize) {
        let target = i as i64 + 1;
        let tx = conn.transaction().map_err(|e| e.to_string())?;
        tx.execute_batch(sql)
            .map_err(|e| format!("Échec de la migration vers la version {target} : {e}"))?;
        tx.pragma_update(None, "user_version", target)
            .map_err(|e| e.to_string())?;
        tx.commit().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Ouvre (ou crée) la base. Retourne un message lisible en cas d'échec
/// au lieu de paniquer, pour pouvoir l'afficher à l'utilisateur.
pub fn init_db(app_data_dir: &Path) -> Result<Connection, String> {
    std::fs::create_dir_all(app_data_dir).map_err(|e| {
        format!("Impossible de créer le dossier de données « {} » : {e}", app_data_dir.display())
    })?;
    let db_path = app_data_dir.join(DB_FILE);
    let mut conn = Connection::open(&db_path)
        .map_err(|e| format!("Impossible d'ouvrir la base « {} » : {e}", db_path.display()))?;
    migrate(&mut conn).map_err(|e| {
        format!("Impossible d'initialiser la base « {} » (fichier verrouillé ou corrompu ?) : {e}", db_path.display())
    })?;
    Ok(conn)
}

// ==================== Sauvegardes ====================

pub fn backup_dir(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(BACKUP_DIR)
}

/// Copie cohérente de la base ouverte vers `dest` (API backup de SQLite).
pub fn backup_to(conn: &Connection, dest: &Path) -> Result<(), String> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    conn.backup(DatabaseName::Main, dest, None)
        .map_err(|e| format!("Échec de la sauvegarde vers « {} » : {e}", dest.display()))
}

fn sqlite_now(conn: &Connection, fmt: &str) -> Result<String, String> {
    conn.query_row("SELECT strftime(?1, 'now', 'localtime')", [fmt], |r| r.get(0))
        .map_err(|e| e.to_string())
}

/// Une sauvegarde automatique par jour (au premier lancement de la journée),
/// en ne gardant que les `AUTO_BACKUP_KEEP` plus récentes.
/// Retourne le chemin créé, ou `None` si celle du jour existe déjà.
pub fn auto_backup(conn: &Connection, dir: &Path) -> Result<Option<PathBuf>, String> {
    let day = sqlite_now(conn, "%Y-%m-%d")?;
    let dest = dir.join(format!("{AUTO_BACKUP_PREFIX}{day}.sqlite"));
    let created = if dest.exists() {
        None
    } else {
        backup_to(conn, &dest)?;
        Some(dest)
    };
    prune_auto_backups(dir, AUTO_BACKUP_KEEP)?;
    Ok(created)
}

fn prune_auto_backups(dir: &Path, keep: usize) -> Result<(), String> {
    let mut autos: Vec<PathBuf> = list_backups(dir)?
        .into_iter()
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with(AUTO_BACKUP_PREFIX))
        })
        .collect();
    // Noms datés AAAA-MM-JJ : l'ordre alphabétique est l'ordre chronologique.
    autos.sort();
    let excess = autos.len().saturating_sub(keep);
    for old in &autos[..excess] {
        std::fs::remove_file(old).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Fichiers .sqlite du dossier de sauvegardes, du plus récent au plus ancien (par nom).
pub fn list_backups(dir: &Path) -> Result<Vec<PathBuf>, String> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("sqlite"))
        .collect();
    files.sort();
    files.reverse();
    Ok(files)
}

/// Vérifie qu'un fichier est bien une base Big Black Records restaurable,
/// sans le créer ni le modifier.
pub fn check_backup_file(path: &Path) -> Result<(), String> {
    if !path.is_file() {
        return Err(format!("Fichier introuvable : « {} »", path.display()));
    }
    let src = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| format!("Fichier illisible : {e}"))?;
    let tables: i64 = src
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('clients', 'projets')",
            [],
            |r| r.get(0),
        )
        .map_err(|_| "Ce fichier n'est pas une base SQLite valide.".to_string())?;
    if tables != 2 {
        return Err("Ce fichier n'est pas une sauvegarde Big Black Records (tables clients/projets absentes).".into());
    }
    let version = user_version(&src)?;
    if version > schema_version() {
        return Err(format!(
            "Cette sauvegarde (version {version}) vient d'une version plus récente de l'application."
        ));
    }
    Ok(())
}

/// Remplace le contenu de la base ouverte par celui de `src`.
/// Une copie de sécurité de l'état actuel est faite d'abord dans `safety_dir`.
/// Retourne le chemin de cette copie de sécurité.
pub fn restore_from(conn: &mut Connection, src: &Path, safety_dir: &Path) -> Result<PathBuf, String> {
    check_backup_file(src)?;
    let stamp = sqlite_now(conn, "%Y-%m-%d-%H%M%S")?;
    let safety = safety_dir.join(format!("bigblack-avant-restauration-{stamp}.sqlite"));
    backup_to(conn, &safety)?;
    conn.restore(DatabaseName::Main, src, None::<fn(rusqlite::backup::Progress)>)
        .map_err(|e| format!("Échec de la restauration : {e}"))?;
    // Une sauvegarde ancienne peut avoir un schéma plus vieux : on la remet à niveau.
    migrate(conn)?;
    Ok(safety)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("bbr_db_test_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn mem_db() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn).unwrap();
        conn
    }

    fn count_clients(conn: &Connection) -> i64 {
        conn.query_row("SELECT COUNT(*) FROM clients", [], |r| r.get(0)).unwrap()
    }

    #[test]
    fn fresh_db_is_migrated_to_latest_version() {
        let conn = mem_db();
        assert_eq!(user_version(&conn).unwrap(), schema_version());
    }

    #[test]
    fn migrate_is_idempotent() {
        let mut conn = mem_db();
        migrate(&mut conn).unwrap();
        assert_eq!(user_version(&conn).unwrap(), schema_version());
    }

    #[test]
    fn legacy_unversioned_db_keeps_its_data() {
        // Base créée avant le versionnage : schéma appliqué, user_version = 0.
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(MIGRATIONS[0]).unwrap();
        conn.execute("INSERT INTO clients (nom) VALUES ('Ancien')", []).unwrap();
        assert_eq!(user_version(&conn).unwrap(), 0);
        migrate(&mut conn).unwrap();
        assert_eq!(user_version(&conn).unwrap(), schema_version());
        assert_eq!(count_clients(&conn), 1);
    }

    #[test]
    fn db_from_newer_app_is_refused() {
        let mut conn = mem_db();
        conn.pragma_update(None, "user_version", schema_version() + 1).unwrap();
        assert!(migrate(&mut conn).is_err());
    }

    #[test]
    fn backup_then_restore_roundtrip() {
        let dir = tmp_dir("roundtrip");
        let mut conn = mem_db();
        conn.execute("INSERT INTO clients (nom) VALUES ('Avant')", []).unwrap();
        let file = dir.join("save.sqlite");
        backup_to(&conn, &file).unwrap();

        conn.execute("INSERT INTO clients (nom) VALUES ('Après')", []).unwrap();
        assert_eq!(count_clients(&conn), 2);

        let safety = restore_from(&mut conn, &file, &dir).unwrap();
        assert_eq!(count_clients(&conn), 1);
        assert!(safety.exists(), "la copie de sécurité doit exister");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn restore_refuses_non_bbr_files() {
        let dir = tmp_dir("refuse");
        let other = dir.join("autre.sqlite");
        Connection::open(&other).unwrap()
            .execute_batch("CREATE TABLE truc (x INTEGER);").unwrap();
        let junk = dir.join("junk.sqlite");
        std::fs::write(&junk, b"pas une base").unwrap();

        let mut conn = mem_db();
        conn.execute("INSERT INTO clients (nom) VALUES ('Garde')", []).unwrap();
        assert!(restore_from(&mut conn, &other, &dir).is_err());
        assert!(restore_from(&mut conn, &junk, &dir).is_err());
        assert!(restore_from(&mut conn, &dir.join("absent.sqlite"), &dir).is_err());
        assert_eq!(count_clients(&conn), 1, "la base ne doit pas changer");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn auto_backup_once_per_day_and_pruned() {
        let dir = tmp_dir("auto");
        // Anciennes sauvegardes automatiques factices.
        for d in 1..=20 {
            std::fs::write(dir.join(format!("{AUTO_BACKUP_PREFIX}2000-01-{d:02}.sqlite")), b"x").unwrap();
        }
        std::fs::write(dir.join("manuelle.sqlite"), b"x").unwrap();

        let conn = mem_db();
        assert!(auto_backup(&conn, &dir).unwrap().is_some());
        assert!(auto_backup(&conn, &dir).unwrap().is_none(), "une seule par jour");

        let files = list_backups(&dir).unwrap();
        let autos = files.iter()
            .filter(|p| p.file_name().unwrap().to_str().unwrap().starts_with(AUTO_BACKUP_PREFIX))
            .count();
        assert_eq!(autos, AUTO_BACKUP_KEEP);
        assert!(dir.join("manuelle.sqlite").exists(), "les sauvegardes manuelles ne sont jamais supprimées");
        std::fs::remove_dir_all(&dir).ok();
    }
}
