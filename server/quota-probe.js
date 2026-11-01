/**
 * Antigravity Real Quota Probe
 * Connects directly to the local Antigravity `language_server` Connect-RPC API:
 * 1. /exa.language_server_pb.LanguageServerService/RetrieveUserQuotaSummary
 * 2. /exa.language_server_pb.LanguageServerService/GetUserStatus
 * 3. /exa.language_server_pb.LanguageServerService/GetCommandModelConfigs
 */

const https = require('https');
const http = require('http');
const { execFile } = require('child_process');

let cachedEndpoint = null; // { pid, port, scheme, csrfToken }
let cachedQuota = null;
let lastFetchTs = 0;
let inflightPromise = null;

function execCmd(cmd, args, timeout = 2500) {
  return new Promise((resolve) => {
    execFile(cmd, args, { timeout, maxBuffer: 1024 * 1024 }, (err, stdout) => {
      if (err) return resolve('');
      resolve(String(stdout || ''));
    });
  });
}

/**
 * Format an ISO-8601 timestamp or epoch seconds into a concise reset countdown (e.g. "2h 55m", "4h 55m", "4d 8h")
 */
function formatResetCountdown(resetTimeRaw) {
  if (!resetTimeRaw) return '';
  let targetMs = NaN;
  if (typeof resetTimeRaw === 'number') {
    targetMs = resetTimeRaw > 1e12 ? resetTimeRaw : resetTimeRaw * 1000;
  } else if (typeof resetTimeRaw === 'string') {
    const trimmed = resetTimeRaw.trim();
    if (/^\d+(\.\d+)?$/.test(trimmed)) {
      const n = Number(trimmed);
      targetMs = n > 1e12 ? n : n * 1000;
    } else {
      targetMs = Date.parse(trimmed);
    }
  } else if (typeof resetTimeRaw === 'object' && resetTimeRaw !== null) {
    const sec = resetTimeRaw.seconds ?? resetTimeRaw.secs;
    if (sec !== undefined) {
      targetMs = Number(sec) * 1000;
    }
  }
  if (Number.isNaN(targetMs)) return '';

  const diffSec = Math.max(0, Math.floor((targetMs - Date.now()) / 1000));
  if (diffSec <= 0) return '即将重置';

  const days = Math.floor(diffSec / 86400);
  const hours = Math.floor((diffSec % 86400) / 3600);
  const mins = Math.floor((diffSec % 3600) / 60);

  if (days > 0) {
    return `${days}d ${hours}h`;
  }
  if (hours > 0) {
    return `${hours}h ${mins}m`;
  }
  return `${mins}m`;
}

/**
 * Discover running Antigravity language_server processes and extract PID, CSRF token, and listening TCP ports
 */
async function discoverLanguageServerCandidates() {
  const candidates = [];

  if (process.platform === 'win32') {
    const psOut = await execCmd('powershell', [
      '-NoProfile',
      '-Command',
      "Get-CimInstance Win32_Process | Where-Object { $_.Name -match 'language_server|agy' } | Select-Object ProcessId,CommandLine | ConvertTo-Json -Compress"
    ]);
    if (psOut) {
      try {
        const parsed = JSON.parse(psOut);
        const list = Array.isArray(parsed) ? parsed : [parsed];
        for (const item of list) {
          if (!item || !item.ProcessId) continue;
          const cmd = String(item.CommandLine || '');
          const csrfMatch =
            cmd.match(/--csrf_token[=\s]+([^\s"]+)/i) ||
            cmd.match(/--extension_server_csrf_token[=\s]+([^\s"]+)/i);
          const extPortMatch = cmd.match(/--extension_server_port[=\s]+(\d+)/i);
          candidates.push({
            pid: Number(item.ProcessId),
            csrfToken: csrfMatch ? csrfMatch[1] : '',
            extPort: extPortMatch ? Number(extPortMatch[1]) : null,
            priority: /antigravity/i.test(cmd) ? 1 : 2
          });
        }
      } catch (e) {}
    }

    const tcpMap = new Map();
    const tcpJson = await execCmd('powershell', [
      '-NoProfile',
      '-Command',
      "Get-NetTCPConnection -ErrorAction SilentlyContinue | Where-Object { $_.LocalAddress -in @('127.0.0.1', '0.0.0.0', '::', '::1') } | Select-Object OwningProcess,LocalPort | ConvertTo-Json -Compress"
    ]);
    if (tcpJson) {
      try {
        const rows = JSON.parse(tcpJson);
        const list = Array.isArray(rows) ? rows : [rows];
        for (const r of list) {
          const pid = Number(r.OwningProcess);
          const port = Number(r.LocalPort);
          if (pid && port > 1024 && port < 65535) {
            if (!tcpMap.has(pid)) tcpMap.set(pid, new Set());
            tcpMap.get(pid).add(port);
          }
        }
      } catch (e) {}
    }

    const netstatOut = await execCmd('netstat', ['-ano']);
    if (netstatOut) {
      for (const line of netstatOut.split(/\r?\n/)) {
        const parts = line.trim().split(/\s+/);
        if (parts.length >= 5) {
          const pid = Number(parts[parts.length - 1]);
          const localAddr = parts[1] || '';
          const m = localAddr.match(/:(\d+)$/);
          if (pid && m) {
            const port = Number(m[1]);
            if (port > 1024 && port < 65535) {
              if (!tcpMap.has(pid)) tcpMap.set(pid, new Set());
              tcpMap.get(pid).add(port);
            }
          }
        }
      }
    }

    for (const c of candidates) {
      const ports = new Set(tcpMap.get(c.pid) || []);
      if (c.extPort) {
        ports.add(c.extPort);
        ports.add(c.extPort + 1);
        ports.add(c.extPort + 2);
      }
      c.ports = Array.from(ports);
    }
    return candidates.filter((c) => c.ports.length > 0).sort((a, b) => a.priority - b.priority);
  }

  // macOS & Linux process discovery
  let psOut = await execCmd('/bin/ps', ['-ax', '-o', 'pid=,command=']);
  if (!psOut) {
    psOut = await execCmd('ps', ['-ax', '-o', 'pid=,command=']);
  }

  const lines = psOut.split(/\r?\n/);
  for (const rawLine of lines) {
    const line = rawLine.trim();
    if (!line) continue;
    if (!/language_server|language-server|\bagy\b/i.test(line)) continue;
    if (/grep|quota-probe/i.test(line)) continue;

    const m = line.match(/^(\d+)\s+(.+)$/);
    if (!m) continue;
    const pid = Number(m[1]);
    const cmd = m[2];

    const csrfMatch =
      cmd.match(/--csrf_token[=\s]+([^\s"']+)/) ||
      cmd.match(/--extension_server_csrf_token[=\s]+([^\s"']+)/);
    const extPortMatch = cmd.match(/--extension_server_port[=\s]+(\d+)/);

    const isAntigravityApp = /Antigravity\.app|--app_data_dir\s+antigravity\b/i.test(cmd);
    const isAntigravityAny = /antigravity|\bagy\b/i.test(cmd);

    candidates.push({
      pid,
      csrfToken: csrfMatch ? csrfMatch[1] : '',
      extPort: extPortMatch ? Number(extPortMatch[1]) : null,
      priority: isAntigravityApp ? 1 : isAntigravityAny ? 2 : 3,
      ports: []
    });
  }

  candidates.sort((a, b) => a.priority - b.priority);

  for (const c of candidates) {
    const portSet = new Set();
    let lsofOut = await execCmd('/usr/sbin/lsof', [
      '-nP',
      '-iTCP',
      '-sTCP:LISTEN',
      '-a',
      '-p',
      String(c.pid)
    ]);
    if (!lsofOut) {
      lsofOut = await execCmd('lsof', [
        '-nP',
        '-iTCP',
        '-sTCP:LISTEN',
        '-a',
        '-p',
        String(c.pid)
      ]);
    }
    const portRegex = /:(\d+)\s+\(LISTEN\)/g;
    let pm;
    while ((pm = portRegex.exec(lsofOut)) !== null) {
      portSet.add(Number(pm[1]));
    }
    if (c.extPort) {
      portSet.add(c.extPort);
      portSet.add(c.extPort + 1);
      portSet.add(c.extPort + 2);
    }
    c.ports = Array.from(portSet);
  }

  return candidates.filter((c) => c.ports.length > 0);
}

/**
 * Perform a Connect-RPC POST request to the local language_server
 */
function postConnectRpc({ scheme, port, rpcMethod, csrfToken, timeout = 1200 }) {
  return new Promise((resolve) => {
    const payload = JSON.stringify({
      metadata: {
        ideName: 'antigravity',
        extensionName: 'antigravity',
        locale: 'en',
        ideVersion: 'unknown'
      }
    });

    const headers = {
      'Content-Type': 'application/json',
      'Content-Length': Buffer.byteLength(payload),
      'Connect-Protocol-Version': '1'
    };
    if (csrfToken) {
      headers['X-Codeium-Csrf-Token'] = csrfToken;
    }

    const options = {
      hostname: '127.0.0.1',
      port,
      path: `/exa.language_server_pb.LanguageServerService/${rpcMethod}`,
      method: 'POST',
      headers,
      timeout,
      rejectUnauthorized: false
    };

    const transport = scheme === 'https' ? https : http;
    const req = transport.request(options, (res) => {
      let body = '';
      res.on('data', (chunk) => (body += chunk));
      res.on('end', () => {
        if (res.statusCode !== 200) {
          return resolve(null);
        }
        try {
          resolve(JSON.parse(body));
        } catch (e) {
          resolve(null);
        }
      });
    });

    req.on('timeout', () => {
      req.destroy();
      resolve(null);
    });
    req.on('error', () => resolve(null));
    req.write(payload);
    req.end();
  });
}

function extractBucketFraction(bucket) {
  if (!bucket || typeof bucket !== 'object') return null;
  const candidates = [
    bucket.remainingFraction,
    bucket.remaining_fraction,
    bucket.remaining?.remainingFraction,
    bucket.remaining?.remaining_fraction,
    bucket.fraction
  ];
  for (const val of candidates) {
    if (typeof val === 'number' && !Number.isNaN(val)) {
      return Math.max(0, Math.min(1, val));
    }
    if (typeof val === 'string' && val.trim() !== '' && !Number.isNaN(Number(val))) {
      return Math.max(0, Math.min(1, Number(val)));
    }
  }
  if (typeof bucket.remainingPercent === 'number') {
    return Math.max(0, Math.min(1, bucket.remainingPercent / 100));
  }
  const desc = String(bucket.description || '');
  if (/hit your .* limit|exhausted/i.test(desc)) {
    return 0;
  }
  return null;
}

/**
 * Parse RetrieveUserQuotaSummary (or nested quotaSummary) into { fiveHour, weekly, reset5h, resetWeekly, meta, source }
 */
function parseQuotaSummaryResponse(data) {
  if (!data || typeof data !== 'object') return null;

  const root =
    data.command?.data ||
    data.response ||
    data.quotaSummary ||
    data.userQuotaSummary ||
    data.userStatus?.quotaSummary ||
    data;

  const allBuckets = [];

  if (Array.isArray(root.groups)) {
    for (const grp of root.groups) {
      const groupName = String(grp.displayName || grp.display_name || grp.title || grp.id || '');
      const buckets = Array.isArray(grp.buckets) ? grp.buckets : [];
      for (const b of buckets) {
        allBuckets.push({ ...b, _groupName: groupName });
      }
    }
  }
  if (Array.isArray(root.buckets)) {
    for (const b of root.buckets) {
      allBuckets.push({ ...b, _groupName: b._groupName || '' });
    }
  }

  if (allBuckets.length === 0) return null;

  let gemini5h = null;
  let geminiWeekly = null;
  let fallback5h = null;
  let fallbackWeekly = null;

  for (const b of allBuckets) {
    const frac = extractBucketFraction(b);
    if (frac === null) continue;

    const pct = +(frac * 100).toFixed(1);
    const id = String(b.bucketId || b.bucket_id || b.id || '').toLowerCase();
    const win = String(b.window || b.cadence || b.windowType || '').toLowerCase();
    const name = String(b.displayName || b.display_name || b.label || '').toLowerCase();
    const grp = String(b._groupName || '').toLowerCase();
    const resetTime = b.resetTime || b.reset_time || b.remaining?.resetTime || b.remaining?.reset_time || '';
    const resetLabel = formatResetCountdown(resetTime);

    const isThirdParty =
      id.includes('3p') ||
      id.includes('claude') ||
      id.includes('gpt') ||
      grp.includes('claude') ||
      grp.includes('gpt');

    const isGemini = id.includes('gemini') || grp.includes('gemini') || !isThirdParty;

    const is5h =
      id.includes('5h') ||
      id.includes('session') ||
      win.includes('5h') ||
      win.includes('five') ||
      win.includes('session') ||
      name.includes('5h') ||
      name.includes('five hour');

    const isWeekly =
      id.includes('week') ||
      id.includes('7d') ||
      win.includes('week') ||
      win.includes('7d') ||
      name.includes('week');

    const entry = { pct, resetLabel, resetTime, id };

    if (isGemini && is5h) {
      if (!gemini5h || id === 'gemini-5h' || pct < gemini5h.pct) {
        gemini5h = entry;
      }
    } else if (isGemini && isWeekly) {
      if (!geminiWeekly || id === 'gemini-weekly' || pct < geminiWeekly.pct) {
        geminiWeekly = entry;
      }
    } else if (is5h && !fallback5h) {
      fallback5h = entry;
    } else if (isWeekly && !fallbackWeekly) {
      fallbackWeekly = entry;
    }
  }

  const chosen5h = gemini5h || fallback5h;
  const chosenWeekly = geminiWeekly || fallbackWeekly;

  if (!chosen5h && !chosenWeekly) return null;

  const fiveHour = chosen5h ? chosen5h.pct : chosenWeekly.pct;
  const weekly = chosenWeekly ? chosenWeekly.pct : chosen5h.pct;
  const reset5h = chosen5h?.resetLabel || '';
  const resetWeekly = chosenWeekly?.resetLabel || '';

  const metaParts = [];
  if (reset5h) metaParts.push(`5h重置 ${reset5h}`);
  if (resetWeekly) metaParts.push(`周重置 ${resetWeekly}`);
  const meta = metaParts.length > 0 ? metaParts.join(' · ') : 'Antigravity 实时额度已同步';

  return {
    fiveHour,
    weekly,
    reset5h,
    resetWeekly,
    meta,
    source: 'antigravity-live'
  };
}

/**
 * Parse GetUserStatus / GetCommandModelConfigs response if RetrieveUserQuotaSummary is unavailable
 */
function parseGetUserStatusResponse(data) {
  if (!data || typeof data !== 'object') return null;

  // First check if GetUserStatus embedded a quota summary object
  const embeddedSummary = parseQuotaSummaryResponse(data);
  if (embeddedSummary) return embeddedSummary;

  const configs =
    data?.userStatus?.cascadeModelConfigData?.clientModelConfigs ||
    data?.cascadeModelConfigData?.clientModelConfigs ||
    data?.clientModelConfigs;
  if (!Array.isArray(configs) || configs.length === 0) return null;

  let gemini5h = null;
  let geminiWeekly = null;

  for (const m of configs) {
    const label = String(m.label || m.displayName || m.modelOrAlias?.model || '').toLowerCase();
    const isGemini = label.includes('gemini') || (!label.includes('claude') && !label.includes('gpt'));
    if (!isGemini) continue;

    // Check primary quotaInfo (5h rolling limit)
    if (m.quotaInfo) {
      const frac = extractBucketFraction(m.quotaInfo);
      if (frac !== null) {
        const pct = +(frac * 100).toFixed(1);
        const resetLabel = formatResetCountdown(m.quotaInfo.resetTime || m.quotaInfo.reset_time);
        if (!gemini5h || pct < gemini5h.pct) {
          gemini5h = { pct, resetLabel };
        }
      }
    }

    // Check weeklyQuotaInfo if present on model config
    const wInfo = m.weeklyQuotaInfo || m.weekly_quota_info || m.quotaInfo?.weeklyQuotaInfo;
    if (wInfo) {
      const wFrac = extractBucketFraction(wInfo);
      if (wFrac !== null) {
        const wPct = +(wFrac * 100).toFixed(1);
        const wReset = formatResetCountdown(wInfo.resetTime || wInfo.reset_time);
        if (!geminiWeekly || wPct < geminiWeekly.pct) {
          geminiWeekly = { pct: wPct, resetLabel: wReset };
        }
      }
    }
  }

  if (!gemini5h && !geminiWeekly) return null;

  const fiveHour = gemini5h ? gemini5h.pct : geminiWeekly.pct;
  const weekly = geminiWeekly ? geminiWeekly.pct : gemini5h.pct;
  const reset5h = gemini5h?.resetLabel || '';
  const resetWeekly = geminiWeekly?.resetLabel || '';

  const metaParts = [];
  if (reset5h) metaParts.push(`5h重置 ${reset5h}`);
  if (resetWeekly) metaParts.push(`周重置 ${resetWeekly}`);
  const meta = metaParts.length > 0 ? metaParts.join(' · ') : 'Antigravity 实时额度已同步';

  return {
    fiveHour,
    weekly,
    reset5h,
    resetWeekly,
    meta,
    source: geminiWeekly ? 'antigravity-live' : 'antigravity-user-status'
  };
}

/**
 * Probe a specific { port, scheme, csrfToken } for QuotaSummary or UserStatus
 */
async function probeEndpoint(ep) {
  const summaryData = await postConnectRpc({
    scheme: ep.scheme,
    port: ep.port,
    csrfToken: ep.csrfToken,
    rpcMethod: 'RetrieveUserQuotaSummary'
  });
  const parsedSummary = parseQuotaSummaryResponse(summaryData);
  if (parsedSummary) {
    return parsedSummary;
  }

  const statusData = await postConnectRpc({
    scheme: ep.scheme,
    port: ep.port,
    csrfToken: ep.csrfToken,
    rpcMethod: 'GetUserStatus'
  });
  const parsedStatus = parseGetUserStatusResponse(statusData);
  if (parsedStatus) {
    return parsedStatus;
  }

  const cmdConfigData = await postConnectRpc({
    scheme: ep.scheme,
    port: ep.port,
    csrfToken: ep.csrfToken,
    rpcMethod: 'GetCommandModelConfigs'
  });
  return parseGetUserStatusResponse(cmdConfigData);
}

/**
 * Main entry point: Fetch real-time Antigravity quota with smart endpoint caching
 */
async function fetchAntigravityQuota(forceRefresh = false) {
  const now = Date.now();
  if (!forceRefresh && cachedQuota && now - lastFetchTs < 6000) {
    return cachedQuota;
  }
  if (inflightPromise) {
    return inflightPromise;
  }

  inflightPromise = (async () => {
    try {
      // 1. Try cached working endpoint first for instant (<15ms) response
      if (cachedEndpoint) {
        const res = await probeEndpoint(cachedEndpoint);
        if (res) {
          cachedQuota = res;
          lastFetchTs = Date.now();
          return res;
        }
        cachedEndpoint = null;
      }

      // 2. Discover running Antigravity language_server processes and probe their ports
      const candidates = await discoverLanguageServerCandidates();
      let bestFallback = null;

      for (const cand of candidates) {
        for (const port of cand.ports) {
          for (const scheme of ['https', 'http']) {
            const ep = { pid: cand.pid, port, scheme, csrfToken: cand.csrfToken };
            const res = await probeEndpoint(ep);
            if (res) {
              if (res.source === 'antigravity-live') {
                cachedEndpoint = ep;
                cachedQuota = res;
                lastFetchTs = Date.now();
                return res;
              }
              if (!bestFallback) {
                bestFallback = { ep, res };
              }
            }
          }
        }
      }

      if (bestFallback) {
        cachedEndpoint = bestFallback.ep;
        cachedQuota = bestFallback.res;
        lastFetchTs = Date.now();
        return bestFallback.res;
      }
      return cachedQuota;
    } finally {
      inflightPromise = null;
    }
  })();

  return inflightPromise;
}

module.exports = {
  fetchAntigravityQuota
};
