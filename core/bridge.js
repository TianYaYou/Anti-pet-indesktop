/**
 * Anti-Pet Bridge - Telemetry, SSE Event Stream & Desktop Wry IPC Bridge
 */

class DesktopIPC {
  constructor() {
    this.lastIgnoreState = null;
    this.lastRectsSignature = '';
  }

  get isDesktop() {
    return Boolean(window.ipc && typeof window.ipc.postMessage === 'function');
  }

  sendRaw(msg) {
    if (this.isDesktop) {
      try {
        window.ipc.postMessage(msg);
      } catch (err) {
        console.warn('[DesktopIPC] postMessage failed:', err);
      }
    }
  }

  setIgnoreCursorEvents(ignore) {
    const flag = Boolean(ignore);
    if (this.lastIgnoreState === flag) return;
    this.lastIgnoreState = flag;
    this.sendRaw(`set_ignore_cursor_events:${flag}`);
  }

  setDragging(dragging) {
    this.sendRaw(`set_dragging:${Boolean(dragging)}`);
  }

  setAlwaysOnTop(enabled) {
    this.sendRaw(`set_always_on_top:${Boolean(enabled)}`);
  }

  setVisible(visible) {
    this.sendRaw(`set_visible:${Boolean(visible)}`);
  }

  setDndPassthrough(enabled, wakeRect = null) {
    if (enabled && wakeRect) {
      const r = `${Math.round(wakeRect.x)},${Math.round(wakeRect.y)},${Math.round(wakeRect.w)},${Math.round(wakeRect.h)}`;
      this.sendRaw(`set_dnd:true:${r}`);
    } else {
      this.sendRaw(`set_dnd:${Boolean(enabled)}`);
    }
  }

  updateHitRects(rects) {
    const serialized = rects
      .filter((r) => r && r.w > 0 && r.h > 0)
      .map((r) => `${Math.round(r.x)},${Math.round(r.y)},${Math.round(r.w)},${Math.round(r.h)}`)
      .join(';');
    if (serialized === this.lastRectsSignature) return;
    this.lastRectsSignature = serialized;
    this.sendRaw(`hit_rects:${serialized}`);
  }
}

class PetBridge {
  constructor(options = {}) {
    this.baseUrl = options.baseUrl || window.location.origin;
    this.onEvent = options.onEvent || (() => {});
    this.onQuota = options.onQuota || (() => {});
    this.onControl = options.onControl || (() => {});
    this.eventSource = null;
    this.connected = false;
  }

  start() {
    this.connectSSE();
  }

  connectSSE() {
    try {
      this.eventSource = new EventSource(`${this.baseUrl}/api/events`);

      this.eventSource.onopen = () => {
        this.connected = true;
        console.log('[PetBridge] Connected to Antigravity live telemetry stream.');
      };

      this.eventSource.addEventListener('state', (e) => {
        try {
          const data = JSON.parse(e.data);
          this.onEvent(data);
        } catch (err) {
          console.error('[PetBridge] Failed to parse state event', err);
        }
      });

      this.eventSource.addEventListener('quota', (e) => {
        try {
          const data = JSON.parse(e.data);
          this.onQuota(data);
        } catch (err) {
          console.error('[PetBridge] Failed to parse quota event', err);
        }
      });

      this.eventSource.addEventListener('control', (e) => {
        try {
          const data = JSON.parse(e.data);
          this.onControl(data);
        } catch (err) {
          console.error('[PetBridge] Failed to parse control event', err);
        }
      });

      this.eventSource.onerror = () => {
        this.connected = false;
        this.eventSource.close();
        setTimeout(() => this.connectSSE(), 3000);
      };
    } catch (err) {
      console.warn('[PetBridge] SSE not available, falling back to polling', err);
      this.startPolling();
    }
  }

  startPolling() {
    setInterval(async () => {
      try {
        const res = await fetch(`${this.baseUrl}/api/state`);
        if (res.ok) {
          const data = await res.json();
          this.onEvent(data);
        }
      } catch (err) {
        // Server might be paused
      }
    }, 2500);
  }
}

window.DesktopIPC = new DesktopIPC();
window.PetBridge = PetBridge;
