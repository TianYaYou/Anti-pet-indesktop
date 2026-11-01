/**
 * Anti-Pet Core Engine - State machine, physics, audio & interactions
 */
(function() {
  const container = document.getElementById('pet-container');
  const petFrame = document.getElementById('pet-frame');
  const speechBubble = document.getElementById('speech-bubble');
  const bubbleText = document.getElementById('bubble-text');
  const effectLayer = document.getElementById('effect-layer');
  const statusBadge = document.getElementById('status-badge');
  const badgeLabel = document.getElementById('badge-label');
  const quotaSection = document.getElementById('quota-section');
  const quota5hBar = document.getElementById('quota-5h');
  const quota5hVal = document.getElementById('quota-5h-val');
  const quotaWeekBar = document.getElementById('quota-week');
  const quotaWeekVal = document.getElementById('quota-week-val');
  const contextMenu = document.getElementById('context-menu');
  const soundStatus = document.getElementById('sound-status');

  // Application State
  const state = {
    skinId: 'fat-cat',
    skinConfig: null,
    dialogues: [],
    dialogueIndex: 0,
    currentState: 'idle',
    soundEnabled: true,
    facing: 'left', // 'left' or 'right'
    isDragging: false,
    dragStartX: 0,
    dragStartY: 0,
    petInitialX: 0,
    petInitialY: 0,
    hasMoved: false,
    stateTimer: null,
    idleDialogueTimer: null,
    audioContext: null,
    audioBuffers: {}
  };

  // Audio System (Web Audio API)
  function initAudio() {
    if (state.audioContext) return;
    try {
      const AudioCtx = window.AudioContext || window.webkitAudioContext;
      if (AudioCtx) {
        state.audioContext = new AudioCtx();
      }
    } catch (e) {
      console.warn('[Audio] Web Audio initialization deferred', e);
    }
  }

  async function loadAudioBuffer(name, url) {
    if (!state.audioContext) return;
    try {
      const res = await fetch(url);
      const arrayBuffer = await res.arrayBuffer();
      const buffer = await state.audioContext.decodeAudioData(arrayBuffer);
      state.audioBuffers[name] = buffer;
    } catch (e) {
      console.warn(`[Audio] Failed to load sound: ${name}`, e);
    }
  }

  function playSound(name) {
    if (!state.soundEnabled || !state.audioContext || !state.audioBuffers[name]) return;
    try {
      if (state.audioContext.state === 'suspended') {
        state.audioContext.resume();
      }
      const source = state.audioContext.createBufferSource();
      source.buffer = state.audioBuffers[name];
      const gainNode = state.audioContext.createGain();
      gainNode.gain.value = 0.6;
      source.connect(gainNode);
      gainNode.connect(state.audioContext.destination);
      source.start(0);
    } catch (e) {
      console.warn('[Audio] Playback error', e);
    }
  }

  // Load Skin Manifest
  async function loadSkin(skinId) {
    try {
      const res = await fetch(`../skins/${skinId}/skin.json`);
      state.skinConfig = await res.json();
      state.skinId = skinId;

      // Load dialogue
      if (state.skinConfig.dialogue) {
        const diagRes = await fetch(`../skins/${skinId}/${state.skinConfig.dialogue}`);
        const diagData = await diagRes.json();
        state.dialogues = diagData.lines?.idle || ["喵~ 认真写代码哦！"];
      }

      // Preload audio
      initAudio();
      if (state.skinConfig.audio) {
        for (const [key, relPath] of Object.entries(state.skinConfig.audio)) {
          loadAudioBuffer(key, `../skins/${skinId}/${relPath}`);
        }
      }

      // Apply initial state
      setState('idle');
      showBubble(getRandomDialogue(), 5000);
    } catch (err) {
      console.error('[Pet] Error loading skin:', err);
    }
  }

  // Set Pet State & Emotion
  function setState(newState, text = null, durationMs = 0) {
    if (!state.skinConfig) return;
    const cfg = state.skinConfig.states?.[newState] || state.skinConfig.states?.['idle'];
    if (!cfg) return;

    if (state.stateTimer) {
      clearTimeout(state.stateTimer);
      state.stateTimer = null;
    }

    state.currentState = newState;

    // Update container classes
    container.className = `pet-container facing-${state.facing} state-${newState}`;

    // Update image frame
    const frameName = cfg.frame || 'idle';
    const framePath = state.skinConfig.frames?.[frameName] || 'frames/idle.png';
    petFrame.src = `../skins/${state.skinId}/${framePath}`;

    // Update floating effect
    if (cfg.effect) {
      effectLayer.innerHTML = `<img src="../skins/${state.skinId}/${cfg.effect}" alt="effect">`;
    } else {
      effectLayer.innerHTML = '';
    }

    // Update status badge
    updateBadge(newState);

    // Update dialogue bubble if specified
    const displayMsg = text || cfg.bubbleText;
    if (displayMsg) {
      showBubble(displayMsg, durationMs || 4500);
    }

    // Auto revert to idle if duration specified
    const autoRevertTime = durationMs || cfg.durationMs;
    if (autoRevertTime && autoRevertTime > 0) {
      state.stateTimer = setTimeout(() => {
        setState('idle');
      }, autoRevertTime);
    }
  }

  function updateBadge(s) {
    statusBadge.className = `status-badge ${s}`;
    const labels = {
      idle: '待命中',
      thinking: '思考中...',
      working: '执行工具...',
      success: '任务完成！',
      error: '遇到报错',
      petting: '呼噜呼噜~',
      pressed: '按压中',
      dragging: '移动中'
    };
    badgeLabel.textContent = labels[s] || s;
  }

  // Dialogue & Bubble Management
  function getRandomDialogue() {
    if (!state.dialogues.length) return "喵~";
    const idx = Math.floor(Math.random() * state.dialogues.length);
    return state.dialogues[idx];
  }

  function showBubble(text, ttlMs = 4500) {
    bubbleText.textContent = text;
    speechBubble.classList.remove('hidden');
    if (ttlMs > 0) {
      setTimeout(() => {
        // Keep quota visible if active, only hide text if idle
        if (state.currentState === 'idle') {
          // Subtle fade
        }
      }, ttlMs);
    }
  }

  function hideBubble() {
    speechBubble.classList.add('hidden');
  }

  // Quota bar updater
  function updateQuota(fiveHourPct, weekPct) {
    if (typeof fiveHourPct === 'number') {
      quota5hBar.style.width = `${Math.min(100, Math.max(0, fiveHourPct))}%`;
      quota5hVal.textContent = `${Math.round(fiveHourPct)}%`;
      quota5hBar.className = `quota-bar-fill ${fiveHourPct <= 10 ? 'danger' : fiveHourPct <= 30 ? 'warning' : ''}`;
    }
    if (typeof weekPct === 'number') {
      quotaWeekBar.style.width = `${Math.min(100, Math.max(0, weekPct))}%`;
      quotaWeekVal.textContent = `${Math.round(weekPct)}%`;
    }
  }

  // Drag & Physics Interactions
  container.addEventListener('pointerdown', (e) => {
    if (e.button === 2) return; // Right click handled separately
    initAudio();
    state.isDragging = true;
    state.hasMoved = false;
    state.dragStartX = e.clientX;
    state.dragStartY = e.clientY;

    const rect = container.getBoundingClientRect();
    state.petInitialX = rect.left;
    state.petInitialY = rect.top;

    container.classList.add('dragging');
    setState('pressed');
    playSound('press');

    container.setPointerCapture(e.pointerId);
  });

  window.addEventListener('pointermove', (e) => {
    if (!state.isDragging) return;

    const dx = e.clientX - state.dragStartX;
    const dy = e.clientY - state.dragStartY;

    if (Math.abs(dx) > 3 || Math.abs(dy) > 3) {
      state.hasMoved = true;
      setState('dragging');
      // Update facing direction
      if (dx > 2 && state.facing !== 'right') {
        state.facing = 'right';
        container.classList.remove('facing-left');
        container.classList.add('facing-right');
      } else if (dx < -2 && state.facing !== 'left') {
        state.facing = 'left';
        container.classList.remove('facing-right');
        container.classList.add('facing-left');
      }
    }

    let newX = state.petInitialX + dx;
    let newY = state.petInitialY + dy;

    // Viewport clamping
    const pad = 10;
    const maxW = window.innerWidth - container.offsetWidth - pad;
    const maxH = window.innerHeight - container.offsetHeight - pad;
    newX = Math.max(pad, Math.min(newX, maxW));
    newY = Math.max(pad, Math.min(newY, maxH));

    container.style.left = `${newX}px`;
    container.style.top = `${newY}px`;
    container.style.bottom = 'auto';
    container.style.right = 'auto';
  });

  window.addEventListener('pointerup', (e) => {
    if (!state.isDragging) return;
    state.isDragging = false;
    container.classList.remove('dragging');

    if (!state.hasMoved) {
      // Tap / Click action
      playSound('clickRelease');
      setState('rebound', null, 500);
      showBubble(getRandomDialogue(), 4000);
    } else {
      playSound('release');
      setState('rebound', null, 500);
    }
  });

  // Bubble text click to cycle lines
  bubbleText.addEventListener('click', (e) => {
    e.stopPropagation();
    initAudio();
    playSound('clickPress');
    showBubble(getRandomDialogue(), 4500);
  });

  // Context Menu
  container.addEventListener('contextmenu', (e) => {
    e.preventDefault();
    contextMenu.style.left = `${Math.min(window.innerWidth - 200, e.clientX)}px`;
    contextMenu.style.top = `${Math.min(window.innerHeight - 200, e.clientY)}px`;
    contextMenu.classList.remove('hidden');
  });

  document.addEventListener('click', (e) => {
    if (!contextMenu.contains(e.target)) {
      contextMenu.classList.add('hidden');
    }
  });

  contextMenu.addEventListener('click', (e) => {
    const item = e.target.closest('.menu-item');
    if (!item) return;
    const action = item.dataset.action;

    switch (action) {
      case 'toggle-quota':
        quotaSection.style.display = quotaSection.style.display === 'none' ? 'flex' : 'none';
        break;
      case 'random-dialogue':
        showBubble(getRandomDialogue(), 4500);
        break;
      case 'pet-action':
        setState('petting', '呼噜呼噜~ 摸摸头，今天也是最棒的代码大师！', 2500);
        break;
      case 'toggle-sound':
        state.soundEnabled = !state.soundEnabled;
        soundStatus.textContent = state.soundEnabled ? '开启' : '静音';
        break;
      case 'switch-skin':
        showBubble('当前已激活【大肥猫·星语伴侣】，可在 skins/ 中自由扩充新皮肤！', 3500);
        break;
      case 'close-menu':
        contextMenu.classList.add('hidden');
        break;
    }
    contextMenu.classList.add('hidden');
  });

  // Connect to Antigravity IDE Telemetry via PetBridge
  const bridge = new window.PetBridge({
    onEvent: (event) => {
      console.log('[Pet] Received event:', event);
      if (event.state) {
        setState(event.state, event.bubbleText, event.durationMs);
      }
    },
    onQuota: (quota) => {
      console.log('[Pet] Received quota:', quota);
      updateQuota(quota.fiveHour, quota.weekly);
    }
  });
  bridge.start();

  // Periodic Idle Dialogue (every 45s)
  setInterval(() => {
    if (state.currentState === 'idle' && !state.isDragging) {
      showBubble(getRandomDialogue(), 5000);
    }
  }, 45000);

  // Initialize
  loadSkin('fat-cat');
})();
