/**
 * Anti-Pet Core Engine
 * - Cloud Thought Bubble ("小猫挂件在想的东西") with Dynamic Screen-Edge Clamping
 * - Cursor-Anchored Right-Click Context Menu
 * - Auto-Hide Thought Bubble when Cursor Leaves, Window Blurs, or User Types
 * - Dual-Ledge Gravity Physics (Mac Dock Bar & 0px Fullscreen Bottom) + Side Edges
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
  const dndWakePill = document.getElementById('dnd-wake-pill');
  const zoomPill = document.getElementById('zoom-pill');
  const zoomStatus = document.getElementById('zoom-status');

  // Settings Modal DOM References
  const settingsModal = document.getElementById('settings-modal');
  const settingsBackdrop = document.getElementById('settings-backdrop');
  const settingsWindow = settingsModal ? settingsModal.querySelector('.settings-window') : null;
  const btnSettingsCloseX = document.getElementById('btn-settings-close-x');
  const btnSettingsCancel = document.getElementById('btn-settings-cancel');
  const btnSettingsSave = document.getElementById('btn-settings-save');
  const navTabs = document.querySelectorAll('.nav-tab');
  const tabPanes = document.querySelectorAll('.tab-pane');

  const settingScale = document.getElementById('setting-scale');
  const settingScaleVal = document.getElementById('setting-scale-val');
  const settingFloorMode = document.getElementById('setting-floor-mode');
  const settingGravity = document.getElementById('setting-gravity');
  const settingSound = document.getElementById('setting-sound');
  const settingVolume = document.getElementById('setting-volume');
  const btnTestSound = document.getElementById('btn-test-sound');
  const settingShowQuota = document.getElementById('setting-show-quota');
  const settingBubbleDuration = document.getElementById('setting-bubble-duration');
  const settingBubbleDurationVal = document.getElementById('setting-bubble-duration-val');
  const settingIdeOnly = document.getElementById('setting-ide-only');
  const settingDnd = document.getElementById('setting-dnd');

  const settingAutostart = document.getElementById('setting-autostart');
  const autostartDesc = document.getElementById('autostart-desc');
  const autostartStatusNote = document.getElementById('autostart-status-note');
  const updateCurrentVer = document.getElementById('update-current-ver');
  const updateStatusMsg = document.getElementById('update-status-msg');
  const btnCheckUpdate = document.getElementById('btn-check-update');
  const updateDetailsBox = document.getElementById('update-details-box');
  const updateBadgeTag = document.getElementById('update-badge-tag');
  const updateDate = document.getElementById('update-date');
  const updateChangelog = document.getElementById('update-changelog');
  const btnUpdateDownload = document.getElementById('btn-update-download');

  const skinsCount = document.getElementById('skins-count');
  const btnShowCreateSkin = document.getElementById('btn-show-create-skin');
  const createSkinForm = document.getElementById('create-skin-form');
  const newSkinId = document.getElementById('new-skin-id');
  const newSkinName = document.getElementById('new-skin-name');
  const newSkinAuthor = document.getElementById('new-skin-author');
  const newSkinDesc = document.getElementById('new-skin-desc');
  const btnCancelCreateSkin = document.getElementById('btn-cancel-create-skin');
  const btnConfirmCreateSkin = document.getElementById('btn-confirm-create-skin');
  const skinsGrid = document.getElementById('skins-grid');

  const dialogueSkinSelect = document.getElementById('dialogue-skin-select');
  const btnResetDialogue = document.getElementById('btn-reset-dialogue');
  const dialogueCategoryNav = document.getElementById('dialogue-category-nav');
  const inputNewLine = document.getElementById('input-new-line');
  const btnAddLine = document.getElementById('btn-add-line');
  const dialogueLinesList = document.getElementById('dialogue-lines-list');
  const settingsStatusHint = document.getElementById('settings-status-hint');

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
    if (settingsModal && !settingsModal.classList.contains('hidden')) {
      if (isPointInElement(clientX, clientY, settingsWindow || settingsModal)) return true;
    }
    if (state.isDragging) return true;
    if (state.dndPassthrough) {
      return isPointInElement(clientX, clientY, dndWakePill);
    }
    if (container.classList.contains('ide-hidden')) {
      return false;
    }
    if (!contextMenu.classList.contains('hidden')) {
      const mRect = contextMenu.getBoundingClientRect();
      const catRect = petFrame.getBoundingClientRect();
      const dist = Math.min(distanceToRect(clientX, clientY, mRect), distanceToRect(clientX, clientY, catRect));
      if (dist <= 300) return true;
    }
    if (isPointInElement(clientX, clientY, bubbleCloud || speechBubble)) return true;
    return isOpaquePixelOnCat(clientX, clientY);
  }

  function syncHitRectsToDesktop() {
    if (!ipc) return;
    if (settingsModal && !settingsModal.classList.contains('hidden') && settingsWindow) {
      const sRect = settingsWindow.getBoundingClientRect();
      const rects = [{
        x: sRect.left - 4,
        y: sRect.top - 4,
        w: sRect.width + 8,
        h: sRect.height + 8
      }];
      ipc.updateHitRects(rects);
      return;
    }
    if (state.dndPassthrough) {
      const pillRect = dndWakePill.getBoundingClientRect();
      ipc.setDndPassthrough(true, {
        x: pillRect.left - 4,
        y: pillRect.top - 4,
        w: pillRect.width + 8,
        h: pillRect.height + 8
      });
      return;
    }

    const rects = [];
    if (!container.classList.contains('ide-hidden')) {
      const catRect = petFrame.getBoundingClientRect();
      rects.push({
        x: catRect.left - 6,
        y: catRect.top - 6,
        w: catRect.width + 12,
        h: catRect.height + 12
      });

      if (!speechBubble.classList.contains('hidden')) {
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

  /**
   * Automatically shifts the cloud thought bubble horizontally & vertically
   * when the cat is close to the left, right, or top edges of the screen.
   */
  function adjustBubblePosition() {
    if (!speechBubble || speechBubble.classList.contains('hidden')) return;

    const catRect = container.getBoundingClientRect();
    const bubbleW = speechBubble.offsetWidth || 204;
    const bubbleH = (bubbleCloud || speechBubble).offsetHeight || 96;
    const screenMargin = 10;
    const minLeft = state.dockLeft + screenMargin;
    const maxRight = window.innerWidth - state.dockRight - screenMargin;

    // Centered bubble left & right in viewport coordinates
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

    // Flip thought bubble below the cat if the cat is dragged too close to the top of the screen
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
      // Cursor left the pet & bubble: fade out thought bubble shortly after (do not close context menu here)
      if (state.leaveHideTimer) clearTimeout(state.leaveHideTimer);
      state.leaveHideTimer = setTimeout(() => {
        if (!state.isHoveringPet && !state.isDragging) {
          hideBubbleImmediately();
        }
      }, 1200);
    }
  }

  // Window blur (user clicked into IDE / browser / another app)
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
    if (state.isDragging || container.classList.contains('ide-hidden')) return;
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

    // If context menu is open and cursor moved far away from both menu and cat (> 300px), close it
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
    if (!state.soundEnabled || !state.audioContext || !state.audioBuffers[name]) return;
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
    if (!state.soundEnabled) return;
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
    if (!state.soundEnabled) return;
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
    // Do not interrupt with ambient bubbles if the user is actively typing
    if (displayMsg && (!state.isUserTyping || state.isHoveringPet || newState === 'error')) {
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
  // 6. Dual-Ledge Gravity Drop & Adaptive Screen-Edge Snapping
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
    if (state.floorMode === 'taskbar') {
      return dockLedgeY;
    }
    if (state.floorMode === 'free') {
      const rect = currentRect || container.getBoundingClientRect();
      return rect.top;
    }

    // 1. Fullscreen mode or no bottom taskbar: fall to the screen bottom edge
    if (dockBottom <= 2) {
      return trueBottomY;
    }

    // 2. If user dragged Hakimi down into/past the taskbar: fall to screen bottom edge
    const rect = currentRect || container.getBoundingClientRect();
    if (rect.top >= dockLedgeY + Math.min(16, dockBottom * 0.4)) {
      return trueBottomY;
    }

    // 3. Default: fall on top of the taskbar
    return dockLedgeY;
  }

  function snapToCurrentFloor() {
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

    // Automatically adjust X near screen edges so pet & bubble sit cleanly inside bounds
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
    const bounceFactor = -0.34;
    let bounceCount = 0;

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
        if (!silent && Math.abs(vy) > 2.6 && bounceCount < 2) {
          vy *= bounceFactor;
          bounceCount += 1;
          playSoftBumpSound();
          if (bounceCount === 1) {
            setState('rebound', getRandomDialogue('release'), 500);
          }
        } else {
          container.style.left = `${curX}px`;
          container.style.top = `${targetFloor}px`;
          container.style.bottom = 'auto';
          container.style.right = 'auto';
          state.gravityRaf = null;
          if (!silent) playSound('release');
          adjustBubblePosition();
          syncHitRectsToDesktop();
          debouncedSaveSettings({
            posX: Math.round(curX),
            posY: Math.round(targetFloor)
          });
          return;
        }
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
    if (e.button === 2 || state.dndPassthrough) return;
    if (e.target.closest('#speech-bubble')) return;

    closeContextMenu();
    initAudio();
    cancelGravityAnimation();

    state.isDragging = true;
    state.hasMoved = false;
    state.dragStartX = e.clientX;
    state.dragStartY = e.clientY;
    state.lastDragX = e.clientX;
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

  // Continuous wheel scaling (Mac trackpad 2-finger scroll, mouse wheel, or pinch)
  container.addEventListener('wheel', (e) => {
    if (state.dndPassthrough) return;
    e.preventDefault();
    e.stopPropagation();

    let delta = e.deltaY;
    if (e.deltaMode === 1) delta *= 20; // lines
    else if (e.deltaMode === 2) delta *= 300; // pages

    // If ctrlKey is true (Mac trackpad pinch gesture), delta is naturally finer
    const zoomSensitivity = e.ctrlKey ? 0.006 : 0.0018;
    const zoomFactor = Math.exp(-delta * zoomSensitivity);
    const clampedFactor = Math.max(0.85, Math.min(1.15, zoomFactor));
    smoothZoomTo(state.targetScale * clampedFactor);
  }, { passive: false });

  // WebKit native gesture listeners for macOS trackpad pinch-to-zoom
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
    dndWakePill.classList.toggle('hidden', !state.dndPassthrough);
    closeContextMenu();

    if (state.dndPassthrough) {
      showBubble('已开启完全穿透免打扰，点击右下角按钮即可唤醒~', 2200);
    } else {
      showBubble('本喵回来啦！', 2600);
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

  window.__antiPetCycleSkin = () => { cycleSkin(); };
  window.__antiPetToggleDnd = () => { toggleDndPassthrough(); };

  dndWakePill.addEventListener('click', (e) => {
    e.stopPropagation();
    initAudio();
    playSound('clickRelease');
    toggleDndPassthrough(false);
  });

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
      if (settingsModal && !settingsModal.classList.contains('hidden')) {
        cancelSettings();
        return;
      }
    }
    if (e.target && (e.target.tagName === 'INPUT' || e.target.tagName === 'SELECT' || e.target.tagName === 'TEXTAREA')) {
      return;
    }
    // Hide bubble & menu immediately when user presses Escape or types
    closeContextMenu();
    hideBubbleImmediately();
  });

  /**
   * Open the right-click context menu anchored directly at the mouse cursor (e.clientX, e.clientY).
   * Automatically flips upward/leftward when near screen edges or Mac Dock so it is never cut off.
   */
  container.addEventListener('contextmenu', (e) => {
    e.preventDefault();
    e.stopPropagation();
    if (state.dndPassthrough) return;

    // Make visible first so offsetWidth and offsetHeight are accurately measured
    contextMenu.classList.remove('hidden');
    const menuW = contextMenu.offsetWidth || 196;
    const menuH = contextMenu.offsetHeight || 275;

    const minX = state.dockLeft + 8;
    const maxX = window.innerWidth - state.dockRight - menuW - 8;
    const minY = 8;
    const maxY = window.innerHeight - state.dockBottom - menuH - 8;

    // Anchor at cursor; flip left if overflowing right edge, flip up if overflowing bottom/Dock edge
    let x = e.clientX;
    let y = e.clientY;
    if (x + menuW > window.innerWidth - state.dockRight - 8) {
      x = e.clientX - menuW;
    }
    if (y + menuH > window.innerHeight - state.dockBottom - 8) {
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
  }

  async function initSettings() {
    // 1. Local storage fast path
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

    // 2. Load persistent settings from Server API
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
        if (typeof remote.posX === 'number' && typeof remote.posY === 'number') {
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

  // =========================================================================
  // 9. Settings Center Controller (Tabs, Autostart, Updates, Skins, Dialogue)
  // =========================================================================

  let draftState = null;
  let originalSnapshot = null;
  let activeDialogueCategory = 'idle';
  let activeDialogueSkinId = 'fat-cat';
  let activeDialogueData = null;
  let defaultDialogueBackup = null;

  function showToast(text, duration = 2500) {
    let toast = document.querySelector('.settings-toast');
    if (!toast) {
      toast = document.createElement('div');
      toast.className = 'settings-toast';
      document.body.appendChild(toast);
    }
    toast.textContent = text;
    toast.classList.add('show');
    setTimeout(() => {
      toast.classList.remove('show');
    }, duration);
  }

  function switchSettingsTab(tabName) {
    navTabs.forEach((tab) => {
      const match = tab.getAttribute('data-tab') === tabName;
      tab.classList.toggle('active', match);
    });
    tabPanes.forEach((pane) => {
      pane.classList.toggle('hidden', pane.id !== `tab-${tabName}`);
    });
  }

  navTabs.forEach((tab) => {
    tab.addEventListener('click', () => {
      const tabName = tab.getAttribute('data-tab');
      if (tabName) switchSettingsTab(tabName);
    });
  });

  async function openSettings(targetTab = 'general') {
    if (!settingsModal) return;

    originalSnapshot = {
      scale: state.scale,
      floorMode: state.floorMode || 'bottom',
      gravityEnabled: state.gravityEnabled,
      soundEnabled: state.soundEnabled,
      soundVolume: state.soundVolume !== undefined ? state.soundVolume : 0.8,
      showQuota: state.showQuota !== false,
      bubbleDurationMs: state.bubbleDurationMs || 4000,
      ideOnlyMode: state.ideOnlyMode,
      dndPassthrough: state.dndPassthrough,
      skinId: state.skinId || 'fat-cat'
    };

    draftState = { ...originalSnapshot, autostartEnabled: false };

    closeContextMenu();
    hideBubbleImmediately();

    settingsModal.classList.remove('hidden');
    syncHitRectsToDesktop();
    ipc.setIgnoreCursorEvents(false);

    switchSettingsTab(targetTab);

    if (settingScale) {
      settingScale.value = draftState.scale;
      if (settingScaleVal) settingScaleVal.textContent = `${Math.round(draftState.scale * 100)}%`;
    }
    if (settingFloorMode) settingFloorMode.value = draftState.floorMode;
    if (settingGravity) settingGravity.checked = draftState.gravityEnabled;
    if (settingSound) settingSound.checked = draftState.soundEnabled;
    if (settingVolume) settingVolume.value = draftState.soundVolume;
    if (settingShowQuota) settingShowQuota.checked = draftState.showQuota;
    if (settingBubbleDuration) {
      settingBubbleDuration.value = draftState.bubbleDurationMs;
      if (settingBubbleDurationVal) {
        settingBubbleDurationVal.textContent = `${(draftState.bubbleDurationMs / 1000).toFixed(1)}s`;
      }
    }
    if (settingIdeOnly) settingIdeOnly.checked = draftState.ideOnlyMode;
    if (settingDnd) settingDnd.checked = draftState.dndPassthrough;

    loadAutostartInfo();
    await renderSkinsGrid();

    activeDialogueSkinId = draftState.skinId;
    await populateDialogueSkinSelector();
    await loadDialogueForEditor(activeDialogueSkinId);
  }

  function cancelSettings() {
    if (!settingsModal || settingsModal.classList.contains('hidden')) return;

    if (originalSnapshot) {
      if (originalSnapshot.scale !== state.scale) {
        applyScale(originalSnapshot.scale);
        state.scale = originalSnapshot.scale;
        state.targetScale = originalSnapshot.scale;
      }
    }

    if (createSkinForm) createSkinForm.classList.add('hidden');

    settingsModal.classList.add('hidden');
    draftState = null;
    originalSnapshot = null;
    activeDialogueData = null;

    syncHitRectsToDesktop();
  }

  async function saveSettings() {
    if (!draftState || !settingsModal || settingsModal.classList.contains('hidden')) return;

    if (settingScale) draftState.scale = parseFloat(settingScale.value) || 1.0;
    if (settingFloorMode) draftState.floorMode = settingFloorMode.value || 'bottom';
    if (settingGravity) draftState.gravityEnabled = settingGravity.checked;
    if (settingSound) draftState.soundEnabled = settingSound.checked;
    if (settingVolume) draftState.soundVolume = parseFloat(settingVolume.value) || 0.8;
    if (settingShowQuota) draftState.showQuota = settingShowQuota.checked;
    if (settingBubbleDuration) draftState.bubbleDurationMs = parseInt(settingBubbleDuration.value, 10) || 4000;
    if (settingIdeOnly) draftState.ideOnlyMode = settingIdeOnly.checked;
    if (settingDnd) draftState.dndPassthrough = settingDnd.checked;

    const updates = {
      scale: draftState.scale,
      floorMode: draftState.floorMode,
      gravityEnabled: draftState.gravityEnabled,
      soundEnabled: draftState.soundEnabled,
      soundVolume: draftState.soundVolume,
      showQuota: draftState.showQuota,
      bubbleDurationMs: draftState.bubbleDurationMs,
      ideOnlyMode: draftState.ideOnlyMode,
      dndPassthrough: draftState.dndPassthrough,
      skinId: draftState.skinId
    };

    state.scale = updates.scale;
    state.targetScale = updates.scale;
    applyScale(state.scale);
    state.floorMode = updates.floorMode;
    state.gravityEnabled = updates.gravityEnabled;
    state.soundEnabled = updates.soundEnabled;
    state.soundVolume = updates.soundVolume;
    state.showQuota = updates.showQuota;
    state.bubbleDurationMs = updates.bubbleDurationMs;
    state.ideOnlyMode = updates.ideOnlyMode;

    if (quotaSection) quotaSection.style.display = state.showQuota ? 'flex' : 'none';
    if (soundStatus) soundStatus.textContent = state.soundEnabled ? '开启' : '静音';
    if (gravityStatus) gravityStatus.textContent = state.gravityEnabled ? '开启' : '关闭';
    if (ideOnlyStatus) ideOnlyStatus.textContent = state.ideOnlyMode ? '开启' : '关闭';

    if (updates.dndPassthrough !== state.dndPassthrough) {
      toggleDndPassthrough(updates.dndPassthrough);
    }

    if (updates.skinId !== state.skinId) {
      await loadSkin(updates.skinId);
    }

    saveSettingsImmediately(updates);

    if (settingAutostart && settingAutostart.dataset.original !== String(settingAutostart.checked)) {
      try {
        await fetch('/api/autostart', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ enabled: settingAutostart.checked })
        });
        settingAutostart.dataset.original = String(settingAutostart.checked);
      } catch (e) {}
    }

    if (activeDialogueData && activeDialogueSkinId) {
      try {
        await fetch('/api/dialogue', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            skinId: activeDialogueSkinId,
            lines: activeDialogueData.lines,
            policy: activeDialogueData.policy
          })
        });
        if (activeDialogueSkinId === state.skinId) {
          state.dialogueBank = activeDialogueData.lines;
          state.dialogues = activeDialogueData.lines[state.currentState] || activeDialogueData.lines.idle || [];
        }
      } catch (e) {}
    }

    showToast('配置已保存并生效');
    showBubble('设置已保存', 2500);

    settingsModal.classList.add('hidden');
    draftState = null;
    originalSnapshot = null;
    syncHitRectsToDesktop();
  }

  async function loadAutostartInfo() {
    if (!autostartStatusNote) return;
    try {
      const res = await fetch('/api/autostart');
      if (res.ok) {
        const data = await res.json();
        if (settingAutostart) {
          settingAutostart.checked = !!data.enabled;
          settingAutostart.dataset.original = String(!!data.enabled);
        }
        if (data.available) {
          autostartStatusNote.className = data.enabled ? 'status-note success' : 'status-note info';
          autostartStatusNote.textContent = data.enabled
            ? `已启用开机自启 (${data.platform})`
            : `未开启自启，点击开关可在系统登录时自动启动大肥猫 (${data.platform})`;
        } else {
          autostartStatusNote.className = 'status-note warn';
          autostartStatusNote.textContent = '当前平台未检测到原生自启管理器';
        }
      }
    } catch (e) {
      if (autostartStatusNote) {
        autostartStatusNote.className = 'status-note warn';
        autostartStatusNote.textContent = '无法获取自启状态';
      }
    }
  }

  if (btnCheckUpdate) {
    btnCheckUpdate.addEventListener('click', async () => {
      btnCheckUpdate.disabled = true;
      btnCheckUpdate.textContent = '正在检查...';
      if (updateStatusMsg) updateStatusMsg.textContent = '正在联网查询 GitHub 最新 Release...';

      try {
        const res = await fetch('/api/update-check');
        const data = await res.json();
        btnCheckUpdate.disabled = false;
        btnCheckUpdate.textContent = '检查更新';

        const curVer = data.currentVersion || data.current || 'v0.1.1';
        const latestVer = data.latestVersion || data.latest || curVer;

        if (updateCurrentVer) {
          updateCurrentVer.textContent = curVer;
        }

        if (data.hasUpdate) {
          if (updateStatusMsg) updateStatusMsg.textContent = `发现新版本 ${latestVer}`;
          if (updateDetailsBox) {
            updateDetailsBox.classList.remove('hidden');
            if (updateBadgeTag) updateBadgeTag.textContent = `发现新版 ${latestVer}`;
            if (updateDate) updateDate.textContent = `发布于: ${data.publishedAt ? new Date(data.publishedAt).toLocaleDateString() : '最新'}`;
            if (updateChangelog) updateChangelog.textContent = data.body || '暂无详细更新日志。';
            if (btnUpdateDownload && data.htmlUrl) {
              btnUpdateDownload.href = data.htmlUrl;
            }
          }
        } else {
          if (updateStatusMsg) updateStatusMsg.textContent = `已是最新版本 (${curVer})`;
          if (updateDetailsBox) updateDetailsBox.classList.add('hidden');
        }
      } catch (err) {
        btnCheckUpdate.disabled = false;
        btnCheckUpdate.textContent = '检查更新';
        if (updateStatusMsg) updateStatusMsg.textContent = '检查更新失败，请检查网络连接';
      }
    });
  }

  async function renderSkinsGrid() {
    if (!skinsGrid) return;
    await fetchAvailableSkins();
    if (skinsCount) skinsCount.textContent = state.availableSkins.length;

    skinsGrid.innerHTML = '';
    state.availableSkins.forEach((skin) => {
      const isCurrent = (draftState ? draftState.skinId : state.skinId) === skin.id;
      const card = document.createElement('div');
      card.className = `skin-card ${isCurrent ? 'active' : ''}`;
      card.dataset.skinId = skin.id;

      card.innerHTML = `
        <img class="skin-preview-img" src="${skin.previewUrl || `/skins/${skin.id}/preview.png`}" alt="${skin.name}" onerror="this.src='/skins/fat-cat/preview.png'">
        <div class="skin-name">${skin.name || skin.id}</div>
        <div class="skin-author">作者: ${skin.author || '未知'}</div>
        ${isCurrent ? '<span class="skin-active-pill">生效中</span>' : ''}
      `;

      card.addEventListener('click', () => {
        if (draftState) draftState.skinId = skin.id;
        document.querySelectorAll('.skin-card').forEach((c) => {
          c.classList.toggle('active', c.dataset.skinId === skin.id);
          const pill = c.querySelector('.skin-active-pill');
          if (c.dataset.skinId === skin.id) {
            if (!pill) {
              const newPill = document.createElement('span');
              newPill.className = 'skin-active-pill';
              newPill.textContent = '待保存';
              c.appendChild(newPill);
            } else {
              pill.textContent = '待保存';
            }
          } else if (pill) {
            pill.remove();
          }
        });
      });

      skinsGrid.appendChild(card);
    });
  }

  if (btnShowCreateSkin) {
    btnShowCreateSkin.addEventListener('click', () => {
      if (createSkinForm) createSkinForm.classList.toggle('hidden');
    });
  }
  if (btnCancelCreateSkin) {
    btnCancelCreateSkin.addEventListener('click', () => {
      if (createSkinForm) createSkinForm.classList.add('hidden');
    });
  }
  if (btnConfirmCreateSkin) {
    btnConfirmCreateSkin.addEventListener('click', async () => {
      const id = (newSkinId.value || '').trim();
      const name = (newSkinName.value || '').trim();
      const author = (newSkinAuthor.value || '').trim() || 'Custom Author';
      const desc = (newSkinDesc.value || '').trim() || '自定义皮肤';

      if (!id || !/^[a-zA-Z0-9_-]+$/.test(id)) {
        alert('请输入有效的皮肤 ID (英文数字下划线减号)');
        return;
      }
      if (!name) {
        alert('请输入皮肤名称');
        return;
      }

      btnConfirmCreateSkin.disabled = true;
      btnConfirmCreateSkin.textContent = '创建中...';

      try {
        const res = await fetch('/api/skins/create', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ skinId: id, name, author, description: desc })
        });
        const result = await res.json();
        btnConfirmCreateSkin.disabled = false;
        btnConfirmCreateSkin.textContent = '确认创建';

        if (result.ok) {
          showToast(`皮肤 [${name}] 创建成功`);
          createSkinForm.classList.add('hidden');
          newSkinId.value = '';
          newSkinName.value = '';
          newSkinAuthor.value = '';
          newSkinDesc.value = '';
          if (draftState) draftState.skinId = id;
          await renderSkinsGrid();
          await populateDialogueSkinSelector();
        } else {
          alert(`创建失败: ${result.error || '未知错误'}`);
        }
      } catch (err) {
        btnConfirmCreateSkin.disabled = false;
        btnConfirmCreateSkin.textContent = '确认创建';
        alert(`请求失败: ${err.message}`);
      }
    });
  }

  async function populateDialogueSkinSelector() {
    if (!dialogueSkinSelect) return;
    await fetchAvailableSkins();
    dialogueSkinSelect.innerHTML = '';
    state.availableSkins.forEach((skin) => {
      const opt = document.createElement('option');
      opt.value = skin.id;
      opt.textContent = `${skin.name || skin.id} (${skin.id})`;
      if (skin.id === activeDialogueSkinId) opt.selected = true;
      dialogueSkinSelect.appendChild(opt);
    });
  }

  if (dialogueSkinSelect) {
    dialogueSkinSelect.addEventListener('change', async (e) => {
      activeDialogueSkinId = e.target.value;
      await loadDialogueForEditor(activeDialogueSkinId);
    });
  }

  async function loadDialogueForEditor(skinId) {
    try {
      const res = await fetch(`/api/dialogue?skinId=${encodeURIComponent(skinId)}`);
      if (res.ok) {
        activeDialogueData = await res.json();
        if (!defaultDialogueBackup) {
          defaultDialogueBackup = JSON.parse(JSON.stringify(activeDialogueData));
        }
        renderDialogueLines();
      }
    } catch (e) {}
  }

  function renderDialogueLines() {
    if (!dialogueLinesList || !activeDialogueData || !activeDialogueData.lines) return;
    const lines = activeDialogueData.lines[activeDialogueCategory] || [];
    dialogueLinesList.innerHTML = '';

    if (lines.length === 0) {
      dialogueLinesList.innerHTML = '<div style="font-size: 11.5px; color: var(--text-secondary); padding: 12px; text-align: center;">该分类暂无台词，点击上方添加一条吧~</div>';
      return;
    }

    lines.forEach((lineText, idx) => {
      const row = document.createElement('div');
      row.className = 'dialogue-line-item';

      const input = document.createElement('input');
      input.className = 'dialogue-line-input';
      input.type = 'text';
      input.value = lineText;
      input.addEventListener('input', (e) => {
        lines[idx] = e.target.value;
      });

      const delBtn = document.createElement('button');
      delBtn.className = 'btn-del-line';
      delBtn.title = '删除此条';
      delBtn.textContent = '删除';
      delBtn.addEventListener('click', () => {
        lines.splice(idx, 1);
        renderDialogueLines();
      });

      row.appendChild(input);
      row.appendChild(delBtn);
      dialogueLinesList.appendChild(row);
    });
  }

  if (dialogueCategoryNav) {
    dialogueCategoryNav.addEventListener('click', (e) => {
      const pill = e.target.closest('.cat-pill');
      if (!pill) return;
      const stateKey = pill.dataset.state;
      if (!stateKey) return;
      activeDialogueCategory = stateKey;
      dialogueCategoryNav.querySelectorAll('.cat-pill').forEach((p) => {
        p.classList.toggle('active', p === pill);
      });
      renderDialogueLines();
    });
  }

  if (btnAddLine && inputNewLine) {
    const handleAdd = () => {
      const text = (inputNewLine.value || '').trim();
      if (!text) return;
      if (!activeDialogueData) activeDialogueData = { lines: {}, policy: {} };
      if (!activeDialogueData.lines) activeDialogueData.lines = {};
      if (!activeDialogueData.lines[activeDialogueCategory]) {
        activeDialogueData.lines[activeDialogueCategory] = [];
      }
      activeDialogueData.lines[activeDialogueCategory].push(text);
      inputNewLine.value = '';
      renderDialogueLines();
    };

    btnAddLine.addEventListener('click', handleAdd);
    inputNewLine.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        e.preventDefault();
        handleAdd();
      }
    });
  }

  if (btnResetDialogue) {
    btnResetDialogue.addEventListener('click', () => {
      if (!defaultDialogueBackup) return;
      if (confirm('确认恢复该皮肤的出厂预设台词吗？')) {
        activeDialogueData.lines = JSON.parse(JSON.stringify(defaultDialogueBackup.lines || {}));
        renderDialogueLines();
        showToast('已重置为出厂预设台词');
      }
    });
  }

  if (settingScale) {
    settingScale.addEventListener('input', (e) => {
      const val = parseFloat(e.target.value);
      if (settingScaleVal) settingScaleVal.textContent = `${Math.round(val * 100)}%`;
      applyScale(val);
      state.scale = val;
      state.targetScale = val;
    });
  }

  if (settingBubbleDuration) {
    settingBubbleDuration.addEventListener('input', (e) => {
      const val = parseInt(e.target.value, 10);
      if (settingBubbleDurationVal) {
        settingBubbleDurationVal.textContent = `${(val / 1000).toFixed(1)}s`;
      }
    });
  }

  if (btnTestSound) {
    btnTestSound.addEventListener('click', () => {
      initAudio();
      if (settingVolume) {
        state.soundVolume = parseFloat(settingVolume.value) || 0.8;
      }
      playSound('clickRelease');
    });
  }

  if (btnSettingsSave) btnSettingsSave.addEventListener('click', saveSettings);
  if (btnSettingsCancel) btnSettingsCancel.addEventListener('click', cancelSettings);
  if (btnSettingsCloseX) btnSettingsCloseX.addEventListener('click', cancelSettings);
  if (settingsBackdrop) settingsBackdrop.addEventListener('click', cancelSettings);

  if (settingsWindow) {
    settingsWindow.addEventListener('mouseenter', () => ipc.setIgnoreCursorEvents(false));
    settingsWindow.addEventListener('mousemove', () => ipc.setIgnoreCursorEvents(false));
  }

  window.__openSettings = openSettings;
  window.__closeSettings = cancelSettings;

  // Periodic check for IDE-only visibility; only show idle thought bubble if user is hovering the pet
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

  window.__antiPetOnModeChange = function (mode) {
    if (mode === 'compact') {
      document.body.classList.add('compact-mode');
    } else {
      document.body.classList.remove('compact-mode');
    }
    snapToCurrentFloor();
  };