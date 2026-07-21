use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Client {
    pub id: i64,
    pub nom: String,
    pub responsable: String,
    pub telephone: String,
    pub quartier: String,
    pub observation: String,
}

#[derive(Deserialize)]
pub struct NewClient {
    pub nom: String,
    pub responsable: String,
    pub telephone: String,
    pub quartier: String,
    pub observation: String,
}

#[derive(Serialize, Deserialize)]
pub struct Projet {
    pub id: i64,
    pub client_id: i64,
    pub titre: String,
    pub genre: String,
    pub statut: String,
    pub date_creation: Option<String>,
    pub date_livraison: Option<String>,
    pub avance: i64,
    pub cachet: i64,
    pub statut_paiement: String,
    pub solde: Option<i64>,
}

#[derive(Deserialize)]
pub struct NewProjet {
    pub client_id: i64,
    pub titre: String,
    pub genre: String,
    pub statut: String,
    pub date_creation: Option<String>,
    pub date_livraison: Option<String>,
    pub avance: i64,
    pub cachet: i64,
    pub statut_paiement: String,
    pub solde: Option<i64>,
}
