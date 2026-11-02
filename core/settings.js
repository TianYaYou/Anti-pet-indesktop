/**
 * Anti-Pet In Desktop - Native Settings Controller
 * Handles configuration persistence, autostart, skin management & dialogues
 */

(function () {
  'use strict';

  // Desktop IPC Helper
  const isDesktop = typeof window.ipc !== 'undefined' && typeof window.ipc.postMessage === 'function';

  function closeNativeWindow() {
    if (isDesktop) {
      window.ipc.postMessage('close_settings');
    } else {
      window.close();
    }
  }

  // DOM Elements
  const tabButtons = document.querySelectorAll('.nav-item');
  const tabPanes = document.querySelectorAll('.tab-pane');
  const macHeaderTitle = document.getElementById('mac-header-title');
  const navPrev = document.getElementById('nav-prev');
  const navNext = document.getElementById('nav-next');
  const sidebarSearch = document.getElementById('sidebar-search');

  const tabHistory = ['general'];
  let historyIndex = 0;

  const TAB_TITLES = {
    general: '常规与显示',
    system: '自启动与更新',
    skins: '皮肤管理',
    dialogue: '台词与问候',
    menu: '右键菜单'
  };

  function updateNavArrows() {
    if (navPrev) navPrev.disabled = historyIndex <= 0;
    if (navNext) navNext.disabled = historyIndex >= tabHistory.length - 1;
  }

  function switchToTab(targetTab, addToHistory = true) {
    if (!targetTab) return;
    tabButtons.forEach(b => {
      const isTarget = b.getAttribute('data-tab') === targetTab;
      b.classList.toggle('active', isTarget);
    });
    tabPanes.forEach(pane => {
      pane.classList.toggle('active', pane.id === `tab-${targetTab}`);
    });
    if (macHeaderTitle) {
      macHeaderTitle.textContent = TAB_TITLES[targetTab] || '偏好设置';
    }
    if (addToHistory) {
      if (tabHistory[historyIndex] !== targetTab) {
        tabHistory.splice(historyIndex + 1);
        tabHistory.push(targetTab);
        historyIndex = tabHistory.length - 1;
      }
    }
    updateNavArrows();
  }

  // Controls - General
  const settingScale = document.getElementById('setting-scale');
  const settingScaleVal = document.getElementById('setting-scale-val');
  const settingFloorMode = document.getElementById('setting-floor-mode');
  const settingGravity = document.getElementById('setting-gravity');
  const settingShowQuota = document.getElementById('setting-show-quota');
  const settingIdeOnly = document.getElementById('setting-ide-only');
  const settingDnd = document.getElementById('setting-dnd');
  const settingBubbleDuration = document.getElementById('setting-bubble-duration');
  const settingBubbleDurationVal = document.getElementById('setting-bubble-duration-val');
  const settingSound = document.getElementById('setting-sound');
  const settingVolume = document.getElementById('setting-volume');
  const settingVolumeVal = document.getElementById('setting-volume-val');

  // Controls - System & Autostart
  const settingAutostart = document.getElementById('setting-autostart');
  const autostartStatusNote = document.getElementById('autostart-status-note');
  const btnCheckUpdate = document.getElementById('btn-check-update');
  const updateStatusNote = document.getElementById('update-status-note');

  // Controls - Skins
  const skinsGrid = document.getElementById('skins-grid');
  const btnToggleCreateSkin = document.getElementById('btn-toggle-create-skin');
  const btnToggleImportSkin = document.getElementById('btn-toggle-import-skin');
  const createSkinForm = document.getElementById('create-skin-form');
  const importSkinForm = document.getElementById('import-skin-form');
  const newSkinId = document.getElementById('new-skin-id');
  const newSkinName = document.getElementById('new-skin-name');
  const newSkinAuthor = document.getElementById('new-skin-author');
  const newSkinDesc = document.getElementById('new-skin-desc');
  const btnConfirmCreateSkin = document.getElementById('btn-confirm-create-skin');
  const btnCancelCreateSkin = document.getElementById('btn-cancel-create-skin');
  const importSkinJson = document.getElementById('import-skin-json');
  const importSkinFile = document.getElementById('import-skin-file');
  const importSkinFrameFile = document.getElementById('import-skin-frame-file');
  const importDetectedFormat = document.getElementById('import-detected-format');
  const btnConfirmImportSkin = document.getElementById('btn-confirm-import-skin');
  const btnCancelImportSkin = document.getElementById('btn-cancel-import-skin');

  let importedFrameBase64 = null;

  const MENU_KEYS = [
    'openAntigravity',
    'settings',
    'quota',
    'randomDialogue',
    'petAction',
    'zoom',
    'dnd',
    'ideOnly',
    'gravity',
    'switchSkin',
    'sound'
  ];

  // Controls - Dialogue
  const dialogueSkinSelector = document.getElementById('dialogue-skin-selector');
  const btnAddLines = document.querySelectorAll('.btn-add-line');

  // Actions
  const btnCancel = document.getElementById('btn-cancel');
  const btnSave = document.getElementById('btn-save');
  const toastEl = document.getElementById('settings-toast');

  // Local State
  let initialSettings = null;
  let activeSkinId = 'fat-cat';
  let activeDialogueSkinId = 'fat-cat';
  let currentDialogues = {};
  let originalAutostart = false;

  function showToast(msg) {
    if (!toastEl) return;
    toastEl.textContent = msg;
    toastEl.classList.add('show');
    setTimeout(() => {
      toastEl.classList.remove('show');
    }, 2200);
  }

  // Tab Navigation & History Arrows
  tabButtons.forEach(btn => {
    btn.addEventListener('click', () => {
      const targetTab = btn.getAttribute('data-tab');
      switchToTab(targetTab, true);
    });
  });

  if (navPrev) {
    navPrev.addEventListener('click', () => {
      if (historyIndex > 0) {
        historyIndex -= 1;
        switchToTab(tabHistory[historyIndex], false);
      }
    });
  }

  if (navNext) {
    navNext.addEventListener('click', () => {
      if (historyIndex < tabHistory.length - 1) {
        historyIndex += 1;
        switchToTab(tabHistory[historyIndex], false);
      }
    });
  }

  // Sidebar Search
  if (sidebarSearch) {
    sidebarSearch.addEventListener('input', () => {
      const q = sidebarSearch.value.trim().toLowerCase();
      const menuRows = document.querySelectorAll('.mac-menu-row');
      menuRows.forEach(row => {
        const text = (row.textContent || '').toLowerCase();
        row.style.display = !q || text.includes(q) ? '' : 'none';
      });
      const settingsRows = document.querySelectorAll('.settings-row');
      settingsRows.forEach(row => {
        const text = (row.textContent || '').toLowerCase();
        row.style.display = !q || text.includes(q) ? '' : 'none';
      });
    });
  }

  // Sliders Dynamic Value Updates
  if (settingScale && settingScaleVal) {
    settingScale.addEventListener('input', () => {
      settingScaleVal.textContent = `${Math.round(parseFloat(settingScale.value) * 100)}%`;
    });
  }

  if (settingBubbleDuration && settingBubbleDurationVal) {
    settingBubbleDuration.addEventListener('input', () => {
      settingBubbleDurationVal.textContent = `${(parseInt(settingBubbleDuration.value, 10) / 1000).toFixed(1)}s`;
    });
  }

  if (settingVolume && settingVolumeVal) {
    settingVolume.addEventListener('input', () => {
      settingVolumeVal.textContent = `${Math.round(parseFloat(settingVolume.value) * 100)}%`;
    });
  }

  // Load Settings
  async function loadSettings() {
    try {
      const res = await fetch('/api/settings');
      if (!res.ok) throw new Error('HTTP ' + res.status);
      const data = await res.json();
      initialSettings = { ...data };

      if (settingScale) {
        settingScale.value = data.scale || 1.0;
        if (settingScaleVal) settingScaleVal.textContent = `${Math.round((data.scale || 1.0) * 100)}%`;
      }
      if (settingFloorMode) settingFloorMode.value = data.floorMode || 'bottom';
      if (settingGravity) settingGravity.checked = data.gravityEnabled !== false;
      if (settingShowQuota) settingShowQuota.checked = data.showQuota !== false;
      if (settingIdeOnly) settingIdeOnly.checked = !!data.ideOnlyMode;
      if (settingDnd) settingDnd.checked = !!data.dndPassthrough;
      if (settingBubbleDuration) {
        settingBubbleDuration.value = data.bubbleDurationMs || 4000;
        if (settingBubbleDurationVal) {
          settingBubbleDurationVal.textContent = `${((data.bubbleDurationMs || 4000) / 1000).toFixed(1)}s`;
        }
      }
      if (settingSound) settingSound.checked = data.soundEnabled !== false;
      if (settingVolume) {
        settingVolume.value = data.soundVolume !== undefined ? data.soundVolume : 0.8;
        if (settingVolumeVal) settingVolumeVal.textContent = `${Math.round((data.soundVolume !== undefined ? data.soundVolume : 0.8) * 100)}%`;
      }

      activeSkinId = data.skinId || 'fat-cat';
      activeDialogueSkinId = activeSkinId;

      const menuConfig = data.menuConfig || {};
      MENU_KEYS.forEach((key) => {
        const chk = document.getElementById(`menu-cfg-${key}`);
        const sel = document.getElementById(`menu-select-${key}`);
        const row = document.querySelector(`.mac-menu-row[data-key="${key}"]`);
        const val = menuConfig[key];

        if (val === false || val === 'never') {
          if (chk) chk.checked = false;
          if (sel) sel.value = 'never';
          if (row) row.classList.add('row-disabled');
        } else if (val === 'auto') {
          if (chk) chk.checked = true;
          if (sel) sel.value = 'auto';
          if (row) row.classList.remove('row-disabled');
        } else if (val === 'active') {
          if (chk) chk.checked = true;
          if (sel) sel.value = 'active';
          if (row) row.classList.remove('row-disabled');
        } else {
          // true or 'always' (default)
          if (chk) chk.checked = true;
          if (sel) sel.value = key === 'openAntigravity' ? 'auto' : 'always';
          if (row) row.classList.remove('row-disabled');
        }
      });
    } catch (e) {
      console.error('加载设置失败:', e);
    }
  }

  function syncMenuRow(key) {
    const chk = document.getElementById(`menu-cfg-${key}`);
    const sel = document.getElementById(`menu-select-${key}`);
    const row = document.querySelector(`.mac-menu-row[data-key="${key}"]`);
    if (!chk || !sel) return;

    if (sel.value === 'never') {
      chk.checked = false;
      if (row) row.classList.add('row-disabled');
    } else {
      chk.checked = true;
      if (row) row.classList.remove('row-disabled');
    }
  }

  function setupMenuControls() {
    MENU_KEYS.forEach((key) => {
      const chk = document.getElementById(`menu-cfg-${key}`);
      const sel = document.getElementById(`menu-select-${key}`);
      const row = document.querySelector(`.mac-menu-row[data-key="${key}"]`);

      if (chk) {
        chk.addEventListener('change', () => {
          if (chk.checked) {
            if (sel && sel.value === 'never') {
              sel.value = key === 'openAntigravity' ? 'auto' : 'always';
            }
            if (row) row.classList.remove('row-disabled');
          } else {
            if (sel) sel.value = 'never';
            if (row) row.classList.add('row-disabled');
          }
        });
      }

      if (sel) {
        sel.addEventListener('change', () => {
          syncMenuRow(key);
        });
      }
    });
  }

  // Load Autostart Info
  async function loadAutostartInfo() {
    if (!autostartStatusNote) return;
    try {
      const res = await fetch('/api/autostart');
      if (res.ok) {
        const data = await res.json();
        originalAutostart = !!data.enabled;
        if (settingAutostart) settingAutostart.checked = originalAutostart;
        if (data.available) {
          autostartStatusNote.className = data.enabled ? 'status-bar success' : 'status-bar neutral';
          autostartStatusNote.textContent = data.enabled
            ? `已启用开机自启 (${data.platform})`
            : `未开启自启，点击开关可在系统登录时自动启动伴侣 (${data.platform})`;
        } else {
          autostartStatusNote.className = 'status-bar warn';
          autostartStatusNote.textContent = '当前平台未检测到原生自启管理器';
        }
      }
    } catch (e) {
      autostartStatusNote.className = 'status-bar warn';
      autostartStatusNote.textContent = '读取自启配置异常';
    }
  }

  // Check Update
  if (btnCheckUpdate) {
    btnCheckUpdate.addEventListener('click', async () => {
      if (!updateStatusNote) return;
      btnCheckUpdate.disabled = true;
      updateStatusNote.textContent = '正在联网检查 GitHub 最新发布版本...';
      try {
        const res = await fetch('/api/update-check');
        const data = await res.json();
        if (data.hasUpdate) {
          updateStatusNote.className = 'status-bar success';
          updateStatusNote.textContent = `发现新版本: ${data.remoteVersion} (当前为 ${data.localVersion})`;
        } else {
          updateStatusNote.className = 'status-bar neutral';
          updateStatusNote.textContent = `当前已是最新版本 (${data.localVersion})`;
        }
      } catch (e) {
        updateStatusNote.className = 'status-bar warn';
        updateStatusNote.textContent = '检查更新失败，请确认网络连接';
      } finally {
        btnCheckUpdate.disabled = false;
      }
    });
  }

  // Load Skins
  async function loadSkins() {
    if (!skinsGrid) return;
    try {
      const res = await fetch('/api/skins');
      if (!res.ok) return;
      const skins = await res.json();

      skinsGrid.innerHTML = '';
      if (dialogueSkinSelector) dialogueSkinSelector.innerHTML = '';

      skins.forEach(skin => {
        // Dialogue Selector option
        if (dialogueSkinSelector) {
          const opt = document.createElement('option');
          opt.value = skin.id;
          opt.textContent = `${skin.name} (${skin.id})`;
          if (skin.id === activeDialogueSkinId) opt.selected = true;
          dialogueSkinSelector.appendChild(opt);
        }

        // Skin Card
        const card = document.createElement('div');
        card.className = `skin-card ${skin.id === activeSkinId ? 'active' : ''}`;
        card.setAttribute('data-skin-id', skin.id);

        const previewSrc = skin.preview || `/skins/${skin.id}/frames/idle.png`;
        card.innerHTML = `
          <img class="skin-preview-img" src="${previewSrc}" alt="${skin.name}">
          <div class="skin-name">${skin.name}</div>
          <div class="skin-author">${skin.author || '未知作者'}</div>
          ${skin.id === activeSkinId ? '<span class="skin-status-tag">当前生效</span>' : ''}
        `;

        card.addEventListener('click', () => {
          activeSkinId = skin.id;
          document.querySelectorAll('.skin-card').forEach(c => {
            const isMatch = c.getAttribute('data-skin-id') === skin.id;
            c.classList.toggle('active', isMatch);
            const tag = c.querySelector('.skin-status-tag');
            if (isMatch && !tag) {
              const newTag = document.createElement('span');
              newTag.className = 'skin-status-tag';
              newTag.textContent = '当前生效';
              c.appendChild(newTag);
            } else if (!isMatch && tag) {
              tag.remove();
            }
          });
        });

        skinsGrid.appendChild(card);
      });
    } catch (e) {
      console.error('加载皮肤列表异常:', e);
    }
  }

  // Create Skin Toggle
  if (btnToggleCreateSkin && createSkinForm) {
    btnToggleCreateSkin.addEventListener('click', () => {
      createSkinForm.classList.toggle('hidden');
      if (importSkinForm) importSkinForm.classList.add('hidden');
    });
  }

  if (btnCancelCreateSkin && createSkinForm) {
    btnCancelCreateSkin.addEventListener('click', () => {
      createSkinForm.classList.add('hidden');
    });
  }

  if (btnConfirmCreateSkin) {
    btnConfirmCreateSkin.addEventListener('click', async () => {
      const id = (newSkinId.value || '').trim();
      const name = (newSkinName.value || '').trim();
      const author = (newSkinAuthor.value || '').trim();
      const desc = (newSkinDesc.value || '').trim();

      if (!id || !/^[a-zA-Z0-9_-]+$/.test(id)) {
        showToast('皮肤 ID 仅限英文字母、数字及连字符');
        return;
      }
      if (!name) {
        showToast('请填写皮肤名称');
        return;
      }

      btnConfirmCreateSkin.disabled = true;
      try {
        const res = await fetch('/api/skins', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ skinId: id, name, author, description: desc })
        });
        const data = await res.json();
        if (data.ok) {
          showToast(`已创建皮肤模版: ${name}`);
          createSkinForm.classList.add('hidden');
          newSkinId.value = '';
          newSkinName.value = '';
          newSkinAuthor.value = '';
          newSkinDesc.value = '';
          await loadSkins();
        } else {
          showToast(`创建失败: ${data.error || '未知错误'}`);
        }
      } catch (e) {
        showToast('网络请求失败');
      } finally {
        btnConfirmCreateSkin.disabled = false;
      }
    });
  }

  // Helper: Detect import format (dsh-pet / Codex vs Anti-Pet)
  function checkImportFormat(text) {
    if (!importDetectedFormat) return;
    try {
      const parsed = JSON.parse(text);
      if (parsed.displayName || parsed.spritesheetPath || parsed.frame) {
        importDetectedFormat.className = 'status-bar success';
        importDetectedFormat.textContent = `检测到 dsh-pet / Codex 宠物格式 [${parsed.displayName || parsed.name || '未知'}]，导入时将自动兼容转换！`;
        importDetectedFormat.classList.remove('hidden');
      } else if (parsed.schemaVersion || (parsed.frames && parsed.states)) {
        importDetectedFormat.className = 'status-bar neutral';
        importDetectedFormat.textContent = `检测到 Anti-Pet 标准皮肤格式 [${parsed.name || parsed.id || '标准皮肤'}]`;
        importDetectedFormat.classList.remove('hidden');
      } else {
        importDetectedFormat.classList.add('hidden');
      }
    } catch (e) {
      importDetectedFormat.classList.add('hidden');
    }
  }

  if (importSkinJson) {
    importSkinJson.addEventListener('input', () => {
      checkImportFormat(importSkinJson.value.trim());
    });
  }

  if (importSkinFile) {
    importSkinFile.addEventListener('change', () => {
      const file = importSkinFile.files && importSkinFile.files[0];
      if (file) {
        const reader = new FileReader();
        reader.onload = (e) => {
          importSkinJson.value = e.target.result;
          checkImportFormat(e.target.result);
        };
        reader.readAsText(file);
      }
    });
  }

  if (importSkinFrameFile) {
    importSkinFrameFile.addEventListener('change', () => {
      const file = importSkinFrameFile.files && importSkinFrameFile.files[0];
      if (file) {
        const reader = new FileReader();
        reader.onload = (e) => {
          importedFrameBase64 = e.target.result;
          showToast(`已载入立绘图片: ${file.name}`);
        };
        reader.readAsDataURL(file);
      }
    });
  }

  // Import Skin Toggle
  if (btnToggleImportSkin && importSkinForm) {
    btnToggleImportSkin.addEventListener('click', () => {
      importSkinForm.classList.toggle('hidden');
      if (createSkinForm) createSkinForm.classList.add('hidden');
    });
  }

  if (btnCancelImportSkin && importSkinForm) {
    btnCancelImportSkin.addEventListener('click', () => {
      importSkinForm.classList.add('hidden');
      importedFrameBase64 = null;
      if (importSkinFile) importSkinFile.value = '';
      if (importSkinFrameFile) importSkinFrameFile.value = '';
      if (importDetectedFormat) importDetectedFormat.classList.add('hidden');
    });
  }

  if (btnConfirmImportSkin) {
    btnConfirmImportSkin.addEventListener('click', async () => {
      const content = (importSkinJson.value || '').trim();
      if (!content) {
        showToast('请选择配置文件或粘贴配置代码');
        return;
      }
      try {
        const parsed = JSON.parse(content);
        btnConfirmImportSkin.disabled = true;
        btnConfirmImportSkin.textContent = '导入中...';

        const res = await fetch('/api/skins/import', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            config: parsed,
            frameBase64: importedFrameBase64
          })
        });
        const data = await res.json();
        if (data.ok) {
          const skinName = (data.skin && data.skin.name) || '新皮肤';
          showToast(data.isDshPet ? `成功导入并适配 dsh-pet 宠物: ${skinName}` : `成功导入皮肤: ${skinName}`);
          importSkinForm.classList.add('hidden');
          importSkinJson.value = '';
          importedFrameBase64 = null;
          if (importSkinFile) importSkinFile.value = '';
          if (importSkinFrameFile) importSkinFrameFile.value = '';
          if (importDetectedFormat) importDetectedFormat.classList.add('hidden');
          await loadSkins();
        } else {
          showToast(`导入失败: ${data.error || '未知错误'}`);
        }
      } catch (e) {
        showToast('JSON 格式解析错误，请检查语法');
      } finally {
        btnConfirmImportSkin.disabled = false;
        btnConfirmImportSkin.textContent = '确认导入';
      }
    });
  }

  // Load Dialogue for Skin
  async function loadDialogue(skinId) {
    try {
      const res = await fetch(`/api/dialogue?skinId=${encodeURIComponent(skinId)}`);
      if (!res.ok) return;
      const data = await res.json();
      currentDialogues = (data && data.lines) ? data.lines : {};

      ['idle', 'petting', 'success', 'thinking', 'error'].forEach(cat => {
        renderDialogueCategory(cat);
      });
    } catch (e) {
      console.error('加载台词失败:', e);
    }
  }

  function renderDialogueCategory(cat) {
    const container = document.getElementById(`lines-${cat}`);
    if (!container) return;
    container.innerHTML = '';

    const lines = currentDialogues[cat] || [];
    lines.forEach((line, idx) => {
      const row = document.createElement('div');
      row.className = 'line-item-row';
      row.innerHTML = `
        <input type="text" class="native-input line-input" value="${escapeHtml(line)}" data-cat="${cat}" data-idx="${idx}">
        <button class="native-btn danger btn-remove-line" data-cat="${cat}" data-idx="${idx}">删除</button>
      `;

      const input = row.querySelector('.line-input');
      input.addEventListener('input', () => {
        currentDialogues[cat][idx] = input.value;
      });

      const removeBtn = row.querySelector('.btn-remove-line');
      removeBtn.addEventListener('click', () => {
        currentDialogues[cat].splice(idx, 1);
        renderDialogueCategory(cat);
      });

      container.appendChild(row);
    });
  }

  function escapeHtml(str) {
    return String(str)
      .replace(/&/g, '&amp;')
      .replace(/"/g, '&quot;')
      .replace(/'/g, '&#39;')
      .replace(/</g, '&lt;')
      .replace(/>/g, '&gt;');
  }

  // Add line to category
  btnAddLines.forEach(btn => {
    btn.addEventListener('click', () => {
      const cat = btn.getAttribute('data-cat');
      if (!cat) return;
      if (!currentDialogues[cat]) currentDialogues[cat] = [];
      currentDialogues[cat].push('新台词条目');
      renderDialogueCategory(cat);
      const container = document.getElementById(`lines-${cat}`);
      if (container) {
        const lastInput = container.querySelector('.line-item-row:last-child .line-input');
        if (lastInput) {
          lastInput.focus();
          lastInput.select();
        }
      }
    });
  });

  // Dialogue Skin Selector change
  if (dialogueSkinSelector) {
    dialogueSkinSelector.addEventListener('change', () => {
      activeDialogueSkinId = dialogueSkinSelector.value;
      loadDialogue(activeDialogueSkinId);
    });
  }

  // Cancel Button
  if (btnCancel) {
    btnCancel.addEventListener('click', () => {
      closeNativeWindow();
    });
  }

  // Save Button
  if (btnSave) {
    btnSave.addEventListener('click', async () => {
      btnSave.disabled = true;
      btnSave.textContent = '保存中...';

      try {
        const menuConfig = {};
        MENU_KEYS.forEach((key) => {
          const chk = document.getElementById(`menu-cfg-${key}`);
          const sel = document.getElementById(`menu-select-${key}`);
          if (sel && sel.value) {
            menuConfig[key] = sel.value;
          } else if (chk) {
            menuConfig[key] = chk.checked ? (key === 'openAntigravity' ? 'auto' : 'always') : 'never';
          }
        });

        const newSettings = {
          scale: parseFloat(settingScale.value) || 1.0,
          floorMode: settingFloorMode.value || 'bottom',
          gravityEnabled: settingGravity.checked,
          showQuota: settingShowQuota.checked,
          ideOnlyMode: settingIdeOnly.checked,
          dndPassthrough: settingDnd.checked,
          bubbleDurationMs: parseInt(settingBubbleDuration.value, 10) || 4000,
          soundEnabled: settingSound.checked,
          soundVolume: parseFloat(settingVolume.value) || 0.8,
          skinId: activeSkinId,
          menuConfig
        };

        // 1. Save general settings
        await fetch('/api/settings', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(newSettings)
        });

        // 2. Save autostart if changed
        if (settingAutostart && settingAutostart.checked !== originalAutostart) {
          await fetch('/api/autostart', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ enabled: settingAutostart.checked })
          });
          originalAutostart = settingAutostart.checked;
        }

        // 3. Save dialogues for active dialogue skin
        if (activeDialogueSkinId && currentDialogues) {
          await fetch('/api/dialogue', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({
              skinId: activeDialogueSkinId,
              lines: currentDialogues
            })
          });
        }

        showToast('设置已保存并生效');

        setTimeout(() => {
          closeNativeWindow();
        }, 250);
      } catch (e) {
        showToast('保存设置失败');
      } finally {
        btnSave.disabled = false;
        btnSave.textContent = '保存';
      }
    });
  }

  // Keyboard Shortcuts
  window.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') {
      closeNativeWindow();
    } else if ((e.metaKey || e.ctrlKey) && e.key === 's') {
      e.preventDefault();
      if (btnSave) btnSave.click();
    }
  });

  // Initialization
  async function init() {
    setupMenuControls();
    await loadSettings();
    await loadAutostartInfo();
    await loadSkins();
    if (activeDialogueSkinId) {
      await loadDialogue(activeDialogueSkinId);
    }
  }

  init();
})();
