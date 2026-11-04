#!/usr/bin/env node
/**
 * Anti-Pet Local Hub, Real Antigravity Quota Sync, Live Transcript & Settings Store
 * Zero-dependency native Node.js HTTP & SSE server
 */

const http = require('http');
const https = require('https');
const fs = require('fs');
const path = require('path');
const { exec } = require('child_process');
const { fetchAntigravityQuota } = require('./quota-probe');
const { TranscriptWatcher } = require('./transcript-watcher');

const PORT = parseInt(process.env.ANTI_PET_PORT || '4199', 10);
const ROOT_DIR = path.resolve(__dirname, '..');
const STATE_FILE = path.join(ROOT_DIR, '.antigravity-pet-state.json');
const LEDGER_FILE = path.join(ROOT_DIR, '.antigravity-pet-ledger.json');
const SETTINGS_FILE = path.join(ROOT_DIR, '.antigravity-pet-settings.json');

const MIME_TYPES = {
  '.html': 'text/html; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.js': 'application/javascript; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.png': 'image/png',
  '.jpg': 'image/jpeg',
  '.jpeg': 'image/jpeg',
  '.svg': 'image/svg+xml',
  '.wav': 'audio/wav',
  '.mp3': 'audio/mpeg',
  '.webp': 'image/webp',
  '.ico': 'image/x-icon'
};

const DEFAULT_SETTINGS = {
  theme: 'system', // 'system', 'light', 'dark'
  scale: 1.0,
  skinId: 'fat-cat',
  soundEnabled: true,
  soundVolume: 0.8,
  gravityEnabled: true,
  floorMode: 'bottom', // 'bottom', 'taskbar', 'free'
  ideOnlyMode: false,
  dndPassthrough: false,
  bubbleDurationMs: 4000,
  showQuota: true,
  posX: null,
  posY: null,
  resetSeq: 0,
  facing: 'left',
  menuConfig: {
    openAntigravity: true,
    settings: true,
    quota: true,
    randomDialogue: true,
    petAction: true,
    zoom: true,
    dnd: true,
    ideOnly: true,
    gravity: true,
    switchSkin: true,
    sound: true
  },
  antigravityPath: null
};

function loadSettings() {
  try {
    if (fs.existsSync(SETTINGS_FILE)) {
      return { ...DEFAULT_SETTINGS, ...JSON.parse(fs.readFileSync(SETTINGS_FILE, 'utf-8')) };
    }
  } catch (e) {}
  return { ...DEFAULT_SETTINGS };
}

let currentSettings = loadSettings();

function saveSettings(newSettings) {
  currentSettings = { ...currentSettings, ...newSettings };
  const tmpFile = `${SETTINGS_FILE}.${process.pid}.${Date.now()}.tmp`;
  try {
    fs.writeFileSync(tmpFile, JSON.stringify(currentSettings, null, 2), 'utf-8');
    fs.renameSync(tmpFile, SETTINGS_FILE);
  } catch (e) {
    try {
      if (fs.existsSync(tmpFile)) {
        try { fs.copyFileSync(tmpFile, SETTINGS_FILE); } catch (_) {}
        fs.unlinkSync(tmpFile);
      } else {
        fs.writeFileSync(SETTINGS_FILE, JSON.stringify(currentSettings, null, 2), 'utf-8');
      }
    } catch (_) {}
  }
  return currentSettings;
}

const sseClients = new Set();

const MAX_LEDGER_ENTRIES = 500;
const ONE_WEEK_MS = 7 * 24 * 60 * 60 * 1000;

function recordLocalLedger(evt) {
  const now = Date.now();
  let ledger = { entries: [] };
  if (fs.existsSync(LEDGER_FILE)) {
    try {
      ledger = JSON.parse(fs.readFileSync(LEDGER_FILE, 'utf-8'));
      if (!Array.isArray(ledger.entries)) ledger.entries = [];
    } catch (e) {
      ledger = { entries: [] };
    }
  }
  ledger.entries = ledger.entries.filter((item) => now - item.ts < ONE_WEEK_MS);
  const weight =
    evt === 'PreInvocation' ? 1.2 : evt === 'PreToolUse' ? 0.6 : evt === 'PostToolUse' ? 0.4 : 0.1;
  if (weight > 0) {
    ledger.entries.push({ ts: now, event: evt, weight });
  }
  if (ledger.entries.length > MAX_LEDGER_ENTRIES) {
    ledger.entries = ledger.entries.slice(-MAX_LEDGER_ENTRIES);
  }

  const tmpFile = `${LEDGER_FILE}.${process.pid}.${Date.now()}.tmp`;
  try {
    fs.writeFileSync(tmpFile, JSON.stringify(ledger, null, 2), 'utf-8');
    fs.renameSync(tmpFile, LEDGER_FILE);
  } catch (e) {
    try {
      if (fs.existsSync(tmpFile)) {
        try { fs.copyFileSync(tmpFile, LEDGER_FILE); } catch (_) {}
        fs.unlinkSync(tmpFile);
      }
    } catch (_) {}
  }
}

function readFallbackLedgerQuota() {
  const now = Date.now();
  const FIVE_HOURS_MS = 5 * 60 * 60 * 1000;
  const FIVE_HOUR_BUDGET = 160;
  const WEEKLY_BUDGET = 1200;

  if (!fs.existsSync(LEDGER_FILE)) {
    return {
      fiveHour: 100,
      weekly: 100,
      meta: '等待连接 Antigravity 语言服务...'
    };
  }
  try {
    const ledger = JSON.parse(fs.readFileSync(LEDGER_FILE, 'utf-8'));
    const entries = Array.isArray(ledger.entries)
      ? ledger.entries.filter((item) => now - item.ts < ONE_WEEK_MS)
      : [];
    const entries5h = entries.filter((item) => now - item.ts <= FIVE_HOURS_MS);
    const used5h = entries5h.reduce((acc, item) => acc + (item.weight || 0.5), 0);
    const usedWeek = entries.reduce((acc, item) => acc + (item.weight || 0.5), 0);

    const fiveHour = Math.max(5, Math.min(100, +(((FIVE_HOUR_BUDGET - used5h) / FIVE_HOUR_BUDGET) * 100).toFixed(1)));
    const weekly = Math.max(5, Math.min(100, +(((WEEKLY_BUDGET - usedWeek) / WEEKLY_BUDGET) * 100).toFixed(1)));
    return {
      fiveHour,
      weekly,
      meta: `本地估算 (近5h ${entries5h.length} 次)`
    };
  } catch (e) {
    return { fiveHour: 100, weekly: 100, meta: '等待同步 Antigravity 额度' };
  }
}

let latestQuota = readFallbackLedgerQuota();

async function refreshLiveQuota(force = false) {
  try {
    const live = await fetchAntigravityQuota(force);
    if (live && typeof live.fiveHour === 'number' && typeof live.weekly === 'number') {
      const changed =
        live.fiveHour !== latestQuota.fiveHour ||
        live.weekly !== latestQuota.weekly ||
        live.meta !== latestQuota.meta;
      latestQuota = live;
      if (changed) {
        broadcastSSE('quota', latestQuota);
      }
      return latestQuota;
    }
  } catch (e) {}
  return latestQuota;
}

// Poll live Antigravity language_server quota every 15 seconds
setInterval(() => {
  refreshLiveQuota(true);
}, 15000);
refreshLiveQuota(true);

let currentState = {
  timestamp: Date.now(),
  event: 'init',
  state: 'idle',
  bubbleText: '大肥猫已就位！随时为你护航代码~',
  emotion: 'idle',
  durationMs: 0,
  quota: latestQuota
};

if (fs.existsSync(STATE_FILE)) {
  try {
    currentState = { ...currentState, ...JSON.parse(fs.readFileSync(STATE_FILE, 'utf-8')) };
  } catch (e) {}
}

fs.watchFile(STATE_FILE, { interval: 250 }, async () => {
  try {
    if (fs.existsSync(STATE_FILE)) {
      const data = JSON.parse(fs.readFileSync(STATE_FILE, 'utf-8'));
      const q = await refreshLiveQuota(false);
      data.quota = q;
      currentState = data;
      broadcastSSE('state', data);
      broadcastSSE('quota', q);
    }
  } catch (e) {}
});

function broadcastSSE(eventType, data) {
  const payload = `event: ${eventType}\ndata: ${JSON.stringify(data)}\n\n`;
  for (const client of sseClients) {
    try {
      client.write(payload);
    } catch (e) {
      sseClients.delete(client);
    }
  }
}

// Initialize Real-time Antigravity Brain Transcript Watcher
const transcriptWatcher = new TranscriptWatcher({
  onState: (stateUpdate) => {
    currentState = {
      ...currentState,
      ...stateUpdate,
      quota: latestQuota,
      timestamp: Date.now()
    };
    broadcastSSE('state', currentState);
    try {
      fs.writeFileSync(STATE_FILE, JSON.stringify(currentState, null, 2), 'utf-8');
    } catch (e) {}
  },
  onActivity: (act) => {
    broadcastSSE('activity', { active: true, ...act, timestamp: Date.now() });
  }
});
transcriptWatcher.start();

function getAutostartStatus() {
  return new Promise((resolve) => {
    if (process.platform === 'darwin') {
      exec('osascript -e \'tell application "System Events" to get the name of every login item\'', (err, stdout) => {
        if (err) return resolve({ enabled: false, platform: 'darwin' });
        const items = (stdout || '').split(',').map((s) => s.trim().toLowerCase());
        const hasItem = items.includes('anti-pet') || items.includes('anti-pet.app') || items.includes('anti-pet-desktop');
        resolve({ enabled: hasItem, platform: 'darwin' });
      });
    } else if (process.platform === 'win32') {
      exec('reg query "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run" /v "AntiPet"', (err, stdout) => {
        resolve({ enabled: !err && (stdout || '').includes('AntiPet'), platform: 'win32' });
      });
    } else {
      resolve({ enabled: false, platform: process.platform });
    }
  });
}

function setAutostartStatus(enable) {
  return new Promise((resolve) => {
    if (process.platform === 'darwin') {
      if (enable) {
        let appPath = path.join(ROOT_DIR, 'dist', 'Anti-Pet.app');
        if (!fs.existsSync(appPath)) {
          appPath = path.join(ROOT_DIR, 'desktop', 'target', 'release', 'anti-pet-desktop');
        }
        if (!fs.existsSync(appPath)) {
          appPath = path.join(ROOT_DIR, 'anti-pet-desktop');
        }
        const script = `tell application "System Events" to make login item at end with properties {path:"${appPath}", hidden:false, name:"Anti-Pet"}`;
        exec(`osascript -e '${script}'`, (err) => {
          if (err) return resolve({ ok: false, error: err.message });
          resolve({ ok: true, enabled: true });
        });
      } else {
        const script = `tell application "System Events" to delete (every login item whose name is "Anti-Pet" or name is "Anti-Pet.app")`;
        exec(`osascript -e '${script}'`, (err) => {
          if (err) return resolve({ ok: false, error: err.message });
          resolve({ ok: true, enabled: false });
        });
      }
    } else if (process.platform === 'win32') {
      if (enable) {
        let exePath = path.join(ROOT_DIR, 'anti-pet-desktop.exe');
        if (!fs.existsSync(exePath)) {
          exePath = path.join(ROOT_DIR, 'desktop', 'target', 'release', 'anti-pet-desktop.exe');
        }
        const cmd = `reg add "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run" /v "AntiPet" /t REG_SZ /d "\\"${exePath}\\"" /f`;
        exec(cmd, (err) => {
          if (err) return resolve({ ok: false, error: err.message });
          resolve({ ok: true, enabled: true });
        });
      } else {
        const cmd = `reg delete "HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Run" /v "AntiPet" /f`;
        exec(cmd, () => {
          resolve({ ok: true, enabled: false });
        });
      }
    } else {
      resolve({ ok: true, enabled: enable });
    }
  });
}

function checkGitHubUpdates() {
  return new Promise((resolve) => {
    let currentVersion = 'v0.5.1';
    try {
      const pkg = JSON.parse(fs.readFileSync(path.join(ROOT_DIR, 'package.json'), 'utf-8'));
      if (pkg.version) currentVersion = 'v' + pkg.version.replace(/^v/, '');
    } catch (e) {}

    const req = https.get(
      {
        hostname: 'api.github.com',
        path: '/repos/TianYaYou/Anti-pet-indesktop/releases/latest',
        headers: { 'User-Agent': 'Anti-Pet-Desktop' },
        timeout: 6000
      },
      (res) => {
        let raw = '';
        res.on('data', (c) => (raw += c));
        res.on('end', () => {
          try {
            const data = JSON.parse(raw);
            const latestTag = data.tag_name || currentVersion;
            const hasUpdate = latestTag !== currentVersion;
            resolve({
              currentVersion,
              latestVersion: latestTag,
              hasUpdate,
              releaseName: data.name || latestTag,
              htmlUrl: data.html_url || 'https://github.com/TianYaYou/Anti-pet-indesktop/releases',
              publishedAt: data.published_at || new Date().toISOString(),
              body: data.body || ''
            });
          } catch (e) {
            resolve({ currentVersion, latestVersion: currentVersion, hasUpdate: false, error: e.message });
          }
        });
      }
    );
    req.on('error', (e) => {
      resolve({ currentVersion, latestVersion: currentVersion, hasUpdate: false, error: e.message });
    });
    req.on('timeout', () => {
      req.destroy();
      resolve({ currentVersion, latestVersion: currentVersion, hasUpdate: false, error: 'Request timeout' });
    });
  });
}

const server = http.createServer(async (req, res) => {
  const parsedUrl = new URL(req.url, `http://${req.headers.host || '127.0.0.1'}`);
  let pathname = decodeURIComponent(parsedUrl.pathname);

  res.setHeader('Access-Control-Allow-Origin', '*');
  res.setHeader('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
  res.setHeader('Access-Control-Allow-Headers', 'Content-Type');

  if (req.method === 'OPTIONS') {
    res.writeHead(204);
    return res.end();
  }

  // 1. SSE Stream: /api/events
  if (pathname === '/api/events') {
    res.writeHead(200, {
      'Content-Type': 'text/event-stream',
      'Cache-Control': 'no-cache',
      'Connection': 'keep-alive'
    });

    sseClients.add(res);
    req.on('close', () => sseClients.delete(res));

    res.write(`event: state\ndata: ${JSON.stringify(currentState)}\n\n`);
    res.write(`event: settings\ndata: ${JSON.stringify(currentSettings)}\n\n`);
    const quota = await refreshLiveQuota(true);
    res.write(`event: quota\ndata: ${JSON.stringify(quota)}\n\n`);
    return;
  }

  // 2. Settings API: /api/settings (GET & POST)
  if (pathname === '/api/settings') {
    if (req.method === 'GET') {
      res.writeHead(200, { 'Content-Type': 'application/json' });
      return res.end(JSON.stringify(currentSettings));
    }
    if (req.method === 'POST') {
      let body = '';
      req.on('data', (chunk) => (body += chunk));
      req.on('end', () => {
        try {
          const updates = JSON.parse(body);
          if (updates.resetPosition) {
            const seq = Date.now();
            updates.resetSeq = seq;
            updates.posX = null;
            updates.posY = null;
            delete updates.resetPosition;
            broadcastSSE('control', { action: 'reset_position', seq });
          }
          const saved = saveSettings(updates);
          broadcastSSE('settings', saved);
          res.writeHead(200, { 'Content-Type': 'application/json' });
          res.end(JSON.stringify({ ok: true, settings: saved }));
        } catch (e) {
          res.writeHead(400, { 'Content-Type': 'application/json' });
          res.end(JSON.stringify({ error: 'Invalid JSON' }));
        }
      });
      return;
    }
  }

  // 3. IDE Status API: /api/ide-status
  if (pathname === '/api/ide-status') {
    const lastActive = transcriptWatcher.getLastActiveTime();
    const isActive = Date.now() - lastActive < 120000;
    res.writeHead(200, { 'Content-Type': 'application/json' });
    return res.end(JSON.stringify({
      running: true,
      active: isActive,
      lastActiveTime: lastActive
    }));
  }

  // 4. Post Event: /api/event
  if (pathname === '/api/event' && req.method === 'POST') {
    let body = '';
    req.on('data', (chunk) => (body += chunk));
    req.on('end', async () => {
      try {
        const payload = JSON.parse(body);
        if (payload.event) {
          recordLocalLedger(payload.event);
        }
        const liveQ = await refreshLiveQuota(true);
        payload.quota = liveQ;
        currentState = payload;
        broadcastSSE('state', payload);
        broadcastSSE('quota', liveQ);
        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ ok: true, quota: liveQ }));
      } catch (e) {
        res.writeHead(400, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ error: 'Invalid JSON' }));
      }
    });
    return;
  }

  // 5. Post Control Command: /api/control
  if (pathname === '/api/control' && req.method === 'POST') {
    let body = '';
    req.on('data', (chunk) => (body += chunk));
    req.on('end', () => {
      try {
        const ctrl = JSON.parse(body);
        broadcastSSE('control', ctrl);
        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ ok: true }));
      } catch (e) {
        res.writeHead(400, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ error: 'Invalid JSON' }));
      }
    });
    return;
  }

  // 6. Get State & Quota: /api/state & /api/quota
  if (pathname === '/api/state') {
    const quota = await refreshLiveQuota(false);
    res.writeHead(200, { 'Content-Type': 'application/json' });
    return res.end(JSON.stringify({ ...currentState, quota }));
  }

  if (pathname === '/api/quota') {
    const quota = await refreshLiveQuota(true);
    res.writeHead(200, { 'Content-Type': 'application/json' });
    return res.end(JSON.stringify(quota));
  }

  
  // 7. Autostart API: /api/autostart (GET & POST)
  if (pathname === '/api/autostart') {
    if (req.method === 'GET') {
      const status = await getAutostartStatus();
      res.writeHead(200, { 'Content-Type': 'application/json' });
      return res.end(JSON.stringify(status));
    }
    if (req.method === 'POST') {
      let body = '';
      req.on('data', (chunk) => (body += chunk));
      req.on('end', async () => {
        try {
          const payload = JSON.parse(body);
          const result = await setAutostartStatus(!!payload.enabled);
          res.writeHead(200, { 'Content-Type': 'application/json' });
          res.end(JSON.stringify(result));
        } catch (e) {
          res.writeHead(400, { 'Content-Type': 'application/json' });
          res.end(JSON.stringify({ error: e.message }));
        }
      });
      return;
    }
  }

  // 8. Update Check API: /api/update-check
  if (pathname === '/api/update-check') {
    const updateInfo = await checkGitHubUpdates();
    res.writeHead(200, { 'Content-Type': 'application/json' });
    return res.end(JSON.stringify(updateInfo));
  }

  // 8.1 Antigravity Status API: /api/antigravity/status
  if (pathname === '/api/antigravity/status') {
    if (process.platform === 'darwin' || process.platform === 'linux') {
      exec('pgrep -i antigravity', (err, stdout) => {
        const isRunning = !err && Boolean(stdout && stdout.trim().length > 0);
        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ running: isRunning }));
      });
    } else if (process.platform === 'win32') {
      exec('tasklist /FI "IMAGENAME eq Antigravity.exe"', (err, stdout) => {
        const isRunning = !err && Boolean(stdout && stdout.toLowerCase().includes('antigravity.exe'));
        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ running: isRunning }));
      });
    } else {
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ running: false }));
    }
    return;
  }

  // 8.2 Open Antigravity API: /api/antigravity/open
  if (pathname === '/api/antigravity/open' && req.method === 'POST') {
    const customPath = currentSettings.antigravityPath;
    let cmd = '';
    if (process.platform === 'darwin') {
      if (customPath && fs.existsSync(customPath)) {
        cmd = `open "${customPath}"`;
      } else if (fs.existsSync('/Applications/Antigravity.app')) {
        cmd = 'open -a /Applications/Antigravity.app';
      } else {
        cmd = 'open -a Antigravity';
      }
    } else if (process.platform === 'win32') {
      if (customPath && fs.existsSync(customPath)) {
        cmd = `start "" "${customPath}"`;
      } else {
        cmd = 'start antigravity:';
      }
    } else {
      cmd = customPath || 'antigravity';
    }

    exec(cmd, (err) => {
      res.writeHead(200, { 'Content-Type': 'application/json' });
      res.end(JSON.stringify({ ok: !err, error: err ? err.message : null }));
    });
    return;
  }

  // 9. Shutdown: /api/shutdown
  if (pathname === '/api/shutdown') {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ ok: true, message: 'Server shutting down' }));
    setTimeout(() => process.exit(0), 100);
    return;
  }

  // 10. List Skins: /api/skins
  if (pathname === '/api/skins' && req.method === 'GET') {
    const skinsDir = path.join(ROOT_DIR, 'skins');
    try {
      const skins = fs
        .readdirSync(skinsDir, { withFileTypes: true })
        .filter((d) => d.isDirectory())
        .map((d) => {
          const skinJsonPath = path.join(skinsDir, d.name, 'skin.json');
          if (fs.existsSync(skinJsonPath)) {
            const data = JSON.parse(fs.readFileSync(skinJsonPath, 'utf-8'));
            return {
              ...data,
              id: data.id || d.name,
              previewUrl: `/skins/${d.name}/frames/idle.png`,
              isCurrent: d.name === currentSettings.skinId
            };
          }
          return {
            id: d.name,
            name: d.name,
            previewUrl: `/skins/${d.name}/frames/idle.png`,
            isCurrent: d.name === currentSettings.skinId
          };
        });
      res.writeHead(200, { 'Content-Type': 'application/json' });
      return res.end(JSON.stringify(skins));
    } catch (e) {
      res.writeHead(500, { 'Content-Type': 'application/json' });
      return res.end(JSON.stringify({ error: e.message }));
    }
  }

  // 11. Create Skin: /api/skins/create
  if (pathname === '/api/skins/create' && req.method === 'POST') {
    let body = '';
    req.on('data', (c) => (body += c));
    req.on('end', () => {
      try {
        const payload = JSON.parse(body);
        const skinId = (payload.skinId || payload.id || '').trim().toLowerCase().replace(/[^a-z0-9_-]/g, '');
        if (!skinId) {
          res.writeHead(400, { 'Content-Type': 'application/json' });
          return res.end(JSON.stringify({ error: '无效的皮肤ID (仅支持英文小写与数字/下划线)' }));
        }

        const skinDir = path.join(ROOT_DIR, 'skins', skinId);
        if (fs.existsSync(skinDir)) {
          res.writeHead(400, { 'Content-Type': 'application/json' });
          return res.end(JSON.stringify({ error: `皮肤 ID "${skinId}" 已存在` }));
        }

        fs.mkdirSync(skinDir, { recursive: true });
        fs.mkdirSync(path.join(skinDir, 'frames'), { recursive: true });
        fs.mkdirSync(path.join(skinDir, 'audio'), { recursive: true });
        fs.mkdirSync(path.join(skinDir, 'ui'), { recursive: true });

        // Copy default frame
        const defaultFrame = path.join(ROOT_DIR, 'skins', 'fat-cat', 'frames', 'idle.png');
        if (fs.existsSync(defaultFrame)) {
          fs.copyFileSync(defaultFrame, path.join(skinDir, 'frames', 'idle.png'));
        }

        // Copy default audio
        const audioSrc = path.join(ROOT_DIR, 'skins', 'fat-cat', 'audio');
        if (fs.existsSync(audioSrc)) {
          for (const f of fs.readdirSync(audioSrc)) {
            fs.copyFileSync(path.join(audioSrc, f), path.join(skinDir, 'audio', f));
          }
        }

        // Copy default ui icons
        const uiSrc = path.join(ROOT_DIR, 'skins', 'fat-cat', 'ui');
        if (fs.existsSync(uiSrc)) {
          for (const f of fs.readdirSync(uiSrc)) {
            fs.copyFileSync(path.join(uiSrc, f), path.join(skinDir, 'ui', f));
          }
        }

        // Create skin.json
        const newSkinMeta = {
          schemaVersion: 1,
          id: skinId,
          name: payload.name || skinId,
          version: '1.0.0',
          author: payload.author || 'Custom Creator',
          description: payload.description || '自定义桌宠皮肤',
          render: {
            defaultWidth: parseInt(payload.defaultWidth || '136', 10),
            minWidth: 80,
            maxWidth: 260,
            motionPaddingPx: 0,
            facingTransitionMs: 220
          },
          frames: {
            idle: 'frames/idle.png'
          },
          states: {
            idle: { frame: 'idle', motion: 'breath', loop: true },
            thinking: { frame: 'idle', motion: 'ponder', loop: true },
            working: { frame: 'idle', motion: 'working', loop: true },
            success: { frame: 'idle', motion: 'duang', durationMs: 3000, loop: false },
            error: { frame: 'idle', motion: 'worry', durationMs: 3500, loop: false },
            pressed: { frame: 'idle', motion: 'compress', loop: false },
            rebound: { frame: 'idle', motion: 'duang', durationMs: 500, loop: false }
          }
        };
        fs.writeFileSync(path.join(skinDir, 'skin.json'), JSON.stringify(newSkinMeta, null, 2), 'utf-8');

        // Copy dialogue.json template
        const defaultDialogue = path.join(ROOT_DIR, 'skins', 'fat-cat', 'dialogue.json');
        if (fs.existsSync(defaultDialogue)) {
          fs.copyFileSync(defaultDialogue, path.join(skinDir, 'dialogue.json'));
        }

        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ ok: true, skin: newSkinMeta }));
      } catch (e) {
        res.writeHead(500, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ error: e.message }));
      }
    });
    return;
  }

  // 11.1 Import Skin (Compatible with dsh-pet / Codex pet.json and Anti-Pet skin.json)
  if (pathname === '/api/skins/import' && req.method === 'POST') {
    let body = '';
    req.on('data', (c) => (body += c));
    req.on('end', () => {
      try {
        const payload = JSON.parse(body);
        const rawConfig = payload.config || payload;
        const isDshPet = Boolean(rawConfig.displayName || rawConfig.spritesheetPath || rawConfig.frame);

        // Adapt config if needed
        const id = (payload.skinId || rawConfig.id || rawConfig.petId || rawConfig.name || 'custom-pet')
          .trim()
          .toLowerCase()
          .replace(/[^a-z0-9_-]/g, '-');
        if (!id) {
          res.writeHead(400, { 'Content-Type': 'application/json' });
          return res.end(JSON.stringify({ error: '无效的宠物 ID' }));
        }

        const name = rawConfig.displayName || rawConfig.name || rawConfig.title || id;
        const author = rawConfig.author || rawConfig.creator || (isDshPet ? 'dsh-pet 社区' : '自定义作者');
        const description = rawConfig.description || (isDshPet ? '兼容 dsh-pet 格式导入宠物' : '导入的桌面伴侣皮肤');

        let defaultWidth = 136;
        if (rawConfig.frame && rawConfig.frame.width) {
          defaultWidth = Math.min(240, Math.max(80, Math.round(rawConfig.frame.width)));
        } else if (rawConfig.render && rawConfig.render.defaultWidth) {
          defaultWidth = rawConfig.render.defaultWidth;
        }

        let idleFrame = 'frames/idle.png';
        if (rawConfig.frames && typeof rawConfig.frames === 'object' && rawConfig.frames.idle) {
          idleFrame = rawConfig.frames.idle;
        } else if (rawConfig.spritesheetPath) {
          idleFrame = 'frames/idle.png'; // Will be mapped to idle.png
        }

        const states = rawConfig.states || {
          idle: { frame: 'idle', motion: 'breath', loop: true },
          thinking: { frame: 'idle', motion: 'ponder', bubbleText: '正在思考中...', loop: true },
          working: { frame: 'idle', motion: 'working', bubbleText: '正在努力写代码！', loop: true },
          success: { frame: 'idle', motion: 'duang', bubbleText: '任务完成！', durationMs: 3000, loop: false },
          error: { frame: 'idle', motion: 'worry', bubbleText: '遇到阻碍，排查中...', durationMs: 3500, loop: false },
          petting: { frame: 'idle', motion: 'nuzzle', bubbleText: '摸摸头真舒服~', durationMs: 2400, loop: false },
          dragging: { frame: 'idle', motion: 'held', loop: false },
          pressed: { frame: 'idle', motion: 'compress', loop: false },
          rebound: { frame: 'idle', motion: 'duang', durationMs: 500, loop: false }
        };

        const adaptedMeta = {
          schemaVersion: 1,
          id,
          name,
          version: rawConfig.version || '1.0.0',
          author,
          description,
          render: {
            defaultWidth,
            minWidth: 80,
            maxWidth: 260,
            motionPaddingPx: 0,
            facingTransitionMs: 220
          },
          frames: {
            idle: idleFrame
          },
          states,
          audio: {
            press: 'audio/pop-press.wav',
            release: 'audio/pop-release.wav',
            clickPress: 'audio/click-press.wav',
            clickRelease: 'audio/click-release.wav'
          },
          dialogue: 'dialogue.json'
        };

        const skinDir = path.join(ROOT_DIR, 'skins', id);
        fs.mkdirSync(skinDir, { recursive: true });
        fs.mkdirSync(path.join(skinDir, 'frames'), { recursive: true });
        fs.mkdirSync(path.join(skinDir, 'audio'), { recursive: true });
        fs.mkdirSync(path.join(skinDir, 'ui'), { recursive: true });

        // Save frame image if provided as base64, otherwise copy default idle frame
        if (payload.frameBase64) {
          const base64Data = payload.frameBase64.replace(/^data:image\/\w+;base64,/, '');
          fs.writeFileSync(path.join(skinDir, 'frames', 'idle.png'), Buffer.from(base64Data, 'base64'));
        } else {
          const defaultFrame = path.join(ROOT_DIR, 'skins', 'fat-cat', 'frames', 'idle.png');
          if (fs.existsSync(defaultFrame)) {
            fs.copyFileSync(defaultFrame, path.join(skinDir, 'frames', 'idle.png'));
          }
        }

        // Copy audio and ui
        const audioSrc = path.join(ROOT_DIR, 'skins', 'fat-cat', 'audio');
        if (fs.existsSync(audioSrc)) {
          for (const f of fs.readdirSync(audioSrc)) {
            fs.copyFileSync(path.join(audioSrc, f), path.join(skinDir, 'audio', f));
          }
        }
        const uiSrc = path.join(ROOT_DIR, 'skins', 'fat-cat', 'ui');
        if (fs.existsSync(uiSrc)) {
          for (const f of fs.readdirSync(uiSrc)) {
            fs.copyFileSync(path.join(uiSrc, f), path.join(skinDir, 'ui', f));
          }
        }

        // Save skin.json
        fs.writeFileSync(path.join(skinDir, 'skin.json'), JSON.stringify(adaptedMeta, null, 2), 'utf-8');

        // Copy or write dialogue.json
        const defaultDialogue = path.join(ROOT_DIR, 'skins', 'fat-cat', 'dialogue.json');
        if (fs.existsSync(defaultDialogue)) {
          fs.copyFileSync(defaultDialogue, path.join(skinDir, 'dialogue.json'));
        }

        broadcastSSE('skins-updated', { skinId: id });

        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ ok: true, skin: adaptedMeta, isDshPet }));
      } catch (e) {
        res.writeHead(500, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ error: e.message }));
      }
    });
    return;
  }

  // 12. Save Skin Metadata: /api/skins/save
  if (pathname === '/api/skins/save' && req.method === 'POST') {
    let body = '';
    req.on('data', (c) => (body += c));
    req.on('end', () => {
      try {
        const payload = JSON.parse(body);
        const skinId = payload.id;
        const skinJsonPath = path.join(ROOT_DIR, 'skins', skinId, 'skin.json');
        if (!fs.existsSync(skinJsonPath)) {
          res.writeHead(404, { 'Content-Type': 'application/json' });
          return res.end(JSON.stringify({ error: '皮肤不存在' }));
        }

        const existing = JSON.parse(fs.readFileSync(skinJsonPath, 'utf-8'));
        const updated = {
          ...existing,
          name: payload.name || existing.name,
          author: payload.author || existing.author,
          description: payload.description || existing.description,
          render: { ...existing.render, ...(payload.render || {}) }
        };
        fs.writeFileSync(skinJsonPath, JSON.stringify(updated, null, 2), 'utf-8');

        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ ok: true, skin: updated }));
      } catch (e) {
        res.writeHead(500, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ error: e.message }));
      }
    });
    return;
  }

  // 13. Dialogue API: /api/dialogue (GET & POST)
  if (pathname === '/api/dialogue') {
    const skinId = parsedUrl.searchParams.get('skinId') || currentSettings.skinId || 'fat-cat';
    const dialoguePath = path.join(ROOT_DIR, 'skins', skinId, 'dialogue.json');

    if (req.method === 'GET') {
      try {
        if (!fs.existsSync(dialoguePath)) {
          return res.writeHead(404, { 'Content-Type': 'application/json' }).end(JSON.stringify({ error: 'Dialogue not found' }));
        }
        const data = JSON.parse(fs.readFileSync(dialoguePath, 'utf-8'));
        res.writeHead(200, { 'Content-Type': 'application/json' });
        return res.end(JSON.stringify(data));
      } catch (e) {
        res.writeHead(500, { 'Content-Type': 'application/json' });
        return res.end(JSON.stringify({ error: e.message }));
      }
    }

    if (req.method === 'POST') {
      let body = '';
      req.on('data', (c) => (body += c));
      req.on('end', () => {
        try {
          const payload = JSON.parse(body);
          const targetSkin = payload.skinId || skinId;
          const targetPath = path.join(ROOT_DIR, 'skins', targetSkin, 'dialogue.json');

          let existing = {};
          if (fs.existsSync(targetPath)) {
            existing = JSON.parse(fs.readFileSync(targetPath, 'utf-8'));
          }

          const updated = {
            ...existing,
            lines: payload.lines || existing.lines || {},
            policy: { ...existing.policy, ...(payload.policy || {}) }
          };

          fs.writeFileSync(targetPath, JSON.stringify(updated, null, 2), 'utf-8');
          broadcastSSE('dialogue-updated', { skinId: targetSkin });

          res.writeHead(200, { 'Content-Type': 'application/json' });
          res.end(JSON.stringify({ ok: true, dialogue: updated }));
        } catch (e) {
          res.writeHead(400, { 'Content-Type': 'application/json' });
          res.end(JSON.stringify({ error: e.message }));
        }
      });
      return;
    }
  }

  // 8. Static File Serving (Support both root `/pet.css` and `/core/pet.css`)
  if (pathname === '/' || pathname === '/index.html') {
    pathname = '/core/index.html';
  } else if (
    pathname === '/pet.css' ||
    pathname === '/pet.js' ||
    pathname === '/bridge.js' ||
    pathname === '/settings.html' ||
    pathname === '/settings.css' ||
    pathname === '/settings.js'
  ) {
    pathname = `/core${pathname}`;
  }

  let filePath = path.join(ROOT_DIR, pathname);
  if (!filePath.startsWith(ROOT_DIR)) {
    res.writeHead(403);
    return res.end('Access denied');
  }

  if (!fs.existsSync(filePath)) {
    const coreFallback = path.join(ROOT_DIR, 'core', pathname);
    if (coreFallback.startsWith(ROOT_DIR) && fs.existsSync(coreFallback)) {
      filePath = coreFallback;
    }
  }

  fs.stat(filePath, (err, stats) => {
    if (err || !stats.isFile()) {
      res.writeHead(404, { 'Content-Type': 'text/plain; charset=utf-8' });
      return res.end('404 Not Found');
    }

    const ext = path.extname(filePath).toLowerCase();
    const contentType = MIME_TYPES[ext] || 'application/octet-stream';

    res.writeHead(200, {
      'Content-Type': contentType,
      'Content-Length': stats.size,
      'Cache-Control': 'no-cache'
    });

    fs.createReadStream(filePath).pipe(res);
  });
});


server.on('error', (err) => {
  if (err.code === 'EADDRINUSE') {
    console.log('[Anti-Pet] 端口 ' + PORT + ' 已有运行中的旧服务实例在监听');
    process.exit(0);
  } else {
    console.error('[Anti-Pet] 服务异常:', err);
    process.exit(1);
  }
});

server.listen(PORT, '127.0.0.1', () => {
  console.log(`\n======================================================`);
  console.log(`大肥猫 · Antigravity 伴侣服务已就绪`);
  console.log(`预览地址: http://127.0.0.1:${PORT}`);
  console.log(`状态通信: http://127.0.0.1:${PORT}/api/events`);
  console.log(`配置持久: http://127.0.0.1:${PORT}/api/settings`);
  console.log(`实时额度: http://127.0.0.1:${PORT}/api/quota`);
  console.log(`======================================================\n`);
});
