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

/// Parses an ISO-8601 UTC timestamp (e.g. `2026-11-08T14:21:23Z`) into unix epoch seconds.
pub fn parse_iso8601_to_unix(s: &str) -> Option<i64> {
    let s = s.trim();
    if s.len() < 19 {
        return None;
    }
    let year: i32 = s.get(0..4)?.parse().ok()?;
    let month: u32 = s.get(5..7)?.parse().ok()?;
    let day: u32 = s.get(8..10)?.parse().ok()?;
    let hour: u32 = s.get(11..13)?.parse().ok()?;
    let min: u32 = s.get(14..16)?.parse().ok()?;
    let sec: u32 = s.get(17..19)?.parse().ok()?;

    if month < 1 || month > 12 || day < 1 || day > 31 || hour > 23 || min > 59 || sec > 59 {
        return None;
    }

    // Howard Hinnant algorithm for civil calendar days to unix epoch days
    let y = if month <= 2 { year - 1 } else { year } as i64;
    let m = if month <= 2 { month + 12 } else { month } as i64;
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (m - 3) + 2) / 5 + (day as i64) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;

    Some(days * 86400 + (hour as i64) * 3600 + (min as i64) * 60 + (sec as i64))
}

/// Formats a target ISO-8601 timestamp string into a concise reset countdown (e.g. `4h 11m`, `6d 6h`).
pub fn format_reset_countdown(reset_time_raw: &str) -> String {
    let target_unix = if let Some(sec) = parse_iso8601_to_unix(reset_time_raw) {
        sec
    } else if let Ok(num) = reset_time_raw.trim().parse::<i64>() {
        if num > 1_000_000_000_000 {
            num / 1000
        } else {
            num
        }
    } else {
        return String::new();
    };

    let now_unix = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(dur) => dur.as_secs() as i64,
        Err(_) => return String::new(),
    };

    let diff_sec = (target_unix - now_unix).max(0);
    if diff_sec <= 0 {
        return "即将重置".to_string();
    }

    let days = diff_sec / 86400;
    let hours = (diff_sec % 86400) / 3600;
    let mins = (diff_sec % 3600) / 60;

    if days > 0 {
        format!("{}d {}h", days, hours)
    } else if hours > 0 {
        format!("{}h {}m", hours, mins)
    } else {
        format!("{}m", mins)
    }
}

/// Extracts a string property value enclosed in quotes: `"key": "value"`
fn extract_string_prop(obj_str: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\"", key);
    let idx = obj_str.find(&pattern)?;
    let after_key = &obj_str[idx + pattern.len()..];
    let colon = after_key.find(':')?;
    let after_colon = after_key[colon + 1..].trim_start();
    let quote_start = after_colon.find('"')?;
    let rest = &after_colon[quote_start + 1..];
    let quote_end = rest.find('"')?;
    Some(rest[..quote_end].to_string())
}

/// Extracts a float property value: `"key": 0.85` or `"key": "0.85"`
fn extract_float_prop(obj_str: &str, key: &str) -> Option<f32> {
    let pattern = format!("\"{}\"", key);
    let idx = obj_str.find(&pattern)?;
    let after_key = &obj_str[idx + pattern.len()..];
    let colon = after_key.find(':')?;
    let after_colon = after_key[colon + 1..].trim_start();
    let num_str: String = after_colon
        .trim_start_matches('"')
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
        .collect();
    num_str.parse::<f32>().ok()
}

/// Zero-dependency parser for `RetrieveUserQuotaSummary` and `GetUserStatus` JSON responses.
/// Faithfully reproduces the logic in `server/quota-probe.js`.
pub fn parse_quota_json(json: &str) -> Option<QuotaSnapshot> {
    if !json.contains("remainingFraction") && !json.contains("remaining_fraction") {
        return None;
    }

    let mut gemini_5h: Option<(f32, String)> = None; // (pct, reset_label)
    let mut gemini_weekly: Option<(f32, String)> = None;
    let mut fallback_5h: Option<(f32, String)> = None;
    let mut fallback_weekly: Option<(f32, String)> = None;

    // Scan individual JSON bucket objects delimited by '{' and '}'
    for key in ["\"remainingFraction\"", "\"remaining_fraction\""] {
        let mut search_from = 0usize;
        while let Some(rel_pos) = json[search_from..].find(key) {
            let pos = search_from + rel_pos;
            search_from = pos + key.len();

            // Find the opening '{' before pos and closing '}' after pos
            let Some(open_brace) = json[..pos].rfind('{') else {
                continue;
            };
            let Some(close_offset) = json[pos..].find('}') else {
                continue;
            };
            let close_brace = pos + close_offset;
            let bucket_obj = &json[open_brace..=close_brace];

            let frac = extract_float_prop(bucket_obj, "remainingFraction")
                .or_else(|| extract_float_prop(bucket_obj, "remaining_fraction"))
                .or_else(|| extract_float_prop(bucket_obj, "fraction"));
            let Some(f) = frac else {
                continue;
            };
            let pct = (f.clamp(0.0, 1.0) * 1000.0).round() / 10.0;

            let bucket_id = extract_string_prop(bucket_obj, "bucketId")
                .or_else(|| extract_string_prop(bucket_obj, "bucket_id"))
                .or_else(|| extract_string_prop(bucket_obj, "id"))
                .unwrap_or_default()
                .to_lowercase();

            let window = extract_string_prop(bucket_obj, "window")
                .or_else(|| extract_string_prop(bucket_obj, "cadence"))
                .or_else(|| extract_string_prop(bucket_obj, "windowType"))
                .unwrap_or_default()
                .to_lowercase();

            let reset_time = extract_string_prop(bucket_obj, "resetTime")
                .or_else(|| extract_string_prop(bucket_obj, "reset_time"))
                .unwrap_or_default();
            let reset_label = format_reset_countdown(&reset_time);

            let is_third_party = bucket_id.contains("3p")
                || bucket_id.contains("claude")
                || bucket_id.contains("gpt");
            let is_gemini = !is_third_party
                || bucket_id.contains("gemini")
                || bucket_obj.to_lowercase().contains("gemini models");

            let is_5h = bucket_id.contains("5h")
                || bucket_id.contains("session")
                || window.contains("5h")
                || window.contains("five")
                || window.contains("session");

            let is_weekly = bucket_id.contains("week")
                || bucket_id.contains("7d")
                || window.contains("week")
                || window.contains("7d");

            if is_gemini && is_5h {
                if gemini_5h.is_none() || bucket_id == "gemini-5h" || pct < gemini_5h.as_ref().unwrap().0 {
                    gemini_5h = Some((pct, reset_label));
                }
            } else if is_gemini && is_weekly {
                if gemini_weekly.is_none() || bucket_id == "gemini-weekly" || pct < gemini_weekly.as_ref().unwrap().0 {
                    gemini_weekly = Some((pct, reset_label));
                }
            } else if is_5h && fallback_5h.is_none() {
                fallback_5h = Some((pct, reset_label));
            } else if is_weekly && fallback_weekly.is_none() {
                fallback_weekly = Some((pct, reset_label));
            }
        }
    }

    let chosen_5h = gemini_5h.or(fallback_5h);
    let chosen_weekly = gemini_weekly.or(fallback_weekly);

    if chosen_5h.is_none() && chosen_weekly.is_none() {
        return None;
    }

    let fh = chosen_5h.as_ref().map(|x| x.0).or_else(|| chosen_weekly.as_ref().map(|x| x.0))?;
    let wk = chosen_weekly.as_ref().map(|x| x.0).unwrap_or(fh);

    let reset_5h = chosen_5h.as_ref().map(|x| x.1.as_str()).unwrap_or("");
    let reset_wk = chosen_weekly.as_ref().map(|x| x.1.as_str()).unwrap_or("");

    let meta = match (!reset_5h.is_empty(), !reset_wk.is_empty()) {
        (true, true) => format!("5h重置 {} · 周重置 {}", reset_5h, reset_wk),
        (true, false) => format!("5h重置 {}", reset_5h),
        (false, true) => format!("周重置 {}", reset_wk),
        (false, false) => "Antigravity 实时额度已同步".to_string(),
    };

    Some(QuotaSnapshot {
        five_hour: fh,
        weekly: wk,
        meta,
        source: "antigravity-live".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_iso8601() {
        assert_eq!(parse_iso8601_to_unix("1970-01-01T00:00:00Z"), Some(0));
        assert!(parse_iso8601_to_unix("2026-11-08T14:21:23Z").is_some());
    }

    #[test]
    fn test_parse_real_antigravity_quota_json() {
        let json = r#"{"response":{"groups":[{"displayName":"Gemini Models", "description":"Models within this group: Gemini Flash, Gemini Pro", "buckets":[{"bucketId":"gemini-weekly", "displayName":"Weekly Limit Remaining", "description":"You have used some of your weekly limit, it will fully refresh in 6 days, 6 hours.", "window":"weekly", "remainingFraction":0.80794483, "resetTime":"2026-11-08T14:21:23Z"}, {"bucketId":"gemini-5h", "displayName":"Five Hour Limit Remaining", "description":"You have used some of your 5-hour limit, it will fully refresh in 4 hours, 12 minutes.", "window":"5h", "remainingFraction":0.9409432, "resetTime":"2026-11-02T11:55:01Z"}]}, {"displayName":"Claude and GPT models", "description":"Models within this group: Claude Opus, Claude Sonnet, GPT-OSS", "buckets":[{"bucketId":"3p-weekly", "displayName":"Weekly Limit Remaining", "description":"You have used some of your weekly limit, it will fully refresh in 6 days, 17 hours.", "window":"weekly", "remainingFraction":0.5344932, "resetTime":"2026-11-09T01:28:46Z"}, {"bucketId":"3p-5h", "displayName":"Five Hour Limit Remaining", "window":"5h", "remainingFraction":1, "resetTime":"2026-11-02T12:42:05Z"}]}]}}"#;
        let snap = parse_quota_json(json).expect("should parse real response");
        assert_eq!(snap.five_hour, 94.1);
        assert_eq!(snap.weekly, 80.8);
        assert!(snap.meta.contains("5h重置"));
        assert!(snap.meta.contains("周重置"));
    }
}
