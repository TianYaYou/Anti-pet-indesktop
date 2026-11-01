#!/usr/bin/env node
/**
 * Anti-Pet Local Hub, Real Antigravity Quota Sync & Telemetry Server
 * Zero-dependency native Node.js HTTP & SSE server
 */

const http = require('http');
const fs = require('fs');
const path = require('path');
const { fetchAntigravityQuota } = require('./quota-probe');

const PORT = parseInt(process.env.ANTI_PET_PORT || '4199', 10);
const ROOT_DIR = path.resolve(__dirname, '..');
const STATE_FILE = path.join(ROOT_DIR, '.antigravity-pet-state.json');
const LEDGER_FILE = path.join(ROOT_DIR, '.antigravity-pet-ledger.json');

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
    const quota = await refreshLiveQuota(true);
    res.write(`event: quota\ndata: ${JSON.stringify(quota)}\n\n`);
    return;
  }

  // 2. Post Event: /api/event
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

  // 3. Post Control Command: /api/control
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

  // 4. Get State & Quota: /api/state & /api/quota
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

  // 5. List Skins: /api/skins
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

  // 6. Static File Serving (Support both root `/pet.css` and `/core/pet.css`)
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

server.listen(PORT, '127.0.0.1', () => {
  console.log(`\n======================================================`);
  console.log(`大肥猫 · Antigravity 伴侣服务已就绪`);
  console.log(`预览地址: http://127.0.0.1:${PORT}`);
  console.log(`状态通信: http://127.0.0.1:${PORT}/api/events`);
  console.log(`实时额度: http://127.0.0.1:${PORT}/api/quota`);
  console.log(`======================================================\n`);
});
