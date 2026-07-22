# Plan de test manuel — Big Black Records

À exécuter dans l'app lancée via `npm.cmd run tauri dev` (ou le build packagé). Coche au fur et à
mesure ; note tout écart par rapport au comportement attendu.

## 0. Démarrage

- [ ] L'app s'ouvre sans erreur, la fenêtre affiche « Big Black Records — Gestion ».
- [ ] L'onglet **T.D.B** est actif par défaut et affiche des données (pas de zéros partout si des
      clients/projets existent déjà en base).
- [ ] Aucun bandeau d'erreur rouge (`#globalError`) ne s'affiche au chargement.

## 1. Module Client

### 1.1 Client

- [ ] La liste des clients affiche tous les clients, triés par nom.
- [ ] Cliquer un client l'affiche en surbrillance et remplit le panneau « Info Perso ».
- [ ] Rechercher un nom partiel dans la barre de recherche filtre la liste en direct.
- [ ] **Ajouter un client** (bouton `+`) :
  - [ ] Soumettre sans nom → message d'erreur, la modale reste ouverte.
  - [ ] Soumettre avec un nom → le client apparaît immédiatement dans la liste et devient
        sélectionné automatiquement.
- [ ] **Modifier un client** (icône crayon) : les champs sont pré-remplis, la modification est
      reflétée immédiatement dans la liste et le panneau, sans perdre la sélection.
- [ ] **Supprimer un client** (icône poubelle sur la ligne) :
  - [ ] Le clic sur la poubelle n'ouvre PAS la fiche du client (pas de sélection accidentelle).
  - [ ] La modale de confirmation mentionne le nombre de projets qui seront supprimés avec.
  - [ ] Après confirmation, le client disparaît, un autre client est sélectionné automatiquement
        (ou l'état vide s'affiche si c'était le dernier).
  - [ ] Annuler la suppression ne modifie rien.

### 1.2 Projet

- [ ] Sélectionner un client affiche ses projets dans la colonne du milieu.
- [ ] Cliquer un projet remplit le panneau « Info Projet » à droite.
- [ ] Rechercher un titre partiel filtre la liste des projets en direct.
- [ ] **Ajouter un projet** :
  - [ ] Soumettre sans titre → erreur, modale ouverte.
  - [ ] Cocher « Impayé » affiche le champ « Reste à payer » ; « Payé » le masque.
  - [ ] **Date de livraison antérieure à la date de création** → message d'erreur explicite,
        enregistrement bloqué.
  - [ ] **Avance supérieure au cachet** → message d'erreur explicite, enregistrement bloqué.
  - [ ] Un projet valide s'ajoute et apparaît sélectionné immédiatement.
- [ ] **Modifier un projet** : mêmes règles de validation (dates, avance/cachet) appliquées à
      l'édition.
- [ ] **Supprimer un projet** : confirmation, suppression sans affecter les autres projets ni le
      client.

## 2. Module Filtre (recherche croisée)

Teste chaque filtre isolément, puis au moins une combinaison de 2-3 filtres :

- [ ] Nom du groupe (partiel, insensible à la casse)
- [ ] Nom du projet (partiel)
- [ ] Téléphone (avec ou sans espaces dans la saisie)
- [ ] Quartier (menu déroulant)
- [ ] État du projet (En cours / Prêt / Livré)
- [ ] Plage de date de création
- [ ] Plage de date de livraison (vérifier qu'un projet sans date de livraison est bien exclu
      quand ce filtre est actif)
- [ ] Plage d'avance
- [ ] Plage de reste à payer
- [ ] Combinaison d'au moins 3 filtres simultanés → résultat cohérent avec l'intersection attendue
- [ ] Les chips de filtres actifs s'affichent et sont retirables individuellement (bouton ×)
- [ ] « Réinitialiser » vide tous les champs et réaffiche tous les projets
- [ ] Cliquer un résultat bascule vers l'onglet Client avec le bon client et projet sélectionnés

## 3. Tableau de bord (T.D.B)

- [ ] « Client par Quartier » : les comptes correspondent au nombre réel de clients par quartier.
- [ ] « Projet » : les 3 compteurs (En cours / Prêt / Livré) correspondent au total réel de
      projets par statut.
- [ ] « Paiement » : comptes et montants (Payé / Impayé) correspondent à la somme réelle des
      cachets (payés) et soldes (impayés).
- [ ] « Liste Projet Impayé » : tous les projets impayés y figurent, avec le bon montant.
- [ ] Graphique « Nombre de Vente » : change bien en changeant l'année (onglets 2026-2029) ; les
      barres correspondent au nombre de projets créés par mois pour l'année choisie.
- [ ] Graphique « Chiffre d'Affaires » : idem, correspond à la somme des cachets par mois.
- [ ] « Comparaison des Dettes » : change en cliquant Mois / Semaine / An ; les deux valeurs
      affichées sont plausibles par rapport aux projets impayés créés récemment.
- [ ] Ajouter/modifier/supprimer un client ou projet puis revenir sur l'onglet T.D.B → les
      chiffres se mettent à jour (le dashboard se recharge à chaque activation de l'onglet).

## 4. Export PDF / XLSX

- [ ] Sélectionner un client avec au moins un projet, cliquer « PDF » : une boîte de dialogue
      « Enregistrer sous » native s'ouvre avec un nom de fichier suggéré.
  - [ ] Choisir un emplacement et valider → le fichier est bien créé à cet endroit.
  - [ ] Ouvrir le PDF : en-tête « BIG BLACK RECORDS », infos client, tableau des projets avec les
        bonnes colonnes et valeurs (Avance, Cachet, Reste à payer, statut).
  - [ ] Annuler la boîte de dialogue → aucun fichier créé, pas de message d'erreur.
- [ ] Même test avec « XLSX » : deux feuilles (« Client » et « Projets »), contenu correct à
      l'ouverture dans un tableur.
- [ ] Cliquer export sans client sélectionné → message d'alerte clair, pas de plantage.

## 5. Robustesse

- [ ] Fermer et rouvrir l'app : les données persistent (rien perdu depuis la session précédente).
- [ ] Avec la base de données déplacée/renommée temporairement (simulation de fichier manquant),
      un message d'erreur clair apparaît au démarrage plutôt qu'un plantage silencieux — *test
      optionnel, à ne faire que si tu es à l'aise pour manipuler le fichier `bigblack.sqlite` dans
      `%APPDATA%\com.bigblackrecords.app\`*.
