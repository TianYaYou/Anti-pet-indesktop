use crate::physics::PetMotionState;
use crate::quota::QuotaSnapshot;
use std::path::PathBuf;
use std::time::SystemTime;

/// Persistent user settings loaded/saved directly by the Rust desktop binary (`.antigravity-pet-settings.json`).
#[derive(Clone, Debug)]
pub struct PetSettings {
    pub scale: f32,
    pub sound_enabled: bool,
    pub gravity_enabled: bool,
    pub floor_mode: String,
    pub dnd_passthrough: bool,
    pub show_quota: bool,
    pub pos_x: Option<f64>,
    pub pos_y: Option<f64>,
}

impl Default for PetSettings {
    fn default() -> Self {
        Self {
            scale: 1.0,
            sound_enabled: true,
            gravity_enabled: true,
            floor_mode: "bottom".to_string(),
            dnd_passthrough: false,
            show_quota: true,
            pos_x: None,
            pos_y: None,
        }
    }
}

/// Event update parsed from `.antigravity-pet-state.json` (written by Antigravity IDE hooks).
#[derive(Clone, Debug)]
pub struct HookStateUpdate {
    pub motion: PetMotionState,
    pub bubble_text: Option<String>,
    pub duration_ms: u64,
    pub embedded_quota: Option<QuotaSnapshot>,
}

pub struct NativeStateWatcher {
    root_dir: PathBuf,
    state_file: PathBuf,
    settings_file: PathBuf,
    last_state_mtime: Option<SystemTime>,
}

impl NativeStateWatcher {
    pub fn new() -> Self {
        let root_dir = resolve_project_root();
        let state_file = root_dir.join(".antigravity-pet-state.json");
        let settings_file = root_dir.join(".antigravity-pet-settings.json");
        Self {
            root_dir,
            state_file,
            settings_file,
            last_state_mtime: None,
        }
    }

    pub fn root_dir(&self) -> &PathBuf {
        &self.root_dir
    }

    pub fn load_settings(&self) -> PetSettings {
        let mut settings = PetSettings::default();
        let Ok(raw) = std::fs::read_to_string(&self.settings_file) else {
            return settings;
        };

        if let Some(v) = extract_json_f64(&raw, "scale") {
            settings.scale = (v as f32).clamp(0.6, 1.8);
        }
        if let Some(v) = extract_json_bool(&raw, "soundEnabled") {
            settings.sound_enabled = v;
        }
        if let Some(v) = extract_json_bool(&raw, "gravityEnabled") {
            settings.gravity_enabled = v;
        }
        if let Some(v) = extract_json_str(&raw, "floorMode") {
            settings.floor_mode = v;
        }
        if let Some(v) = extract_json_bool(&raw, "dndPassthrough") {
            settings.dnd_passthrough = v;
        }
        if let Some(v) = extract_json_bool(&raw, "showQuota") {
            settings.show_quota = v;
        }
        settings.pos_x = extract_json_f64(&raw, "posX");
        settings.pos_y = extract_json_f64(&raw, "posY");
        settings
    }

    pub fn save_settings(&self, s: &PetSettings) {
        let pos_x_str = s
            .pos_x
            .map(|v| format!("{:.1}", v))
            .unwrap_or_else(|| "null".to_string());
        let pos_y_str = s
            .pos_y
            .map(|v| format!("{:.1}", v))
            .unwrap_or_else(|| "null".to_string());

        let json = format!(
            "{{\n  \"scale\": {:.2},\n  \"skinId\": \"fat-cat\",\n  \"soundEnabled\": {},\n  \"gravityEnabled\": {},\n  \"floorMode\": \"{}\",\n  \"dndPassthrough\": {},\n  \"showQuota\": {},\n  \"posX\": {},\n  \"posY\": {}\n}}\n",
            s.scale,
            s.sound_enabled,
            s.gravity_enabled,
            s.floor_mode,
            s.dnd_passthrough,
            s.show_quota,
            pos_x_str,
            pos_y_str
        );
        let _ = std::fs::write(&self.settings_file, json);
    }

    /// Checks if `.antigravity-pet-state.json` was modified by an Antigravity IDE hook (`PreInvocation`, `PreToolUse`, `Stop`, etc.).
    pub fn poll_state_change(&mut self) -> Option<HookStateUpdate> {
        let meta = std::fs::metadata(&self.state_file).ok()?;
        let mtime = meta.modified().ok()?;
        if self.last_state_mtime == Some(mtime) {
            return None;
        }
        self.last_state_mtime = Some(mtime);

        let raw = std::fs::read_to_string(&self.state_file).ok()?;
        let state_str = extract_json_str(&raw, "state").unwrap_or_else(|| "idle".to_string());
        let bubble_text = extract_json_str(&raw, "bubbleText");
        let duration_ms = extract_json_f64(&raw, "durationMs")
            .map(|v| v.max(0.0) as u64)
            .unwrap_or(3200);

        let motion = match state_str.as_str() {
            "thinking" | "ponder" => PetMotionState::Thinking,
            "working" | "tool" => PetMotionState::Working,
            "success" | "proud" | "complete" => PetMotionState::Success,
            "error" | "crying" => PetMotionState::Error,
            _ => PetMotionState::Idle,
        };

        let embedded_quota = if let (Some(fh), Some(wk)) = (
            extract_json_f64(&raw, "fiveHour"),
            extract_json_f64(&raw, "weekly"),
        ) {
            Some(QuotaSnapshot {
                five_hour: fh as f32,
                weekly: wk as f32,
                meta: extract_json_str(&raw, "meta")
                    .unwrap_or_else(|| "Antigravity 实时额度已同步".to_string()),
                source: "state-file".to_string(),
            })
        } else {
            None
        };

        Some(HookStateUpdate {
            motion,
            bubble_text,
            duration_ms,
            embedded_quota,
        })
    }
}

fn resolve_project_root() -> PathBuf {
    if let Ok(cwd) = std::env::current_dir() {
        if cwd.join("skins").exists() {
            return cwd;
        }
        if let Some(parent) = cwd.parent() {
            if parent.join("skins").exists() {
                return parent.to_path_buf();
            }
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        for ancestor in exe.ancestors().skip(1) {
            if ancestor.join("skins").exists() {
                return ancestor.to_path_buf();
            }
            let res = ancestor.join("Resources");
            if res.join("skins").exists() {
                return res;
            }
        }
    }
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

fn extract_json_str(json: &str, key: &str) -> Option<String> {
    let pat = format!("\"{}\"", key);
    let idx = json.find(&pat)?;
    let rest = &json[idx + pat.len()..];
    let colon = rest.find(':')?;
    let after = rest[colon + 1..].trim_start();
    if !after.starts_with('"') {
        return None;
    }
    let mut out = String::new();
    let mut escaped = false;
    for ch in after[1..].chars() {
        if escaped {
            out.push(ch);
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            return Some(out);
        } else {
            out.push(ch);
        }
    }
    None
}

fn extract_json_f64(json: &str, key: &str) -> Option<f64> {
    let pat = format!("\"{}\"", key);
    let idx = json.find(&pat)?;
    let rest = &json[idx + pat.len()..];
    let colon = rest.find(':')?;
    let after = rest[colon + 1..].trim_start();
    if after.starts_with("null") {
        return None;
    }
    let num_str: String = after
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
        .collect();
    num_str.parse::<f64>().ok()
}

fn extract_json_bool(json: &str, key: &str) -> Option<bool> {
    let pat = format!("\"{}\"", key);
    let idx = json.find(&pat)?;
    let rest = &json[idx + pat.len()..];
    let colon = rest.find(':')?;
    let after = rest[colon + 1..].trim_start();
    if after.starts_with("true") {
        Some(true)
    } else if after.starts_with("false") {
        Some(false)
    } else {
        None
    }
}
