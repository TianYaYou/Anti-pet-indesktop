/**
 * Anti-Pet Bridge - Telemetry & Event receiver from Antigravity IDE
 */
class PetBridge {
  constructor(options = {}) {
    this.baseUrl = options.baseUrl || window.location.origin;
    this.onEvent = options.onEvent || (() => {});
    this.onQuota = options.onQuota || (() => {});
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

      this.eventSource.onerror = () => {
        this.connected = false;
        this.eventSource.close();
        // Retry after 3 seconds
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

window.PetBridge = PetBridge;
