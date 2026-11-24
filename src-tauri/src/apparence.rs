//! Apparence de l'application : thème, couleur d'accent, jeu de polices, taille du texte
//! et image de fond.
//!
//! Les préférences sont stockées en JSON dans la table `parametres` (clé `apparence`).
//! L'image de fond est COPIÉE dans le dossier de données (`fond.<ext>`) : elle reste
//! affichée même si l'original est déplacé ou supprimé.

use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::db::{DataDir, DbState};

const KEY_APPARENCE: &str = "apparence";
const FOND_STEM: &str = "fond";
const FOND_EXTENSIONS: [&str; 4] = ["png", "jpg", "jpeg", "webp"];
const FOND_MAX_BYTES: u64 = 15 * 1024 * 1024;

/// Doivent correspondre aux clés THEMES / FONT_SETS de src/main.js.
pub const THEMES: [&str; 5] = ["or-noir", "clair", "bleu-nuit", "rouge-vinyle", "vert-studio"];
pub const POLICES: [&str; 4] = ["studio", "windows", "classique", "moderne"];
const TAILLE_MIN: f64 = 0.85;
const TAILLE_MAX: f64 = 1.25;
const ASSOMBRISSEMENT_MAX: i64 = 90;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Apparence {
    pub theme: String,
    /// Couleur d'accent personnalisée « #rrggbb », ou None pour celle du thème.
    pub accent: Option<String>,
    pub police: String,
    /// Facteur de taille du texte (1 = normal).
    pub taille: f64,
    /// Assombrissement de l'image de fond, en pourcentage (0 à 90).
    pub assombrissement: i64,
}

impl Default for Apparence {
    fn default() -> Self {
        Apparence {
            theme: "or-noir".into(),
            accent: None,
            police: "studio".into(),
            taille: 1.0,
            assombrissement: 45,
        }
    }
}

/// Ce que reçoit le frontend : les préférences + l'image de fond actuelle, s'il y en a une.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApparenceInfo {
    #[serde(flatten)]
    pub prefs: Apparence,
    /// Type MIME de l'image de fond enregistrée (None = pas d'image).
    pub fond_mime: Option<String>,
}

fn is_hex_color(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].chars().all(|c| c.is_ascii_hexdigit())
}

pub fn validate(a: &Apparence) -> Result<Apparence, String> {
    if !THEMES.contains(&a.theme.as_str()) {
        return Err(format!("Thème inconnu : {}", a.theme));
    }
    if !POLICES.contains(&a.police.as_str()) {
        return Err(format!("Jeu de polices inconnu : {}", a.police));
    }
    let accent = match a.accent.as_deref().map(str::trim) {
        None | Some("") => None,
        Some(c) if is_hex_color(c) => Some(c.to_ascii_lowercase()),
        Some(c) => return Err(format!("Couleur invalide : {c} (attendu #rrggbb)")),
    };
    if !a.taille.is_finite() || a.taille < TAILLE_MIN || a.taille > TAILLE_MAX {
        return Err("Taille de texte hors limites.".into());
    }
    if !(0..=ASSOMBRISSEMENT_MAX).contains(&a.assombrissement) {
        return Err("Assombrissement hors limites.".into());
    }
    Ok(Apparence { accent, ..a.clone() })
}

/// Préférences enregistrées ; valeurs par défaut si rien n'est enregistré ou si le JSON
/// est invalide (une préférence cassée ne doit jamais empêcher l'application de s'ouvrir).
pub fn read_apparence(conn: &Connection) -> Result<Apparence, String> {
    let raw: Option<String> = conn
        .query_row(
            "SELECT valeur FROM parametres WHERE cle = ?1",
            params![KEY_APPARENCE],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    Ok(raw
        .and_then(|json| serde_json::from_str::<Apparence>(&json).ok())
        .and_then(|a| validate(&a).ok())
        .unwrap_or_default())
}

pub fn save_apparence_impl(conn: &Connection, a: &Apparence) -> Result<Apparence, String> {
    let clean = validate(a)?;
    let json = serde_json::to_string(&clean).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO parametres (cle, valeur) VALUES (?1, ?2)
         ON CONFLICT(cle) DO UPDATE SET valeur = excluded.valeur",
        params![KEY_APPARENCE, json],
    )
    .map_err(|e| e.to_string())?;
    Ok(clean)
}

fn mime_for(ext: &str) -> &'static str {
    match ext {
        "png" => "image/png",
        "webp" => "image/webp",
        _ => "image/jpeg",
    }
}

/// Image de fond actuellement enregistrée dans `dir`, s'il y en a une.
pub fn current_fond(dir: &Path) -> Option<(PathBuf, &'static str)> {
    FOND_EXTENSIONS.iter().find_map(|ext| {
        let p = dir.join(format!("{FOND_STEM}.{ext}"));
        p.is_file().then(|| (p, mime_for(ext)))
    })
}

fn remove_fond(dir: &Path) -> Result<(), String> {
    for ext in FOND_EXTENSIONS {
        let p = dir.join(format!("{FOND_STEM}.{ext}"));
        if p.exists() {
            std::fs::remove_file(&p).map_err(|e| format!("Impossible de supprimer l'ancienne image : {e}"))?;
        }
    }
    Ok(())
}

/// Copie `source` comme nouvelle image de fond (remplace l'ancienne). Retourne son type MIME.
pub fn set_fond_impl(dir: &Path, source: &str) -> Result<&'static str, String> {
    let src = Path::new(source);
    if !src.is_absolute() {
        return Err("Chemin d'image invalide.".into());
    }
    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .filter(|e| FOND_EXTENSIONS.contains(&e.as_str()))
        .ok_or("Formats acceptés : PNG, JPG ou WEBP.")?;
    let meta = std::fs::metadata(src).map_err(|e| format!("Image introuvable : {e}"))?;
    if !meta.is_file() {
        return Err("Ce chemin n'est pas un fichier.".into());
    }
    if meta.len() > FOND_MAX_BYTES {
        return Err("Image trop lourde (15 Mo maximum).".into());
    }
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    // Copie d'abord vers un fichier temporaire : si la copie échoue, l'ancienne image reste.
    let tmp = dir.join(format!("{FOND_STEM}.tmp"));
    std::fs::copy(src, &tmp).map_err(|e| format!("Copie de l'image impossible : {e}"))?;
    remove_fond(dir)?;
    std::fs::rename(&tmp, dir.join(format!("{FOND_STEM}.{ext}"))).map_err(|e| e.to_string())?;
    Ok(mime_for(&ext))
}

// ==================== Commandes Tauri ====================

#[tauri::command]
pub fn get_apparence(state: State<DbState>, data_dir: State<DataDir>) -> Result<ApparenceInfo, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    Ok(ApparenceInfo {
        prefs: read_apparence(&conn)?,
        fond_mime: current_fond(&data_dir.0).map(|(_, m)| m.to_string()),
    })
}

#[tauri::command]
pub fn save_apparence(state: State<DbState>, apparence: Apparence) -> Result<Apparence, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    save_apparence_impl(&conn, &apparence)
}

#[tauri::command]
pub fn set_fond_image(data_dir: State<DataDir>, path: String) -> Result<String, String> {
    set_fond_impl(&data_dir.0, &path).map(String::from)
}

/// Octets de l'image de fond, envoyés en binaire (pas en tableau JSON, trop lent pour une image).
#[tauri::command]
pub fn get_fond_image(data_dir: State<DataDir>) -> Result<tauri::ipc::Response, String> {
    let (path, _) = current_fond(&data_dir.0).ok_or("Aucune image de fond.")?;
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    Ok(tauri::ipc::Response::new(bytes))
}

#[tauri::command]
pub fn clear_fond_image(data_dir: State<DataDir>) -> Result<(), String> {
    remove_fond(&data_dir.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_conn() -> Connection {
        let mut conn = Connection::open_in_memory().unwrap();
        crate::db::migrate(&mut conn).unwrap();
        conn
    }

    fn tmp_dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("bbr_apparence_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn defaults_when_nothing_saved() {
        assert_eq!(read_apparence(&test_conn()).unwrap(), Apparence::default());
    }

    #[test]
    fn save_then_read_roundtrip() {
        let conn = test_conn();
        let a = Apparence {
            theme: "bleu-nuit".into(),
            accent: Some("#3A7BD5".into()),
            police: "windows".into(),
            taille: 1.12,
            assombrissement: 30,
        };
        let saved = save_apparence_impl(&conn, &a).unwrap();
        assert_eq!(saved.accent.as_deref(), Some("#3a7bd5"));
        assert_eq!(read_apparence(&conn).unwrap(), saved);
    }

    #[test]
    fn invalid_values_are_rejected() {
        let ok = Apparence::default();
        let bad = [
            Apparence { theme: "rose".into(), ..ok.clone() },
            Apparence { police: "comic".into(), ..ok.clone() },
            Apparence { accent: Some("red".into()), ..ok.clone() },
            Apparence { accent: Some("#12345z".into()), ..ok.clone() },
            Apparence { taille: 3.0, ..ok.clone() },
            Apparence { assombrissement: 120, ..ok.clone() },
        ];
        for a in bad {
            assert!(validate(&a).is_err(), "{a:?} devrait être refusé");
        }
        assert_eq!(validate(&Apparence { accent: Some(" ".into()), ..ok }).unwrap().accent, None);
    }

    #[test]
    fn corrupted_json_falls_back_to_defaults() {
        let conn = test_conn();
        conn.execute(
            "INSERT INTO parametres (cle, valeur) VALUES ('apparence', '{pas du json')",
            [],
        )
        .unwrap();
        assert_eq!(read_apparence(&conn).unwrap(), Apparence::default());
    }

    #[test]
    fn fond_copy_replace_and_clear() {
        let src_dir = tmp_dir("src");
        let data = tmp_dir("data");
        let png = src_dir.join("photo.PNG");
        let jpg = src_dir.join("autre.jpg");
        std::fs::write(&png, b"png-bytes").unwrap();
        std::fs::write(&jpg, b"jpg-bytes").unwrap();

        assert_eq!(set_fond_impl(&data, png.to_str().unwrap()).unwrap(), "image/png");
        let (p, _) = current_fond(&data).unwrap();
        assert_eq!(std::fs::read(&p).unwrap(), b"png-bytes");

        // L'original peut disparaître : la copie reste.
        std::fs::remove_file(&png).unwrap();
        assert!(current_fond(&data).is_some());

        // Remplacement : une seule image à la fois.
        assert_eq!(set_fond_impl(&data, jpg.to_str().unwrap()).unwrap(), "image/jpeg");
        assert!(!data.join("fond.png").exists());
        assert_eq!(std::fs::read(current_fond(&data).unwrap().0).unwrap(), b"jpg-bytes");

        remove_fond(&data).unwrap();
        assert!(current_fond(&data).is_none());
        std::fs::remove_dir_all(&src_dir).ok();
        std::fs::remove_dir_all(&data).ok();
    }

    #[test]
    fn fond_rejects_bad_paths() {
        let data = tmp_dir("bad");
        let exe = data.join("virus.exe");
        std::fs::write(&exe, b"x").unwrap();
        assert!(set_fond_impl(&data, exe.to_str().unwrap()).is_err());
        assert!(set_fond_impl(&data, "relatif.png").is_err());
        assert!(set_fond_impl(&data, data.join("absent.png").to_str().unwrap()).is_err());
        assert!(current_fond(&data).is_none());
        std::fs::remove_dir_all(&data).ok();
    }
}
