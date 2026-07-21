PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS clients (
  id            INTEGER PRIMARY KEY AUTOINCREMENT,
  nom           TEXT NOT NULL,
  responsable   TEXT NOT NULL DEFAULT '',
  telephone     TEXT NOT NULL DEFAULT '',
  quartier      TEXT NOT NULL DEFAULT '',
  observation   TEXT NOT NULL DEFAULT '',
  created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE TABLE IF NOT EXISTS projets (
  id               INTEGER PRIMARY KEY AUTOINCREMENT,
  client_id        INTEGER NOT NULL REFERENCES clients(id) ON DELETE CASCADE,
  titre            TEXT NOT NULL,
  genre            TEXT NOT NULL DEFAULT '',
  statut           TEXT NOT NULL CHECK (statut IN ('en-cours','attente','termine')),
  date_creation    TEXT,                 -- ISO YYYY-MM-DD
  date_livraison   TEXT,                 -- ISO YYYY-MM-DD, nullable
  avance           INTEGER NOT NULL DEFAULT 0 CHECK (avance >= 0),
  cachet           INTEGER NOT NULL DEFAULT 0 CHECK (cachet >= 0),
  statut_paiement  TEXT NOT NULL CHECK (statut_paiement IN ('paye','impaye')),
  solde            INTEGER,              -- reste à payer saisi manuellement, NULL = calculé
  created_at       TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now')),
  updated_at       TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
);

CREATE INDEX IF NOT EXISTS idx_projets_client    ON projets(client_id);
CREATE INDEX IF NOT EXISTS idx_projets_statut    ON projets(statut);
CREATE INDEX IF NOT EXISTS idx_projets_paiement  ON projets(statut_paiement);
CREATE INDEX IF NOT EXISTS idx_projets_creation  ON projets(date_creation);
CREATE INDEX IF NOT EXISTS idx_projets_livraison ON projets(date_livraison);
CREATE INDEX IF NOT EXISTS idx_clients_quartier  ON clients(quartier);
CREATE INDEX IF NOT EXISTS idx_clients_nom       ON clients(nom);

-- Vue qui reproduit la logique du prototype JS:
-- p.paiement==='paye' ? 0 : (solde ?? max(cachet-avance,0))
CREATE VIEW IF NOT EXISTS projets_view AS
SELECT
  p.*,
  c.nom       AS client_nom,
  c.telephone AS client_telephone,
  c.quartier  AS client_quartier,
  CASE
    WHEN p.statut_paiement = 'paye' THEN 0
    ELSE COALESCE(p.solde, MAX(p.cachet - p.avance, 0))
  END AS reste
FROM projets p
JOIN clients c ON c.id = p.client_id;

CREATE TRIGGER IF NOT EXISTS trg_clients_updated_at AFTER UPDATE ON clients
BEGIN UPDATE clients SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id = NEW.id; END;

CREATE TRIGGER IF NOT EXISTS trg_projets_updated_at AFTER UPDATE ON projets
BEGIN UPDATE projets SET updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now') WHERE id = NEW.id; END;
