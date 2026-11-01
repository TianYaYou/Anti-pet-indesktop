/**
 * Anti-Pet Core Engine
 * - Dynamic Click-Through & Offscreen Canvas Pixel-Level Alpha Hit-Testing
 * - Cursor Gaze Tracking & Head-Rubbing (Petting) Gesture Recognition
 * - Gravity Physics, Elastic Rebound, Dock/Taskbar Snapping & Screen-Edge Peeking
 * - Antigravity IDE Live Telemetry, Dual-Window Quota & Tray Mode Controls
 */
(function () {
  const container = document.getElementById('pet-container');
  const spriteWrapper = document.getElementById('sprite-wrapper');
  const petFrame = document.getElementById('pet-frame');
  const toolWrench = document.getElementById('tool-wrench');
  const speechBubble = document.getElementById('speech-bubble');
  const bubbleText = document.getElementById('bubble-text');
  const effectLayer = document.getElementById('effect-layer');
  const particleLayer = document.getElementById('particle-layer');
  const statusBadge = document.getElementById('status-badge');
  const badgeLabel = document.getElementById('badge-label');
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

  const ipc = window.DesktopIPC;

  // Offscreen Canvas for Pixel-Level Alpha Hit-Testing
  const hitCanvas = document.createElement('canvas');
  const hitCtx = hitCanvas.getContext('2d', { willReadFrequently: true });
  let hitCanvasReady = false;

  // Application State
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
    lastIdeActiveAt: Date.now(),
    facing: 'left', // 'left' or 'right'
    dockedEdge: null, // 'left' | 'right' | null
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
    // Petting / Head-rubbing gesture tracking
    rubLastX: null,
    rubLastDir: 0,
    rubCount: 0,
    rubResetTimer: null,
    lastPettingAt: 0,
    // Timers & Audio
    stateTimer: null,
    bubbleHideTimer: null,
    blinkTimer: null,
    audioContext: null,
    audioBuffers: {}
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
    if (!hitCanvasReady) {
      return true; // Fallback to bounding box if canvas not yet decoded
    }
    let normX = (clientX - rect.left) / rect.width;
    const normY = (clientY - rect.top) / rect.height;
    // Account for horizontal flip when facing right
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
    if (state.dndPassthrough) {
      return isPointInElement(clientX, clientY, dndWakePill);
    }
    if (container.classList.contains('ide-hidden')) {
      return false;
    }
    if (isPointInElement(clientX, clientY, contextMenu)) return true;
    if (isPointInElement(clientX, clientY, speechBubble)) return true;
    if (isPointInElement(clientX, clientY, statusBadge)) return true;
    return isOpaquePixelOnCat(clientX, clientY);
  }

  function syncHitRectsToDesktop() {
    if (!ipc) return;
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

      const badgeRect = statusBadge.getBoundingClientRect();
      rects.push({
        x: badgeRect.left - 4,
        y: badgeRect.top - 4,
        w: badgeRect.width + 8,
        h: badgeRect.height + 8
      });

      if (!speechBubble.classList.contains('hidden')) {
        const bRect = speechBubble.getBoundingClientRect();
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
      rects.push({
        x: mRect.left - 6,
        y: mRect.top - 6,
        w: mRect.width + 12,
        h: mRect.height + 12
      });
    }

    ipc.updateHitRects(rects);
  }

  // Explicit mouseenter / mouseleave & pixel-level hit-testing
  [container, speechBubble, contextMenu, dndWakePill].forEach((el) => {
    if (!el) return;
    el.addEventListener('mouseenter', () => {
      if (!state.dndPassthrough || el === dndWakePill) {
        ipc.setIgnoreCursorEvents(false);
      }
    });
    el.addEventListener('mouseleave', (e) => {
      if (state.isDragging) return;
      const stillOver = evaluateInteractiveHit(e.clientX, e.clientY);
      ipc.setIgnoreCursorEvents(!stillOver);
    });
  });

  // =========================================================================
  // 2. Cursor Gaze Tracking & Petting (Head-Rubbing) Gesture Recognition
  // =========================================================================

  function updateCursorGaze(clientX, clientY) {
    if (state.isDragging || container.classList.contains('ide-hidden')) return;
    const rect = petFrame.getBoundingClientRect();
    if (rect.width <= 0) return;

    const centerX = rect.left + rect.width * 0.5;
    const centerY = rect.top + rect.height * 0.42;
    const dx = clientX - centerX;
    const dy = clientY - centerY;

    const maxShiftX = 4.5;
    const maxShiftY = 3.2;
    const maxTilt = 3.2;

    const normX = Math.max(-1, Math.min(1, dx / 260));
    const normY = Math.max(-1, Math.min(1, dy / 220));

    const shiftX = (normX * maxShiftX).toFixed(2);
    const shiftY = (normY * maxShiftY).toFixed(2);
    const tilt = (normX * maxTilt).toFixed(2);

    document.documentElement.style.setProperty('--gaze-x', `${shiftX}px`);
    document.documentElement.style.setProperty('--gaze-y', `${shiftY}px`);
    document.documentElement.style.setProperty('--gaze-tilt', `${tilt}deg`);

    // Subtle facing turn if cursor is strongly to one side while idle and not docked
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
          spawnFloatingParticle(`../skins/${state.skinId}/ui/heart.svg`, clientX - rect.left);

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

  function spawnFloatingParticle(imgSrc, offsetX = null) {
    if (!particleLayer) return;
    const img = document.createElement('img');
    img.src = imgSrc;
    img.className = 'floating-particle';
    const x = offsetX !== null ? Math.max(20, Math.min(165, offsetX)) : 65 + Math.random() * 70;
    const drift = (Math.random() - 0.5) * 34;
    img.style.left = `${x}px`;
    img.style.top = `${18 + Math.random() * 20}px`;
    img.style.setProperty('--px', `${drift.toFixed(1)}px`);
    particleLayer.appendChild(img);
    setTimeout(() => img.remove(), 1150);
  }

  function triggerPettingAction() {
    initAudio();
    playPurrSound();
    const petLine = getRandomDialogue('petting');
    setState('petting', petLine, 2600);
    for (let i = 0; i < 3; i++) {
      setTimeout(() => {
        spawnFloatingParticle(`../skins/${state.skinId}/ui/heart.svg`);
      }, i * 240);
    }
  }

  // Global hook called by Rust 35Hz cursor tracker even during click-through
  window.__antiPetOnGlobalCursor = function (localX, localY) {
    updateCursorGaze(localX, localY);
    const shouldIntercept = evaluateInteractiveHit(localX, localY);
    ipc.setIgnoreCursorEvents(!shouldIntercept);
  };

  window.addEventListener('pointermove', (e) => {
    updateCursorGaze(e.clientX, e.clientY);
    if (!state.isDragging) {
      const shouldIntercept = evaluateInteractiveHit(e.clientX, e.clientY);
      ipc.setIgnoreCursorEvents(!shouldIntercept);
      if (shouldIntercept) {
        detectHeadRubGesture(e.clientX, e.clientY);
      }
    }
  });

  // =========================================================================
  // 3. Audio System (WAV Buffers + Synthesized Purr & Bump Feedback)
  // =========================================================================

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

  // Synthesize a warm, rhythmic cat purr using Web Audio oscillators
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

      mainGain.gain.setValueAtTime(0.01, now);
      mainGain.gain.linearRampToValueAtTime(0.18, now + 0.15);
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

  // Soft thump sound when snapping to screen edge or taskbar
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
  // 4. Skin Loading, State Machine & Dialogue System
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
      const res = await fetch(`../skins/${skinId}/skin.json`);
      state.skinConfig = await res.json();
      state.skinId = skinId;
      if (skinStatus) {
        skinStatus.textContent = (state.skinConfig.name || skinId).split('·')[0].trim();
      }

      if (state.skinConfig.dialogue) {
        const diagRes = await fetch(`../skins/${skinId}/${state.skinConfig.dialogue}`);
        const diagData = await diagRes.json();
        state.dialogueBank = diagData.lines || {};
        state.dialogues = state.dialogueBank.idle || ['喵~ 认真写代码哦！'];
      }

      initAudio();
      if (state.skinConfig.audio) {
        for (const [key, relPath] of Object.entries(state.skinConfig.audio)) {
          loadAudioBuffer(key, `../skins/${skinId}/${relPath}`);
        }
      }

      setState('idle');
      showBubble(getRandomDialogue('idle'), 5000);
      scheduleIdleBlink();
    } catch (err) {
      console.error('[Pet] Error loading skin:', err);
    }
  }

  function scheduleIdleBlink() {
    if (state.blinkTimer) clearTimeout(state.blinkTimer);
    const nextBlinkMs = 3200 + Math.random() * 4200;
    state.blinkTimer = setTimeout(() => {
      if (
        state.currentState === 'idle' &&
        !state.isDragging &&
        state.skinConfig?.frames?.blink
      ) {
        petFrame.src = `../skins/${state.skinId}/${state.skinConfig.frames.blink}`;
        setTimeout(() => {
          if (state.currentState === 'idle' && state.skinConfig?.frames?.idle) {
            petFrame.src = `../skins/${state.skinId}/${state.skinConfig.frames.idle}`;
          }
        }, 160);
      }
      scheduleIdleBlink();
    }, nextBlinkMs);
  }

  function setState(newState, text = null, durationMs = 0, toolName = '') {
    if (!state.skinConfig) return;
    const cfg = state.skinConfig.states?.[newState] || state.skinConfig.states?.['idle'];
    if (!cfg) return;

    if (state.stateTimer) {
      clearTimeout(state.stateTimer);
      state.stateTimer = null;
    }

    state.currentState = newState;

    // Preserve modifiers (docked, dnd-ghost, ide-hidden, dragging)
    const classes = ['pet-container', `facing-${state.facing}`, `state-${newState}`];
    if (state.isDragging) classes.push('dragging');
    if (state.dockedEdge) classes.push(`docked-${state.dockedEdge}`);
    if (state.dndPassthrough) classes.push('dnd-ghost');
    if (state.ideOnlyMode && Date.now() - state.lastIdeActiveAt > 120000) {
      classes.push('ide-hidden');
    }
    container.className = classes.join(' ');

    // Update sprite image
    const frameName = cfg.frame || 'idle';
    const framePath = state.skinConfig.frames?.[frameName] || 'frames/idle.png';
    petFrame.src = `../skins/${state.skinId}/${framePath}`;

    // Show wrench overlay when working on a tool
    if (toolWrench) {
      toolWrench.classList.toggle('hidden', newState !== 'working');
    }

    // Update floating effect
    if (cfg.effect) {
      effectLayer.innerHTML = `<img src="../skins/${state.skinId}/${cfg.effect}" alt="effect">`;
    } else {
      effectLayer.innerHTML = '';
    }

    // Spawn celebration sparkles on success
    if (newState === 'success') {
      for (let i = 0; i < 3; i++) {
        setTimeout(() => {
          spawnFloatingParticle(`../skins/${state.skinId}/ui/sparkle.svg`);
        }, i * 200);
      }
    }

    updateBadge(newState, toolName);

    const displayMsg = text || cfg.bubbleText;
    if (displayMsg) {
      showBubble(displayMsg, durationMs || 4500);
    }

    const autoRevertTime = durationMs || cfg.durationMs;
    if (autoRevertTime && autoRevertTime > 0) {
      state.stateTimer = setTimeout(() => {
        setState('idle');
      }, autoRevertTime);
    }

    setTimeout(syncHitRectsToDesktop, 60);
  }

  function updateBadge(s, toolName = '') {
    statusBadge.className = `status-badge ${s}`;
    const labels = {
      idle: '待命中',
      thinking: '思考探头中...',
      working: toolName ? `执行: ${toolName}` : '挥舞扳手中...',
      success: '搞定得意！',
      error: '遇到报错',
      petting: '呼噜享受中~',
      pressed: '按压蓄力',
      rebound: 'Duang~',
      dragging: '被拎起悬空'
    };
    badgeLabel.textContent = labels[s] || s;
  }

  function getRandomDialogue(category = 'idle') {
    const pool =
      state.dialogueBank?.[category] && state.dialogueBank[category].length
        ? state.dialogueBank[category]
        : state.dialogues;
    if (!pool || !pool.length) return '喵~';
    const idx = Math.floor(Math.random() * pool.length);
    return pool[idx];
  }

  function showBubble(text, ttlMs = 4500) {
    bubbleText.textContent = text;
    speechBubble.classList.remove('hidden');
    syncHitRectsToDesktop();

    if (state.bubbleHideTimer) {
      clearTimeout(state.bubbleHideTimer);
      state.bubbleHideTimer = null;
    }

    if (ttlMs > 0) {
      state.bubbleHideTimer = setTimeout(() => {
        if (state.currentState === 'idle' && quotaSection.style.display === 'none') {
          speechBubble.classList.add('hidden');
          syncHitRectsToDesktop();
        }
      }, ttlMs);
    }
  }

  // =========================================================================
  // 5. Physics Dragging, Gravity Drop, Bottom Taskbar & Screen-Edge Snapping
  // =========================================================================

  function cancelGravityAnimation() {
    if (state.gravityRaf) {
      cancelAnimationFrame(state.gravityRaf);
      state.gravityRaf = null;
    }
  }

  function startReleasePhysics() {
    cancelGravityAnimation();

    const rect = container.getBoundingClientRect();
    const winW = window.innerWidth;
    const winH = window.innerHeight;
    const edgeThreshold = 34;

    // 1. Check Left / Right Screen Edge Peeking (贴边半隐藏窥视)
    if (rect.left <= edgeThreshold) {
      state.dockedEdge = 'left';
      state.facing = 'right';
      container.style.left = '0px';
      container.classList.remove('facing-left', 'docked-right');
      container.classList.add('facing-right', 'docked-left');
      playSoftBumpSound();
      setState('idle');
      showBubble('本喵在屏幕左边暗中观察你写代码~', 3500);
      syncHitRectsToDesktop();
      return;
    }

    if (rect.right >= winW - edgeThreshold) {
      state.dockedEdge = 'right';
      state.facing = 'left';
      container.style.left = `${winW - container.offsetWidth}px`;
      container.classList.remove('facing-right', 'docked-left');
      container.classList.add('facing-left', 'docked-right');
      playSoftBumpSound();
      setState('idle');
      showBubble('躲在右边偷看你，不许写 Bug 哦！', 3500);
      syncHitRectsToDesktop();
      return;
    }

    state.dockedEdge = null;
    container.classList.remove('docked-left', 'docked-right');

    // 2. If Gravity Snapping is disabled, rebound in place
    const floorY = winH - container.offsetHeight - 18;
    if (!state.gravityEnabled || rect.top >= floorY - 4) {
      container.style.top = `${Math.min(rect.top, floorY)}px`;
      playSound('release');
      setState('rebound', getRandomDialogue('release'), 520);
      syncHitRectsToDesktop();
      return;
    }

    // 3. Gravity Drop & Elastic Damping Bounce onto Dock/Taskbar Baseline
    let curY = rect.top;
    let curX = rect.left;
    let vy = 0;
    let vx = state.vx * 0.35;
    const gravity = 0.92;
    const bounceFactor = -0.36;
    let bounceCount = 0;

    function stepGravity() {
      vy += gravity;
      curY += vy;
      curX += vx;
      vx *= 0.94;

      const minX = 12;
      const maxX = window.innerWidth - container.offsetWidth - 12;
      curX = Math.max(minX, Math.min(curX, maxX));

      const targetFloor = window.innerHeight - container.offsetHeight - 18;
      if (curY >= targetFloor) {
        curY = targetFloor;
        if (Math.abs(vy) > 2.4 && bounceCount < 2) {
          vy *= bounceFactor;
          bounceCount += 1;
          playSoftBumpSound();
          if (bounceCount === 1) {
            setState('rebound', getRandomDialogue('release'), 520);
          }
        } else {
          container.style.left = `${curX}px`;
          container.style.top = `${targetFloor}px`;
          state.gravityRaf = null;
          playSound('release');
          syncHitRectsToDesktop();
          return;
        }
      }

      container.style.left = `${curX}px`;
      container.style.top = `${curY}px`;
      state.gravityRaf = requestAnimationFrame(stepGravity);
    }

    state.gravityRaf = requestAnimationFrame(stepGravity);
  }

  container.addEventListener('pointerdown', (e) => {
    if (e.button === 2 || state.dndPassthrough) return;
    if (e.target.closest('#speech-bubble')) return;

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

      // Swing angle proportional to horizontal drag velocity
      const swingDeg = Math.max(-14, Math.min(14, -instantVx * 0.75));
      document.documentElement.style.setProperty('--drag-swing', `${swingDeg.toFixed(1)}deg`);

      // Update facing direction toward movement
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

    const pad = 2;
    const maxW = window.innerWidth - container.offsetWidth - pad;
    const maxH = window.innerHeight - container.offsetHeight - pad;
    newX = Math.max(pad, Math.min(newX, maxW));
    newY = Math.max(pad, Math.min(newY, maxH));

    container.style.left = `${newX}px`;
    container.style.top = `${newY}px`;
    container.style.bottom = 'auto';
    container.style.right = 'auto';
  });

  window.addEventListener('pointerup', () => {
    if (!state.isDragging) return;
    state.isDragging = false;
    container.classList.remove('dragging');
    document.documentElement.style.setProperty('--drag-swing', '0deg');
    ipc.setDragging(false);

    if (!state.hasMoved) {
      playSound('clickRelease');
      setState('rebound', null, 520);
      showBubble(getRandomDialogue('idle'), 4500);
      syncHitRectsToDesktop();
    } else {
      startReleasePhysics();
    }
  });

  // Bubble text click to cycle tsundere lines
  bubbleText.addEventListener('click', (e) => {
    e.stopPropagation();
    initAudio();
    playSound('clickPress');
    showBubble(getRandomDialogue('idle'), 4500);
  });

  // =========================================================================
  // 6. Quota Updater & Mode Controls (DND Passthrough, IDE-Only, Skins, Shortcuts)
  // =========================================================================

  function updateQuota(fiveHourPct, weekPct, metaText = null) {
    if (typeof fiveHourPct === 'number') {
      const pct5 = Math.min(100, Math.max(0, fiveHourPct));
      quota5hBar.style.width = `${pct5}%`;
      quota5hVal.textContent = `${Math.round(pct5)}%`;
      quota5hBar.className = `quota-bar-fill ${pct5 <= 10 ? 'danger' : pct5 <= 30 ? 'warning' : ''}`;
      if (pct5 <= 25 && state.currentState === 'idle') {
        showBubble(getRandomDialogue('warning'), 5000);
      }
    }
    if (typeof weekPct === 'number') {
      const pctW = Math.min(100, Math.max(0, weekPct));
      quotaWeekBar.style.width = `${pctW}%`;
      quotaWeekVal.textContent = `${Math.round(pctW)}%`;
    }
    if (metaText && quotaMeta) {
      quotaMeta.textContent = metaText;
    }
  }

  function toggleDndPassthrough(forceVal = null) {
    state.dndPassthrough = typeof forceVal === 'boolean' ? forceVal : !state.dndPassthrough;
    if (dndStatus) dndStatus.textContent = state.dndPassthrough ? '开启' : '关闭';
    container.classList.toggle('dnd-ghost', state.dndPassthrough);
    dndWakePill.classList.toggle('hidden', !state.dndPassthrough);
    contextMenu.classList.add('hidden');

    if (state.dndPassthrough) {
      showBubble('已开启【完全穿透·免打扰】，鼠标可直接点击底层窗口~', 2800);
    } else {
      showBubble('本喵回来啦！随时听候差遣~', 3200);
    }
    setTimeout(syncHitRectsToDesktop, 50);
  }

  function toggleIdeOnlyMode(forceVal = null) {
    state.ideOnlyMode = typeof forceVal === 'boolean' ? forceVal : !state.ideOnlyMode;
    if (ideOnlyStatus) ideOnlyStatus.textContent = state.ideOnlyMode ? '开启' : '关闭';
    state.lastIdeActiveAt = Date.now();
    container.classList.remove('ide-hidden');
    showBubble(
      state.ideOnlyMode
        ? '已开启【仅 Antigravity 激活时显示】，长时间无事件将自动隐身~'
        : '已恢复【常驻桌面陪伴】模式！',
      3500
    );
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
      showBubble(`已切换至皮肤：【${nextSkin.name || nextSkin.id}】！`, 3500);
    } else {
      showBubble('当前已激活【大肥猫·星语伴侣】，在 skins/ 目录放入新皮肤即可一键切换！', 3800);
    }
  }

  dndWakePill.addEventListener('click', (e) => {
    e.stopPropagation();
    initAudio();
    playSound('clickRelease');
    toggleDndPassthrough(false);
  });

  // Keyboard Shortcut: Cmd/Ctrl + Shift + P to toggle DND passthrough / wake
  window.addEventListener('keydown', (e) => {
    if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key.toLowerCase() === 'p') {
      e.preventDefault();
      toggleDndPassthrough();
    }
  });

  // Context Menu
  container.addEventListener('contextmenu', (e) => {
    e.preventDefault();
    if (state.dndPassthrough) return;
    const menuW = 225;
    const menuH = 290;
    contextMenu.style.left = `${Math.max(8, Math.min(window.innerWidth - menuW, e.clientX))}px`;
    contextMenu.style.top = `${Math.max(8, Math.min(window.innerHeight - menuH, e.clientY))}px`;
    contextMenu.classList.remove('hidden');
    ipc.setIgnoreCursorEvents(false);
    syncHitRectsToDesktop();
  });

  document.addEventListener('click', (e) => {
    if (!contextMenu.classList.contains('hidden') && !contextMenu.contains(e.target)) {
      contextMenu.classList.add('hidden');
      syncHitRectsToDesktop();
    }
  });

  contextMenu.addEventListener('click', (e) => {
    const item = e.target.closest('.menu-item');
    if (!item) return;
    const action = item.dataset.action;

    switch (action) {
      case 'toggle-quota':
        quotaSection.style.display = quotaSection.style.display === 'none' ? 'flex' : 'none';
        speechBubble.classList.remove('hidden');
        break;
      case 'random-dialogue':
        playSound('clickPress');
        showBubble(getRandomDialogue('idle'), 4500);
        break;
      case 'pet-action':
        triggerPettingAction();
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
        showBubble(
          state.gravityEnabled ? '已开启重力下落与底栏吸附~' : '已开启自由空中悬停模式~',
          3200
        );
        break;
      case 'toggle-sound':
        state.soundEnabled = !state.soundEnabled;
        soundStatus.textContent = state.soundEnabled ? '开启' : '静音';
        break;
      case 'switch-skin':
        cycleSkin();
        break;
      case 'close-menu':
        break;
    }
    contextMenu.classList.add('hidden');
    syncHitRectsToDesktop();
  });

  // =========================================================================
  // 7. Connect to Antigravity IDE Telemetry & Control Bus
  // =========================================================================

  const bridge = new window.PetBridge({
    onEvent: (event) => {
      state.lastIdeActiveAt = Date.now();
      container.classList.remove('ide-hidden');
      if (event.state) {
        const linePoolKey =
          event.state === 'thinking'
            ? 'thinking'
            : event.state === 'success'
            ? 'success'
            : event.state === 'error'
            ? 'error'
            : null;
        const msg =
          event.bubbleText || (linePoolKey ? getRandomDialogue(linePoolKey) : null);
        setState(event.state, msg, event.durationMs, event.tool || '');
      }
      if (event.quota) {
        updateQuota(event.quota.fiveHour, event.quota.weekly, event.quota.meta);
      }
    },
    onQuota: (quota) => {
      updateQuota(quota.fiveHour, quota.weekly, quota.meta);
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

  // Periodic Idle Dialogue & IDE-Only Visibility Check
  setInterval(() => {
    if (state.ideOnlyMode && Date.now() - state.lastIdeActiveAt > 120000) {
      container.classList.add('ide-hidden');
      syncHitRectsToDesktop();
      return;
    }
    if (state.currentState === 'idle' && !state.isDragging && !state.dndPassthrough) {
      showBubble(getRandomDialogue('idle'), 5000);
    }
  }, 45000);

  window.addEventListener('resize', () => {
    syncHitRectsToDesktop();
  });

  // Initialize
  fetchAvailableSkins();
  loadSkin('fat-cat');
})();
