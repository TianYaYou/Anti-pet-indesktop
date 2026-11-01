#!/usr/bin/env node
/**
 * Anti-Pet Local Hub & Telemetry Server
 * Zero-dependency native Node.js HTTP & SSE server
 */

const http = require('http');
const fs = require('fs');
const path = require('path');
const url = require('url');

const PORT = parseInt(process.env.ANTI_PET_PORT || '4199', 10);
const ROOT_DIR = path.resolve(__dirname, '..');
const STATE_FILE = path.join(ROOT_DIR, '.antigravity-pet-state.json');

// MIME types table
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

// Connected SSE clients
const sseClients = new Set();

let currentState = {
  timestamp: Date.now(),
  event: 'init',
  state: 'idle',
  bubbleText: '大肥猫已就位！随时为你护航代码~',
  emotion: 'idle',
  durationMs: 0
};

// Load initial state if exists
if (fs.existsSync(STATE_FILE)) {
  try {
    currentState = JSON.parse(fs.readFileSync(STATE_FILE, 'utf-8'));
  } catch (e) {}
}

// Watch state file for changes
fs.watchFile(STATE_FILE, { interval: 300 }, () => {
  try {
    if (fs.existsSync(STATE_FILE)) {
      const data = JSON.parse(fs.readFileSync(STATE_FILE, 'utf-8'));
      currentState = data;
      broadcastSSE('state', data);
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

// Create HTTP Server
const server = http.createServer((req, res) => {
  const parsedUrl = new URL(req.url, `http://${req.headers.host || '127.0.0.1'}`);
  let pathname = decodeURIComponent(parsedUrl.pathname);

  // CORS headers
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

    res.write(`event: state\ndata: ${JSON.stringify(currentState)}\n\n`);
    res.write(`event: quota\ndata: ${JSON.stringify({ fiveHour: 84.5, weekly: 96.0 })}\n\n`);

    sseClients.add(res);

    req.on('close', () => {
      sseClients.delete(res);
    });
    return;
  }

  // 2. Post Event: /api/event
  if (pathname === '/api/event' && req.method === 'POST') {
    let body = '';
    req.on('data', chunk => body += chunk);
    req.on('end', () => {
      try {
        const payload = JSON.parse(body);
        currentState = payload;
        broadcastSSE('state', payload);
        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ ok: true }));
      } catch (e) {
        res.writeHead(400, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ error: 'Invalid JSON' }));
      }
    });
    return;
  }

  // 3. Get State: /api/state
  if (pathname === '/api/state') {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    return res.end(JSON.stringify(currentState));
  }

  // 4. List Skins: /api/skins
  if (pathname === '/api/skins') {
    const skinsDir = path.join(ROOT_DIR, 'skins');
    try {
      const skins = fs.readdirSync(skinsDir, { withFileTypes: true })
        .filter(d => d.isDirectory())
        .map(d => {
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

  // 5. Static File Serving
  if (pathname === '/' || pathname === '/index.html') {
    pathname = '/core/index.html';
  }

  const filePath = path.join(ROOT_DIR, pathname);

  // Security check: ensure path is within ROOT_DIR
  if (!filePath.startsWith(ROOT_DIR)) {
    res.writeHead(403);
    return res.end('Access denied');
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

    const stream = fs.createReadStream(filePath);
    stream.pipe(res);
  });
});

server.listen(PORT, '127.0.0.1', () => {
  console.log(`\n======================================================`);
  console.log(`🐾 大肥猫 · Antigravity 伴侣服务已就绪！`);
  console.log(`🔗 预览地址: http://127.0.0.1:${PORT}`);
  console.log(`📡 状态通信: http://127.0.0.1:${PORT}/api/events`);
  console.log(`======================================================\n`);
});
