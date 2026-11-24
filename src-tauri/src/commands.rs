use rusqlite::{named_params, params, Connection, Row};
use std::collections::HashMap;
use tauri::State;

use crate::db::{DataDir, DbState};
use crate::models::{
    BackupInfo, Client, ClientProjetCree, DashboardStats, DettesComparaison, FiltreCriteria, FiltreResult, ImpayeItem,
    NewClient, NewProjet, Projet, QuartierCount, RestoreResult,
};

fn row_to_client(row: &Row) -> rusqlite::Result<Client> {
    Ok(Client {
        id: row.get(0)?,
        nom: row.get(1)?,
        responsable: row.get(2)?,
        telephone: row.get(3)?,
        quartier: row.get(4)?,
        observation: row.get(5)?,
    })
}

fn row_to_projet(row: &Row) -> rusqlite::Result<Projet> {
    Ok(Projet {
        id: row.get(0)?,
        client_id: row.get(1)?,
        titre: row.get(2)?,
        genre: row.get(3)?,
        statut: row.get(4)?,
        date_creation: row.get(5)?,
        date_livraison: row.get(6)?,
        avance: row.get(7)?,
        cachet: row.get(8)?,
        statut_paiement: row.get(9)?,
        solde: row.get(10)?,
    })
}

fn row_to_filtre_result(row: &Row) -> rusqlite::Result<FiltreResult> {
    Ok(FiltreResult {
        id: row.get(0)?,
        client_id: row.get(1)?,
        client_nom: row.get(2)?,
        client_telephone: row.get(3)?,
        client_quartier: row.get(4)?,
        titre: row.get(5)?,
        genre: row.get(6)?,
        statut: row.get(7)?,
        date_creation: row.get(8)?,
        date_livraison: row.get(9)?,
        avance: row.get(10)?,
        cachet: row.get(11)?,
        statut_paiement: row.get(12)?,
        solde: row.get(13)?,
        reste: row.get(14)?,
    })
}

const CLIENT_COLUMNS: &str = "id, nom, responsable, telephone, quartier, observation";
const PROJET_COLUMNS: &str =
    "id, client_id, titre, genre, statut, date_creation, date_livraison, avance, cachet, statut_paiement, solde";

const FILTER_SQL: &str = "
    SELECT id, client_id, client_nom, client_telephone, client_quartier, titre, genre, statut,
           date_creation, date_livraison, avance, cachet, statut_paiement, solde, reste
    FROM projets_view
    WHERE (:nom_groupe IS NULL OR client_nom LIKE '%' || :nom_groupe || '%')
      AND (:nom_projet IS NULL OR titre LIKE '%' || :nom_projet || '%')
      AND (:telephone IS NULL OR REPLACE(client_telephone, ' ', '') LIKE '%' || REPLACE(:telephone, ' ', '') || '%')
      AND (:quartier IS NULL OR client_quartier = :quartier OR client_quartier LIKE :quartier || ',%')
      AND (:etat IS NULL OR statut = :etat)
      AND (:creation_from IS NULL OR date_creation >= :creation_from)
      AND (:creation_to IS NULL OR date_creation <= :creation_to)
      AND (:livraison_from IS NULL OR date_livraison >= :livraison_from)
      AND (:livraison_to IS NULL OR date_livraison <= :livraison_to)
      AND (:avance_min IS NULL OR avance >= :avance_min)
      AND (:avance_max IS NULL OR avance <= :avance_max)
      AND (:reste_min IS NULL OR reste >= :reste_min)
      AND (:reste_max IS NULL OR reste <= :reste_max)
    ORDER BY id
";

fn validate_projet_fields(
    titre: &str,
    date_creation: &Option<String>,
    date_livraison: &Option<String>,
    avance: i64,
    cachet: i64,
) -> Result<(), String> {
    if titre.trim().is_empty() {
        return Err("Le titre du projet est obligatoire.".into());
    }
    if let (Some(dc), Some(dl)) = (date_creation, date_livraison) {
        if dl < dc {
            return Err(
                "La date de livraison ne peut pas être antérieure à la date de création.".into(),
            );
        }
    }
    if avance > cachet {
        return Err("L'avance ne peut pas dépasser le cachet.".into());
    }
    Ok(())
}

// ==================== Implémentations testables (Connection nue, sans State<DbState>) ====================

fn list_clients_impl(conn: &Connection) -> Result<Vec<Client>, String> {
    let mut stmt = conn
        .prepare(&format!("SELECT {CLIENT_COLUMNS} FROM clients ORDER BY nom"))
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], row_to_client)
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

fn get_client_impl(conn: &Connection, id: i64) -> Result<Client, String> {
    conn.query_row(
        &format!("SELECT {CLIENT_COLUMNS} FROM clients WHERE id = ?1"),
        params![id],
        row_to_client,
    )
    .map_err(|e| e.to_string())
}

fn create_client_impl(conn: &Connection, client: &NewClient) -> Result<Client, String> {
    if client.nom.trim().is_empty() {
        return Err("Le nom du groupe est obligatoire.".into());
    }
    conn.execute(
        "INSERT INTO clients (nom, responsable, telephone, quartier, observation)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            client.nom,
            client.responsable,
            client.telephone,
            client.quartier,
            client.observation
        ],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    get_client_impl(conn, id)
}

fn update_client_impl(conn: &Connection, client: &Client) -> Result<(), String> {
    if client.nom.trim().is_empty() {
        return Err("Le nom du groupe est obligatoire.".into());
    }
    conn.execute(
        "UPDATE clients SET nom = ?1, responsable = ?2, telephone = ?3, quartier = ?4, observation = ?5
         WHERE id = ?6",
        params![
            client.nom,
            client.responsable,
            client.telephone,
            client.quartier,
            client.observation,
            client.id
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn delete_client_impl(conn: &Connection, id: i64) -> Result<(), String> {
    conn.execute("DELETE FROM clients WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn list_projets_impl(conn: &Connection, client_id: i64) -> Result<Vec<Projet>, String> {
    let mut stmt = conn
        .prepare(&format!(
            "SELECT {PROJET_COLUMNS} FROM projets WHERE client_id = ?1 ORDER BY id"
        ))
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(params![client_id], row_to_projet)
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// Tous les projets en une requête (évite un appel par client au démarrage).
fn list_all_projets_impl(conn: &Connection) -> Result<Vec<Projet>, String> {
    let mut stmt = conn
        .prepare(&format!("SELECT {PROJET_COLUMNS} FROM projets ORDER BY client_id, id"))
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], row_to_projet).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

fn get_projet_impl(conn: &Connection, id: i64) -> Result<Projet, String> {
    conn.query_row(
        &format!("SELECT {PROJET_COLUMNS} FROM projets WHERE id = ?1"),
        params![id],
        row_to_projet,
    )
    .map_err(|e| e.to_string())
}

fn create_projet_impl(conn: &Connection, projet: &NewProjet) -> Result<Projet, String> {
    validate_projet_fields(
        &projet.titre,
        &projet.date_creation,
        &projet.date_livraison,
        projet.avance,
        projet.cachet,
    )?;
    conn.execute(
        "INSERT INTO projets
            (client_id, titre, genre, statut, date_creation, date_livraison, avance, cachet, statut_paiement, solde)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            projet.client_id,
            projet.titre,
            projet.genre,
            projet.statut,
            projet.date_creation,
            projet.date_livraison,
            projet.avance,
            projet.cachet,
            projet.statut_paiement,
            projet.solde,
        ],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    get_projet_impl(conn, id)
}

fn update_projet_impl(conn: &Connection, projet: &Projet) -> Result<(), String> {
    validate_projet_fields(
        &projet.titre,
        &projet.date_creation,
        &projet.date_livraison,
        projet.avance,
        projet.cachet,
    )?;
    conn.execute(
        "UPDATE projets SET titre = ?1, genre = ?2, statut = ?3, date_creation = ?4, date_livraison = ?5,
            avance = ?6, cachet = ?7, statut_paiement = ?8, solde = ?9
         WHERE id = ?10",
        params![
            projet.titre,
            projet.genre,
            projet.statut,
            projet.date_creation,
            projet.date_livraison,
            projet.avance,
            projet.cachet,
            projet.statut_paiement,
            projet.solde,
            projet.id,
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn delete_projet_impl(conn: &Connection, id: i64) -> Result<(), String> {
    conn.execute("DELETE FROM projets WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

fn filter_projets_impl(conn: &Connection, criteria: &FiltreCriteria) -> Result<Vec<FiltreResult>, String> {
    let mut stmt = conn.prepare(FILTER_SQL).map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(
            named_params! {
                ":nom_groupe": criteria.nom_groupe,
                ":nom_projet": criteria.nom_projet,
                ":telephone": criteria.telephone,
                ":quartier": criteria.quartier,
                ":etat": criteria.etat,
                ":creation_from": criteria.creation_from,
                ":creation_to": criteria.creation_to,
                ":livraison_from": criteria.livraison_from,
                ":livraison_to": criteria.livraison_to,
                ":avance_min": criteria.avance_min,
                ":avance_max": criteria.avance_max,
                ":reste_min": criteria.reste_min,
                ":reste_max": criteria.reste_max,
            },
            row_to_filtre_result,
        )
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

fn get_dashboard_stats_impl(conn: &Connection) -> Result<DashboardStats, String> {
    let clients_total: i64 = conn
        .query_row("SELECT COUNT(*) FROM clients", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;

    let quartier_values: Vec<String> = {
        let mut stmt = conn
            .prepare("SELECT quartier FROM clients")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?
    };
    let mut counts: HashMap<String, i64> = HashMap::new();
    for q in quartier_values {
        let key = q.split(',').next().unwrap_or("").trim().to_string();
        *counts.entry(key).or_insert(0) += 1;
    }
    let mut quartiers: Vec<QuartierCount> = counts
        .into_iter()
        .map(|(quartier, count)| QuartierCount { quartier, count })
        .collect();
    quartiers.sort_by(|a, b| a.quartier.cmp(&b.quartier));

    let (projets_en_cours, projets_attente, projets_termine): (i64, i64, i64) = conn
        .query_row(
            "SELECT
                COALESCE(SUM(CASE WHEN statut='en-cours' THEN 1 ELSE 0 END),0),
                COALESCE(SUM(CASE WHEN statut='attente' THEN 1 ELSE 0 END),0),
                COALESCE(SUM(CASE WHEN statut='termine' THEN 1 ELSE 0 END),0)
             FROM projets",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .map_err(|e| e.to_string())?;

    let (payes_count, payes_total, impayes_count, impayes_total): (i64, i64, i64, i64) = conn
        .query_row(
            "SELECT
                COALESCE(SUM(CASE WHEN statut_paiement='paye' THEN 1 ELSE 0 END),0),
                COALESCE(SUM(CASE WHEN statut_paiement='paye' THEN cachet ELSE 0 END),0),
                COALESCE(SUM(CASE WHEN statut_paiement='impaye' THEN 1 ELSE 0 END),0),
                COALESCE(SUM(CASE WHEN statut_paiement='impaye' THEN reste ELSE 0 END),0)
             FROM projets_view",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .map_err(|e| e.to_string())?;

    let impayes: Vec<ImpayeItem> = {
        let mut stmt = conn
            .prepare(
                // `reste` (et non `solde`) : un impayé sans solde saisi doit
                // compter cachet - avance, comme dans le filtre.
                "SELECT client_nom, titre, genre, reste FROM projets_view
                 WHERE statut_paiement = 'impaye' ORDER BY id",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok(ImpayeItem {
                    client_nom: r.get(0)?,
                    titre: r.get(1)?,
                    genre: r.get(2)?,
                    solde: r.get::<_, Option<i64>>(3)?.unwrap_or(0),
                })
            })
            .map_err(|e| e.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?
    };

    Ok(DashboardStats {
        quartiers,
        clients_total,
        projets_en_cours,
        projets_attente,
        projets_termine,
        payes_count,
        payes_total,
        impayes_count,
        impayes_total,
        impayes,
    })
}

/// Années proposées dans les onglets des graphiques : celles qui ont des projets,
/// plus l'année en cours, triées par ordre croissant.
fn get_chart_years_impl(conn: &Connection) -> Result<Vec<i64>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT CAST(strftime('%Y', date_creation) AS INTEGER) FROM projets
             WHERE strftime('%Y', date_creation) IS NOT NULL
             UNION
             SELECT CAST(strftime('%Y', 'now', 'localtime') AS INTEGER)
             ORDER BY 1",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |r| r.get::<_, i64>(0)).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

fn get_chart_data_impl(conn: &Connection, annee: i64, kind: &str) -> Result<Vec<i64>, String> {
    let sql = match kind {
        "ventes" => {
            "SELECT strftime('%m', date_creation) AS mois, COUNT(*)
             FROM projets WHERE strftime('%Y', date_creation) = ?1 GROUP BY mois"
        }
        "ca" => {
            "SELECT strftime('%m', date_creation) AS mois, COALESCE(SUM(cachet),0)
             FROM projets WHERE strftime('%Y', date_creation) = ?1 GROUP BY mois"
        }
        _ => return Err(format!("kind invalide: {kind} (attendu 'ventes' ou 'ca')")),
    };
    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let mut values = [0i64; 12];
    let rows = stmt
        .query_map(params![annee.to_string()], |row| {
            let mois: String = row.get(0)?;
            let val: i64 = row.get(1)?;
            Ok((mois, val))
        })
        .map_err(|e| e.to_string())?;
    for r in rows {
        let (mois, val) = r.map_err(|e| e.to_string())?;
        if let Ok(idx) = mois.parse::<usize>() {
            if (1..=12).contains(&idx) {
                values[idx - 1] = val;
            }
        }
    }
    Ok(values.to_vec())
}

// Hypothèse (voir Phase 0 du plan) : la comparaison des dettes se base sur date_creation,
// période courante vs période précédente. "semaine" utilise une fenêtre glissante de 7 jours
// (pas une semaine calendaire ISO) pour rester simple.
fn get_dettes_comparaison_impl(conn: &Connection, periode: &str) -> Result<DettesComparaison, String> {
    let (current_sql, previous_sql): (&str, &str) = match periode {
        "mois" => (
            "SELECT COALESCE(SUM(reste),0) FROM projets_view
             WHERE strftime('%Y-%m', date_creation) = strftime('%Y-%m','now')",
            "SELECT COALESCE(SUM(reste),0) FROM projets_view
             WHERE strftime('%Y-%m', date_creation) = strftime('%Y-%m','now','-1 month')",
        ),
        "semaine" => (
            "SELECT COALESCE(SUM(reste),0) FROM projets_view
             WHERE date_creation >= date('now','-6 days') AND date_creation <= date('now')",
            "SELECT COALESCE(SUM(reste),0) FROM projets_view
             WHERE date_creation >= date('now','-13 days') AND date_creation <= date('now','-7 days')",
        ),
        "an" => (
            "SELECT COALESCE(SUM(reste),0) FROM projets_view
             WHERE strftime('%Y', date_creation) = strftime('%Y','now')",
            "SELECT COALESCE(SUM(reste),0) FROM projets_view
             WHERE strftime('%Y', date_creation) = strftime('%Y','now','-1 year')",
        ),
        _ => return Err(format!("periode invalide: {periode} (attendu 'mois', 'semaine' ou 'an')")),
    };
    let current: i64 = conn
        .query_row(current_sql, [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let previous: i64 = conn
        .query_row(previous_sql, [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    Ok(DettesComparaison { a: previous, b: current })
}

// ==================== Commandes Tauri (fines : verrou + délégation) ====================

#[tauri::command]
pub fn list_clients(state: State<DbState>) -> Result<Vec<Client>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    list_clients_impl(&conn)
}

#[tauri::command]
pub fn get_client(state: State<DbState>, id: i64) -> Result<Client, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    get_client_impl(&conn, id)
}

#[tauri::command]
pub fn create_client(state: State<DbState>, client: NewClient) -> Result<Client, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    create_client_impl(&conn, &client)
}

#[tauri::command]
pub fn update_client(state: State<DbState>, client: Client) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    update_client_impl(&conn, &client)
}

#[tauri::command]
pub fn delete_client(state: State<DbState>, id: i64) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    delete_client_impl(&conn, id)
}

#[tauri::command]
pub fn list_all_projets(state: State<DbState>) -> Result<Vec<Projet>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    list_all_projets_impl(&conn)
}

#[tauri::command]
pub fn get_chart_years(state: State<DbState>) -> Result<Vec<i64>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    get_chart_years_impl(&conn)
}

#[tauri::command]
pub fn list_projets(state: State<DbState>, client_id: i64) -> Result<Vec<Projet>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    list_projets_impl(&conn, client_id)
}

#[tauri::command]
pub fn create_projet(state: State<DbState>, projet: NewProjet) -> Result<Projet, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    create_projet_impl(&conn, &projet)
}

#[tauri::command]
pub fn update_projet(state: State<DbState>, projet: Projet) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    update_projet_impl(&conn, &projet)
}

#[tauri::command]
pub fn delete_projet(state: State<DbState>, id: i64) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    delete_projet_impl(&conn, id)
}

#[tauri::command]
pub fn filter_projets(state: State<DbState>, criteria: FiltreCriteria) -> Result<Vec<FiltreResult>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    filter_projets_impl(&conn, &criteria)
}

#[tauri::command]
pub fn get_dashboard_stats(state: State<DbState>) -> Result<DashboardStats, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    get_dashboard_stats_impl(&conn)
}

#[tauri::command]
pub fn get_chart_data(state: State<DbState>, annee: i64, kind: String) -> Result<Vec<i64>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    get_chart_data_impl(&conn, annee, &kind)
}

#[tauri::command]
pub fn get_dettes_comparaison(state: State<DbState>, periode: String) -> Result<DettesComparaison, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    get_dettes_comparaison_impl(&conn, &periode)
}

/// N'accepte que les exports de l'application (PDF / XLSX) vers un chemin absolu :
/// la page ne peut pas s'en servir pour écraser un fichier quelconque.
fn check_export_path(path: &str) -> Result<(), String> {
    let p = std::path::Path::new(path);
    if !p.is_absolute() {
        return Err("Chemin d'export invalide.".into());
    }
    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase());
    match ext.as_deref() {
        Some("pdf") | Some("xlsx") => Ok(()),
        _ => Err("Seuls les exports .pdf et .xlsx sont autorisés.".into()),
    }
}

#[tauri::command]
pub fn save_binary_file(path: String, data: Vec<u8>) -> Result<(), String> {
    check_export_path(&path)?;
    std::fs::write(&path, data).map_err(|e| e.to_string())
}

// ==================== Sauvegarde / restauration de la base ====================

fn check_backup_path(path: &str) -> Result<(), String> {
    let p = std::path::Path::new(path);
    if !p.is_absolute() {
        return Err("Chemin de sauvegarde invalide.".into());
    }
    match p.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref() {
        Some("sqlite") => Ok(()),
        _ => Err("Une sauvegarde doit avoir l'extension .sqlite.".into()),
    }
}

#[tauri::command]
pub fn get_backup_info(data_dir: State<DataDir>) -> Result<BackupInfo, String> {
    let dir = crate::db::backup_dir(&data_dir.0);
    let files = crate::db::list_backups(&dir)?
        .into_iter()
        .filter_map(|p| p.file_name().and_then(|n| n.to_str()).map(String::from))
        .collect();
    Ok(BackupInfo { dir: dir.display().to_string(), files })
}

#[tauri::command]
pub fn backup_db(state: State<DbState>, path: String) -> Result<(), String> {
    check_backup_path(&path)?;
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    crate::db::backup_to(&conn, std::path::Path::new(&path))
}

#[tauri::command]
pub fn restore_db(state: State<DbState>, data_dir: State<DataDir>, path: String) -> Result<RestoreResult, String> {
    check_backup_path(&path)?;
    let mut conn = state.0.lock().map_err(|e| e.to_string())?;
    let safety = crate::db::restore_from(
        &mut conn,
        std::path::Path::new(&path),
        &crate::db::backup_dir(&data_dir.0),
    )?;
    Ok(RestoreResult { safety_backup: safety.display().to_string() })
}

// ==================== Création combinée client + projet ====================

/// Client existant dont le nom correspond (sans tenir compte des espaces autour ni de la casse).
fn find_client_by_nom_impl(conn: &Connection, nom: &str) -> Result<Option<Client>, String> {
    let wanted = nom.trim().to_lowercase();
    if wanted.is_empty() {
        return Ok(None);
    }
    Ok(list_clients_impl(conn)?
        .into_iter()
        .filter(|c| c.nom.trim().to_lowercase() == wanted)
        .min_by_key(|c| c.id))
}

/// Titre libre pour ce client : « Titre », sinon « Titre v2 », « Titre v3 »…
fn titre_disponible(conn: &Connection, client_id: i64, titre: &str) -> Result<String, String> {
    let base = titre.trim();
    let mut stmt = conn
        .prepare("SELECT titre FROM projets WHERE client_id = ?1")
        .map_err(|e| e.to_string())?;
    let existants: std::collections::HashSet<String> = stmt
        .query_map(params![client_id], |r| r.get::<_, String>(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|t| t.trim().to_lowercase())
        .collect();
    if !existants.contains(&base.to_lowercase()) {
        return Ok(base.to_string());
    }
    let mut n = 2;
    loop {
        let candidat = format!("{base} v{n}");
        if !existants.contains(&candidat.to_lowercase()) {
            return Ok(candidat);
        }
        n += 1;
    }
}

/// Crée le client (ou réutilise `rattacher_a`), puis son projet minimal, dans une seule
/// transaction : si la création du projet échoue, le client n'est pas créé non plus.
/// Projet minimal : statut « en-cours », impayé, avance/cachet à 0, créé aujourd'hui (heure locale).
fn create_client_avec_projet_impl(
    conn: &mut Connection,
    client: &NewClient,
    titre: &str,
    rattacher_a: Option<i64>,
) -> Result<ClientProjetCree, String> {
    if titre.trim().is_empty() {
        return Err("Le titre du projet est obligatoire.".into());
    }
    let tx = conn.transaction().map_err(|e| e.to_string())?;

    let (client_row, client_cree) = match rattacher_a {
        Some(id) => (get_client_impl(&tx, id)?, false),
        None => (create_client_impl(&tx, client)?, true),
    };

    let titre_final = titre_disponible(&tx, client_row.id, titre)?;
    let aujourd_hui: String = tx
        .query_row("SELECT date('now','localtime')", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let projet = create_projet_impl(
        &tx,
        &NewProjet {
            client_id: client_row.id,
            titre: titre_final.clone(),
            genre: String::new(),
            statut: "en-cours".into(),
            date_creation: Some(aujourd_hui),
            date_livraison: None,
            avance: 0,
            cachet: 0,
            statut_paiement: "impaye".into(),
            solde: None,
        },
    )?;

    tx.commit().map_err(|e| e.to_string())?;
    Ok(ClientProjetCree {
        titre_renomme: titre_final != titre.trim(),
        client: client_row,
        projet,
        client_cree,
    })
}

#[tauri::command]
pub fn find_client_by_nom(state: State<DbState>, nom: String) -> Result<Option<Client>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    find_client_by_nom_impl(&conn, &nom)
}

#[tauri::command]
pub fn create_client_avec_projet(
    state: State<DbState>,
    client: NewClient,
    titre: String,
    rattacher_a: Option<i64>,
) -> Result<ClientProjetCree, String> {
    let mut conn = state.0.lock().map_err(|e| e.to_string())?;
    create_client_avec_projet_impl(&mut conn, &client, &titre, rattacher_a)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_conn() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::db::migrate(&mut conn).unwrap();
        conn
    }

    fn new_client(nom: &str, quartier: &str) -> NewClient {
        NewClient {
            nom: nom.into(),
            responsable: "Resp".into(),
            telephone: "034 00 000 00".into(),
            quartier: quartier.into(),
            observation: "".into(),
        }
    }

    fn new_projet(
        client_id: i64,
        titre: &str,
        statut: &str,
        date_creation: Option<&str>,
        avance: i64,
        cachet: i64,
        statut_paiement: &str,
        solde: Option<i64>,
    ) -> NewProjet {
        NewProjet {
            client_id,
            titre: titre.into(),
            genre: "Genre".into(),
            statut: statut.into(),
            date_creation: date_creation.map(String::from),
            date_livraison: None,
            avance,
            cachet,
            statut_paiement: statut_paiement.into(),
            solde,
        }
    }

    // ---- validate_projet_fields (règles de la Phase 9) ----

    #[test]
    fn rejects_empty_titre() {
        assert!(validate_projet_fields("  ", &None, &None, 0, 100).is_err());
    }

    #[test]
    fn rejects_livraison_before_creation() {
        let dc = Some("2026-05-01".to_string());
        let dl = Some("2026-04-01".to_string());
        assert!(validate_projet_fields("Titre", &dc, &dl, 0, 100).is_err());
    }

    #[test]
    fn accepts_livraison_same_day_as_creation() {
        let dc = Some("2026-05-01".to_string());
        let dl = Some("2026-05-01".to_string());
        assert!(validate_projet_fields("Titre", &dc, &dl, 0, 100).is_ok());
    }

    #[test]
    fn accepts_missing_livraison_regardless_of_creation() {
        let dc = Some("2026-05-01".to_string());
        assert!(validate_projet_fields("Titre", &dc, &None, 0, 100).is_ok());
    }

    #[test]
    fn rejects_avance_greater_than_cachet() {
        assert!(validate_projet_fields("Titre", &None, &None, 600_000, 500_000).is_err());
    }

    #[test]
    fn accepts_avance_equal_to_cachet() {
        assert!(validate_projet_fields("Titre", &None, &None, 500_000, 500_000).is_ok());
    }

    #[test]
    fn accepts_fully_valid_projet() {
        let dc = Some("2026-01-01".to_string());
        let dl = Some("2026-06-01".to_string());
        assert!(validate_projet_fields("Album", &dc, &dl, 150_000, 600_000).is_ok());
    }

    // ---- clients: create/list/get/update/delete ----

    #[test]
    fn create_client_then_appears_in_list() {
        let conn = test_conn();
        let created = create_client_impl(&conn, &new_client("Kaiamba Quartet", "Analakely, Antananarivo")).unwrap();
        assert!(created.id > 0);
        let all = list_clients_impl(&conn).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].nom, "Kaiamba Quartet");
    }

    #[test]
    fn create_client_rejects_empty_nom() {
        let conn = test_conn();
        let err = create_client_impl(&conn, &new_client("   ", "Analakely")).unwrap_err();
        assert!(err.contains("obligatoire"));
    }

    #[test]
    fn get_client_returns_correct_row() {
        let conn = test_conn();
        let a = create_client_impl(&conn, &new_client("A", "Q1")).unwrap();
        let b = create_client_impl(&conn, &new_client("B", "Q2")).unwrap();
        let fetched = get_client_impl(&conn, b.id).unwrap();
        assert_eq!(fetched.nom, "B");
        assert_ne!(a.id, b.id);
    }

    #[test]
    fn update_client_preserves_id_and_changes_fields() {
        let conn = test_conn();
        let c = create_client_impl(&conn, &new_client("Old Name", "Old Quartier")).unwrap();
        let updated = Client {
            id: c.id,
            nom: "New Name".into(),
            responsable: "New Resp".into(),
            telephone: c.telephone.clone(),
            quartier: "New Quartier".into(),
            observation: "note".into(),
        };
        update_client_impl(&conn, &updated).unwrap();
        let fetched = get_client_impl(&conn, c.id).unwrap();
        assert_eq!(fetched.id, c.id);
        assert_eq!(fetched.nom, "New Name");
        assert_eq!(fetched.quartier, "New Quartier");
    }

    #[test]
    fn delete_client_removes_it_and_cascades_projets() {
        let conn = test_conn();
        let c = create_client_impl(&conn, &new_client("ToDelete", "Q")).unwrap();
        create_projet_impl(&conn, &new_projet(c.id, "P1", "en-cours", Some("2026-01-01"), 0, 1000, "paye", None)).unwrap();
        delete_client_impl(&conn, c.id).unwrap();
        assert!(get_client_impl(&conn, c.id).is_err());
        assert_eq!(list_projets_impl(&conn, c.id).unwrap().len(), 0);
    }

    // ---- projets: create/list/update/delete ----

    #[test]
    fn create_projet_then_appears_in_list_projets() {
        let conn = test_conn();
        let c = create_client_impl(&conn, &new_client("C", "Q")).unwrap();
        create_projet_impl(&conn, &new_projet(c.id, "Album A", "en-cours", Some("2026-01-01"), 100, 500, "paye", None)).unwrap();
        let projets = list_projets_impl(&conn, c.id).unwrap();
        assert_eq!(projets.len(), 1);
        assert_eq!(projets[0].titre, "Album A");
    }

    #[test]
    fn create_projet_rejects_invalid_dates_and_amounts() {
        let conn = test_conn();
        let c = create_client_impl(&conn, &new_client("C", "Q")).unwrap();
        let mut p = new_projet(c.id, "Bad", "en-cours", Some("2026-05-01"), 0, 100, "paye", None);
        p.date_livraison = Some("2026-04-01".into());
        assert!(create_projet_impl(&conn, &p).is_err());

        let p2 = new_projet(c.id, "Bad2", "en-cours", Some("2026-01-01"), 600, 500, "paye", None);
        assert!(create_projet_impl(&conn, &p2).is_err());
    }

    #[test]
    fn update_projet_changes_fields_and_id_stable() {
        let conn = test_conn();
        let c = create_client_impl(&conn, &new_client("C", "Q")).unwrap();
        let created = create_projet_impl(&conn, &new_projet(c.id, "Orig", "en-cours", Some("2026-01-01"), 0, 500, "paye", None)).unwrap();
        let updated = Projet {
            id: created.id,
            client_id: c.id,
            titre: "Renamed".into(),
            genre: "Genre".into(),
            statut: "termine".into(),
            date_creation: Some("2026-01-01".into()),
            date_livraison: Some("2026-02-01".into()),
            avance: 200,
            cachet: 500,
            statut_paiement: "impaye".into(),
            solde: Some(300),
        };
        update_projet_impl(&conn, &updated).unwrap();
        let fetched = get_projet_impl(&conn, created.id).unwrap();
        assert_eq!(fetched.titre, "Renamed");
        assert_eq!(fetched.statut, "termine");
        assert_eq!(fetched.solde, Some(300));
    }

    #[test]
    fn delete_projet_removes_only_that_one() {
        let conn = test_conn();
        let c = create_client_impl(&conn, &new_client("C", "Q")).unwrap();
        let p1 = create_projet_impl(&conn, &new_projet(c.id, "P1", "en-cours", Some("2026-01-01"), 0, 500, "paye", None)).unwrap();
        let p2 = create_projet_impl(&conn, &new_projet(c.id, "P2", "en-cours", Some("2026-01-01"), 0, 500, "paye", None)).unwrap();
        delete_projet_impl(&conn, p1.id).unwrap();
        let remaining = list_projets_impl(&conn, c.id).unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].id, p2.id);
    }

    // ---- reste (projets_view / filter_projets) dans les 3 cas ----

    #[test]
    fn reste_computed_correctly_in_three_cases() {
        let conn = test_conn();
        let c = create_client_impl(&conn, &new_client("C", "Q")).unwrap();
        create_projet_impl(&conn, &new_projet(c.id, "Paye", "termine", Some("2026-01-01"), 500, 500, "paye", None)).unwrap();
        create_projet_impl(&conn, &new_projet(c.id, "ImpayeAvecSolde", "en-cours", Some("2026-01-01"), 100, 1000, "impaye", Some(400))).unwrap();
        create_projet_impl(&conn, &new_projet(c.id, "ImpayeSansSolde", "attente", Some("2026-01-01"), 200, 1000, "impaye", None)).unwrap();

        let results = filter_projets_impl(&conn, &empty_criteria()).unwrap();
        let reste_of = |titre: &str| results.iter().find(|r| r.titre == titre).unwrap().reste;

        assert_eq!(reste_of("Paye"), 0);
        assert_eq!(reste_of("ImpayeAvecSolde"), 400);
        assert_eq!(reste_of("ImpayeSansSolde"), 800); // max(cachet-avance,0) = 1000-200
    }

    fn empty_criteria() -> FiltreCriteria {
        FiltreCriteria {
            nom_groupe: None, nom_projet: None, telephone: None, quartier: None, etat: None,
            creation_from: None, creation_to: None, livraison_from: None, livraison_to: None,
            avance_min: None, avance_max: None, reste_min: None, reste_max: None,
        }
    }

    // ---- filter_projets: critères combinés ----

    fn seed_filter_dataset(conn: &Connection) {
        let kaiamba = create_client_impl(conn, &new_client("Kaiamba Quartet", "Analakely, Antananarivo")).unwrap();
        let zafy = create_client_impl(conn, &new_client("Zafy & The Riddim", "Tsaralalàna, Antananarivo")).unwrap();
        create_projet_impl(conn, &new_projet(kaiamba.id, "Tany EP", "en-cours", Some("2026-02-03"), 150_000, 600_000, "paye", None)).unwrap();
        create_projet_impl(conn, &new_projet(kaiamba.id, "Live Session", "termine", Some("2025-11-11"), 300_000, 300_000, "paye", None)).unwrap();
        create_projet_impl(conn, &new_projet(zafy.id, "Malaza Album", "en-cours", Some("2026-04-15"), 400_000, 1_200_000, "paye", None)).unwrap();
        create_projet_impl(conn, &new_projet(zafy.id, "Freestyle", "en-cours", Some("2026-06-01"), 50_000, 200_000, "impaye", Some(10_000))).unwrap();
    }

    #[test]
    fn filter_by_nom_groupe_and_etat_combined() {
        let conn = test_conn();
        seed_filter_dataset(&conn);
        let mut criteria = empty_criteria();
        criteria.nom_groupe = Some("zafy".into()); // insensible à la casse
        criteria.etat = Some("en-cours".into());
        let results = filter_projets_impl(&conn, &criteria).unwrap();
        let titres: Vec<_> = results.iter().map(|r| r.titre.as_str()).collect();
        assert_eq!(titres, vec!["Malaza Album", "Freestyle"]);
    }

    #[test]
    fn filter_by_avance_range_and_paiement_state() {
        let conn = test_conn();
        seed_filter_dataset(&conn);
        let mut criteria = empty_criteria();
        criteria.avance_min = Some(100_000.0);
        criteria.avance_max = Some(350_000.0);
        let results = filter_projets_impl(&conn, &criteria).unwrap();
        let titres: Vec<_> = results.iter().map(|r| r.titre.as_str()).collect();
        assert_eq!(titres, vec!["Tany EP", "Live Session"]);
    }

    #[test]
    fn filter_by_creation_range_quartier_and_reste_combined() {
        let conn = test_conn();
        seed_filter_dataset(&conn);
        let mut criteria = empty_criteria();
        criteria.quartier = Some("Tsaralalàna".into());
        criteria.creation_from = Some("2026-01-01".into());
        criteria.reste_min = Some(1.0);
        let results = filter_projets_impl(&conn, &criteria).unwrap();
        let titres: Vec<_> = results.iter().map(|r| r.titre.as_str()).collect();
        assert_eq!(titres, vec!["Freestyle"]);
    }

    // ---- get_dashboard_stats ----

    #[test]
    fn dashboard_stats_counts_and_totals_are_correct() {
        let conn = test_conn();
        seed_filter_dataset(&conn);
        let stats = get_dashboard_stats_impl(&conn).unwrap();
        assert_eq!(stats.clients_total, 2);
        assert_eq!(stats.quartiers.len(), 2);
        assert_eq!(stats.projets_en_cours, 3);
        assert_eq!(stats.projets_termine, 1);
        assert_eq!(stats.payes_count, 3);
        assert_eq!(stats.payes_total, 600_000 + 300_000 + 1_200_000);
        assert_eq!(stats.impayes_count, 1);
        assert_eq!(stats.impayes_total, 10_000);
        assert_eq!(stats.impayes.len(), 1);
        assert_eq!(stats.impayes[0].titre, "Freestyle");
    }

    #[test]
    fn export_path_only_allows_absolute_pdf_and_xlsx() {
        let base = std::env::temp_dir();
        assert!(check_export_path(base.join("a.pdf").to_str().unwrap()).is_ok());
        assert!(check_export_path(base.join("a.XLSX").to_str().unwrap()).is_ok());
        assert!(check_export_path(base.join("a.exe").to_str().unwrap()).is_err());
        assert!(check_export_path(base.join("noext").to_str().unwrap()).is_err());
        assert!(check_export_path("relatif.pdf").is_err());
    }

    #[test]
    fn list_all_projets_returns_every_client_projects() {
        let conn = test_conn();
        seed_filter_dataset(&conn);
        let all = list_all_projets_impl(&conn).unwrap();
        assert_eq!(all.len(), 4);
        let mut ids: Vec<_> = all.iter().map(|p| p.client_id).collect();
        ids.dedup();
        assert_eq!(ids.len(), 2, "triés par client");
    }

    #[test]
    fn chart_years_include_project_years_and_current_year() {
        let conn = test_conn();
        seed_filter_dataset(&conn); // projets en 2025 et 2026
        let years = get_chart_years_impl(&conn).unwrap();
        let now: i64 = conn
            .query_row("SELECT CAST(strftime('%Y','now','localtime') AS INTEGER)", [], |r| r.get(0))
            .unwrap();
        assert!(years.contains(&2025) && years.contains(&2026) && years.contains(&now));
        assert!(years.windows(2).all(|w| w[0] < w[1]), "triées, sans doublon");
    }

    #[test]
    fn backup_path_must_be_absolute_sqlite() {
        let base = std::env::temp_dir();
        assert!(check_backup_path(base.join("s.sqlite").to_str().unwrap()).is_ok());
        assert!(check_backup_path(base.join("s.pdf").to_str().unwrap()).is_err());
        assert!(check_backup_path("s.sqlite").is_err());
    }

    #[test]
    fn dashboard_impayes_use_reste_when_solde_missing() {
        let conn = test_conn();
        let c = create_client_impl(&conn, &new_client("C", "Q")).unwrap();
        create_projet_impl(&conn, &new_projet(c.id, "AvecSolde", "en-cours", Some("2026-01-01"), 100, 1000, "impaye", Some(400))).unwrap();
        create_projet_impl(&conn, &new_projet(c.id, "SansSolde", "attente", Some("2026-01-01"), 200, 1000, "impaye", None)).unwrap();
        let stats = get_dashboard_stats_impl(&conn).unwrap();
        assert_eq!(stats.impayes_total, 400 + 800);
        let montant = |t: &str| stats.impayes.iter().find(|i| i.titre == t).unwrap().solde;
        assert_eq!(montant("AvecSolde"), 400);
        assert_eq!(montant("SansSolde"), 800);
    }

    // ---- get_chart_data ----

    #[test]
    fn chart_data_ventes_and_ca_group_by_month() {
        let conn = test_conn();
        seed_filter_dataset(&conn);
        let ventes = get_chart_data_impl(&conn, 2026, "ventes").unwrap();
        // Tany EP (fev), Malaza Album (avr), Freestyle (jun) -> index 1,3,5
        assert_eq!(ventes[1], 1);
        assert_eq!(ventes[3], 1);
        assert_eq!(ventes[5], 1);
        assert_eq!(ventes.iter().sum::<i64>(), 3);

        let ca = get_chart_data_impl(&conn, 2026, "ca").unwrap();
        assert_eq!(ca[1], 600_000);
        assert_eq!(ca[3], 1_200_000);
        assert_eq!(ca[5], 200_000);
    }

    #[test]
    fn chart_data_rejects_invalid_kind() {
        let conn = test_conn();
        assert!(get_chart_data_impl(&conn, 2026, "invalide").is_err());
    }

    // ---- get_dettes_comparaison ----

    #[test]
    fn dettes_comparaison_rejects_invalid_periode() {
        let conn = test_conn();
        assert!(get_dettes_comparaison_impl(&conn, "invalide").is_err());
    }

    #[test]
    fn dettes_comparaison_mois_includes_projet_created_this_month() {
        let conn = test_conn();
        let c = create_client_impl(&conn, &new_client("C", "Q")).unwrap();
        conn.execute(
            "INSERT INTO projets (client_id, titre, statut, date_creation, avance, cachet, statut_paiement, solde)
             VALUES (?1, 'Ce mois', 'attente', date('now'), 0, 7000, 'impaye', 7000)",
            params![c.id],
        ).unwrap();
        let cmp = get_dettes_comparaison_impl(&conn, "mois").unwrap();
        assert!(cmp.b >= 7000);
    }

    // ---- save_binary_file ----

    #[test]
    fn save_binary_file_writes_exact_bytes() {
        let path = std::env::temp_dir().join("bbr_test_save_binary_file.pdf");
        let data = vec![1u8, 2, 3, 4, 5];
        save_binary_file(path.to_string_lossy().to_string(), data.clone()).unwrap();
        let read_back = std::fs::read(&path).unwrap();
        assert_eq!(read_back, data);
        std::fs::remove_file(&path).ok();
    }

    // ---- create_client_avec_projet ----

    fn count(conn: &Connection, table: &str) -> i64 {
        conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0)).unwrap()
    }

    #[test]
    fn creates_client_and_minimal_projet_together() {
        let mut conn = test_conn();
        let res = create_client_avec_projet_impl(&mut conn, &new_client("KST", "Analakely"), "I Love You", None).unwrap();
        assert!(res.client_cree);
        assert!(!res.titre_renomme);
        assert_eq!(res.client.nom, "KST");
        assert_eq!(res.projet.client_id, res.client.id);
        assert_eq!(res.projet.titre, "I Love You");
        assert_eq!(res.projet.statut, "en-cours");
        assert_eq!(res.projet.statut_paiement, "impaye");
        assert_eq!((res.projet.avance, res.projet.cachet), (0, 0));
        assert!(res.projet.date_creation.is_some());
        assert_eq!((count(&conn, "clients"), count(&conn, "projets")), (1, 1));
    }

    #[test]
    fn empty_titre_creates_nothing() {
        let mut conn = test_conn();
        assert!(create_client_avec_projet_impl(&mut conn, &new_client("KST", ""), "  ", None).is_err());
        assert_eq!((count(&conn, "clients"), count(&conn, "projets")), (0, 0));
    }

    #[test]
    fn invalid_client_creates_nothing() {
        let mut conn = test_conn();
        assert!(create_client_avec_projet_impl(&mut conn, &new_client(" ", ""), "Titre", None).is_err());
        assert_eq!((count(&conn, "clients"), count(&conn, "projets")), (0, 0));
    }

    #[test]
    fn projet_failure_rolls_back_client() {
        let mut conn = test_conn();
        // Fait échouer l'INSERT du projet à l'intérieur de la transaction.
        conn.execute_batch(
            "CREATE TRIGGER t_fail BEFORE INSERT ON projets BEGIN SELECT RAISE(ABORT, 'boom'); END;",
        )
        .unwrap();
        let err = create_client_avec_projet_impl(&mut conn, &new_client("KST", ""), "Titre", None).unwrap_err();
        assert!(err.contains("boom"), "{err}");
        assert_eq!(count(&conn, "clients"), 0);
    }

    #[test]
    fn attaches_projet_to_existing_client() {
        let mut conn = test_conn();
        let kst = create_client_impl(&conn, &new_client("KST", "Analakely")).unwrap();
        let res = create_client_avec_projet_impl(&mut conn, &new_client("ignoré", ""), "Nouveau", Some(kst.id)).unwrap();
        assert!(!res.client_cree);
        assert_eq!(res.client.id, kst.id);
        assert_eq!(res.projet.client_id, kst.id);
        assert_eq!(count(&conn, "clients"), 1);
    }

    #[test]
    fn attaching_to_unknown_client_fails() {
        let mut conn = test_conn();
        assert!(create_client_avec_projet_impl(&mut conn, &new_client("X", ""), "T", Some(999)).is_err());
        assert_eq!(count(&conn, "projets"), 0);
    }

    #[test]
    fn duplicate_titre_gets_version_suffix() {
        let mut conn = test_conn();
        let first = create_client_avec_projet_impl(&mut conn, &new_client("KST", ""), "I Love You", None).unwrap();
        let id = Some(first.client.id);
        let v2 = create_client_avec_projet_impl(&mut conn, &new_client("KST", ""), "i love you ", id).unwrap();
        assert_eq!(v2.projet.titre, "i love you v2");
        assert!(v2.titre_renomme);
        let v3 = create_client_avec_projet_impl(&mut conn, &new_client("KST", ""), "I Love You", id).unwrap();
        assert_eq!(v3.projet.titre, "I Love You v3");
        // le même titre chez un autre client n'est pas suffixé
        let other = create_client_avec_projet_impl(&mut conn, &new_client("Zafy", ""), "I Love You", None).unwrap();
        assert_eq!(other.projet.titre, "I Love You");
    }

    #[test]
    fn find_client_by_nom_ignores_case_and_spaces() {
        let conn = test_conn();
        let kst = create_client_impl(&conn, &new_client("KST", "")).unwrap();
        assert_eq!(find_client_by_nom_impl(&conn, "  kst ").unwrap().map(|c| c.id), Some(kst.id));
        assert!(find_client_by_nom_impl(&conn, "KS").unwrap().is_none());
        assert!(find_client_by_nom_impl(&conn, "  ").unwrap().is_none());
    }
}
