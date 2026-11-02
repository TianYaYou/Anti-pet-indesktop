use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::process::Command;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct QuotaSnapshot {
    pub five_hour: f32,
    pub weekly: f32,
    pub meta: String,
    pub source: String,
}

impl Default for QuotaSnapshot {
    fn default() -> Self {
        Self {
            five_hour: 100.0,
            weekly: 100.0,
            meta: "Antigravity 实时额度已同步".to_string(),
            source: "default".to_string(),
        }
    }
}

#[derive(Clone, Debug)]
struct EndpointCandidate {
    pid: u32,
    csrf_token: String,
    ports: Vec<u16>,
    priority: u8,
}

/// Native Rust Antigravity `language_server` Quota Prober (replaces `server/quota-probe.js`).
/// Queries `RetrieveUserQuotaSummary` and `GetUserStatus` directly from the local Antigravity process.
pub struct NativeQuotaProber {
    cached_endpoint: Option<(u16, bool, String)>, // (port, is_https, csrf_token)
    last_snapshot: QuotaSnapshot,
}

impl Default for NativeQuotaProber {
    fn default() -> Self {
        Self {
            cached_endpoint: None,
            last_snapshot: QuotaSnapshot::default(),
        }
    }
}

impl NativeQuotaProber {
    pub fn fetch_live_quota(&mut self) -> QuotaSnapshot {
        // 1. Try cached endpoint first for fast (< 15ms) polling
        if let Some((port, is_https, ref token)) = self.cached_endpoint.clone() {
            if let Some(snap) = probe_single_port(port, is_https, token) {
                self.last_snapshot = snap.clone();
                return snap;
            }
            self.cached_endpoint = None;
        }

        // 2. Discover running Antigravity language_server processes and their listening ports
        let candidates = discover_language_servers();
        let mut fallback_hit: Option<((u16, bool, String), QuotaSnapshot)> = None;

        for cand in candidates {
            for port in cand.ports {
                for &is_https in &[true, false] {
                    if let Some(snap) = probe_single_port(port, is_https, &cand.csrf_token) {
                        if snap.source == "antigravity-live" {
                            self.cached_endpoint = Some((port, is_https, cand.csrf_token.clone()));
                            self.last_snapshot = snap.clone();
                            return snap;
                        }
                        if fallback_hit.is_none() {
                            fallback_hit =
                                Some(((port, is_https, cand.csrf_token.clone()), snap));
                        }
                    }
                }
            }
        }

        if let Some((ep, snap)) = fallback_hit {
            self.cached_endpoint = Some(ep);
            self.last_snapshot = snap.clone();
            return snap;
        }

        self.last_snapshot.clone()
    }
}

fn discover_language_servers() -> Vec<EndpointCandidate> {
    let mut out_list = Vec::new();

    #[cfg(not(target_os = "windows"))]
    {
        let ps_out = Command::new("/bin/ps")
            .args(["-ax", "-o", "pid=,command="])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .unwrap_or_default();

        for raw_line in ps_out.lines() {
            let line = raw_line.trim();
            if line.is_empty() {
                continue;
            }
            let lower = line.to_lowercase();
            if !lower.contains("language_server")
                && !lower.contains("language-server")
                && !lower.contains("antigravity")
            {
                continue;
            }
            if lower.contains("grep") || lower.contains("anti-pet") {
                continue;
            }

            let mut parts = line.splitn(2, char::is_whitespace);
            let Some(pid_str) = parts.next() else {
                continue;
            };
            let Ok(pid) = pid_str.trim().parse::<u32>() else {
                continue;
            };
            let cmd = parts.next().unwrap_or("").trim();

            let csrf_token = extract_flag_value(cmd, "--csrf_token")
                .or_else(|| extract_flag_value(cmd, "--extension_server_csrf_token"))
                .unwrap_or_default();
            let ext_port = extract_flag_value(cmd, "--extension_server_port")
                .and_then(|v| v.parse::<u16>().ok());

            let priority = if lower.contains("antigravity.app")
                || lower.contains("--app_data_dir antigravity")
            {
                1
            } else if lower.contains("antigravity") {
                2
            } else {
                3
            };

            let mut ports = Vec::new();
            if let Some(ep) = ext_port {
                ports.push(ep);
                ports.push(ep.saturating_add(1));
                ports.push(ep.saturating_add(2));
            }

            if let Ok(lsof) = Command::new("/usr/sbin/lsof")
                .args([
                    "-nP",
                    "-iTCP",
                    "-sTCP:LISTEN",
                    "-a",
                    "-p",
                    &pid.to_string(),
                ])
                .output()
            {
                let text = String::from_utf8_lossy(&lsof.stdout);
                for l in text.lines() {
                    if l.contains("(LISTEN)") {
                        if let Some(colon_idx) = l.rfind(':') {
                            let after = &l[colon_idx + 1..];
                            if let Some(port_tok) = after.split_whitespace().next() {
                                if let Ok(p) = port_tok.parse::<u16>() {
                                    if !ports.contains(&p) {
                                        ports.push(p);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if !ports.is_empty() {
                out_list.push(EndpointCandidate {
                    pid,
                    csrf_token,
                    ports,
                    priority,
                });
            }
        }
    }

    out_list.sort_by_key(|c| c.priority);
    out_list
}

fn extract_flag_value(cmd: &str, flag: &str) -> Option<String> {
    let idx = cmd.find(flag)?;
    let rest = cmd[idx + flag.len()..].trim_start();
    let rest = rest.strip_prefix('=').unwrap_or(rest).trim_start();
    let val: String = rest
        .chars()
        .take_while(|c| !c.is_whitespace() && *c != '"' && *c != '\'')
        .collect();
    if val.is_empty() {
        None
    } else {
        Some(val)
    }
}

fn probe_single_port(port: u16, is_https: bool, csrf_token: &str) -> Option<QuotaSnapshot> {
    for method in [
        "RetrieveUserQuotaSummary",
        "GetUserStatus",
        "GetCommandModelConfigs",
    ] {
        if let Some(body) = call_connect_rpc(port, is_https, csrf_token, method) {
            if let Some(snap) = parse_quota_json(&body) {
                return Some(snap);
            }
        }
    }
    None
}

fn call_connect_rpc(port: u16, is_https: bool, csrf_token: &str, rpc_method: &str) -> Option<String> {
    let path = format!(
        "/exa.language_server_pb.LanguageServerService/{}",
        rpc_method
    );
    let payload = r#"{"metadata":{"ideName":"antigravity","extensionName":"antigravity","locale":"en","ideVersion":"unknown"}}"#;

    if !is_https {
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let mut stream = TcpStream::connect_timeout(&addr, Duration::from_millis(600)).ok()?;
        let _ = stream.set_read_timeout(Some(Duration::from_millis(900)));
        let _ = stream.set_write_timeout(Some(Duration::from_millis(600)));

        let mut req = format!(
            "POST {} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nContent-Type: application/json\r\nConnect-Protocol-Version: 1\r\nContent-Length: {}\r\nConnection: close\r\n",
            path,
            port,
            payload.len()
        );
        if !csrf_token.is_empty() {
            req.push_str(&format!("X-Codeium-Csrf-Token: {}\r\n", csrf_token));
        }
        req.push_str("\r\n");
        req.push_str(payload);

        stream.write_all(req.as_bytes()).ok()?;
        let mut resp = String::new();
        stream.read_to_string(&mut resp).ok()?;
        if !resp.starts_with("HTTP/1.1 200") && !resp.starts_with("HTTP/1.0 200") {
            return None;
        }
        let (_, body) = resp.split_once("\r\n\r\n")?;
        return Some(body.trim().to_string());
    }

    // For local self-signed HTTPS language_server endpoints, invoke system curl with 1s timeout
    let url = format!("https://127.0.0.1:{}{}", port, path);
    let mut cmd = Command::new("curl");
    cmd.args([
        "-sk",
        "--max-time",
        "1",
        "-X",
        "POST",
        "-H",
        "Content-Type: application/json",
        "-H",
        "Connect-Protocol-Version: 1",
    ]);
    if !csrf_token.is_empty() {
        cmd.arg("-H")
            .arg(format!("X-Codeium-Csrf-Token: {}", csrf_token));
    }
    cmd.arg("-d").arg(payload).arg(&url);

    let out = cmd.output().ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8(out.stdout).ok()?;
    if text.trim().starts_with('{') {
        Some(text)
    } else {
        None
    }
}

/// Zero-dependency parser for `RetrieveUserQuotaSummary` and `GetUserStatus` JSON responses.
pub fn parse_quota_json(json: &str) -> Option<QuotaSnapshot> {
    if !json.contains("remainingFraction") && !json.contains("remaining_fraction") {
        return None;
    }

    let mut five_hour: Option<f32> = None;
    let mut weekly: Option<f32> = None;

    // Scan JSON object segments around each `remainingFraction` / `remaining_fraction`
    for key in ["\"remainingFraction\"", "\"remaining_fraction\""] {
        let mut search_from = 0usize;
        while let Some(rel_pos) = json[search_from..].find(key) {
            let pos = search_from + rel_pos;
            search_from = pos + key.len();

            let after_key = &json[search_from..];
            let Some(colon) = after_key.find(':') else {
                continue;
            };
            let val_part = after_key[colon + 1..].trim_start();
            let num_str: String = val_part
                .trim_start_matches('"')
                .chars()
                .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
                .collect();
            let Ok(frac) = num_str.parse::<f32>() else {
                continue;
            };
            let pct = (frac.clamp(0.0, 1.0) * 1000.0).round() / 10.0;

            let ctx_start = pos.saturating_sub(260);
            let ctx_end = (pos + 260).min(json.len());
            let context = json[ctx_start..ctx_end].to_lowercase();

            if context.contains("claude") || context.contains("gpt") || context.contains("3p") {
                continue;
            }

            if context.contains("week") || context.contains("7d") {
                weekly = Some(weekly.map_or(pct, |prev| prev.min(pct)));
            } else if context.contains("5h")
                || context.contains("five")
                || context.contains("session")
                || context.contains("gemini")
            {
                five_hour = Some(five_hour.map_or(pct, |prev| prev.min(pct)));
            } else if five_hour.is_none() {
                five_hour = Some(pct);
            } else if weekly.is_none() {
                weekly = Some(pct);
            }
        }
    }

    let fh = five_hour.or(weekly)?;
    let wk = weekly.unwrap_or(fh);

    Some(QuotaSnapshot {
        five_hour: fh,
        weekly: wk,
        meta: "Antigravity 实时额度已同步".to_string(),
        source: if weekly.is_some() {
            "antigravity-live".to_string()
        } else {
            "antigravity-user-status".to_string()
        },
    })
}
