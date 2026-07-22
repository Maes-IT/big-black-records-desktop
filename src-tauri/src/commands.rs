use rusqlite::{named_params, params, Row};
use std::collections::HashMap;
use tauri::State;

use crate::db::DbState;
use crate::models::{
    Client, DashboardStats, DettesComparaison, FiltreCriteria, FiltreResult, ImpayeItem,
    NewClient, NewProjet, Projet, QuartierCount,
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

const CLIENT_COLUMNS: &str = "id, nom, responsable, telephone, quartier, observation";
const PROJET_COLUMNS: &str =
    "id, client_id, titre, genre, statut, date_creation, date_livraison, avance, cachet, statut_paiement, solde";

#[tauri::command]
pub fn list_clients(state: State<DbState>) -> Result<Vec<Client>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let mut stmt = conn
        .prepare(&format!("SELECT {CLIENT_COLUMNS} FROM clients ORDER BY nom"))
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], row_to_client)
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_client(state: State<DbState>, id: i64) -> Result<Client, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    conn.query_row(
        &format!("SELECT {CLIENT_COLUMNS} FROM clients WHERE id = ?1"),
        params![id],
        row_to_client,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn create_client(state: State<DbState>, client: NewClient) -> Result<Client, String> {
    if client.nom.trim().is_empty() {
        return Err("Le nom du groupe est obligatoire.".into());
    }
    let conn = state.0.lock().map_err(|e| e.to_string())?;
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
    conn.query_row(
        &format!("SELECT {CLIENT_COLUMNS} FROM clients WHERE id = ?1"),
        params![id],
        row_to_client,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_client(state: State<DbState>, client: Client) -> Result<(), String> {
    if client.nom.trim().is_empty() {
        return Err("Le nom du groupe est obligatoire.".into());
    }
    let conn = state.0.lock().map_err(|e| e.to_string())?;
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

#[tauri::command]
pub fn delete_client(state: State<DbState>, id: i64) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM clients WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn list_projets(state: State<DbState>, client_id: i64) -> Result<Vec<Projet>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
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

#[tauri::command]
pub fn create_projet(state: State<DbState>, projet: NewProjet) -> Result<Projet, String> {
    if projet.titre.trim().is_empty() {
        return Err("Le titre du projet est obligatoire.".into());
    }
    let conn = state.0.lock().map_err(|e| e.to_string())?;
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
    conn.query_row(
        &format!("SELECT {PROJET_COLUMNS} FROM projets WHERE id = ?1"),
        params![id],
        row_to_projet,
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_projet(state: State<DbState>, projet: Projet) -> Result<(), String> {
    if projet.titre.trim().is_empty() {
        return Err("Le titre du projet est obligatoire.".into());
    }
    let conn = state.0.lock().map_err(|e| e.to_string())?;
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

#[tauri::command]
pub fn delete_projet(state: State<DbState>, id: i64) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM projets WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(())
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

#[tauri::command]
pub fn filter_projets(
    state: State<DbState>,
    criteria: FiltreCriteria,
) -> Result<Vec<FiltreResult>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
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

#[tauri::command]
pub fn get_dashboard_stats(state: State<DbState>) -> Result<DashboardStats, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;

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
                COALESCE(SUM(CASE WHEN statut_paiement='impaye' THEN solde ELSE 0 END),0)
             FROM projets",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .map_err(|e| e.to_string())?;

    let impayes: Vec<ImpayeItem> = {
        let mut stmt = conn
            .prepare(
                "SELECT client_nom, titre, genre, solde FROM projets_view
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

#[tauri::command]
pub fn get_chart_data(state: State<DbState>, annee: i64, kind: String) -> Result<Vec<i64>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let sql = match kind.as_str() {
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
#[tauri::command]
pub fn get_dettes_comparaison(
    state: State<DbState>,
    periode: String,
) -> Result<DettesComparaison, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let (current_sql, previous_sql): (&str, &str) = match periode.as_str() {
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
