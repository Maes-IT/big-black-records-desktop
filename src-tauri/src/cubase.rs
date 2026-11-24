//! Sessions Cubase : création du dossier « ARTISTE - TITRE », copie du projet modèle
//! renommé en « TITRE.cpr », puis ouverture dans Cubase (association de fichier Windows).
//!
//! Les deux chemins (modèle .cpr et dossier racine des sessions) sont stockés dans la
//! table `parametres` et modifiables depuis l'écran de réglages de l'application.

use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Component, Path, PathBuf};

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::db::DbState;

const KEY_TEMPLATE: &str = "cubase_template_path";
const KEY_ROOT: &str = "cubase_sessions_root";

/// Marge sous la limite historique MAX_PATH (260) de Windows, que Cubase ne dépasse pas toujours bien.
const MAX_PROJECT_PATH_LEN: usize = 240;

const RESERVED_NAMES: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CubaseSettings {
    pub template_path: String,
    pub sessions_root: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionPreview {
    pub dossier: String,
    pub fichier: String,
    pub chemin_projet: String,
    /// Le dossier de session existe déjà (le .cpr y sera ajouté).
    pub dossier_existe: bool,
    /// Le .cpr existe déjà : on ne l'écrase jamais, on propose de l'ouvrir.
    pub projet_existe: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionResult {
    pub chemin_projet: String,
    /// true = nouveau projet créé, false = projet existant ouvert.
    pub cree: bool,
}

// ==================== Noms de fichiers ====================

/// Rend un nom saisi utilisable comme nom de fichier/dossier Windows.
/// `/ \ : |` deviennent `-`, `"` devient `'`, `< > ? *` et les caractères de contrôle sont retirés.
pub fn sanitize_name(raw: &str, label: &str) -> Result<String, String> {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        match ch {
            '/' | '\\' | ':' | '|' => out.push('-'),
            '"' => out.push('\''),
            '<' | '>' | '?' | '*' => {}
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    let collapsed = out.split_whitespace().collect::<Vec<_>>().join(" ");
    // Windows ignore silencieusement les points et espaces finaux : on les retire nous-mêmes.
    let name = collapsed
        .trim_end_matches(|c: char| c == '.' || c.is_whitespace())
        .to_string();

    if name.is_empty() {
        return Err(format!(
            "{label} est vide ou ne contient que des caractères interdits (< > : \" / \\ | ? *)."
        ));
    }
    let base = name.split('.').next().unwrap_or("").trim().to_ascii_uppercase();
    if RESERVED_NAMES.contains(&base.as_str()) {
        return Err(format!("« {name} » est un nom réservé par Windows, choisissez-en un autre."));
    }
    Ok(name)
}

/// (nom du dossier, nom du fichier .cpr)
pub fn session_names(artiste: &str, titre: &str) -> Result<(String, String), String> {
    let artiste = sanitize_name(artiste, "Le nom de l'artiste / groupe")?;
    let titre = sanitize_name(titre, "Le titre du son")?;
    Ok((format!("{artiste} - {titre}"), format!("{titre}.cpr")))
}

// ==================== Chemins ====================

/// Lettre de lecteur (ou partage réseau) absente, typiquement un disque débranché.
fn missing_drive(p: &Path) -> Option<String> {
    if let Some(Component::Prefix(prefix)) = p.components().next() {
        let prefix = prefix.as_os_str().to_string_lossy().to_string();
        if !Path::new(&format!("{prefix}\\")).exists() {
            return Some(prefix);
        }
    }
    None
}

fn inaccessible(what: &str, p: &Path) -> String {
    match missing_drive(p) {
        Some(drive) => format!(
            "Le disque {drive} est introuvable ({what} : « {} »). Est-il bien branché ?",
            p.display()
        ),
        None => format!("{what} introuvable : « {} ».", p.display()),
    }
}

fn check_template(raw: &str) -> Result<PathBuf, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err("Le projet modèle n'est pas configuré. Ouvrez les réglages Cubase (⚙).".into());
    }
    let p = PathBuf::from(raw);
    let is_cpr = p
        .extension()
        .map(|e| e.eq_ignore_ascii_case("cpr"))
        .unwrap_or(false);
    if !is_cpr {
        return Err(format!("Le modèle doit être un projet Cubase (.cpr) : « {} ».", p.display()));
    }
    if !p.is_file() {
        return Err(inaccessible("Projet modèle", &p));
    }
    Ok(p)
}

fn check_root(raw: &str) -> Result<PathBuf, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err(
            "Le dossier des sessions n'est pas configuré. Ouvrez les réglages Cubase (⚙).".into(),
        );
    }
    let p = PathBuf::from(raw);
    if !p.is_dir() {
        return Err(inaccessible("Dossier des sessions", &p));
    }
    Ok(p)
}

struct SessionPaths {
    dossier: String,
    fichier: String,
    dir: PathBuf,
    cpr: PathBuf,
}

fn session_paths(root: &Path, artiste: &str, titre: &str) -> Result<SessionPaths, String> {
    let (dossier, fichier) = session_names(artiste, titre)?;
    let dir = root.join(&dossier);
    let cpr = dir.join(&fichier);
    if cpr.as_os_str().len() > MAX_PROJECT_PATH_LEN {
        return Err(format!(
            "Chemin trop long ({} caractères, max {MAX_PROJECT_PATH_LEN}). Raccourcissez le nom ou le titre.",
            cpr.as_os_str().len()
        ));
    }
    Ok(SessionPaths { dossier, fichier, dir, cpr })
}

/// Copie sans jamais écraser une cible existante (create_new est atomique).
fn copy_no_overwrite(src: &Path, dst: &Path) -> io::Result<()> {
    let mut input = File::open(src)?;
    let mut output = OpenOptions::new().write(true).create_new(true).open(dst)?;
    let res = io::copy(&mut input, &mut output).and_then(|_| output.sync_all());
    if res.is_err() {
        drop(output);
        let _ = fs::remove_file(dst);
    }
    res
}

// ==================== Implémentations testables ====================

pub fn read_settings(conn: &Connection) -> Result<CubaseSettings, String> {
    let get = |key: &str| -> Result<String, String> {
        conn.query_row(
            "SELECT valeur FROM parametres WHERE cle = ?1",
            params![key],
            |r| r.get::<_, String>(0),
        )
        .optional()
        .map(|v| v.unwrap_or_default())
        .map_err(|e| e.to_string())
    };
    Ok(CubaseSettings {
        template_path: get(KEY_TEMPLATE)?,
        sessions_root: get(KEY_ROOT)?,
    })
}

pub fn save_settings_impl(conn: &Connection, s: &CubaseSettings) -> Result<CubaseSettings, String> {
    let template = check_template(&s.template_path)?;
    let root = check_root(&s.sessions_root)?;
    let clean = CubaseSettings {
        template_path: template.to_string_lossy().to_string(),
        sessions_root: root.to_string_lossy().to_string(),
    };
    let sql = "INSERT INTO parametres (cle, valeur) VALUES (?1, ?2)
               ON CONFLICT(cle) DO UPDATE SET valeur = excluded.valeur";
    conn.execute(sql, params![KEY_TEMPLATE, clean.template_path])
        .map_err(|e| e.to_string())?;
    conn.execute(sql, params![KEY_ROOT, clean.sessions_root])
        .map_err(|e| e.to_string())?;
    Ok(clean)
}

pub fn preview_impl(s: &CubaseSettings, artiste: &str, titre: &str) -> Result<SessionPreview, String> {
    let root = check_root(&s.sessions_root)?;
    check_template(&s.template_path)?;
    let p = session_paths(&root, artiste, titre)?;
    Ok(SessionPreview {
        dossier_existe: p.dir.is_dir(),
        projet_existe: p.cpr.exists(),
        chemin_projet: p.cpr.to_string_lossy().to_string(),
        dossier: p.dossier,
        fichier: p.fichier,
    })
}

/// Crée la session et renvoie (chemin du .cpr, créé ?). Si le .cpr existe déjà :
/// erreur, sauf si `ouvrir_si_existe` (on renverra alors le chemin existant à ouvrir).
pub fn create_session_impl(
    s: &CubaseSettings,
    artiste: &str,
    titre: &str,
    ouvrir_si_existe: bool,
) -> Result<(PathBuf, bool), String> {
    let root = check_root(&s.sessions_root)?;
    let template = check_template(&s.template_path)?;
    let p = session_paths(&root, artiste, titre)?;

    if p.cpr.exists() {
        return if ouvrir_si_existe {
            Ok((p.cpr, false))
        } else {
            Err(format!("Le projet « {} » existe déjà.", p.cpr.display()))
        };
    }

    let dir_created = !p.dir.exists();
    fs::create_dir_all(&p.dir)
        .map_err(|e| format!("Impossible de créer le dossier « {} » : {e}", p.dir.display()))?;

    if let Err(e) = copy_no_overwrite(&template, &p.cpr) {
        if dir_created {
            let _ = fs::remove_dir(&p.dir); // ne supprime que si vide
        }
        return Err(format!("Impossible de copier le modèle vers « {} » : {e}", p.cpr.display()));
    }
    Ok((p.cpr, true))
}

fn settings_from_state(state: &State<DbState>) -> Result<CubaseSettings, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    read_settings(&conn)
}

fn open_in_cubase(app: &AppHandle, cpr: &Path) -> Result<(), String> {
    app.opener()
        .open_path(cpr.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| {
            format!(
                "Projet prêt (« {} ») mais impossible de l'ouvrir : {e}. Vérifiez que les fichiers .cpr s'ouvrent avec Cubase.",
                cpr.display()
            )
        })
}

// ==================== Commandes Tauri ====================

#[tauri::command]
pub fn get_cubase_settings(state: State<DbState>) -> Result<CubaseSettings, String> {
    settings_from_state(&state)
}

#[tauri::command]
pub fn save_cubase_settings(
    state: State<DbState>,
    settings: CubaseSettings,
) -> Result<CubaseSettings, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    save_settings_impl(&conn, &settings)
}

#[tauri::command]
pub fn preview_cubase_session(
    state: State<DbState>,
    artiste: String,
    titre: String,
) -> Result<SessionPreview, String> {
    let s = settings_from_state(&state)?;
    preview_impl(&s, &artiste, &titre)
}

#[tauri::command]
pub fn create_cubase_session(
    app: AppHandle,
    state: State<DbState>,
    artiste: String,
    titre: String,
    ouvrir_si_existe: bool,
) -> Result<SessionResult, String> {
    let s = settings_from_state(&state)?; // verrou DB relâché avant les accès disque
    let (cpr, cree) = create_session_impl(&s, &artiste, &titre, ouvrir_si_existe)?;
    open_in_cubase(&app, &cpr)?;
    Ok(SessionResult {
        chemin_projet: cpr.to_string_lossy().to_string(),
        cree,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempDir(PathBuf);
    impl TempDir {
        fn new(tag: &str) -> Self {
            let p = std::env::temp_dir().join(format!(
                "bbr_cubase_{tag}_{}_{:?}",
                std::process::id(),
                std::thread::current().id()
            ));
            let _ = fs::remove_dir_all(&p);
            fs::create_dir_all(&p).unwrap();
            TempDir(p)
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// Modèle « RECORDING PRO.cpr » + dossier racine des sessions.
    fn setup(tag: &str) -> (TempDir, CubaseSettings) {
        let tmp = TempDir::new(tag);
        let tpl_dir = tmp.0.join("Modeles");
        let root = tmp.0.join("Sessions");
        fs::create_dir_all(&tpl_dir).unwrap();
        fs::create_dir_all(&root).unwrap();
        let tpl = tpl_dir.join("RECORDING PRO.cpr");
        fs::write(&tpl, b"CUBASE-TEMPLATE").unwrap();
        let s = CubaseSettings {
            template_path: tpl.to_string_lossy().to_string(),
            sessions_root: root.to_string_lossy().to_string(),
        };
        (tmp, s)
    }

    fn test_conn() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::db::migrate(&mut conn).unwrap();
        conn
    }

    // ---- noms ----

    #[test]
    fn names_follow_artist_dash_title_format() {
        let (d, f) = session_names("KST", "I Love You").unwrap();
        assert_eq!(d, "KST - I Love You");
        assert_eq!(f, "I Love You.cpr");
    }

    #[test]
    fn forbidden_characters_are_replaced_or_removed() {
        assert_eq!(sanitize_name("AC/DC", "x").unwrap(), "AC-DC");
        assert_eq!(sanitize_name("Why?", "x").unwrap(), "Why");
        assert_eq!(sanitize_name("Feat: \"Zaza\"", "x").unwrap(), "Feat- 'Zaza'");
        assert_eq!(sanitize_name("  a <b>  c*  ", "x").unwrap(), "a b c");
        assert_eq!(sanitize_name("Vol. 2...", "x").unwrap(), "Vol. 2");
    }

    #[test]
    fn empty_or_only_forbidden_is_rejected() {
        assert!(sanitize_name("   ", "Le titre").is_err());
        assert!(sanitize_name("???", "Le titre").is_err());
    }

    #[test]
    fn windows_reserved_names_are_rejected() {
        assert!(sanitize_name("CON", "x").is_err());
        assert!(sanitize_name("nul", "x").is_err());
        assert!(sanitize_name("Com1.txt", "x").is_err());
        assert!(sanitize_name("Console", "x").is_ok());
    }

    // ---- création ----

    #[test]
    fn creates_folder_and_copies_renamed_template() {
        let (_tmp, s) = setup("create");
        let (cpr, cree) = create_session_impl(&s, "KST", "I Love You", false).unwrap();
        assert!(cree);
        assert_eq!(
            cpr,
            Path::new(&s.sessions_root).join("KST - I Love You").join("I Love You.cpr")
        );
        assert_eq!(fs::read(&cpr).unwrap(), b"CUBASE-TEMPLATE");
        // le modèle d'origine est intact
        assert!(Path::new(&s.template_path).is_file());
    }

    #[test]
    fn existing_project_is_never_overwritten() {
        let (_tmp, s) = setup("exists");
        let (cpr, _) = create_session_impl(&s, "KST", "Titre", false).unwrap();
        fs::write(&cpr, b"TRAVAIL EN COURS").unwrap();

        let err = create_session_impl(&s, "KST", "Titre", false).unwrap_err();
        assert!(err.contains("existe déjà"), "{err}");
        assert_eq!(fs::read(&cpr).unwrap(), b"TRAVAIL EN COURS");

        let (again, cree) = create_session_impl(&s, "KST", "Titre", true).unwrap();
        assert!(!cree);
        assert_eq!(again, cpr);
        assert_eq!(fs::read(&cpr).unwrap(), b"TRAVAIL EN COURS");
    }

    #[test]
    fn existing_folder_without_project_receives_the_cpr() {
        let (_tmp, s) = setup("folder");
        let dir = Path::new(&s.sessions_root).join("KST - Demo");
        fs::create_dir_all(dir.join("Audio")).unwrap();
        let (cpr, cree) = create_session_impl(&s, "KST", "Demo", false).unwrap();
        assert!(cree);
        assert!(cpr.is_file());
        assert!(dir.join("Audio").is_dir());
    }

    #[test]
    fn missing_template_is_reported_and_nothing_is_created() {
        let (_tmp, mut s) = setup("notpl");
        s.template_path = Path::new(&s.template_path)
            .with_file_name("ABSENT.cpr")
            .to_string_lossy()
            .to_string();
        let err = create_session_impl(&s, "KST", "X", false).unwrap_err();
        assert!(err.contains("introuvable"), "{err}");
        assert!(!Path::new(&s.sessions_root).join("KST - X").exists());
    }

    #[test]
    fn missing_root_is_reported() {
        let (_tmp, mut s) = setup("noroot");
        s.sessions_root = Path::new(&s.sessions_root)
            .join("absent")
            .to_string_lossy()
            .to_string();
        let err = create_session_impl(&s, "KST", "X", false).unwrap_err();
        assert!(err.contains("Dossier des sessions"), "{err}");
    }

    #[test]
    fn unplugged_drive_is_named_in_error() {
        // Lettre très improbable sur une machine de test.
        let err = check_root("Q:\\Sessions").unwrap_err();
        assert!(err.contains("disque Q:"), "{err}");
    }

    #[test]
    fn unconfigured_paths_point_to_settings() {
        let err = create_session_impl(&CubaseSettings::default(), "KST", "X", false).unwrap_err();
        assert!(err.contains("réglages"), "{err}");
    }

    #[test]
    fn preview_reports_existence() {
        let (_tmp, s) = setup("preview");
        let p = preview_impl(&s, "KST", "I Love You").unwrap();
        assert_eq!(p.dossier, "KST - I Love You");
        assert!(!p.dossier_existe && !p.projet_existe);
        create_session_impl(&s, "KST", "I Love You", false).unwrap();
        let p = preview_impl(&s, "KST", "I Love You").unwrap();
        assert!(p.dossier_existe && p.projet_existe);
    }

    // ---- réglages ----

    #[test]
    fn settings_roundtrip_and_default_empty() {
        let conn = test_conn();
        assert_eq!(read_settings(&conn).unwrap(), CubaseSettings::default());
        let (_tmp, s) = setup("settings");
        save_settings_impl(&conn, &s).unwrap();
        assert_eq!(read_settings(&conn).unwrap(), s);
        save_settings_impl(&conn, &s).unwrap(); // upsert
        assert_eq!(read_settings(&conn).unwrap(), s);
    }

    #[test]
    fn settings_reject_non_cpr_template() {
        let conn = test_conn();
        let (tmp, mut s) = setup("noncpr");
        let txt = tmp.0.join("modele.txt");
        fs::write(&txt, b"x").unwrap();
        s.template_path = txt.to_string_lossy().to_string();
        assert!(save_settings_impl(&conn, &s).is_err());
        assert_eq!(read_settings(&conn).unwrap(), CubaseSettings::default());
    }
}
