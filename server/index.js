#!/usr/bin/env node
/**
 * Anti-Pet Local Hub, Real Antigravity Quota Sync, Live Transcript & Settings Store
 * Zero-dependency native Node.js HTTP & SSE server
 */

const http = require('http');
const fs = require('fs');
const path = require('path');
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
  scale: 1.0,
  skinId: 'fat-cat',
  soundEnabled: true,
  gravityEnabled: true,
  ideOnlyMode: false,
  dndPassthrough: false,
  posX: null,
  posY: null,
  facing: 'left'
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
  try {
    fs.writeFileSync(SETTINGS_FILE, JSON.stringify(currentSettings, null, 2), 'utf-8');
  } catch (e) {}
  return currentSettings;
}

const sseClients = new Set();

function readFallbackLedgerQuota() {
  const now = Date.now();
  const FIVE_HOURS_MS = 5 * 60 * 60 * 1000;
  const ONE_WEEK_MS = 7 * 24 * 60 * 60 * 1000;
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

  
  // 8. Shutdown: /api/shutdown
  if (pathname === '/api/shutdown') {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ ok: true, message: 'Server shutting down' }));
    setTimeout(() => process.exit(0), 100);
    return;
  }

  // 7. List Skins: /api/skins
  if (pathname === '/api/skins') {
    const skinsDir = path.join(ROOT_DIR, 'skins');
    try {
      const skins = fs
        .readdirSync(skinsDir, { withFileTypes: true })
        .filter((d) => d.isDirectory())
        .map((d) => {
          const skinJsonPath = path.join(skinsDir, d.name, 'skin.json');
          if (fs.existsSync(skinJsonPath)) {
            return JSON.parse(fs.readFileSync(skinJsonPath, 'utf-8'));
          }
          return { id: d.name, name: d.name };
        });
      res.writeHead(200, { 'Content-Type': 'application/json' });
      return res.end(JSON.stringify(skins));
    } catch (e) {
      res.writeHead(500, { 'Content-Type': 'application/json' });
      return res.end(JSON.stringify({ error: e.message }));
    }
  }

  // 8. Static File Serving (Support both root `/pet.css` and `/core/pet.css`)
  if (pathname === '/' || pathname === '/index.html') {
    pathname = '/core/index.html';
  } else if (pathname === '/pet.css' || pathname === '/pet.js' || pathname === '/bridge.js') {
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
