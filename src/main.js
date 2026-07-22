  const { invoke } = window.__TAURI__.core;

  const TRASH_SVG = `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 6h18"/><path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6"/><path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/></svg>`;

  // ---------- Adaptateurs DB (SQLite via invoke) -> forme historique du prototype ----------
  // Le reste du fichier (T.D.B, Filtre, exports) a été écrit contre cette forme (tel/obs,
  // montants et dates déjà formatés) : on la reconstruit ici pour éviter de tout retoucher.
  function clientFromRow(row, projets){
    return {
      id: row.id,
      nom: row.nom,
      responsable: row.responsable,
      tel: row.telephone,
      quartier: row.quartier,
      obs: row.observation,
      projets,
    };
  }

  function projetFromRow(row){
    const p = {
      id: row.id,
      titre: row.titre,
      genre: row.genre,
      statut: row.statut,
      avance: formatAr(row.avance),
      cachet: formatAr(row.cachet),
      creation: isoToFr(row.dateCreation),
      livraison: row.dateLivraison ? isoToFr(row.dateLivraison) : '—',
      paiement: row.statutPaiement,
    };
    if(row.solde != null) p.solde = formatAr(row.solde);
    return p;
  }

  async function loadClients(){
    const rows = await invoke('list_clients');
    const loaded = [];
    for(const row of rows){
      const projetRows = await invoke('list_projets', { clientId: row.id });
      loaded.push(clientFromRow(row, projetRows.map(projetFromRow)));
    }
    return loaded;
  }

  // ---------- DONNÉES (chargées depuis SQLite au démarrage, voir init() en bas de fichier) ----------
  let clients = [];

  let activeClientId = null;
  let activeProjectId = null;

  const statusLabel = { "en-cours": "En cours", "termine": "Terminé", "attente": "En attente" };
  const statusClass = { "en-cours": "status-en-cours", "termine": "status-termine", "attente": "status-attente" };

  const clientListEl = document.getElementById('clientList');
  const clientSearchEl = document.getElementById('clientSearch');
  const clientNameEl = document.getElementById('clientName');
  const personalFieldsEl = document.getElementById('personalFields');
  const projectListEl = document.getElementById('projectList');
  const projectSearchEl = document.getElementById('projectSearch');
  const projectDetailBodyEl = document.getElementById('projectDetailBody');

  function renderClientList(filter=""){
    const q = filter.trim().toLowerCase();
    clientListEl.innerHTML = "";
    clients
      .filter(c => c.nom.toLowerCase().includes(q))
      .forEach(c => {
        const li = document.createElement('li');
        li.className = c.id === activeClientId ? 'active' : '';
        li.tabIndex = 0;
        li.innerHTML = `
          <span class="dot"></span><span class="name">${c.nom}</span>
          <button class="icon-btn danger" title="Supprimer ce client">${TRASH_SVG}</button>
        `;
        li.addEventListener('click', () => selectClient(c.id));
        li.querySelector('.icon-btn.danger').addEventListener('click', (e) => {
          e.stopPropagation();
          openConfirmModal('Supprimer le client',
            `Supprimer « ${c.nom} » et l'ensemble de ses projets (${c.projets.length}) ? Cette action est irréversible.`,
            () => deleteClient(c.id));
        });
        clientListEl.appendChild(li);
      });
  }

  function selectClient(id){
    activeClientId = id;
    activeProjectId = null;
    renderClientList(clientSearchEl.value);
    renderClientDetail();
    renderProjectDetail();
  }

  async function deleteClient(id){
    try {
      await invoke('delete_client', { id });
    } catch(err) {
      alert(String(err));
      return;
    }
    const idx = clients.findIndex(x => x.id === id);
    if(idx === -1) return;
    clients.splice(idx, 1);
    if(activeClientId === id){
      activeClientId = null;
      activeProjectId = null;
      if(clients.length){ selectClient(clients[0].id); }
      else { renderClientList(clientSearchEl.value); renderClientDetail(); renderProjectDetail(); }
    } else {
      renderClientList(clientSearchEl.value);
    }
  }

  function renderClientDetail(){
    const c = clients.find(x => x.id === activeClientId);
    if(!c){
      clientNameEl.textContent = "—";
      personalFieldsEl.innerHTML = `<div class="field"><div class="value empty">Aucun client sélectionné</div></div>`;
      projectListEl.innerHTML = "";
      return;
    }
    clientNameEl.textContent = c.nom;
    personalFieldsEl.innerHTML = `
      <div class="field"><label>Nom du groupe</label><div class="value">${c.nom}</div></div>
      <div class="field"><label>Responsable</label><div class="value">${c.responsable}</div></div>
      <div class="field"><label>Téléphone</label><div class="value">${c.tel}</div></div>
      <div class="field"><label>Quartier</label><div class="value">${c.quartier}</div></div>
      <div class="field"><label>Observation</label><div class="value obs ${c.obs ? '' : 'empty'}">${c.obs || 'Aucune observation'}</div></div>
    `;
    renderProjectList();
  }

  function renderProjectList(filter=""){
    const c = clients.find(x => x.id === activeClientId);
    projectListEl.innerHTML = "";
    if(!c) return;
    const q = filter.trim().toLowerCase();
    const filtered = c.projets.filter(p => p.titre.toLowerCase().includes(q));
    if(filtered.length === 0){
      projectListEl.innerHTML = `<li style="cursor:default; text-align:center; color:var(--ink-dim-solid); font-size:12.5px;">Aucun projet</li>`;
      return;
    }
    filtered.forEach(p => {
      const li = document.createElement('li');
      li.className = p.id === activeProjectId ? 'active' : '';
      li.tabIndex = 0;
      li.innerHTML = `
        <div class="p-row">
          <div class="p-title">${p.titre}</div>
          <button class="icon-btn danger" title="Supprimer ce projet">${TRASH_SVG}</button>
        </div>
        <div class="p-meta">
          <span>${p.genre}</span>
          <span class="status-chip ${statusClass[p.statut]}">${statusLabel[p.statut]}</span>
        </div>
      `;
      li.addEventListener('click', () => { activeProjectId = p.id; renderProjectList(projectSearchEl.value); renderProjectDetail(); });
      li.querySelector('.icon-btn.danger').addEventListener('click', (e) => {
        e.stopPropagation();
        openConfirmModal('Supprimer le projet',
          `Supprimer le projet « ${p.titre} » ? Cette action est irréversible.`,
          () => deleteProject(p.id));
      });
      projectListEl.appendChild(li);
    });
  }

  async function deleteProject(id){
    try {
      await invoke('delete_projet', { id });
    } catch(err) {
      alert(String(err));
      return;
    }
    const c = clients.find(x => x.id === activeClientId);
    if(!c) return;
    const idx = c.projets.findIndex(x => x.id === id);
    if(idx !== -1) c.projets.splice(idx, 1);
    if(activeProjectId === id){ activeProjectId = null; }
    renderProjectList(projectSearchEl.value);
    renderProjectDetail();
  }

  function renderProjectDetail(){
    const c = clients.find(x => x.id === activeClientId);
    const p = c && activeProjectId !== null ? c.projets.find(x => x.id === activeProjectId) : null;
    if(!p){
      projectDetailBodyEl.innerHTML = `<div class="project-empty">Sélectionnez un projet dans la liste pour afficher ses informations.</div>`;
      return;
    }
    projectDetailBodyEl.innerHTML = `
      <div class="field-list">
        <div class="field"><label>Titre</label><div class="value">${p.titre}</div></div>
        <div class="field"><label>Genre</label><div class="value">${p.genre}</div></div>
        <div class="field"><label>Statut</label><div class="value"><span class="status-chip ${statusClass[p.statut]}">${statusLabel[p.statut]}</span></div></div>
        <div class="field"><label>Avance</label><div class="value">${p.avance}</div></div>
        <div class="field"><label>Cachet</label><div class="value">${p.cachet}</div></div>
        <div class="field"><label>Date de création</label><div class="value">${p.creation}</div></div>
        <div class="field"><label>Date de livraison</label><div class="value">${p.livraison}</div></div>
      </div>
    `;
  }

  clientSearchEl.addEventListener('input', e => renderClientList(e.target.value));
  projectSearchEl.addEventListener('input', e => renderProjectList(e.target.value));

  // ==================== MODAL SYSTEM ====================
  const modalOverlay = document.getElementById('modalOverlay');
  const modalTitleEl = document.getElementById('modalTitle');
  const modalBodyEl = document.getElementById('modalBody');
  const modalCloseBtn = document.getElementById('modalClose');
  const modalCancelBtn = document.getElementById('modalCancel');
  const modalSubmitBtn = document.getElementById('modalSubmit');
  let modalSubmitHandler = null;

  function openModal(title, bodyHtml, onSubmit, options = {}){
    modalTitleEl.textContent = title;
    modalBodyEl.innerHTML = bodyHtml;
    modalSubmitHandler = onSubmit;
    modalSubmitBtn.textContent = options.submitLabel || 'Enregistrer';
    modalSubmitBtn.classList.toggle('btn-danger', !!options.danger);
    modalOverlay.classList.add('active');
    const firstInput = modalBodyEl.querySelector('input, select, textarea');
    if(firstInput) setTimeout(() => firstInput.focus(), 50);
  }
  function closeModal(){
    modalOverlay.classList.remove('active');
    modalBodyEl.innerHTML = '';
    modalSubmitHandler = null;
    modalSubmitBtn.textContent = 'Enregistrer';
    modalSubmitBtn.classList.remove('btn-danger');
  }
  modalCloseBtn.addEventListener('click', closeModal);
  modalCancelBtn.addEventListener('click', closeModal);
  modalOverlay.addEventListener('click', (e) => { if(e.target === modalOverlay) closeModal(); });
  document.addEventListener('keydown', (e) => { if(e.key === 'Escape' && modalOverlay.classList.contains('active')) closeModal(); });
  modalSubmitBtn.addEventListener('click', () => { if(modalSubmitHandler) modalSubmitHandler(); });

  function openConfirmModal(title, message, onConfirm){
    openModal(title, `<p style="margin:0; font-size:13.5px; line-height:1.6; color:var(--ink);">${message}</p>`,
      async () => { await onConfirm(); closeModal(); },
      { submitLabel: 'Supprimer', danger: true });
  }

  function isoToFr(iso){
    if(!iso) return '—';
    const [y, m, d] = iso.split('-');
    return `${d}/${m}/${y}`;
  }
  function frToIso(fr){
    if(!fr || fr === '—') return '';
    const [d, m, y] = fr.split('/');
    if(!d || !m || !y) return '';
    return `${y}-${m}-${d}`;
  }

  // ---- Ajouter un client ----
  document.getElementById('addClientBtn').addEventListener('click', () => {
    openModal('Nouveau client', `
      <div class="f-group">
        <label class="f-required">Nom du groupe</label>
        <input type="text" id="mNom" placeholder="ex. Kaiamba Quartet">
        <div class="modal-error" id="mNomError" style="display:none;">Le nom du groupe est obligatoire.</div>
      </div>
      <div class="f-group">
        <label>Responsable</label>
        <input type="text" id="mResponsable" placeholder="ex. Rija Andriamora">
      </div>
      <div class="f-group">
        <label>Téléphone</label>
        <input type="text" id="mTel" placeholder="ex. 034 12 345 67">
      </div>
      <div class="f-group">
        <label>Quartier</label>
        <input type="text" id="mQuartier" placeholder="ex. Analakely, Antananarivo">
      </div>
      <div class="f-group">
        <label>Observation</label>
        <textarea id="mObs" placeholder="Notes, préférences, remarques…"></textarea>
      </div>
    `, async () => {
      const nom = document.getElementById('mNom').value.trim();
      const errorEl = document.getElementById('mNomError');
      if(!nom){ errorEl.style.display = 'block'; return; }
      errorEl.style.display = 'none';
      try {
        const created = await invoke('create_client', { client: {
          nom,
          responsable: document.getElementById('mResponsable').value.trim(),
          telephone: document.getElementById('mTel').value.trim(),
          quartier: document.getElementById('mQuartier').value.trim(),
          observation: document.getElementById('mObs').value.trim(),
        }});
        clients.push(clientFromRow(created, []));
        selectClient(created.id);
        closeModal();
      } catch(err) {
        errorEl.textContent = String(err);
        errorEl.style.display = 'block';
      }
    });
  });

  // ---- Ajouter un projet ----
  document.getElementById('addProjectBtn').addEventListener('click', () => {
    const c = clients.find(x => x.id === activeClientId);
    if(!c){ closeModal(); alert("Sélectionnez d'abord un client avant d'ajouter un projet."); return; }

    openModal(`Nouveau projet — ${c.nom}`, `
      <div class="f-group">
        <label class="f-required">Titre du projet</label>
        <input type="text" id="mTitre" placeholder="ex. Tany Malalako — EP">
        <div class="modal-error" id="mTitreError" style="display:none;">Le titre est obligatoire.</div>
      </div>
      <div class="f-group">
        <label>Genre</label>
        <input type="text" id="mGenre" placeholder="ex. Salegy">
      </div>
      <div class="f-group">
        <label>État du projet</label>
        <select id="mStatut">
          <option value="attente">Prêt</option>
          <option value="en-cours">En cours</option>
          <option value="termine">Livré</option>
        </select>
      </div>
      <div class="f-group two-col">
        <div>
          <label>Date de création</label>
          <input type="date" id="mCreation">
        </div>
        <div>
          <label>Date de livraison</label>
          <input type="date" id="mLivraison">
        </div>
      </div>
      <div class="f-group two-col">
        <div>
          <label>Avance (Ar)</label>
          <input type="number" id="mAvance" placeholder="0" min="0">
        </div>
        <div>
          <label>Cachet (Ar)</label>
          <input type="number" id="mCachet" placeholder="0" min="0">
        </div>
      </div>
      <div class="f-group">
        <label>Statut de paiement</label>
        <select id="mPaiement">
          <option value="impaye">Impayé</option>
          <option value="paye">Payé</option>
        </select>
      </div>
      <div class="f-group" id="mSoldeGroup">
        <label>Reste à payer (Ar)</label>
        <input type="number" id="mSolde" placeholder="0" min="0">
      </div>
    `, async () => {
      const titre = document.getElementById('mTitre').value.trim();
      const errorEl = document.getElementById('mTitreError');
      if(!titre){ errorEl.style.display = 'block'; return; }
      errorEl.style.display = 'none';

      const genre = document.getElementById('mGenre').value.trim();
      const statut = document.getElementById('mStatut').value;
      const avanceNum = parseFloat(document.getElementById('mAvance').value) || 0;
      const cachetNum = parseFloat(document.getElementById('mCachet').value) || 0;
      const paiement = document.getElementById('mPaiement').value;
      const soldeInput = parseFloat(document.getElementById('mSolde').value) || 0;
      const soldeNum = paiement === 'impaye' ? (soldeInput || Math.max(cachetNum - avanceNum, 0)) : null;

      try {
        const created = await invoke('create_projet', { projet: {
          clientId: c.id,
          titre, genre, statut,
          dateCreation: document.getElementById('mCreation').value || null,
          dateLivraison: document.getElementById('mLivraison').value || null,
          avance: avanceNum,
          cachet: cachetNum,
          statutPaiement: paiement,
          solde: soldeNum,
        }});
        c.projets.push(projetFromRow(created));
        activeProjectId = created.id;
        renderProjectList(projectSearchEl.value);
        renderProjectDetail();
        closeModal();
      } catch(err) {
        errorEl.textContent = String(err);
        errorEl.style.display = 'block';
      }
    });

    // toggle solde field visibility based on paiement selection
    const paiementSel = document.getElementById('mPaiement');
    const soldeGroup = document.getElementById('mSoldeGroup');
    const syncSolde = () => { soldeGroup.style.display = paiementSel.value === 'impaye' ? '' : 'none'; };
    paiementSel.addEventListener('change', syncSolde);
    syncSolde();
  });

  // ---- Modifier le client sélectionné ----
  document.getElementById('editClientBtn').addEventListener('click', () => {
    const c = clients.find(x => x.id === activeClientId);
    if(!c){ alert("Sélectionnez d'abord un client à modifier."); return; }

    openModal(`Modifier — ${c.nom}`, `
      <div class="f-group">
        <label class="f-required">Nom du groupe</label>
        <input type="text" id="mNom" value="${c.nom.replace(/"/g,'&quot;')}">
        <div class="modal-error" id="mNomError" style="display:none;">Le nom du groupe est obligatoire.</div>
      </div>
      <div class="f-group">
        <label>Responsable</label>
        <input type="text" id="mResponsable" value="${c.responsable.replace(/"/g,'&quot;')}">
      </div>
      <div class="f-group">
        <label>Téléphone</label>
        <input type="text" id="mTel" value="${c.tel.replace(/"/g,'&quot;')}">
      </div>
      <div class="f-group">
        <label>Quartier</label>
        <input type="text" id="mQuartier" value="${c.quartier.replace(/"/g,'&quot;')}">
      </div>
      <div class="f-group">
        <label>Observation</label>
        <textarea id="mObs">${c.obs}</textarea>
      </div>
    `, async () => {
      const nom = document.getElementById('mNom').value.trim();
      const errorEl = document.getElementById('mNomError');
      if(!nom){ errorEl.style.display = 'block'; return; }
      errorEl.style.display = 'none';

      const payload = {
        id: c.id,
        nom,
        responsable: document.getElementById('mResponsable').value.trim(),
        telephone: document.getElementById('mTel').value.trim(),
        quartier: document.getElementById('mQuartier').value.trim(),
        observation: document.getElementById('mObs').value.trim(),
      };
      try {
        await invoke('update_client', { client: payload });
        c.nom = payload.nom;
        c.responsable = payload.responsable;
        c.tel = payload.telephone;
        c.quartier = payload.quartier;
        c.obs = payload.observation;
        renderClientList(clientSearchEl.value);
        renderClientDetail();
        closeModal();
      } catch(err) {
        errorEl.textContent = String(err);
        errorEl.style.display = 'block';
      }
    });
  });

  // ---- Modifier le projet sélectionné ----
  document.getElementById('editProjectBtn').addEventListener('click', () => {
    const c = clients.find(x => x.id === activeClientId);
    const p = c && activeProjectId !== null ? c.projets.find(x => x.id === activeProjectId) : null;
    if(!p){ alert("Sélectionnez d'abord un projet à modifier."); return; }

    openModal(`Modifier — ${p.titre}`, `
      <div class="f-group">
        <label class="f-required">Titre du projet</label>
        <input type="text" id="mTitre" value="${p.titre.replace(/"/g,'&quot;')}">
        <div class="modal-error" id="mTitreError" style="display:none;">Le titre est obligatoire.</div>
      </div>
      <div class="f-group">
        <label>Genre</label>
        <input type="text" id="mGenre" value="${p.genre.replace(/"/g,'&quot;')}">
      </div>
      <div class="f-group">
        <label>État du projet</label>
        <select id="mStatut">
          <option value="attente" ${p.statut === 'attente' ? 'selected' : ''}>Prêt</option>
          <option value="en-cours" ${p.statut === 'en-cours' ? 'selected' : ''}>En cours</option>
          <option value="termine" ${p.statut === 'termine' ? 'selected' : ''}>Livré</option>
        </select>
      </div>
      <div class="f-group two-col">
        <div>
          <label>Date de création</label>
          <input type="date" id="mCreation" value="${frToIso(p.creation)}">
        </div>
        <div>
          <label>Date de livraison</label>
          <input type="date" id="mLivraison" value="${frToIso(p.livraison)}">
        </div>
      </div>
      <div class="f-group two-col">
        <div>
          <label>Avance (Ar)</label>
          <input type="number" id="mAvance" value="${parseAr(p.avance)}" min="0">
        </div>
        <div>
          <label>Cachet (Ar)</label>
          <input type="number" id="mCachet" value="${parseAr(p.cachet)}" min="0">
        </div>
      </div>
      <div class="f-group">
        <label>Statut de paiement</label>
        <select id="mPaiement">
          <option value="impaye" ${p.paiement === 'impaye' ? 'selected' : ''}>Impayé</option>
          <option value="paye" ${p.paiement === 'paye' ? 'selected' : ''}>Payé</option>
        </select>
      </div>
      <div class="f-group" id="mSoldeGroup">
        <label>Reste à payer (Ar)</label>
        <input type="number" id="mSolde" value="${parseAr(p.solde || 0)}" min="0">
      </div>
    `, async () => {
      const titre = document.getElementById('mTitre').value.trim();
      const errorEl = document.getElementById('mTitreError');
      if(!titre){ errorEl.style.display = 'block'; return; }
      errorEl.style.display = 'none';

      const avanceNum = parseFloat(document.getElementById('mAvance').value) || 0;
      const cachetNum = parseFloat(document.getElementById('mCachet').value) || 0;
      const paiement = document.getElementById('mPaiement').value;
      const soldeInput = parseFloat(document.getElementById('mSolde').value) || 0;
      const soldeNum = paiement === 'impaye' ? (soldeInput || Math.max(cachetNum - avanceNum, 0)) : null;

      const payload = {
        id: p.id,
        clientId: c.id,
        titre,
        genre: document.getElementById('mGenre').value.trim(),
        statut: document.getElementById('mStatut').value,
        dateCreation: document.getElementById('mCreation').value || null,
        dateLivraison: document.getElementById('mLivraison').value || null,
        avance: avanceNum,
        cachet: cachetNum,
        statutPaiement: paiement,
        solde: soldeNum,
      };

      try {
        await invoke('update_projet', { projet: payload });
        p.titre = payload.titre;
        p.genre = payload.genre;
        p.statut = payload.statut;
        p.creation = isoToFr(payload.dateCreation);
        p.livraison = payload.dateLivraison ? isoToFr(payload.dateLivraison) : '—';
        p.avance = formatAr(payload.avance);
        p.cachet = formatAr(payload.cachet);
        p.paiement = payload.statutPaiement;
        if(payload.statutPaiement === 'impaye'){
          p.solde = formatAr(payload.solde);
        } else {
          delete p.solde;
        }
        renderProjectList(projectSearchEl.value);
        renderProjectDetail();
        closeModal();
      } catch(err) {
        errorEl.textContent = String(err);
        errorEl.style.display = 'block';
      }
    });

    const paiementSel = document.getElementById('mPaiement');
    const soldeGroup = document.getElementById('mSoldeGroup');
    const syncSolde = () => { soldeGroup.style.display = paiementSel.value === 'impaye' ? '' : 'none'; };
    paiementSel.addEventListener('change', syncSolde);
    syncSolde();
  });

  // ==================== EXPORT (PDF / XLSX) ====================
  // TODO Phase 8: réinstaller jspdf / jspdf-autotable / xlsx via npm et réactiver l'export.
  if(window.jspdf && window.applyPlugin){
    try { window.applyPlugin(window.jspdf.jsPDF); } catch(e) { console.error('AutoTable init failed', e); }
  }

  function currentClient(){
    return clients.find(x => x.id === activeClientId);
  }

  function projectExportRows(c){
    return c.projets.map(p => {
      const cachetNum = parseAr(p.cachet);
      const avanceNum = parseAr(p.avance);
      const resteNum = p.paiement === 'paye' ? 0 : (p.solde ? parseAr(p.solde) : Math.max(cachetNum - avanceNum, 0));
      return {
        titre: p.titre,
        genre: p.genre || '—',
        statut: statusLabel[p.statut] || p.statut,
        avance: p.avance,
        cachet: p.cachet,
        reste: resteNum > 0 ? formatAr(resteNum) : 'Soldé',
        paiement: p.paiement === 'paye' ? 'Payé' : 'Impayé',
        creation: p.creation,
        livraison: p.livraison,
      };
    });
  }

  function safeFileName(str){
    return str.normalize('NFD').replace(/[\u0300-\u036f]/g, '').replace(/[^\w\-]+/g, '_');
  }

  // ---- Export XLSX ----
  function exportClientXLSX(){
    const c = currentClient();
    if(!c){ alert("Sélectionnez d'abord un client à exporter."); return; }
    if(typeof XLSX === 'undefined'){ alert("Export XLSX indisponible pour le moment (voir Phase 8)."); return; }

    const wb = XLSX.utils.book_new();

    const wsInfo = XLSX.utils.aoa_to_sheet([
      ['BIG BLACK RECORDS — Fiche client'],
      [],
      ['Champ', 'Valeur'],
      ['Nom du groupe', c.nom],
      ['Responsable', c.responsable || '—'],
      ['Téléphone', c.tel || '—'],
      ['Quartier', c.quartier || '—'],
      ['Observation', c.obs || '—'],
    ]);
    wsInfo['!cols'] = [{ wch: 20 }, { wch: 50 }];
    XLSX.utils.book_append_sheet(wb, wsInfo, 'Client');

    const rows = projectExportRows(c);
    const wsProjets = rows.length
      ? XLSX.utils.json_to_sheet(rows, { header: ['titre','genre','statut','avance','cachet','reste','paiement','creation','livraison'] })
      : XLSX.utils.aoa_to_sheet([['Aucun projet pour ce client']]);
    if(rows.length){
      XLSX.utils.sheet_add_aoa(wsProjets, [[
        'Titre','Genre','Statut','Avance','Cachet','Reste à payer','Paiement','Date de création','Date de livraison'
      ]], { origin: 'A1' });
      wsProjets['!cols'] = [{wch:26},{wch:16},{wch:11},{wch:13},{wch:13},{wch:14},{wch:10},{wch:15},{wch:15}];
    }
    XLSX.utils.book_append_sheet(wb, wsProjets, 'Projets');

    XLSX.writeFile(wb, `${safeFileName(c.nom)}_fiche.xlsx`);
  }

  // ---- Export PDF ----
  function exportClientPDF(){
    const c = currentClient();
    if(!c){ alert("Sélectionnez d'abord un client à exporter."); return; }
    if(typeof window.jspdf === 'undefined'){ alert("Export PDF indisponible pour le moment (voir Phase 8)."); return; }

    const { jsPDF } = window.jspdf;
    const doc = new jsPDF({ unit: 'pt', format: 'a4' });
    const gold = [201, 161, 58];
    const ink = [30, 26, 16];
    const dim = [110, 100, 80];
    const margin = 40;

    doc.setFillColor(20, 17, 9);
    doc.rect(0, 0, 595, 90, 'F');
    doc.setTextColor(...gold);
    doc.setFont('helvetica', 'bold');
    doc.setFontSize(18);
    doc.text('BIG BLACK RECORDS', margin, 42);
    doc.setFontSize(10);
    doc.setTextColor(200, 190, 165);
    doc.text('Fiche client — Gestion & Projets', margin, 60);
    doc.setFontSize(9);
    doc.text(new Date().toLocaleDateString('fr-FR'), 595 - margin, 60, { align: 'right' });

    let y = 120;
    doc.setTextColor(...ink);
    doc.setFont('helvetica', 'bold');
    doc.setFontSize(20);
    doc.text(c.nom, margin, y);
    y += 30;

    const infoFields = [
      ['Responsable', c.responsable || '—'],
      ['Téléphone', c.tel || '—'],
      ['Quartier', c.quartier || '—'],
      ['Observation', c.obs || '—'],
    ];
    doc.setFontSize(10);
    infoFields.forEach(([label, value]) => {
      doc.setFont('helvetica', 'bold');
      doc.setTextColor(...dim);
      doc.text(label.toUpperCase(), margin, y);
      doc.setFont('helvetica', 'normal');
      doc.setTextColor(...ink);
      const lines = doc.splitTextToSize(String(value), 515 - margin);
      doc.text(lines, margin, y + 14);
      y += 14 + lines.length * 13 + 8;
    });

    y += 10;
    doc.setDrawColor(...gold);
    doc.setLineWidth(1);
    doc.line(margin, y, 595 - margin, y);
    y += 22;

    doc.setFont('helvetica', 'bold');
    doc.setFontSize(13);
    doc.setTextColor(...ink);
    doc.text('Projets', margin, y);
    y += 14;

    const rows = projectExportRows(c);
    if(rows.length === 0){
      doc.setFont('helvetica', 'normal');
      doc.setFontSize(10);
      doc.setTextColor(...dim);
      doc.text('Aucun projet enregistré pour ce client.', margin, y + 10);
    } else {
      doc.autoTable({
        startY: y,
        margin: { left: margin, right: margin },
        head: [['Titre', 'Genre', 'Statut', 'Avance', 'Cachet', 'Reste à payer', 'Paiement', 'Créé le', 'Livraison']],
        body: rows.map(r => [r.titre, r.genre, r.statut, r.avance, r.cachet, r.reste, r.paiement, r.creation, r.livraison]),
        styles: { font: 'helvetica', fontSize: 8.5, cellPadding: 6, textColor: ink, lineColor: [225, 218, 200] },
        headStyles: { fillColor: [20, 17, 9], textColor: gold, fontStyle: 'bold' },
        alternateRowStyles: { fillColor: [246, 242, 230] },
        theme: 'grid',
      });
    }

    doc.save(`${safeFileName(c.nom)}_fiche.pdf`);
  }

  document.getElementById('exportPdfBtn').addEventListener('click', exportClientPDF);
  document.getElementById('exportXlsxBtn').addEventListener('click', exportClientXLSX);

  // ---------- NAV TAB SWITCHING ----------
  document.querySelectorAll('.nav-tab').forEach(btn => {
    btn.addEventListener('click', () => {
      document.querySelectorAll('.nav-tab').forEach(b => b.classList.remove('active'));
      btn.classList.add('active');
      const tab = btn.dataset.tab;
      document.querySelectorAll('.view').forEach(v => v.classList.remove('active'));
      document.getElementById('view-' + tab).classList.add('active');
      if(tab === 'tdb') renderTDB();
      if(tab === 'filtre') renderFiltre();
    });
  });

  // console clock readout
  function tick(){
    document.getElementById('clock').textContent = new Date().toLocaleTimeString('fr-FR');
  }
  tick();
  setInterval(tick, 1000);

  // ==================== T.D.B DASHBOARD ====================
  function allProjets(){
    const out = [];
    clients.forEach(c => c.projets.forEach(p => out.push({ ...p, client: c.nom })));
    return out;
  }

  function renderQuartier(){
    const counts = {};
    clients.forEach(c => {
      const q = c.quartier.split(',')[0].trim();
      counts[q] = (counts[q] || 0) + 1;
    });
    const listEl = document.getElementById('quartierList');
    listEl.innerHTML = Object.entries(counts).map(([q, n]) => `
      <li><span class="qname">${q}</span><span class="qcount">${n}</span></li>
    `).join('');
    document.getElementById('quartierTotal').textContent = clients.length;
  }

  function renderProjetStats(){
    const projets = allProjets();
    const enCours = projets.filter(p => p.statut === 'en-cours').length;
    const pret = projets.filter(p => p.statut === 'attente').length;
    const livre = projets.filter(p => p.statut === 'termine').length;
    document.getElementById('projetStats').innerHTML = `
      <div class="stat"><span class="num">${enCours}</span><span class="lbl">En cours</span></div>
      <div class="stat"><span class="num">${pret}</span><span class="lbl">Prêt</span></div>
      <div class="stat"><span class="num">${livre}</span><span class="lbl">Livré</span></div>
    `;
  }

  function parseAr(str){
    return parseInt(String(str).replace(/[^\d]/g, ''), 10) || 0;
  }
  function formatAr(n){
    return n.toLocaleString('fr-FR') + ' Ar';
  }

  function renderPaiementStats(){
    const projets = allProjets();
    const payes = projets.filter(p => p.paiement === 'paye');
    const impayes = projets.filter(p => p.paiement === 'impaye');
    const totalPaye = payes.reduce((s, p) => s + parseAr(p.cachet), 0);
    const totalImpaye = impayes.reduce((s, p) => s + parseAr(p.solde || 0), 0);
    document.getElementById('paiementStats').innerHTML = `
      <div class="stat"><span class="num">${payes.length}</span><span class="lbl">Payé</span><span class="sub">${formatAr(totalPaye)}</span></div>
      <div class="stat"><span class="num">${impayes.length}</span><span class="lbl">Impayé</span><span class="sub">${formatAr(totalImpaye)}</span></div>
    `;
  }

  function renderImpayeList(){
    const projets = allProjets().filter(p => p.paiement === 'impaye');
    const el = document.getElementById('impayeList');
    if(projets.length === 0){
      el.innerHTML = `<div class="impaye-empty">Aucun projet impayé.</div>`;
      return;
    }
    el.innerHTML = projets.map(p => `
      <li>
        <div>
          <div class="il-title">${p.client} — ${p.titre}</div>
          <div class="il-genre">${p.genre}</div>
        </div>
        <div class="il-amount">${p.solde}</div>
      </li>
    `).join('');
  }

  // ---- Charts: seeded pseudo-random monthly data per year ----
  const MONTHS = ['J','F','M','A','M','J','J','A','S','O','N','D'];
  const YEARS = [2026, 2027, 2028, 2029];

  function seededSeries(year, base, spread){
    let seed = year * 97;
    const rnd = () => {
      seed = (seed * 9301 + 49297) % 233280;
      return seed / 233280;
    };
    return MONTHS.map((_, i) => Math.round(base + rnd() * spread + i * (spread / 24)));
  }

  const chartTheme = {
    ink: '#ece2c8',
    gold: '#c9a13a',
    line: '#3a3120',
    dim: '#a89873'
  };

  let salesYear = 2026, revenueYear = 2026;

  function buildYearTabs(container, chartKey){
    container.innerHTML = YEARS.map(y => `<button data-y="${y}" class="${(chartKey==='sales'?salesYear:revenueYear)===y ? 'active':''}">${y}</button>`).join('');
    container.querySelectorAll('button').forEach(btn => {
      btn.addEventListener('click', () => {
        container.querySelectorAll('button').forEach(b => b.classList.remove('active'));
        btn.classList.add('active');
        const y = parseInt(btn.dataset.y, 10);
        if(chartKey === 'sales'){ salesYear = y; renderSalesChart(); }
        else { revenueYear = y; renderRevenueChart(); }
      });
    });
  }

  // ---- dependency-free SVG chart rendering ----
  function niceMax(value){
    if(value <= 0) return 10;
    const magnitude = Math.pow(10, Math.floor(Math.log10(value)));
    const residual = value / magnitude;
    let niceResidual;
    if(residual <= 1) niceResidual = 1;
    else if(residual <= 2) niceResidual = 2;
    else if(residual <= 5) niceResidual = 5;
    else niceResidual = 10;
    return niceResidual * magnitude;
  }
  function formatShort(n){
    return n >= 1000 ? Math.round(n / 1000) + 'K' : String(n);
  }
  function chartGridAndLabels(w, h, padL, padR, padT, padB, max){
    const chartH = h - padT - padB;
    let grid = '', y = '';
    [0, 0.25, 0.5, 0.75, 1].forEach(f => {
      const gy = padT + chartH * (1 - f);
      grid += `<line x1="${padL}" y1="${gy.toFixed(1)}" x2="${w - padR}" y2="${gy.toFixed(1)}" stroke="var(--line)" stroke-width="1"/>`;
      y += `<text x="${padL - 6}" y="${(gy + 3).toFixed(1)}" text-anchor="end" font-family="'IBM Plex Mono',monospace" font-size="9" fill="var(--ink-dim-solid)">${formatShort(Math.round(max * f))}</text>`;
    });
    return grid + y;
  }

  function buildBarChartSVG(values){
    const w = 520, h = 220, padL = 34, padR = 10, padT = 14, padB = 26;
    const chartW = w - padL - padR, chartH = h - padT - padB;
    const max = niceMax(Math.max(...values, 1));
    const gap = 6;
    const barW = chartW / values.length - gap;
    let bars = '', labels = '';
    values.forEach((v, i) => {
      const bh = (v / max) * chartH;
      const x = padL + i * (barW + gap) + gap / 2;
      const y = padT + chartH - bh;
      bars += `<rect x="${x.toFixed(1)}" y="${y.toFixed(1)}" width="${barW.toFixed(1)}" height="${Math.max(bh,1).toFixed(1)}" rx="2.5" fill="var(--gold)"/>`;
      labels += `<text x="${(x + barW / 2).toFixed(1)}" y="${h - 8}" text-anchor="middle" font-family="'IBM Plex Mono',monospace" font-size="9" fill="var(--ink-dim-solid)">${MONTHS[i]}</text>`;
    });
    const grid = chartGridAndLabels(w, h, padL, padR, padT, padB, max);
    return `<svg viewBox="0 0 ${w} ${h}" preserveAspectRatio="none" style="width:100%;height:100%;">${grid}${bars}${labels}</svg>`;
  }

  function buildLineChartSVG(values){
    const w = 520, h = 220, padL = 40, padR = 10, padT = 14, padB = 26;
    const chartW = w - padL - padR, chartH = h - padT - padB;
    const max = niceMax(Math.max(...values, 1));
    const stepX = chartW / (values.length - 1);
    const pts = values.map((v, i) => [padL + i * stepX, padT + chartH - (v / max) * chartH]);
    const linePath = pts.map((p, i) => (i === 0 ? 'M' : 'L') + p[0].toFixed(1) + ',' + p[1].toFixed(1)).join(' ');
    const bottom = (padT + chartH).toFixed(1);
    const areaPath = `${linePath} L${pts[pts.length - 1][0].toFixed(1)},${bottom} L${pts[0][0].toFixed(1)},${bottom} Z`;
    const dots = pts.map(p => `<circle cx="${p[0].toFixed(1)}" cy="${p[1].toFixed(1)}" r="2.5" fill="var(--gold)"/>`).join('');
    const labels = MONTHS.map((m, i) => `<text x="${(padL + i * stepX).toFixed(1)}" y="${h - 8}" text-anchor="middle" font-family="'IBM Plex Mono',monospace" font-size="9" fill="var(--ink-dim-solid)">${m}</text>`).join('');
    const grid = chartGridAndLabels(w, h, padL, padR, padT, padB, max);
    return `<svg viewBox="0 0 ${w} ${h}" preserveAspectRatio="none" style="width:100%;height:100%;">${grid}<path d="${areaPath}" fill="rgba(201,161,58,0.14)" stroke="none"/><path d="${linePath}" fill="none" stroke="var(--gold)" stroke-width="2"/>${dots}${labels}</svg>`;
  }

  function renderSalesChart(){
    document.getElementById('salesChart').innerHTML = buildBarChartSVG(seededSeries(salesYear, 20, 60));
  }
  function renderRevenueChart(){
    document.getElementById('revenueChart').innerHTML = buildLineChartSVG(seededSeries(revenueYear, 40000, 140000));
  }

  // ---- Comparaison des dettes (balance) ----
  const dettesData = {
    mois:   { aLabel:'Mois -1', bLabel:'Mois',  a: 8, b: 3 },
    semaine:{ aLabel:'Sem. -1', bLabel:'Sem.',  a: 5, b: 4 },
    an:     { aLabel:'An -1',   bLabel:'An',    a: 12, b: 9 },
  };
  let activePeriode = 'mois';

  function renderScale(){
    const d = dettesData[activePeriode];
    const max = Math.max(d.a, d.b, 1);
    const tilt = ((d.b - d.a) / max) * 12; // degrees — heavier side (a or b) tilts down
    const wrap = document.getElementById('scaleWrap');
    wrap.innerHTML = `
      <svg viewBox="0 0 260 160" width="100%" height="150" style="max-width:260px;">
        <line x1="130" y1="18" x2="130" y2="130" stroke="${chartTheme.line}" stroke-width="4"/>
        <polygon points="105,140 155,140 145,150 115,150" fill="${chartTheme.line}"/>
        <g transform="rotate(${tilt} 130 28)">
          <line x1="30" y1="28" x2="230" y2="28" stroke="var(--gold-dim)" stroke-width="3"/>
          <line x1="55" y1="28" x2="55" y2="62" stroke="var(--gold-dim)" stroke-width="2"/>
          <line x1="205" y1="28" x2="205" y2="62" stroke="var(--gold-dim)" stroke-width="2"/>
          <path d="M30,62 Q55,90 80,62 Z" fill="none" stroke="var(--gold)" stroke-width="2.5"/>
          <path d="M180,62 Q205,90 230,62 Z" fill="none" stroke="var(--gold)" stroke-width="2.5"/>
        </g>
        <circle cx="130" cy="28" r="6" fill="var(--gold)"/>
        <text x="55" y="115" text-anchor="middle" fill="${chartTheme.ink}" font-family="'IBM Plex Mono', monospace" font-size="18" font-weight="600">${d.a}</text>
        <text x="55" y="132" text-anchor="middle" fill="${chartTheme.dim}" font-family="'IBM Plex Mono', monospace" font-size="10">${d.aLabel}</text>
        <text x="205" y="115" text-anchor="middle" fill="${chartTheme.ink}" font-family="'IBM Plex Mono', monospace" font-size="18" font-weight="600">${d.b}</text>
        <text x="205" y="132" text-anchor="middle" fill="${chartTheme.dim}" font-family="'IBM Plex Mono', monospace" font-size="10">${d.bLabel}</text>
      </svg>
    `;
  }

  document.querySelectorAll('.periode-tab').forEach(btn => {
    btn.addEventListener('click', () => {
      document.querySelectorAll('.periode-tab').forEach(b => b.classList.remove('active'));
      btn.classList.add('active');
      activePeriode = btn.dataset.p;
      renderScale();
    });
  });

  let tdbInitialized = false;
  function renderTDB(){
    renderQuartier();
    renderProjetStats();
    renderPaiementStats();
    renderImpayeList();
    renderScale();
    if(!tdbInitialized){
      buildYearTabs(document.querySelector('.year-tabs[data-chart="sales"]'), 'sales');
      buildYearTabs(document.querySelector('.year-tabs[data-chart="revenue"]'), 'revenue');
      tdbInitialized = true;
    }
    renderSalesChart();
    renderRevenueChart();
  }

  // ==================== FILTRE (RECHERCHE CROISÉE) ====================
  function allProjetsFull(){
    const out = [];
    clients.forEach(c => {
      c.projets.forEach(p => {
        const cachetNum = parseAr(p.cachet);
        const avanceNum = parseAr(p.avance);
        const resteNum = p.paiement === 'paye' ? 0 : (p.solde ? parseAr(p.solde) : Math.max(cachetNum - avanceNum, 0));
        out.push({
          ...p,
          clientId: c.id,
          clientNom: c.nom,
          clientTel: c.tel,
          clientQuartier: c.quartier.split(',')[0].trim(),
          avanceNum, resteNum,
        });
      });
    });
    return out;
  }

  function parseFrDate(str){
    if(!str || str === '—') return null;
    const parts = str.split('/').map(Number);
    if(parts.length !== 3 || parts.some(isNaN)) return null;
    const [d, m, y] = parts;
    return new Date(y, m - 1, d);
  }

  function populateQuartierSelect(){
    const sel = document.getElementById('fQuartier');
    const current = sel.value;
    const quartiers = [...new Set(clients.map(c => c.quartier.split(',')[0].trim()))].sort();
    sel.innerHTML = `<option value="">Tous les quartiers</option>` + quartiers.map(q => `<option value="${q}">${q}</option>`).join('');
    if(quartiers.includes(current)) sel.value = current;
  }

  const etatLabelFiltre = { 'en-cours':'En cours', 'attente':'Prêt', 'termine':'Livré' };

  function getFiltreState(){
    return {
      nomGroupe: document.getElementById('fNomGroupe').value.trim(),
      nomProjet: document.getElementById('fNomProjet').value.trim(),
      telephone: document.getElementById('fTelephone').value.trim(),
      quartier: document.getElementById('fQuartier').value,
      etat: document.getElementById('fEtat').value,
      creationFrom: document.getElementById('fCreationFrom').value,
      creationTo: document.getElementById('fCreationTo').value,
      livraisonFrom: document.getElementById('fLivraisonFrom').value,
      livraisonTo: document.getElementById('fLivraisonTo').value,
      avanceMin: document.getElementById('fAvanceMin').value,
      avanceMax: document.getElementById('fAvanceMax').value,
      resteMin: document.getElementById('fResteMin').value,
      resteMax: document.getElementById('fResteMax').value,
    };
  }

  function matchesFiltre(p, f){
    if(f.nomGroupe && !p.clientNom.toLowerCase().includes(f.nomGroupe.toLowerCase())) return false;
    if(f.nomProjet && !p.titre.toLowerCase().includes(f.nomProjet.toLowerCase())) return false;
    if(f.telephone && !p.clientTel.replace(/\s/g, '').includes(f.telephone.replace(/\s/g, ''))) return false;
    if(f.quartier && p.clientQuartier !== f.quartier) return false;
    if(f.etat && p.statut !== f.etat) return false;

    if(f.creationFrom || f.creationTo){
      const d = parseFrDate(p.creation);
      if(!d) return false;
      if(f.creationFrom && d < new Date(f.creationFrom)) return false;
      if(f.creationTo && d > new Date(f.creationTo)) return false;
    }
    if(f.livraisonFrom || f.livraisonTo){
      const d = parseFrDate(p.livraison);
      if(!d) return false;
      if(f.livraisonFrom && d < new Date(f.livraisonFrom)) return false;
      if(f.livraisonTo && d > new Date(f.livraisonTo)) return false;
    }
    if(f.avanceMin !== '' && p.avanceNum < parseFloat(f.avanceMin)) return false;
    if(f.avanceMax !== '' && p.avanceNum > parseFloat(f.avanceMax)) return false;
    if(f.resteMin !== '' && p.resteNum < parseFloat(f.resteMin)) return false;
    if(f.resteMax !== '' && p.resteNum > parseFloat(f.resteMax)) return false;

    return true;
  }

  function chipDefs(f){
    const chips = [];
    if(f.nomGroupe) chips.push({ key:'nomGroupe', label:`Groupe : ${f.nomGroupe}` });
    if(f.nomProjet) chips.push({ key:'nomProjet', label:`Projet : ${f.nomProjet}` });
    if(f.telephone) chips.push({ key:'telephone', label:`Tél : ${f.telephone}` });
    if(f.quartier) chips.push({ key:'quartier', label:`Quartier : ${f.quartier}` });
    if(f.etat) chips.push({ key:'etat', label:`État : ${etatLabelFiltre[f.etat]}` });
    if(f.creationFrom || f.creationTo) chips.push({ key:'creation', label:`Création : ${f.creationFrom || '…'} → ${f.creationTo || '…'}` });
    if(f.livraisonFrom || f.livraisonTo) chips.push({ key:'livraison', label:`Livraison : ${f.livraisonFrom || '…'} → ${f.livraisonTo || '…'}` });
    if(f.avanceMin || f.avanceMax) chips.push({ key:'avance', label:`Avance : ${f.avanceMin || '0'} → ${f.avanceMax || '∞'} Ar` });
    if(f.resteMin || f.resteMax) chips.push({ key:'reste', label:`Reste : ${f.resteMin || '0'} → ${f.resteMax || '∞'} Ar` });
    return chips;
  }

  function clearFiltreChip(key){
    const map = {
      nomGroupe:['fNomGroupe'], nomProjet:['fNomProjet'], telephone:['fTelephone'], quartier:['fQuartier'], etat:['fEtat'],
      creation:['fCreationFrom','fCreationTo'], livraison:['fLivraisonFrom','fLivraisonTo'],
      avance:['fAvanceMin','fAvanceMax'], reste:['fResteMin','fResteMax'],
    };
    (map[key] || []).forEach(id => document.getElementById(id).value = '');
    renderFiltre();
  }

  function jumpToProject(clientId, projetId){
    document.querySelectorAll('.nav-tab').forEach(b => b.classList.remove('active'));
    document.querySelector('.nav-tab[data-tab="client"]').classList.add('active');
    document.querySelectorAll('.view').forEach(v => v.classList.remove('active'));
    document.getElementById('view-client').classList.add('active');
    selectClient(clientId);
    activeProjectId = projetId;
    renderProjectList(projectSearchEl.value);
    renderProjectDetail();
  }

  function renderFiltre(){
    populateQuartierSelect();
    const f = getFiltreState();
    const results = allProjetsFull().filter(p => matchesFiltre(p, f));

    const chips = chipDefs(f);
    const chipsEl = document.getElementById('activeChips');
    chipsEl.innerHTML = chips.length
      ? chips.map(c => `<span class="chip">${c.label}<button data-key="${c.key}" aria-label="Retirer ce filtre">×</button></span>`).join('')
      : `<span class="chip-empty">Aucun filtre actif — tous les projets sont affichés</span>`;
    chipsEl.querySelectorAll('button').forEach(btn => btn.addEventListener('click', () => clearFiltreChip(btn.dataset.key)));

    document.getElementById('resultsCount').innerHTML = `<em>${results.length}</em> résultat${results.length !== 1 ? 's' : ''}`;

    const listEl = document.getElementById('resultsList');
    if(results.length === 0){
      listEl.innerHTML = `
        <div class="results-empty">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5"><circle cx="11" cy="11" r="7"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>
          <p>Aucun projet ne correspond à ces critères. Essaie de retirer ou d'assouplir un filtre.</p>
        </div>`;
      return;
    }

    listEl.innerHTML = results.map(p => `
      <div class="result-card" data-client="${p.clientId}" data-pid="${p.id}">
        <div class="rc-top">
          <div class="rc-client-block">
            <div class="rc-client">${p.clientNom}</div>
            <div class="rc-meta"><span>${p.clientTel}</span><span>${p.clientQuartier}</span></div>
          </div>
          <span class="status-chip ${statusClass[p.statut]}">${statusLabel[p.statut]}</span>
        </div>
        <div class="rc-project">${p.titre}</div>
        <div class="rc-genre">${p.genre}</div>
        <div class="rc-financials">
          <div class="rc-fin-item"><span class="l">Avance</span><span class="v">${p.avance}</span></div>
          <div class="rc-fin-item"><span class="l">Cachet</span><span class="v">${p.cachet}</span></div>
          <div class="rc-fin-item"><span class="l">Reste à payer</span><span class="v ${p.resteNum > 0 ? 'due' : ''}">${p.resteNum > 0 ? formatAr(p.resteNum) : 'Soldé'}</span></div>
        </div>
        <div class="rc-dates">
          <span><b>Créé le</b> ${p.creation}</span>
          <span><b>Livraison</b> ${p.livraison}</span>
        </div>
      </div>
    `).join('');

    listEl.querySelectorAll('.result-card').forEach(card => {
      card.addEventListener('click', () => {
        jumpToProject(parseInt(card.dataset.client, 10), parseInt(card.dataset.pid, 10));
      });
    });
  }

  ['fNomGroupe','fNomProjet','fTelephone'].forEach(id => document.getElementById(id).addEventListener('input', renderFiltre));
  ['fQuartier','fEtat'].forEach(id => document.getElementById(id).addEventListener('change', renderFiltre));
  ['fCreationFrom','fCreationTo','fLivraisonFrom','fLivraisonTo','fAvanceMin','fAvanceMax','fResteMin','fResteMax']
    .forEach(id => document.getElementById(id).addEventListener('input', renderFiltre));

  document.getElementById('resetFiltres').addEventListener('click', () => {
    document.querySelectorAll('.filtre-form input, .filtre-form select').forEach(el => el.value = '');
    renderFiltre();
  });

  // ==================== INIT ====================
  async function init(){
    clients = await loadClients();
    renderClientList();
    if(clients.length){ selectClient(clients[0].id); }
    else { renderClientDetail(); renderProjectDetail(); }
    renderTDB();
    renderFiltre();
  }
  init();
