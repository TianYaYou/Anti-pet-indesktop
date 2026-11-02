/**
 * Anti-Pet Core Engine
 * - Cloud Thought Bubble ("小猫挂件在想的东西") with Dynamic Screen-Edge Clamping
 * - Cursor-Anchored Right-Click Context Menu (Always accessible even in DND mode)
 * - Auto-Hide Thought Bubble when Cursor Leaves, Window Blurs, or User Types
 * - Dual-Mode Physics: Fullscreen Canvas DOM Physics + Compact Widget Mode Native Window Physics
 */
(function () {
  const container = document.getElementById('pet-container');
  const petFrame = document.getElementById('pet-frame');
  const speechBubble = document.getElementById('speech-bubble');
  const bubbleCloud = document.getElementById('bubble-cloud');
  const bubbleText = document.getElementById('bubble-text');
  const quotaSection = document.getElementById('quota-section');
  const quota5hBar = document.getElementById('quota-5h');
  const quota5hVal = document.getElementById('quota-5h-val');
  const quotaWeekBar = document.getElementById('quota-week');
  const quotaWeekVal = document.getElementById('quota-week-val');
  const quotaMeta = document.getElementById('quota-meta');
  const contextMenu = document.getElementById('context-menu');
  const soundStatus = document.getElementById('sound-status');
  const dndStatus = document.getElementById('dnd-status');
  const ideOnlyStatus = document.getElementById('ide-only-status');
  const gravityStatus = document.getElementById('gravity-status');
  const skinStatus = document.getElementById('skin-status');
  const zoomPill = document.getElementById('zoom-pill');
  const zoomStatus = document.getElementById('zoom-status');

  const ipc = window.DesktopIPC;
  let bridge = null;

  let pendingSettings = {};

  function saveSettingsImmediately(partial) {
    if (!partial || typeof partial !== 'object') return;
    Object.assign(pendingSettings, partial);
    try {
      for (const [k, v] of Object.entries(partial)) {
        localStorage.setItem(`anti-pet-${k}`, typeof v === 'string' ? v : JSON.stringify(v));
      }
    } catch (e) {}

    if (state.settingsSaveTimer) {
      clearTimeout(state.settingsSaveTimer);
      state.settingsSaveTimer = null;
    }
    const payload = { ...pendingSettings };
    pendingSettings = {};
    if (bridge && typeof bridge.saveSettings === 'function') {
      bridge.saveSettings(payload);
    }
  }

  function debouncedSaveSettings(partial) {
    if (!partial || typeof partial !== 'object') return;
    Object.assign(pendingSettings, partial);
    try {
      for (const [k, v] of Object.entries(partial)) {
        localStorage.setItem(`anti-pet-${k}`, typeof v === 'string' ? v : JSON.stringify(v));
      }
    } catch (e) {}

    if (state.settingsSaveTimer) clearTimeout(state.settingsSaveTimer);
    state.settingsSaveTimer = setTimeout(() => {
      const payload = { ...pendingSettings };
      pendingSettings = {};
      if (bridge && typeof bridge.saveSettings === 'function') {
        bridge.saveSettings(payload);
      }
    }, 350);
  }

  const hitCanvas = document.createElement('canvas');
  const hitCtx = hitCanvas.getContext('2d', { willReadFrequently: true });
  let hitCanvasReady = false;

  const state = {
    skinId: 'fat-cat',
    availableSkins: [],
    skinConfig: null,
    dialogueBank: {},
    dialogues: [],
    currentState: 'idle',
    windowMode: window.__ANTI_PET_INITIAL_MODE__ === 'compact' ? 'compact' : 'fullscreen',
    soundEnabled: true,
    dndPassthrough: false,
    ideOnlyMode: false,
    gravityEnabled: true,
    dockBottom: 0,
    dockLeft: 0,
    dockRight: 0,
    hasReceivedDockInfo: false,
    isHoveringPet: false,
    isUserTyping: false,
    lastIdeActiveAt: Date.now(),
    facing: 'left',
    dockedEdge: null,
    isDragging: false,
    dragStartX: 0,
    dragStartY: 0,
    lastDragX: 0,
    dragLastScreenX: 0,
    dragLastScreenY: 0,
    dragTotalMove: 0,
    petInitialX: 0,
    petInitialY: 0,
    hasMoved: false,
    gravityRaf: null,
    vy: 0,
    vx: 0,
    rubLastX: null,
    rubLastDir: 0,
    rubCount: 0,
    rubResetTimer: null,
    lastPettingAt: 0,
    stateTimer: null,
    bubbleHideTimer: null,
    leaveHideTimer: null,
    scale: parseFloat(localStorage.getItem('anti-pet-scale') || '1.0'),
    targetScale: parseFloat(localStorage.getItem('anti-pet-scale') || '1.0'),
    zoomRaf: null,
    zoomPillTimer: null,
    settingsSaveTimer: null,
    audioContext: null,
    audioBuffers: {},
    floorMode: 'bottom',
    soundVolume: 0.8,
    showQuota: true,
    bubbleDurationMs: 4000
  };

  if (state.windowMode === 'compact') {
    document.body.classList.add('compact-mode');
  }

  // =========================================================================
  // 1. Pixel-Level Alpha Hit-Testing & Desktop IPC Synchronization
  // =========================================================================

  function refreshHitCanvas() {
    if (!petFrame.complete || !petFrame.naturalWidth) {
      hitCanvasReady = false;
      return;
    }
    try {
      hitCanvas.width = petFrame.naturalWidth;
      hitCanvas.height = petFrame.naturalHeight;
      hitCtx.clearRect(0, 0, hitCanvas.width, hitCanvas.height);
      hitCtx.drawImage(petFrame, 0, 0);
      hitCanvasReady = true;
    } catch (e) {
      hitCanvasReady = false;
    }
  }

  petFrame.addEventListener('load', () => {
    refreshHitCanvas();
    if (!state.hasMoved && !state.isDragging) {
      snapToCurrentFloor();
    }
    adjustBubblePosition();
    syncHitRectsToDesktop();
  });

  function isOpaquePixelOnCat(clientX, clientY) {
    const rect = petFrame.getBoundingClientRect();
    if (
      clientX < rect.left ||
      clientX > rect.right ||
      clientY < rect.top ||
      clientY > rect.bottom ||
      rect.width <= 0 ||
      rect.height <= 0
    ) {
      return false;
    }
    if (!hitCanvasReady) return true;

    let normX = (clientX - rect.left) / rect.width;
    const normY = (clientY - rect.top) / rect.height;
    if (state.facing === 'right') {
      normX = 1 - normX;
    }
    const px = Math.min(hitCanvas.width - 1, Math.max(0, Math.floor(normX * hitCanvas.width)));
    const py = Math.min(hitCanvas.height - 1, Math.max(0, Math.floor(normY * hitCanvas.height)));
    try {
      const alpha = hitCtx.getImageData(px, py, 1, 1).data[3];
      return alpha > 20;
    } catch (e) {
      return true;
    }
  }

  function distanceToRect(x, y, rect) {
    if (!rect) return 9999;
    const dx = Math.max(rect.left - x, 0, x - rect.right);
    const dy = Math.max(rect.top - y, 0, y - rect.bottom);
    return Math.sqrt(dx * dx + dy * dy);
  }

  function isPointInElement(clientX, clientY, el) {
    if (!el || el.classList.contains('hidden')) return false;
    const style = window.getComputedStyle(el);
    if (style.display === 'none' || style.visibility === 'hidden' || parseFloat(style.opacity) === 0) {
      return false;
    }
    const r = el.getBoundingClientRect();
    return clientX >= r.left && clientX <= r.right && clientY >= r.top && clientY <= r.bottom;
  }

  function evaluateInteractiveHit(clientX, clientY) {
    if (state.isDragging) return true;
    if (container.classList.contains('ide-hidden')) {
      return false;
    }
    if (!contextMenu.classList.contains('hidden')) {
      const mRect = contextMenu.getBoundingClientRect();
      const catRect = petFrame.getBoundingClientRect();
      const dist = Math.min(
        distanceToRect(clientX, clientY, mRect),
        distanceToRect(clientX, clientY, catRect)
      );
      if (dist <= 300) return true;
    }
    if (!state.dndPassthrough && isPointInElement(clientX, clientY, bubbleCloud || speechBubble)) {
      return true;
    }
    // Always allow cursor hit on the cat body itself so the user can right-click to open the menu & exit DND
    return isOpaquePixelOnCat(clientX, clientY);
  }

  function syncHitRectsToDesktop() {
    if (!ipc) return;
    // Never lock Rust into full window passthrough; keep the cat's bounding box active for right-click menu
    ipc.setDndPassthrough(false);

    const rects = [];
    if (!container.classList.contains('ide-hidden')) {
      const catRect = petFrame.getBoundingClientRect();
      rects.push({
        x: catRect.left - 6,
        y: catRect.top - 6,
        w: catRect.width + 12,
        h: catRect.height + 12
      });

      if (!state.dndPassthrough && !speechBubble.classList.contains('hidden')) {
        const bRect = (bubbleCloud || speechBubble).getBoundingClientRect();
        rects.push({
          x: bRect.left - 6,
          y: bRect.top - 6,
          w: bRect.width + 12,
          h: bRect.height + 12
        });
      }
    }

    if (!contextMenu.classList.contains('hidden')) {
      const mRect = contextMenu.getBoundingClientRect();
      const catRect = petFrame.getBoundingClientRect();
      const minX = Math.min(catRect.left, mRect.left) - 40;
      const minY = Math.min(catRect.top, mRect.top) - 40;
      const maxX = Math.max(catRect.right, mRect.right) + 40;
      const maxY = Math.max(catRect.bottom, mRect.bottom) + 40;
      rects.push({
        x: Math.max(0, minX),
        y: Math.max(0, minY),
        w: Math.min(window.innerWidth, maxX - minX),
        h: Math.min(window.innerHeight, maxY - minY)
      });
    }

    ipc.updateHitRects(rects);
  }

  // =========================================================================
  // 2. Dynamic Screen-Edge Clamping for Thought Bubble & Context Menu
  // =========================================================================

  function adjustBubblePosition() {
    if (!speechBubble || speechBubble.classList.contains('hidden')) return;

    const catRect = container.getBoundingClientRect();
    const bubbleW = speechBubble.offsetWidth || 204;
    const bubbleH = (bubbleCloud || speechBubble).offsetHeight || 96;
    const screenMargin = 10;
    const minLeft = (state.windowMode === 'compact' ? 0 : state.dockLeft) + screenMargin;
    const maxRight =
      window.innerWidth - (state.windowMode === 'compact' ? 0 : state.dockRight) - screenMargin;

    const catCenterX = catRect.left + catRect.width * 0.5;
    const idealLeft = catCenterX - bubbleW * 0.5;
    const idealRight = catCenterX + bubbleW * 0.5;

    let shiftX = 0;
    if (idealRight > maxRight) {
      shiftX = maxRight - idealRight;
    } else if (idealLeft < minLeft) {
      shiftX = minLeft - idealLeft;
    }
    document.documentElement.style.setProperty('--bubble-shift-x', `${Math.round(shiftX)}px`);

    if (catRect.top < bubbleH + 28) {
      speechBubble.classList.add('flip-below');
    } else {
      speechBubble.classList.remove('flip-below');
    }
  }

  function hideBubbleImmediately() {
    if (state.bubbleHideTimer) {
      clearTimeout(state.bubbleHideTimer);
      state.bubbleHideTimer = null;
    }
    if (state.leaveHideTimer) {
      clearTimeout(state.leaveHideTimer);
      state.leaveHideTimer = null;
    }
    if (!speechBubble.classList.contains('hidden')) {
      speechBubble.classList.add('hidden');
      syncHitRectsToDesktop();
    }
  }

  function closeContextMenu() {
    if (!contextMenu.classList.contains('hidden')) {
      contextMenu.classList.add('hidden');
      syncHitRectsToDesktop();
    }
  }

  function handleHoverChange(isHovering) {
    if (state.isHoveringPet === isHovering) return;
    state.isHoveringPet = isHovering;

    if (isHovering) {
      if (state.leaveHideTimer) {
        clearTimeout(state.leaveHideTimer);
        state.leaveHideTimer = null;
      }
    } else {
      if (state.leaveHideTimer) clearTimeout(state.leaveHideTimer);
      state.leaveHideTimer = setTimeout(() => {
        if (!state.isHoveringPet && !state.isDragging) {
          hideBubbleImmediately();
        }
      }, 1200);
    }
  }

  window.__antiPetOnWindowBlur = function () {
    hideBubbleImmediately();
  };

  window.addEventListener('blur', () => {
    hideBubbleImmediately();
  });

  // =========================================================================
  // 3. Cursor Gaze Tracking & Petting Gesture
  // =========================================================================

  function updateCursorGaze(clientX, clientY) {
    if (state.isDragging || state.dndPassthrough || container.classList.contains('ide-hidden')) {
      return;
    }
    const rect = petFrame.getBoundingClientRect();
    if (rect.width <= 0) return;

    const centerX = rect.left + rect.width * 0.5;
    const centerY = rect.top + rect.height * 0.42;
    const dx = clientX - centerX;
    const dy = clientY - centerY;

    const normX = Math.max(-1, Math.min(1, dx / 260));
    const normY = Math.max(-1, Math.min(1, dy / 220));

    document.documentElement.style.setProperty('--gaze-x', `${(normX * 3.5).toFixed(2)}px`);
    document.documentElement.style.setProperty('--gaze-y', `${(normY * 2.2).toFixed(2)}px`);
    document.documentElement.style.setProperty('--gaze-tilt', `${(normX * 2.5).toFixed(2)}deg`);

    if (state.currentState === 'idle' && !state.dockedEdge && Math.abs(dx) > 110) {
      const targetFacing = dx > 0 ? 'right' : 'left';
      if (state.facing !== targetFacing) {
        state.facing = targetFacing;
        container.classList.toggle('facing-right', targetFacing === 'right');
        container.classList.toggle('facing-left', targetFacing === 'left');
      }
    }
  }

  function detectHeadRubGesture(clientX, clientY) {
    if (state.isDragging || state.dndPassthrough) return;
    const rect = petFrame.getBoundingClientRect();
    if (
      clientX < rect.left ||
      clientX > rect.right ||
      clientY < rect.top ||
      clientY > rect.top + rect.height * 0.58
    ) {
      return;
    }

    if (state.rubLastX !== null) {
      const deltaX = clientX - state.rubLastX;
      if (Math.abs(deltaX) > 6) {
        const dir = deltaX > 0 ? 1 : -1;
        if (state.rubLastDir !== 0 && dir !== state.rubLastDir) {
          state.rubCount += 1;
          if (state.rubCount >= 4 && Date.now() - state.lastPettingAt > 1800) {
            state.rubCount = 0;
            state.lastPettingAt = Date.now();
            triggerPettingAction();
          }
        }
        state.rubLastDir = dir;
        state.rubLastX = clientX;
      }
    } else {
      state.rubLastX = clientX;
    }

    if (state.rubResetTimer) clearTimeout(state.rubResetTimer);
    state.rubResetTimer = setTimeout(() => {
      state.rubCount = 0;
      state.rubLastDir = 0;
      state.rubLastX = null;
    }, 650);
  }

  function triggerPettingAction() {
    initAudio();
    playPurrSound();
    const petLine = getRandomDialogue('petting');
    setState('petting', petLine, 2400);
  }

  // Global hook called by Rust 40Hz cursor, Mac Dock, typing & IDE focus tracker
  window.__antiPetOnGlobalCursor = function (
    localX,
    localY,
    dockBottom,
    dockLeft,
    dockRight,
    isTyping,
    isIdeActive
  ) {
    if (typeof isIdeActive === 'boolean' && isIdeActive) {
      state.lastIdeActiveAt = Date.now();
      if (container.classList.contains('ide-hidden')) {
        container.classList.remove('ide-hidden');
        syncHitRectsToDesktop();
      }
    }

    if (typeof isTyping === 'boolean') {
      state.isUserTyping = isTyping;
      if (isTyping && !state.isHoveringPet && !state.isDragging) {
        closeContextMenu();
        hideBubbleImmediately();
      }
    }

    if (typeof dockBottom === 'number') {
      const prevBottom = state.dockBottom;
      const prevLeft = state.dockLeft;
      const prevRight = state.dockRight;
      state.dockBottom = Math.max(0, dockBottom);
      state.dockLeft = Math.max(0, dockLeft || 0);
      state.dockRight = Math.max(0, dockRight || 0);

      if (
        state.windowMode !== 'compact' &&
        !state.isDragging &&
        !state.dockedEdge &&
        state.gravityEnabled &&
        (!state.hasReceivedDockInfo ||
          Math.abs(prevBottom - state.dockBottom) > 1.5 ||
          Math.abs(prevLeft - state.dockLeft) > 1.5 ||
          Math.abs(prevRight - state.dockRight) > 1.5)
      ) {
        state.hasReceivedDockInfo = true;
        startReleasePhysics(true);
      } else {
        state.hasReceivedDockInfo = true;
      }
    }

    updateCursorGaze(localX, localY);
    const shouldIntercept = evaluateInteractiveHit(localX, localY);
    handleHoverChange(shouldIntercept);
    ipc.setIgnoreCursorEvents(!shouldIntercept);

    if (!contextMenu.classList.contains('hidden')) {
      const mRect = contextMenu.getBoundingClientRect();
      const catRect = petFrame.getBoundingClientRect();
      const distToMenu = distanceToRect(localX, localY, mRect);
      const distToCat = distanceToRect(localX, localY, catRect);
      if (Math.min(distToMenu, distToCat) > 300) {
        closeContextMenu();
      }
    }
  };

  // Hooks called by Rust when Compact Mode native window bounces or docks on screen edge
  window.__antiPetOnCompactBounce = function () {
    playSound('release');
    setState('rebound', getRandomDialogue('release'), 500);
    adjustBubblePosition();
    syncHitRectsToDesktop();
  };

  window.__antiPetOnCompactEdgeDock = function (edge) {
    state.dockedEdge = edge;
    container.classList.remove('docked-left', 'docked-right');
    if (edge === 'left') {
      state.facing = 'right';
      container.classList.remove('facing-left');
      container.classList.add('facing-right', 'docked-left');
      playSoftBumpSound();
      setState('idle');
      showBubble('贴在左边栏观察你写代码~', 2800);
    } else if (edge === 'right') {
      state.facing = 'left';
      container.classList.remove('facing-right');
      container.classList.add('facing-left', 'docked-right');
      playSoftBumpSound();
      setState('idle');
      showBubble('贴在右边栏偷看你，不许写 Bug 哦！', 2800);
    }
    adjustBubblePosition();
    syncHitRectsToDesktop();
  };

  window.addEventListener('pointermove', (e) => {
    updateCursorGaze(e.clientX, e.clientY);
    if (!state.isDragging) {
      const shouldIntercept = evaluateInteractiveHit(e.clientX, e.clientY);
      handleHoverChange(shouldIntercept);
      ipc.setIgnoreCursorEvents(!shouldIntercept);
      if (shouldIntercept) {
        detectHeadRubGesture(e.clientX, e.clientY);
      }
    }
    if (!contextMenu.classList.contains('hidden')) {
      const mRect = contextMenu.getBoundingClientRect();
      const catRect = petFrame.getBoundingClientRect();
      const distToMenu = distanceToRect(e.clientX, e.clientY, mRect);
      const distToCat = distanceToRect(e.clientX, e.clientY, catRect);
      if (Math.min(distToMenu, distToCat) > 300) {
        closeContextMenu();
      }
    }
  });

  // =========================================================================
  // 4. Audio System
  // =========================================================================

  function initAudio() {
    if (state.audioContext) return;
    try {
      const AudioCtx = window.AudioContext || window.webkitAudioContext;
      if (AudioCtx) {
        state.audioContext = new AudioCtx();
      }
    } catch (e) {}
  }

  async function loadAudioBuffer(name, url) {
    if (!state.audioContext) return;
    try {
      const res = await fetch(url);
      const arrayBuffer = await res.arrayBuffer();
      const buffer = await state.audioContext.decodeAudioData(arrayBuffer);
      state.audioBuffers[name] = buffer;
    } catch (e) {}
  }

  function playSound(name) {
    if (state.dndPassthrough || !state.soundEnabled || !state.audioContext || !state.audioBuffers[name]) return;
    try {
      if (state.audioContext.state === 'suspended') {
        state.audioContext.resume();
      }
      const source = state.audioContext.createBufferSource();
      source.buffer = state.audioBuffers[name];
      const gainNode = state.audioContext.createGain();
      const vol = typeof state.soundVolume === 'number' ? state.soundVolume : 0.8;
      gainNode.gain.value = vol * 0.75;
      source.connect(gainNode);
      gainNode.connect(state.audioContext.destination);
      source.start(0);
    } catch (e) {}
  }

  function playPurrSound() {
    if (state.dndPassthrough || !state.soundEnabled) return;
    initAudio();
    if (!state.audioContext) return;
    try {
      if (state.audioContext.state === 'suspended') {
        state.audioContext.resume();
      }
      const ctx = state.audioContext;
      const now = ctx.currentTime;
      const osc = ctx.createOscillator();
      const lfo = ctx.createOscillator();
      const lfoGain = ctx.createGain();
      const mainGain = ctx.createGain();

      osc.type = 'triangle';
      osc.frequency.setValueAtTime(52, now);
      osc.frequency.exponentialRampToValueAtTime(64, now + 0.45);
      osc.frequency.exponentialRampToValueAtTime(48, now + 0.9);

      lfo.type = 'sine';
      lfo.frequency.setValueAtTime(24, now);
      lfoGain.gain.setValueAtTime(14, now);

      const vol = typeof state.soundVolume === 'number' ? state.soundVolume : 0.8;
      mainGain.gain.setValueAtTime(0.01 * vol, now);
      mainGain.gain.linearRampToValueAtTime(0.22 * vol, now + 0.15);
      mainGain.gain.exponentialRampToValueAtTime(0.005, now + 0.95);

      lfo.connect(lfoGain);
      lfoGain.connect(osc.frequency);
      osc.connect(mainGain);
      mainGain.connect(ctx.destination);

      osc.start(now);
      lfo.start(now);
      osc.stop(now + 0.95);
      lfo.stop(now + 0.95);
    } catch (e) {
      playSound('clickRelease');
    }
  }

  function playSoftBumpSound() {
    if (state.dndPassthrough || !state.soundEnabled) return;
    initAudio();
    if (!state.audioContext) return;
    try {
      if (state.audioContext.state === 'suspended') {
        state.audioContext.resume();
      }
      const ctx = state.audioContext;
      const now = ctx.currentTime;
      const osc = ctx.createOscillator();
      const gain = ctx.createGain();
      osc.type = 'sine';
      osc.frequency.setValueAtTime(130, now);
      osc.frequency.exponentialRampToValueAtTime(45, now + 0.12);
      gain.gain.setValueAtTime(0.22, now);
      gain.gain.exponentialRampToValueAtTime(0.005, now + 0.13);
      osc.connect(gain);
      gain.connect(ctx.destination);
      osc.start(now);
      osc.stop(now + 0.13);
    } catch (e) {}
  }

  // =========================================================================
  // 5. Skin Loading, State Machine & Thought Cloud Bubble
  // =========================================================================

  async function fetchAvailableSkins() {
    try {
      const res = await fetch('/api/skins');
      if (res.ok) {
        state.availableSkins = await res.json();
      }
    } catch (e) {}
  }

  async function loadSkin(skinId) {
    try {
      const res = await fetch(`/skins/${skinId}/skin.json`);
      state.skinConfig = await res.json();
      state.skinId = skinId;
      if (skinStatus) {
        skinStatus.textContent = (state.skinConfig.name || skinId).split('·')[0].trim();
      }

      if (state.skinConfig.dialogue) {
        const diagRes = await fetch(`/skins/${skinId}/${state.skinConfig.dialogue}`);
        const diagData = await diagRes.json();
        state.dialogueBank = diagData.lines || {};
        state.dialogues = state.dialogueBank.idle || ['喵~ 认真写代码哦！'];
      }

      initAudio();
      if (state.skinConfig.audio) {
        for (const [key, relPath] of Object.entries(state.skinConfig.audio)) {
          loadAudioBuffer(key, `/skins/${skinId}/${relPath}`);
        }
      }

      const idlePath = state.skinConfig.frames?.idle || 'frames/idle.png';
      petFrame.src = `/skins/${state.skinId}/${idlePath}`;

      setState('idle');
      showBubble(getRandomDialogue('idle'), 3200);
    } catch (err) {
      console.error('[Pet] Error loading skin:', err);
    }
  }

  function setState(newState, text = null, durationMs = 0) {
    if (!state.skinConfig) return;
    const cfg = state.skinConfig.states?.[newState] || state.skinConfig.states?.['idle'];
    if (!cfg) return;

    if (state.stateTimer) {
      clearTimeout(state.stateTimer);
      state.stateTimer = null;
    }

    state.currentState = newState;

    const classes = ['pet-container', `facing-${state.facing}`, `state-${newState}`];
    if (state.isDragging) classes.push('dragging');
    if (state.dockedEdge) classes.push(`docked-${state.dockedEdge}`);
    if (state.dndPassthrough) classes.push('dnd-ghost');
    if (state.ideOnlyMode && Date.now() - state.lastIdeActiveAt > 120000) {
      classes.push('ide-hidden');
    }
    container.className = classes.join(' ');

    const displayMsg = text || cfg.bubbleText;
    if (displayMsg && !state.dndPassthrough && (!state.isUserTyping || state.isHoveringPet || newState === 'error')) {
      showBubble(displayMsg, durationMs || 3200);
    }

    const autoRevertTime = durationMs || cfg.durationMs;
    if (autoRevertTime && autoRevertTime > 0) {
      state.stateTimer = setTimeout(() => {
        setState('idle');
      }, autoRevertTime);
    }

    setTimeout(syncHitRectsToDesktop, 50);
  }

  function getRandomDialogue(category = 'idle') {
    const pool =
      state.dialogueBank?.[category] && state.dialogueBank[category].length
        ? state.dialogueBank[category]
        : state.dialogues;
    if (!pool || !pool.length) return '喵~';
    return pool[Math.floor(Math.random() * pool.length)];
  }

  function showBubble(text, ttlMs = 0) {
    if (state.dndPassthrough) return;
    bubbleText.textContent = text;
    speechBubble.classList.remove('hidden');
    adjustBubblePosition();
    syncHitRectsToDesktop();

    if (state.bubbleHideTimer) {
      clearTimeout(state.bubbleHideTimer);
      state.bubbleHideTimer = null;
    }

    const duration = ttlMs || state.bubbleDurationMs || 4000;
    if (duration > 0) {
      state.bubbleHideTimer = setTimeout(() => {
        if (!state.isHoveringPet && !state.isDragging) {
          speechBubble.classList.add('hidden');
          syncHitRectsToDesktop();
        }
      }, duration);
    }
  }

  // =========================================================================
  // 6. Dual-Mode Dragging & Gravity Drop Physics (Fullscreen & Compact Modes)
  // =========================================================================

  function cancelGravityAnimation() {
    if (state.gravityRaf) {
      cancelAnimationFrame(state.gravityRaf);
      state.gravityRaf = null;
    }
  }

  function computeTargetFloorY(currentRect = null) {
    const winH = window.innerHeight;
    const petH = container.offsetHeight || 136;
    const trueBottomY = Math.max(0, winH - petH);
    const dockBottom = Math.max(0, state.dockBottom || 0);
    const dockLedgeY = Math.max(0, winH - petH - dockBottom);

    if (state.floorMode === 'bottom') {
      return trueBottomY;
    }
    if (state.floorMode === 'taskbar' || state.floorMode === 'dock') {
      return dockLedgeY;
    }
    if (state.floorMode === 'free') {
      const rect = currentRect || container.getBoundingClientRect();
      return rect.top;
    }

    if (dockBottom <= 2) {
      return trueBottomY;
    }

    const rect = currentRect || container.getBoundingClientRect();
    if (rect.top >= dockLedgeY + Math.min(16, dockBottom * 0.4)) {
      return trueBottomY;
    }

    return dockLedgeY;
  }

  function snapToCurrentFloor() {
    if (state.windowMode === 'compact') {
      container.style.left = '';
      container.style.top = '';
      container.style.bottom = '0px';
      container.style.right = 'auto';
      adjustBubblePosition();
      syncHitRectsToDesktop();
      return;
    }
    const rect = container.getBoundingClientRect();
    const targetY = computeTargetFloorY(rect);
    const curX = Math.max(
      state.dockLeft,
      Math.min(rect.left, window.innerWidth - container.offsetWidth - state.dockRight)
    );
    container.style.left = `${curX}px`;
    container.style.top = `${targetY}px`;
    container.style.bottom = 'auto';
    container.style.right = 'auto';
    adjustBubblePosition();
    syncHitRectsToDesktop();
  }

  function startReleasePhysics(silent = false) {
    cancelGravityAnimation();

    if (state.windowMode === 'compact') {
      snapToCurrentFloor();
      return;
    }

    const rect = container.getBoundingClientRect();
    const winW = window.innerWidth;
    const petW = container.offsetWidth || 136;
    const leftBound = state.dockLeft;
    const rightBound = winW - petW - state.dockRight;
    const edgeSnapThreshold = 36;
    const targetFloor = computeTargetFloorY(rect);

    if (!silent && rect.left <= leftBound + edgeSnapThreshold && rect.top < targetFloor - 28) {
      state.dockedEdge = 'left';
      state.facing = 'right';
      container.style.left = `${leftBound}px`;
      container.style.top = `${Math.max(8, Math.min(rect.top, targetFloor))}px`;
      container.style.bottom = 'auto';
      container.style.right = 'auto';
      container.classList.remove('facing-left', 'docked-right');
      container.classList.add('facing-right', 'docked-left');
      playSoftBumpSound();
      setState('idle');
      showBubble('贴在左边栏观察你写代码~', 2800);
      adjustBubblePosition();
      syncHitRectsToDesktop();
      debouncedSaveSettings({
        posX: Math.round(leftBound),
        posY: Math.round(Math.max(8, Math.min(rect.top, targetFloor)))
      });
      return;
    }

    if (!silent && rect.left >= rightBound - edgeSnapThreshold && rect.top < targetFloor - 28) {
      state.dockedEdge = 'right';
      state.facing = 'left';
      container.style.left = `${rightBound}px`;
      container.style.top = `${Math.max(8, Math.min(rect.top, targetFloor))}px`;
      container.style.bottom = 'auto';
      container.style.right = 'auto';
      container.classList.remove('facing-right', 'docked-left');
      container.classList.add('facing-left', 'docked-right');
      playSoftBumpSound();
      setState('idle');
      showBubble('贴在右边栏偷看你，不许写 Bug 哦！', 2800);
      adjustBubblePosition();
      syncHitRectsToDesktop();
      debouncedSaveSettings({
        posX: Math.round(rightBound),
        posY: Math.round(Math.max(8, Math.min(rect.top, targetFloor)))
      });
      return;
    }

    state.dockedEdge = null;
    container.classList.remove('docked-left', 'docked-right');

    let curX = rect.left;
    if (curX <= leftBound + edgeSnapThreshold) {
      curX = leftBound;
    } else if (curX >= rightBound - edgeSnapThreshold) {
      curX = rightBound;
    } else {
      curX = Math.max(leftBound, Math.min(curX, rightBound));
    }

    if (!state.gravityEnabled) {
      const clampedY = Math.max(0, Math.min(rect.top, window.innerHeight - container.offsetHeight));
      container.style.left = `${curX}px`;
      container.style.top = `${clampedY}px`;
      container.style.bottom = 'auto';
      container.style.right = 'auto';
      if (!silent) {
        playSound('release');
        setState('rebound', getRandomDialogue('release'), 500);
      }
      adjustBubblePosition();
      syncHitRectsToDesktop();
      debouncedSaveSettings({
        posX: Math.round(curX),
        posY: Math.round(clampedY)
      });
      return;
    }

    if (rect.top >= targetFloor - 2) {
      container.style.left = `${curX}px`;
      container.style.top = `${targetFloor}px`;
      container.style.bottom = 'auto';
      container.style.right = 'auto';
      if (!silent) {
        playSound('release');
        setState('rebound', getRandomDialogue('release'), 500);
      }
      adjustBubblePosition();
      syncHitRectsToDesktop();
      debouncedSaveSettings({
        posX: Math.round(curX),
        posY: Math.round(targetFloor)
      });
      return;
    }

    let curY = rect.top;
    let vy = 0;
    let vx = silent ? 0 : state.vx * 0.35;
    const gravity = 1.05;

    function stepGravity() {
      vy += gravity;
      curY += vy;
      curX += vx;
      vx *= 0.94;

      const minX = state.dockLeft;
      const maxX = window.innerWidth - container.offsetWidth - state.dockRight;
      curX = Math.max(minX, Math.min(curX, maxX));

      if (curY >= targetFloor) {
        curY = targetFloor;
        container.style.left = `${curX}px`;
        container.style.top = `${targetFloor}px`;
        container.style.bottom = 'auto';
        container.style.right = 'auto';
        state.gravityRaf = null;
        if (!silent) {
          playSound('release');
          setState('rebound', getRandomDialogue('release'), 500);
        }
        adjustBubblePosition();
        syncHitRectsToDesktop();
        debouncedSaveSettings({
          posX: Math.round(curX),
          posY: Math.round(targetFloor)
        });
        return;
      }

      container.style.left = `${curX}px`;
      container.style.top = `${curY}px`;
      container.style.bottom = 'auto';
      container.style.right = 'auto';
      adjustBubblePosition();
      state.gravityRaf = requestAnimationFrame(stepGravity);
    }

    state.gravityRaf = requestAnimationFrame(stepGravity);
  }

  container.addEventListener('pointerdown', (e) => {
    if (e.button === 2) return;
    if (e.target.closest('#speech-bubble')) return;

    closeContextMenu();
    initAudio();
    cancelGravityAnimation();

    state.isDragging = true;
    state.hasMoved = false;
    state.dragStartX = e.clientX;
    state.dragStartY = e.clientY;
    state.lastDragX = e.clientX;
    state.dragLastScreenX = e.screenX;
    state.dragLastScreenY = e.screenY;
    state.dragTotalMove = 0;
    state.vx = 0;

    if (state.dockedEdge) {
      state.dockedEdge = null;
      container.classList.remove('docked-left', 'docked-right');
    }

    const rect = container.getBoundingClientRect();
    state.petInitialX = rect.left;
    state.petInitialY = rect.top;

    container.classList.add('dragging');
    ipc.setDragging(true);
    ipc.setIgnoreCursorEvents(false);

    setState('pressed', getRandomDialogue('pressed'));
    playSound('press');

    try {
      container.setPointerCapture(e.pointerId);
    } catch (err) {}
  });

  window.addEventListener('pointermove', (e) => {
    if (!state.isDragging) return;

    if (state.windowMode === 'compact' && ipc && ipc.isDesktop) {
      const dxScreen = e.screenX - state.dragLastScreenX;
      const dyScreen = e.screenY - state.dragLastScreenY;
      state.dragLastScreenX = e.screenX;
      state.dragLastScreenY = e.screenY;
      state.vx = dxScreen;
      state.dragTotalMove += Math.hypot(dxScreen, dyScreen);

      if (state.dragTotalMove > 4) {
        if (!state.hasMoved) {
          state.hasMoved = true;
          playSound('clickPress');
          setState('dragging', getRandomDialogue('dragged'));
        }

        const swingDeg = Math.max(-14, Math.min(14, -dxScreen * 0.75));
        document.documentElement.style.setProperty('--drag-swing', `${swingDeg.toFixed(1)}deg`);

        if (dxScreen > 2.0 && state.facing !== 'right') {
          state.facing = 'right';
          container.classList.remove('facing-left');
          container.classList.add('facing-right');
        } else if (dxScreen < -2.0 && state.facing !== 'left') {
          state.facing = 'left';
          container.classList.remove('facing-right');
          container.classList.add('facing-left');
        }

        ipc.moveWindowBy(dxScreen, dyScreen);
      }
      return;
    }

    const dx = e.clientX - state.dragStartX;
    const dy = e.clientY - state.dragStartY;
    const instantVx = e.clientX - state.lastDragX;
    state.lastDragX = e.clientX;
    state.vx = instantVx;

    if (Math.abs(dx) > 4 || Math.abs(dy) > 4) {
      if (!state.hasMoved) {
        state.hasMoved = true;
        playSound('clickPress');
        setState('dragging', getRandomDialogue('dragged'));
      }

      const swingDeg = Math.max(-14, Math.min(14, -instantVx * 0.75));
      document.documentElement.style.setProperty('--drag-swing', `${swingDeg.toFixed(1)}deg`);

      if (instantVx > 2.2 && state.facing !== 'right') {
        state.facing = 'right';
        container.classList.remove('facing-left');
        container.classList.add('facing-right');
      } else if (instantVx < -2.2 && state.facing !== 'left') {
        state.facing = 'left';
        container.classList.remove('facing-right');
        container.classList.add('facing-left');
      }
    }

    let newX = state.petInitialX + dx;
    let newY = state.petInitialY + dy;

    const maxW = window.innerWidth - container.offsetWidth;
    const maxH = window.innerHeight - container.offsetHeight;
    newX = Math.max(0, Math.min(newX, maxW));
    newY = Math.max(0, Math.min(newY, maxH));

    container.style.left = `${newX}px`;
    container.style.top = `${newY}px`;
    container.style.bottom = 'auto';
    container.style.right = 'auto';
    adjustBubblePosition();
  });

  window.addEventListener('pointerup', () => {
    if (!state.isDragging) return;
    state.isDragging = false;
    container.classList.remove('dragging');
    document.documentElement.style.setProperty('--drag-swing', '0deg');
    ipc.setDragging(false);

    if (!state.hasMoved) {
      playSound('clickRelease');
      setState('rebound', null, 500);
      showBubble(getRandomDialogue('idle'), 3200);
      syncHitRectsToDesktop();
    } else if (state.windowMode === 'compact' && ipc && ipc.isDesktop) {
      ipc.releaseWindow(state.vx, state.gravityEnabled, state.floorMode || 'auto');
      syncHitRectsToDesktop();
    } else {
      startReleasePhysics(false);
    }
  });

  bubbleText.addEventListener('click', (e) => {
    e.stopPropagation();
    initAudio();
    playSound('clickPress');
    showBubble(getRandomDialogue('idle'), 3200);
  });

  // =========================================================================
  // Smooth Mouse Wheel Zoom (0.55x ~ 2.40x)
  // =========================================================================

  function showZoomPill(pct) {
    if (!zoomPill) return;
    zoomPill.textContent = `${pct}%`;
    zoomPill.classList.remove('hidden');
    if (state.zoomPillTimer) clearTimeout(state.zoomPillTimer);
    state.zoomPillTimer = setTimeout(() => {
      zoomPill.classList.add('hidden');
      state.zoomPillTimer = null;
    }, 1200);
  }

  function applyScale(val) {
    document.documentElement.style.setProperty('--pet-scale', val.toFixed(3));
    if (zoomStatus) zoomStatus.textContent = `${Math.round(val * 100)}%`;
  }

  function smoothZoomTo(newTarget, persist = true) {
    state.targetScale = Math.max(0.55, Math.min(2.4, newTarget));
    showZoomPill(Math.round(state.targetScale * 100));

    if (state.zoomRaf) return;
    const step = () => {
      const diff = state.targetScale - state.scale;
      if (Math.abs(diff) > 0.002) {
        state.scale += diff * 0.24;
        applyScale(state.scale);
        adjustBubblePosition();
        syncHitRectsToDesktop();
        state.zoomRaf = requestAnimationFrame(step);
      } else {
        state.scale = state.targetScale;
        applyScale(state.scale);
        adjustBubblePosition();
        syncHitRectsToDesktop();
        state.zoomRaf = null;
        if (persist) {
          debouncedSaveSettings({ scale: parseFloat(state.scale.toFixed(2)) });
        }
      }
    };
    state.zoomRaf = requestAnimationFrame(step);
  }

  window.__antiPetResetZoom = () => {
    smoothZoomTo(1.0);
    showBubble('已重置桌宠大小为 100%~', 2400);
  };

  container.addEventListener(
    'wheel',
    (e) => {
      if (state.dndPassthrough) return;
      e.preventDefault();
      e.stopPropagation();

      let delta = e.deltaY;
      if (e.deltaMode === 1) delta *= 20;
      else if (e.deltaMode === 2) delta *= 300;

      const zoomSensitivity = e.ctrlKey ? 0.006 : 0.0018;
      const zoomFactor = Math.exp(-delta * zoomSensitivity);
      const clampedFactor = Math.max(0.85, Math.min(1.15, zoomFactor));
      smoothZoomTo(state.targetScale * clampedFactor);
    },
    { passive: false }
  );

  let initialPinchScale = 1.0;
  container.addEventListener('gesturestart', (e) => {
    if (state.dndPassthrough) return;
    e.preventDefault();
    initialPinchScale = state.targetScale;
  });

  container.addEventListener('gesturechange', (e) => {
    if (state.dndPassthrough) return;
    e.preventDefault();
    smoothZoomTo(initialPinchScale * e.scale, false);
  });

  container.addEventListener('gestureend', (e) => {
    if (state.dndPassthrough) return;
    e.preventDefault();
    debouncedSaveSettings({ scale: parseFloat(state.scale.toFixed(2)) });
  });

  // =========================================================================
  // 7. Quota Updater, Cursor-Anchored Context Menu & Mode Controls
  // =========================================================================

  function updateQuota(fiveHourPct, weekPct, metaText = null) {
    if (typeof fiveHourPct === 'number') {
      const pct5 = Math.min(100, Math.max(0, fiveHourPct));
      quota5hBar.style.width = `${pct5}%`;
      quota5hVal.textContent = `${Math.round(pct5)}%`;
      quota5hBar.className = `quota-bar-fill ${pct5 <= 10 ? 'danger' : pct5 <= 30 ? 'warning' : ''}`;
    }
    if (typeof weekPct === 'number') {
      const pctW = Math.min(100, Math.max(0, weekPct));
      quotaWeekBar.style.width = `${pctW}%`;
      quotaWeekVal.textContent = `${Math.round(pctW)}%`;
    }
    if (metaText && quotaMeta) {
      quotaMeta.textContent = metaText;
    }
    adjustBubblePosition();
  }

  function toggleDndPassthrough(forceVal = null) {
    state.dndPassthrough = typeof forceVal === 'boolean' ? forceVal : !state.dndPassthrough;
    if (dndStatus) dndStatus.textContent = state.dndPassthrough ? '开启' : '关闭';
    container.classList.toggle('dnd-ghost', state.dndPassthrough);
    closeContextMenu();

    if (state.dndPassthrough) {
      hideBubbleImmediately();
    } else {
      showBubble('大肥猫醒来啦！', 2400);
    }
    debouncedSaveSettings({ dndPassthrough: state.dndPassthrough });
    setTimeout(syncHitRectsToDesktop, 50);
  }

  function toggleIdeOnlyMode(forceVal = null) {
    state.ideOnlyMode = typeof forceVal === 'boolean' ? forceVal : !state.ideOnlyMode;
    if (ideOnlyStatus) ideOnlyStatus.textContent = state.ideOnlyMode ? '开启' : '关闭';
    state.lastIdeActiveAt = Date.now();
    container.classList.remove('ide-hidden');
    showBubble(
      state.ideOnlyMode ? '已开启仅 IDE 活跃时显示~' : '已恢复常驻桌面陪伴~',
      2600
    );
    debouncedSaveSettings({ ideOnlyMode: state.ideOnlyMode });
    syncHitRectsToDesktop();
  }

  async function cycleSkin() {
    if (!state.availableSkins.length) {
      await fetchAvailableSkins();
    }
    if (state.availableSkins.length > 1) {
      const idx = state.availableSkins.findIndex((s) => s.id === state.skinId);
      const nextSkin = state.availableSkins[(idx + 1) % state.availableSkins.length];
      await loadSkin(nextSkin.id);
      showBubble(`已切换至皮肤：${nextSkin.name || nextSkin.id}`, 2800);
      debouncedSaveSettings({ skinId: nextSkin.id });
    } else {
      showBubble('当前皮肤：大肥猫', 2600);
    }
  }

  window.__antiPetCycleSkin = () => {
    cycleSkin();
  };
  window.__antiPetToggleDnd = () => {
    toggleDndPassthrough();
  };

  window.__antiPetOnModeChange = function (mode) {
    state.windowMode = mode === 'compact' ? 'compact' : 'fullscreen';
    if (state.windowMode === 'compact') {
      document.body.classList.add('compact-mode');
      state.dockedEdge = null;
      container.classList.remove('docked-left', 'docked-right');
      snapToCurrentFloor();
    } else {
      document.body.classList.remove('compact-mode');
      const maxW = window.innerWidth - (container.offsetWidth || 136) - 24;
      const targetY = computeTargetFloorY();
      container.style.left = `${Math.max(0, maxW)}px`;
      container.style.top = `${targetY}px`;
      container.style.bottom = 'auto';
      container.style.right = 'auto';
      adjustBubblePosition();
      syncHitRectsToDesktop();
    }
  };

  window.addEventListener('keydown', (e) => {
    if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key.toLowerCase() === 'p') {
      e.preventDefault();
      toggleDndPassthrough();
      return;
    }
    if ((e.metaKey || e.ctrlKey) && e.key === ',') {
      e.preventDefault();
      openSettings();
      return;
    }
    if (e.key === 'Escape') {
      closeContextMenu();
      return;
    }
    if (
      e.target &&
      (e.target.tagName === 'INPUT' ||
        e.target.tagName === 'SELECT' ||
        e.target.tagName === 'TEXTAREA')
    ) {
      return;
    }
    closeContextMenu();
    hideBubbleImmediately();
  });

  async function updateContextMenuVisibility() {
    const config = state.menuConfig || {};
    const items = contextMenu.querySelectorAll('[data-menu-key]');
    items.forEach((item) => {
      const key = item.getAttribute('data-menu-key');
      if (key === 'openAntigravity') return;
      const val = config[key];
      let shouldShow = true;
      if (val === false || val === 'never') {
        shouldShow = false;
      } else if (val === 'active') {
        if (key === 'dnd') shouldShow = state.dndPassthrough;
        else if (key === 'ideOnly') shouldShow = state.ideOnlyMode;
        else if (key === 'gravity') shouldShow = state.gravityEnabled;
        else if (key === 'quota') shouldShow = state.showQuota;
      }
      // Always show DND menu item when DND is active so user can exit DND via right-click
      if (key === 'dnd' && state.dndPassthrough) {
        shouldShow = true;
      }
      item.classList.toggle('hidden', !shouldShow);
    });

    const agItem = contextMenu.querySelector('[data-menu-key="openAntigravity"]');
    if (agItem) {
      const agVal = config.openAntigravity;
      if (agVal === false || agVal === 'never') {
        agItem.classList.add('hidden');
      } else if (agVal === 'always') {
        agItem.classList.remove('hidden');
      } else {
        try {
          const res = await fetch('/api/antigravity/status');
          if (res.ok) {
            const data = await res.json();
            agItem.classList.toggle('hidden', Boolean(data.running));
          } else {
            agItem.classList.remove('hidden');
          }
        } catch (err) {
          agItem.classList.remove('hidden');
        }
      }
    }

    const children = Array.from(contextMenu.children);
    let prevWasItem = false;
    children.forEach((el) => {
      if (el.classList.contains('menu-divider')) {
        if (!prevWasItem) {
          el.classList.add('hidden');
        } else {
          el.classList.remove('hidden');
          prevWasItem = false;
        }
      } else if (!el.classList.contains('hidden')) {
        prevWasItem = true;
      }
    });
    for (let i = children.length - 1; i >= 0; i--) {
      if (!children[i].classList.contains('hidden')) {
        if (children[i].classList.contains('menu-divider')) {
          children[i].classList.add('hidden');
        }
        break;
      }
    }
  }

  // Right-click on the cat ALWAYS opens the context menu (including in DND mode to exit DND)
  container.addEventListener('contextmenu', async (e) => {
    e.preventDefault();
    e.stopPropagation();

    await updateContextMenuVisibility();

    contextMenu.classList.remove('hidden');
    const menuW = contextMenu.offsetWidth || 196;
    const menuH = contextMenu.offsetHeight || 275;

    const effDockLeft = state.windowMode === 'compact' ? 0 : state.dockLeft;
    const effDockRight = state.windowMode === 'compact' ? 0 : state.dockRight;
    const effDockBottom = state.windowMode === 'compact' ? 0 : state.dockBottom;

    const minX = effDockLeft + 8;
    const maxX = Math.max(minX, window.innerWidth - effDockRight - menuW - 8);
    const minY = 8;
    const maxY = Math.max(minY, window.innerHeight - effDockBottom - menuH - 8);

    let x = e.clientX;
    let y = e.clientY;
    if (x + menuW > window.innerWidth - effDockRight - 8) {
      x = e.clientX - menuW;
    }
    if (y + menuH > window.innerHeight - effDockBottom - 8) {
      y = e.clientY - menuH;
    }

    x = Math.max(minX, Math.min(x, maxX));
    y = Math.max(minY, Math.min(y, maxY));

    contextMenu.style.left = `${Math.round(x)}px`;
    contextMenu.style.top = `${Math.round(y)}px`;
    ipc.setIgnoreCursorEvents(false);
    syncHitRectsToDesktop();
  });

  document.addEventListener('pointerdown', (e) => {
    if (!contextMenu.classList.contains('hidden') && !contextMenu.contains(e.target)) {
      closeContextMenu();
    }
  });

  contextMenu.addEventListener('click', (e) => {
    const item = e.target.closest('.menu-item');
    if (!item) return;
    const action = item.dataset.action;

    switch (action) {
      case 'open-antigravity':
        fetch('/api/antigravity/open', { method: 'POST' });
        showBubble('正在为你启动 Antigravity...', 3000);
        break;
      case 'open-settings':
        openSettings();
        break;
      case 'toggle-quota':
        quotaSection.style.display = quotaSection.style.display === 'none' ? 'flex' : 'none';
        showBubble(bubbleText.textContent || getRandomDialogue('idle'), 3500);
        break;
      case 'random-dialogue':
        playSound('clickPress');
        showBubble(getRandomDialogue('idle'), 3200);
        break;
      case 'pet-action':
        triggerPettingAction();
        break;
      case 'reset-zoom':
        smoothZoomTo(1.0);
        showBubble('已重置桌宠大小为 100%~', 2400);
        break;
      case 'toggle-dnd':
        toggleDndPassthrough();
        return;
      case 'toggle-ide-only':
        toggleIdeOnlyMode();
        break;
      case 'toggle-gravity':
        state.gravityEnabled = !state.gravityEnabled;
        if (gravityStatus) gravityStatus.textContent = state.gravityEnabled ? '开启' : '关闭';
        saveSettingsImmediately({ gravityEnabled: state.gravityEnabled });
        if (state.gravityEnabled) startReleasePhysics(false);
        break;
      case 'toggle-sound':
        state.soundEnabled = !state.soundEnabled;
        soundStatus.textContent = state.soundEnabled ? '开启' : '静音';
        debouncedSaveSettings({ soundEnabled: state.soundEnabled });
        break;
      case 'switch-skin':
        cycleSkin();
        break;
      case 'close-menu':
        break;
    }
    closeContextMenu();
  });

  // =========================================================================
  // 8. Connect to Antigravity IDE Telemetry & Persistent Settings
  // =========================================================================

  function applyRemoteSettings(remote) {
    if (!remote) return;
    if (typeof remote.scale === 'number' && Math.abs(remote.scale - state.scale) > 0.02) {
      smoothZoomTo(remote.scale, false);
    }
    if (typeof remote.soundEnabled === 'boolean' && remote.soundEnabled !== state.soundEnabled) {
      state.soundEnabled = remote.soundEnabled;
      if (soundStatus) soundStatus.textContent = state.soundEnabled ? '开启' : '静音';
    }
    if (typeof remote.soundVolume === 'number') {
      state.soundVolume = remote.soundVolume;
    }
    if (typeof remote.gravityEnabled === 'boolean' && remote.gravityEnabled !== state.gravityEnabled) {
      state.gravityEnabled = remote.gravityEnabled;
      if (gravityStatus) gravityStatus.textContent = state.gravityEnabled ? '开启' : '关闭';
    }
    if (typeof remote.floorMode === 'string') {
      state.floorMode = remote.floorMode;
    }
    if (typeof remote.showQuota === 'boolean') {
      state.showQuota = remote.showQuota;
      if (quotaSection) quotaSection.style.display = remote.showQuota ? 'flex' : 'none';
    }
    if (typeof remote.bubbleDurationMs === 'number') {
      state.bubbleDurationMs = remote.bubbleDurationMs;
    }
    if (typeof remote.ideOnlyMode === 'boolean' && remote.ideOnlyMode !== state.ideOnlyMode) {
      state.ideOnlyMode = remote.ideOnlyMode;
      if (ideOnlyStatus) ideOnlyStatus.textContent = state.ideOnlyMode ? '开启' : '关闭';
    }
    if (typeof remote.dndPassthrough === 'boolean' && remote.dndPassthrough !== state.dndPassthrough) {
      toggleDndPassthrough(remote.dndPassthrough);
    }
    if (remote.skinId && remote.skinId !== state.skinId) {
      loadSkin(remote.skinId);
    }
    if (remote.menuConfig && typeof remote.menuConfig === 'object') {
      state.menuConfig = remote.menuConfig;
    }
  }

  async function initSettings() {
    try {
      const savedScale = localStorage.getItem('anti-pet-scale');
      if (savedScale) {
        state.scale = parseFloat(JSON.parse(savedScale));
        state.targetScale = state.scale;
        applyScale(state.scale);
      }
      const savedSound = localStorage.getItem('anti-pet-soundEnabled');
      if (savedSound !== null) {
        state.soundEnabled = JSON.parse(savedSound);
        if (soundStatus) soundStatus.textContent = state.soundEnabled ? '开启' : '静音';
      }
      const savedVol = localStorage.getItem('anti-pet-soundVolume');
      if (savedVol !== null) state.soundVolume = parseFloat(savedVol);
      const savedFloor = localStorage.getItem('anti-pet-floorMode');
      if (savedFloor) state.floorMode = savedFloor.replace(/"/g, '');
      const savedQuota = localStorage.getItem('anti-pet-showQuota');
      if (savedQuota !== null) {
        state.showQuota = JSON.parse(savedQuota);
        if (quotaSection) quotaSection.style.display = state.showQuota ? 'flex' : 'none';
      }
      const savedDuration = localStorage.getItem('anti-pet-bubbleDurationMs');
      if (savedDuration !== null) state.bubbleDurationMs = parseInt(savedDuration, 10);
      const savedSkin = localStorage.getItem('anti-pet-skinId');
      if (savedSkin) {
        state.skinId = savedSkin.replace(/"/g, '');
      }
      const savedGravity = localStorage.getItem('anti-pet-gravityEnabled');
      if (savedGravity !== null) {
        state.gravityEnabled = JSON.parse(savedGravity);
        if (gravityStatus) gravityStatus.textContent = state.gravityEnabled ? '开启' : '关闭';
      }
    } catch (e) {}

    if (bridge && typeof bridge.fetchSettings === 'function') {
      const remote = await bridge.fetchSettings();
      if (remote) {
        if (typeof remote.scale === 'number') {
          state.scale = remote.scale;
          state.targetScale = remote.scale;
          applyScale(state.scale);
        }
        if (typeof remote.soundEnabled === 'boolean') {
          state.soundEnabled = remote.soundEnabled;
          if (soundStatus) soundStatus.textContent = state.soundEnabled ? '开启' : '静音';
        }
        if (typeof remote.soundVolume === 'number') state.soundVolume = remote.soundVolume;
        if (typeof remote.floorMode === 'string') state.floorMode = remote.floorMode;
        if (typeof remote.showQuota === 'boolean') {
          state.showQuota = remote.showQuota;
          if (quotaSection) quotaSection.style.display = remote.showQuota ? 'flex' : 'none';
        }
        if (typeof remote.bubbleDurationMs === 'number') state.bubbleDurationMs = remote.bubbleDurationMs;
        if (typeof remote.gravityEnabled === 'boolean') {
          state.gravityEnabled = remote.gravityEnabled;
          if (gravityStatus) gravityStatus.textContent = state.gravityEnabled ? '开启' : '关闭';
        }

        if (typeof remote.ideOnlyMode === 'boolean') {
          state.ideOnlyMode = remote.ideOnlyMode;
          if (ideOnlyStatus) ideOnlyStatus.textContent = state.ideOnlyMode ? '开启' : '关闭';
        }
        if (typeof remote.dndPassthrough === 'boolean' && remote.dndPassthrough !== state.dndPassthrough) {
          toggleDndPassthrough(remote.dndPassthrough);
        }
        if (remote.skinId) {
          state.skinId = remote.skinId;
        }
        if (remote.menuConfig && typeof remote.menuConfig === 'object') {
          state.menuConfig = remote.menuConfig;
        }
        if (
          state.windowMode !== 'compact' &&
          typeof remote.posX === 'number' &&
          typeof remote.posY === 'number'
        ) {
          const maxW = window.innerWidth - (container.offsetWidth || 136);
          const maxH = window.innerHeight - (container.offsetHeight || 136);
          const clampedX = Math.max(0, Math.min(remote.posX, maxW));
          const clampedY = Math.max(0, Math.min(remote.posY, maxH));
          container.style.left = `${clampedX}px`;
          container.style.top = `${clampedY}px`;
          container.style.bottom = 'auto';
          container.style.right = 'auto';
          state.hasMoved = true;
          adjustBubblePosition();
          syncHitRectsToDesktop();
        }
      }
    }
  }

  bridge = new window.PetBridge({
    onEvent: (event) => {
      state.lastIdeActiveAt = Date.now();
      container.classList.remove('ide-hidden');
      if (event.state) {
        const linePoolKey =
          event.state === 'thinking'
            ? 'thinking'
            : event.state === 'working'
            ? 'working'
            : event.state === 'success'
            ? 'success'
            : event.state === 'error'
            ? 'error'
            : null;
        const msg =
          event.bubbleText || (linePoolKey ? getRandomDialogue(linePoolKey) : null);
        setState(event.state, msg, event.durationMs || 3000);
      }
      if (event.quota) {
        updateQuota(event.quota.fiveHour, event.quota.weekly, event.quota.meta);
      }
    },
    onActivity: () => {
      state.lastIdeActiveAt = Date.now();
      if (container.classList.contains('ide-hidden')) {
        container.classList.remove('ide-hidden');
        syncHitRectsToDesktop();
      }
    },
    onSettings: (settings) => {
      applyRemoteSettings(settings);
    },
    onQuota: (quota) => {
      updateQuota(quota.fiveHour, quota.weekly, quota.meta);
    },
    onDialogueUpdated: (data) => {
      if (data && data.skinId === state.skinId) {
        loadSkin(state.skinId);
      }
    },
    onControl: (ctrl) => {
      if (!ctrl || !ctrl.action) return;
      if (ctrl.action === 'toggle-dnd') toggleDndPassthrough(ctrl.enabled);
      if (ctrl.action === 'toggle-visibility') {
        const hidden = container.classList.toggle('ide-hidden');
        ipc.setVisible(!hidden);
        syncHitRectsToDesktop();
      }
      if (ctrl.action === 'switch-skin' && ctrl.skinId) {
        loadSkin(ctrl.skinId);
      }
    }
  });
  bridge.start();

  function openSettings() {
    closeContextMenu();
    if (ipc && ipc.isDesktop) {
      ipc.sendRaw('open_settings');
    } else {
      window.open('/settings.html', 'AntiPetSettings', 'width=680,height=520,resizable=no');
    }
  }

  window.__openSettings = openSettings;

  setInterval(async () => {
    if (state.ideOnlyMode) {
      if (bridge && typeof bridge.fetchIdeStatus === 'function') {
        const status = await bridge.fetchIdeStatus();
        if (status && status.active) {
          state.lastIdeActiveAt = Date.now();
          if (container.classList.contains('ide-hidden')) {
            container.classList.remove('ide-hidden');
            syncHitRectsToDesktop();
          }
        } else if (Date.now() - state.lastIdeActiveAt > 120000) {
          container.classList.add('ide-hidden');
          syncHitRectsToDesktop();
          return;
        }
      }
    }
    if (
      state.currentState === 'idle' &&
      state.isHoveringPet &&
      !state.isDragging &&
      !state.dndPassthrough &&
      !state.isUserTyping
    ) {
      showBubble(getRandomDialogue('idle'), 3500);
    }
  }, 20000);

  window.addEventListener('resize', () => {
    if (!state.isDragging && !state.dockedEdge) {
      snapToCurrentFloor();
    } else {
      adjustBubblePosition();
      syncHitRectsToDesktop();
    }
  });

  applyScale(state.scale);
  fetchAvailableSkins().then(() => {
    initSettings().then(() => {
      loadSkin(state.skinId || 'fat-cat');
    });
  });
})();
