use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Client {
    pub id: i64,
    pub nom: String,
    pub responsable: String,
    pub telephone: String,
    pub quartier: String,
    pub observation: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewClient {
    pub nom: String,
    pub responsable: String,
    pub telephone: String,
    pub quartier: String,
    pub observation: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FiltreCriteria {
    pub nom_groupe: Option<String>,
    pub nom_projet: Option<String>,
    pub telephone: Option<String>,
    pub quartier: Option<String>,
    pub etat: Option<String>,
    pub creation_from: Option<String>,
    pub creation_to: Option<String>,
    pub livraison_from: Option<String>,
    pub livraison_to: Option<String>,
    pub avance_min: Option<f64>,
    pub avance_max: Option<f64>,
    pub reste_min: Option<f64>,
    pub reste_max: Option<f64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FiltreResult {
    pub id: i64,
    pub client_id: i64,
    pub client_nom: String,
    pub client_telephone: String,
    pub client_quartier: String,
    pub titre: String,
    pub genre: String,
    pub statut: String,
    pub date_creation: Option<String>,
    pub date_livraison: Option<String>,
    pub avance: i64,
    pub cachet: i64,
    pub statut_paiement: String,
    pub solde: Option<i64>,
    pub reste: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuartierCount {
    pub quartier: String,
    pub count: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImpayeItem {
    pub client_nom: String,
    pub titre: String,
    pub genre: String,
    pub solde: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardStats {
    pub quartiers: Vec<QuartierCount>,
    pub clients_total: i64,
    pub projets_en_cours: i64,
    pub projets_attente: i64,
    pub projets_termine: i64,
    pub payes_count: i64,
    pub payes_total: i64,
    pub impayes_count: i64,
    pub impayes_total: i64,
    pub impayes: Vec<ImpayeItem>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DettesComparaison {
    pub a: i64,
    pub b: i64,
}
