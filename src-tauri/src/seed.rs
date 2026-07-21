use rusqlite::{params, Connection};

struct SeedProject {
    titre: &'static str,
    genre: &'static str,
    statut: &'static str,
    creation: &'static str,
    livraison: &'static str,
    avance: &'static str,
    cachet: &'static str,
    paiement: &'static str,
    solde: Option<&'static str>,
}

struct SeedClient {
    nom: &'static str,
    responsable: &'static str,
    telephone: &'static str,
    quartier: &'static str,
    observation: &'static str,
    projets: &'static [SeedProject],
}

// Reprend telles quelles les données MOCK_DATA de big-black-record.html / src/main.js,
// pour disposer de données de test réalistes dès le premier lancement.
const SEED: &[SeedClient] = &[
    SeedClient {
        nom: "Kaiamba Quartet",
        responsable: "Rija Andriamora",
        telephone: "034 12 345 67",
        quartier: "Analakely, Antananarivo",
        observation: "Groupe régulier, préfère les sessions en soirée.",
        projets: &[
            SeedProject {
                titre: "Tany Malalako — EP",
                genre: "Salegy",
                statut: "en-cours",
                creation: "03/02/2026",
                livraison: "28/07/2026",
                avance: "150 000 Ar",
                cachet: "600 000 Ar",
                paiement: "paye",
                solde: None,
            },
            SeedProject {
                titre: "Live Session Vol.1",
                genre: "Salegy Fusion",
                statut: "termine",
                creation: "11/11/2025",
                livraison: "20/12/2025",
                avance: "300 000 Ar",
                cachet: "300 000 Ar",
                paiement: "paye",
                solde: None,
            },
        ],
    },
    SeedClient {
        nom: "Voky Groove",
        responsable: "Nomena Rakoto",
        telephone: "032 98 765 43",
        quartier: "Ivandry, Antananarivo",
        observation: "Enregistrement à distance possible, envoie les stems par WeTransfer.",
        projets: &[SeedProject {
            titre: "Riake — Single",
            genre: "Afro-Groove",
            statut: "attente",
            creation: "09/07/2026",
            livraison: "—",
            avance: "0 Ar",
            cachet: "250 000 Ar",
            paiement: "impaye",
            solde: Some("5 000 Ar"),
        }],
    },
    SeedClient {
        nom: "Zafy & The Riddim",
        responsable: "Fenosoa Randria",
        telephone: "033 45 678 90",
        quartier: "Tsaralalàna, Antananarivo",
        observation: "",
        projets: &[
            SeedProject {
                titre: "Malaza Anao — Album",
                genre: "Reggae Gasy",
                statut: "en-cours",
                creation: "15/04/2026",
                livraison: "30/09/2026",
                avance: "400 000 Ar",
                cachet: "1 200 000 Ar",
                paiement: "paye",
                solde: None,
            },
            SeedProject {
                titre: "Freestyle Kilalaky",
                genre: "Kilalaky",
                statut: "en-cours",
                creation: "01/06/2026",
                livraison: "10/08/2026",
                avance: "50 000 Ar",
                cachet: "200 000 Ar",
                paiement: "impaye",
                solde: Some("10 000 Ar"),
            },
        ],
    },
    SeedClient {
        nom: "Malaza Sons",
        responsable: "Hery Ravelo",
        telephone: "034 76 543 21",
        quartier: "Ambohipo, Antananarivo",
        observation: "Groupe en tournée jusqu'en septembre.",
        projets: &[SeedProject {
            titre: "Firaisana — EP",
            genre: "Hip-Hop Gasy",
            statut: "termine",
            creation: "20/09/2025",
            livraison: "18/01/2026",
            avance: "900 000 Ar",
            cachet: "900 000 Ar",
            paiement: "paye",
            solde: None,
        }],
    },
    SeedClient {
        nom: "Tanindrazana",
        responsable: "Miora Rasolofo",
        telephone: "032 11 222 33",
        quartier: "Ankadifotsy, Antananarivo",
        observation: "Sensible sur les délais, envoyer un rappel 48h avant.",
        projets: &[],
    },
];

fn parse_ar(s: &str) -> i64 {
    s.chars()
        .filter(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse()
        .unwrap_or(0)
}

fn fr_to_iso(fr: &str) -> Option<String> {
    let parts: Vec<&str> = fr.split('/').collect();
    if parts.len() != 3 {
        return None;
    }
    Some(format!("{}-{}-{}", parts[2], parts[1], parts[0]))
}

pub fn seed_if_empty(conn: &Connection) -> rusqlite::Result<()> {
    let count: i64 = conn.query_row("SELECT COUNT(*) FROM clients", [], |r| r.get(0))?;
    if count > 0 {
        return Ok(());
    }

    for c in SEED {
        conn.execute(
            "INSERT INTO clients (nom, responsable, telephone, quartier, observation)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![c.nom, c.responsable, c.telephone, c.quartier, c.observation],
        )?;
        let client_id = conn.last_insert_rowid();

        for p in c.projets {
            conn.execute(
                "INSERT INTO projets
                    (client_id, titre, genre, statut, date_creation, date_livraison,
                     avance, cachet, statut_paiement, solde)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    client_id,
                    p.titre,
                    p.genre,
                    p.statut,
                    fr_to_iso(p.creation),
                    fr_to_iso(p.livraison),
                    parse_ar(p.avance),
                    parse_ar(p.cachet),
                    p.paiement,
                    p.solde.map(parse_ar),
                ],
            )?;
        }
    }

    Ok(())
}
