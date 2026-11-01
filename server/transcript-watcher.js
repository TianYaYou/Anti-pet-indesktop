/**
 * Real-time Antigravity Brain Transcript Watcher
 * Monitors active Antigravity conversations, tails transcript.jsonl,
 * and emits real-time agent states (Thinking, Working, Done, Error).
 */

const fs = require('fs');
const path = require('path');
const os = require('os');

class TranscriptWatcher {
  constructor(options = {}) {
    this.brainDir = options.brainDir || path.join(os.homedir(), '.gemini/antigravity/brain');
    this.onState = options.onState || (() => {});
    this.onActivity = options.onActivity || (() => {});
    this.currentConvId = null;
    this.currentLogPath = null;
    this.fileOffset = 0;
    this.pollInterval = null;
    this.lastActiveTime = Date.now();
  }

  start() {
    this.locateActiveConversation();
    // Poll for new lines and new conversation every 800ms
    this.pollInterval = setInterval(() => {
      this.locateActiveConversation();
      this.readNewEntries();
    }, 800);
  }

  getLastActiveTime() {
    return this.lastActiveTime;
  }

  locateActiveConversation() {
    if (!fs.existsSync(this.brainDir)) return;
    try {
      const dirs = fs.readdirSync(this.brainDir, { withFileTypes: true })
        .filter((d) => d.isDirectory() && !d.name.startsWith('temp') && !d.name.startsWith('.'));

      let newestConvId = null;
      let newestLogPath = null;
      let newestMtime = 0;

      for (const d of dirs) {
        const logPath = path.join(this.brainDir, d.name, '.system_generated/logs/transcript.jsonl');
        if (fs.existsSync(logPath)) {
          try {
            const stat = fs.statSync(logPath);
            if (stat.mtimeMs > newestMtime) {
              newestMtime = stat.mtimeMs;
              newestConvId = d.name;
              newestLogPath = logPath;
            }
          } catch (err) {}
        }
      }

      if (!newestLogPath) return;

      if (newestMtime > this.lastActiveTime) {
        this.lastActiveTime = newestMtime;
        this.onActivity({ convId: newestConvId, mtime: newestMtime });
      }

      if (this.currentConvId !== newestConvId || this.currentLogPath !== newestLogPath) {
        this.currentConvId = newestConvId;
        this.currentLogPath = newestLogPath;
        // On initial attach, start near the end of file (last 32KB) so we don't replay old history
        try {
          const stat = fs.statSync(newestLogPath);
          this.fileOffset = Math.max(0, stat.size - 32768);
          this.readNewEntries();
        } catch (e) {
          this.fileOffset = 0;
        }
      }
    } catch (e) {
      // Non-fatal
    }
  }

  readNewEntries() {
    if (!this.currentLogPath || !fs.existsSync(this.currentLogPath)) return;
    try {
      const stat = fs.statSync(this.currentLogPath);
      if (stat.size <= this.fileOffset) {
        if (stat.size < this.fileOffset) {
          // File was truncated or rotated
          this.fileOffset = 0;
        }
        return;
      }

      const fd = fs.openSync(this.currentLogPath, 'r');
      const bytesToRead = stat.size - this.fileOffset;
      const buffer = Buffer.alloc(bytesToRead);
      fs.readSync(fd, buffer, 0, bytesToRead, this.fileOffset);
      fs.closeSync(fd);

      this.fileOffset = stat.size;
      this.lastActiveTime = Date.now();
      this.onActivity({ convId: this.currentConvId, timestamp: this.lastActiveTime });

      const text = buffer.toString('utf-8');
      const lines = text.split('\n');

      for (const line of lines) {
        const trimmed = line.trim();
        if (!trimmed) continue;
        try {
          const step = JSON.parse(trimmed);
          this.processStep(step);
        } catch (e) {
          // Incomplete JSON line chunk, safe to skip or wait
        }
      }
    } catch (e) {
      // Non-fatal
    }
  }

  processStep(step) {
    // 1. User Input -> Agent starts Thinking
    if (step.type === 'USER_INPUT') {
      this.onState({
        state: 'thinking',
        bubbleText: '本喵正在翻书探头，高速运转脑回路...',
        emotion: 'ponder',
        durationMs: 0,
        source: 'transcript-live'
      });
      return;
    }

    // 2. Planner Response
    if (step.type === 'PLANNER_RESPONSE') {
      const toolCalls = step.tool_calls || [];
      if (toolCalls.length > 0) {
        const toolName = toolCalls[0].name || '工具执行';
        let actionDesc = `正在调用工具: ${toolName}`;
        if (toolName.includes('run_command')) actionDesc = '正在执行终端命令...';
        else if (toolName.includes('edit_file') || toolName.includes('write_file')) actionDesc = '正在编写修改代码...';
        else if (toolName.includes('read_file')) actionDesc = '正在阅读代码文件...';

        this.onState({
          state: 'working',
          bubbleText: `${actionDesc}，稍等喵~`,
          emotion: 'working',
          tool: toolName,
          durationMs: 0,
          source: 'transcript-live'
        });
      } else if (step.status === 'DONE') {
        this.onState({
          state: 'success',
          bubbleText: '本轮任务完成！快夸夸本喵~',
          emotion: 'proud',
          durationMs: 4000,
          source: 'transcript-live'
        });
      } else if (step.status === 'ERROR') {
        this.onState({
          state: 'error',
          bubbleText: '呜... 模型思考遇到异常',
          emotion: 'worry',
          durationMs: 4500,
          source: 'transcript-live'
        });
      }
      return;
    }

    // 3. Tool execution progress
    if (step.status === 'RUNNING') {
      this.onState({
        state: 'working',
        bubbleText: '正在执行工具命令，瞧好吧！',
        emotion: 'working',
        durationMs: 0,
        source: 'transcript-live'
      });
    } else if (step.status === 'ERROR') {
      this.onState({
        state: 'error',
        bubbleText: '工具执行遇到报错啦！',
        emotion: 'worry',
        durationMs: 4500,
        source: 'transcript-live'
      });
    }
  }

  stop() {
    if (this.pollInterval) clearInterval(this.pollInterval);
  }
}

module.exports = {
  TranscriptWatcher
};
