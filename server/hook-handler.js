#!/usr/bin/env node
const fs = require('fs');
const path = require('path');
const http = require('http');

const STATE_FILE = path.resolve(__dirname, '../.antigravity-pet-state.json');
const SERVER_PORT = process.env.ANTI_PET_PORT || 4199;

const eventType = process.argv[2] || 'idle';
const toolOrDetail = process.argv[3] || '';

// Parse custom flags if present
let customText = null;
let customState = null;
for (let i = 2; i < process.argv.length; i++) {
  if (process.argv[i] === '--text' && process.argv[i + 1]) customText = process.argv[i + 1];
  if (process.argv[i] === '--state' && process.argv[i + 1]) customState = process.argv[i + 1];
}

// Map Antigravity lifecycle event to Pet State
function resolveState(event) {
  if (customState) return customState;
  switch (event) {
    case 'SessionStart':
      return { state: 'idle', bubbleText: '喵~ 新的会话开始啦！今天写什么代码？', emotion: 'curious' };
    case 'PreInvocation':
      return { state: 'thinking', bubbleText: 'Agent 正在认真思考解决方案...', emotion: 'ponder' };
    case 'PreToolUse':
      return { state: 'working', bubbleText: `正在调用工具: ${toolOrDetail || '终端执行'}...`, emotion: 'working' };
    case 'PostToolUse':
      return { state: 'success', bubbleText: '工具执行成功！快夸夸本喵~', emotion: 'proud', durationMs: 3000 };
    case 'PostInvocation':
      return { state: 'idle', bubbleText: '回答完毕啦！', emotion: 'calm', durationMs: 2500 };
    case 'Stop':
      return { state: 'idle', bubbleText: '本轮任务结束，本喵先伸个懒腰~', emotion: 'relaxed' };
    default:
      return { state: event, bubbleText: customText || '', emotion: 'idle' };
  }
}

const resolved = resolveState(eventType);
if (customText) resolved.bubbleText = customText;

const payload = {
  timestamp: Date.now(),
  event: eventType,
  state: resolved.state,
  bubbleText: resolved.bubbleText,
  emotion: resolved.emotion,
  durationMs: resolved.durationMs || 0,
  tool: toolOrDetail
};

// 1. Write to state file for persistence
try {
  fs.writeFileSync(STATE_FILE, JSON.stringify(payload, null, 2), 'utf-8');
} catch (e) {
  // Silent fallback
}

// 2. Notify running server via HTTP POST (non-blocking, fast fail)
const req = http.request({
  hostname: '127.0.0.1',
  port: SERVER_PORT,
  path: '/api/event',
  method: 'POST',
  headers: {
    'Content-Type': 'application/json'
  },
  timeout: 150
}, (res) => {
  res.resume();
});

req.on('error', () => {
  // Server might not be running, perfectly normal
});

req.write(JSON.stringify(payload));
req.end();
