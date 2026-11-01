#!/usr/bin/env node
/**
 * Anti-Pet Antigravity Lifecycle Hook Handler & Live Quota Synchronizer
 */
const fs = require('fs');
const path = require('path');
const http = require('http');
const { fetchAntigravityQuota } = require('./quota-probe');

const ROOT_DIR = path.resolve(__dirname, '..');
const STATE_FILE = path.join(ROOT_DIR, '.antigravity-pet-state.json');
const LEDGER_FILE = path.join(ROOT_DIR, '.antigravity-pet-ledger.json');
const SERVER_PORT = parseInt(process.env.ANTI_PET_PORT || '4199', 10);

const eventType = process.argv[2] || 'idle';
const toolOrDetail =
  process.argv[3] && !process.argv[3].startsWith('--')
    ? process.argv[3]
    : process.env.TOOL_NAME || '';

let customText = null;
let customState = null;
for (let i = 2; i < process.argv.length; i++) {
  if (process.argv[i] === '--text' && process.argv[i + 1]) customText = process.argv[i + 1];
  if (process.argv[i] === '--state' && process.argv[i + 1]) customState = process.argv[i + 1];
}

function recordLocalLedger(evt) {
  const now = Date.now();
  const ONE_WEEK_MS = 7 * 24 * 60 * 60 * 1000;
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
  try {
    fs.writeFileSync(LEDGER_FILE, JSON.stringify(ledger, null, 2), 'utf-8');
  } catch (e) {}
}

function resolveState(event) {
  if (customState) {
    return { state: customState, bubbleText: customText || '', emotion: customState, durationMs: 3500 };
  }
  switch (event) {
    case 'SessionStart':
      return {
        state: 'idle',
        bubbleText: '喵~ 新的会话开始啦！今天写什么代码？',
        emotion: 'curious',
        durationMs: 4000
      };
    case 'PreInvocation':
      return {
        state: 'thinking',
        bubbleText: '本喵正在探头思考，高速运转脑回路...',
        emotion: 'ponder',
        durationMs: 0
      };
    case 'PreToolUse':
      return {
        state: 'working',
        bubbleText: `正在调用工具: ${toolOrDetail || '终端执行'}...`,
        emotion: 'working',
        durationMs: 0
      };
    case 'PostToolUse':
      return {
        state: 'success',
        bubbleText: `工具 ${toolOrDetail ? `[${toolOrDetail}] ` : ''}搞定啦！快夸本喵~`,
        emotion: 'proud',
        durationMs: 3200
      };
    case 'PostToolUseFailure':
    case 'Error':
      return {
        state: 'error',
        bubbleText: '工具执行好像报错了，陪你一起排查！',
        emotion: 'sad',
        durationMs: 4200
      };
    case 'PostInvocation':
      return {
        state: 'success',
        bubbleText: '本轮推理圆满完成~',
        emotion: 'proud',
        durationMs: 3000
      };
    case 'Stop':
      return {
        state: 'success',
        bubbleText: '任务全部搞定啦！本喵先得意一下~',
        emotion: 'proud',
        durationMs: 3500
      };
    default:
      return {
        state: event,
        bubbleText: customText || '',
        emotion: 'idle',
        durationMs: 3000
      };
  }
}

(async () => {
  recordLocalLedger(eventType);
  const liveQuota = await fetchAntigravityQuota(true);
  const resolved = resolveState(eventType);
  if (customText) resolved.bubbleText = customText;

  const payload = {
    timestamp: Date.now(),
    event: eventType,
    state: resolved.state,
    bubbleText: resolved.bubbleText,
    emotion: resolved.emotion,
    durationMs: resolved.durationMs || 0,
    tool: toolOrDetail,
    ...(liveQuota ? { quota: liveQuota } : {})
  };

  try {
    fs.writeFileSync(STATE_FILE, JSON.stringify(payload, null, 2), 'utf-8');
  } catch (e) {}

  const req = http.request(
    {
      hostname: '127.0.0.1',
      port: SERVER_PORT,
      path: '/api/event',
      method: 'POST',
      headers: {
        'Content-Type': 'application/json'
      },
      timeout: 300
    },
    (res) => {
      res.resume();
    }
  );

  req.on('error', () => {});
  req.write(JSON.stringify(payload));
  req.end();
})();
