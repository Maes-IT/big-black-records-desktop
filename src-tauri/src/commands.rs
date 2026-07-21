use rusqlite::{params, Row};
use tauri::State;

use crate::db::DbState;
use crate::models::{Client, NewClient, NewProjet, Projet};

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
