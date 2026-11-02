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
    last_settings_mtime: Option<SystemTime>,
}

impl NativeStateWatcher {
    pub fn new() -> Self {
        let root_dir = resolve_project_root();
        let state_file = find_file(&root_dir, ".antigravity-pet-state.json");
        let settings_file = find_file(&root_dir, ".antigravity-pet-settings.json");
        Self {
            root_dir,
            state_file,
            settings_file,
            last_state_mtime: None,
            last_settings_mtime: None,
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
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) else {
            return settings;
        };

        if let Some(s) = v.get("scale").and_then(|x| x.as_f64()) {
            settings.scale = (s as f32).clamp(0.5, 2.0);
        }
        if let Some(s) = v.get("soundEnabled").and_then(|x| x.as_bool()) {
            settings.sound_enabled = s;
        }
        if let Some(g) = v.get("gravityEnabled").and_then(|x| x.as_bool()) {
            settings.gravity_enabled = g;
        }
        if let Some(f) = v.get("floorMode").and_then(|x| x.as_str()) {
            settings.floor_mode = f.to_string();
        }
        if let Some(d) = v.get("dndPassthrough").and_then(|x| x.as_bool()) {
            settings.dnd_passthrough = d;
        }
        if let Some(q) = v.get("showQuota").and_then(|x| x.as_bool()) {
            settings.show_quota = q;
        }
        settings.pos_x = v.get("posX").and_then(|x| x.as_f64());
        settings.pos_y = v.get("posY").and_then(|x| x.as_f64());
        settings
    }

    pub fn poll_settings_change(&mut self) -> Option<PetSettings> {
        let meta = std::fs::metadata(&self.settings_file).ok()?;
        let mtime = meta.modified().ok()?;
        if self.last_settings_mtime == Some(mtime) {
            return None;
        }
        self.last_settings_mtime = Some(mtime);
        Some(self.load_settings())
    }

    pub fn save_settings(&self, s: &PetSettings) {
        let pos_x_val = match s.pos_x {
            Some(v) => serde_json::Value::from(v),
            None => serde_json::Value::Null,
        };
        let pos_y_val = match s.pos_y {
            Some(v) => serde_json::Value::from(v),
            None => serde_json::Value::Null,
        };

        let mut map = serde_json::Map::new();
        map.insert("scale".to_string(), serde_json::json!((s.scale * 100.0).round() / 100.0));
        map.insert("skinId".to_string(), serde_json::json!("fat-cat"));
        map.insert("soundEnabled".to_string(), serde_json::json!(s.sound_enabled));
        map.insert("gravityEnabled".to_string(), serde_json::json!(s.gravity_enabled));
        map.insert("floorMode".to_string(), serde_json::json!(s.floor_mode));
        map.insert("dndPassthrough".to_string(), serde_json::json!(s.dnd_passthrough));
        map.insert("showQuota".to_string(), serde_json::json!(s.show_quota));
        map.insert("posX".to_string(), pos_x_val);
        map.insert("posY".to_string(), pos_y_val);

        if let Ok(json_str) = serde_json::to_string_pretty(&map) {
            let _ = std::fs::write(&self.settings_file, &json_str);
            if let Some(home) = dirs::home_dir() {
                let home_settings = home.join(".antigravity-pet-settings.json");
                if home_settings != self.settings_file {
                    let _ = std::fs::write(home_settings, &json_str);
                }
            }
        }
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
        let v: serde_json::Value = serde_json::from_str(&raw).ok()?;

        let state_str = v.get("state").and_then(|x| x.as_str()).unwrap_or("idle");
        let bubble_text = v.get("bubbleText").and_then(|x| x.as_str()).map(|s| s.to_string());
        let duration_ms = v
            .get("durationMs")
            .and_then(|x| x.as_u64())
            .unwrap_or(3200);

        let motion = match state_str {
            "thinking" | "ponder" => PetMotionState::Thinking,
            "working" | "tool" => PetMotionState::Working,
            "success" | "proud" | "complete" => PetMotionState::Success,
            "error" | "crying" => PetMotionState::Error,
            _ => PetMotionState::Idle,
        };

        let embedded_quota = if let Some(q) = v.get("quota") {
            let fh = q.get("fiveHour").and_then(|x| x.as_f64()).unwrap_or(100.0) as f32;
            let wk = q.get("weekly").and_then(|x| x.as_f64()).unwrap_or(100.0) as f32;
            let meta = q
                .get("meta")
                .and_then(|x| x.as_str())
                .unwrap_or("Antigravity 实时额度已同步")
                .to_string();
            Some(QuotaSnapshot {
                five_hour: fh,
                weekly: wk,
                meta,
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

fn find_file(root_dir: &PathBuf, filename: &str) -> PathBuf {
    let project_p = root_dir.join(filename);
    if project_p.exists() {
        return project_p;
    }
    if let Some(home) = dirs::home_dir() {
        let home_p = home.join(filename);
        if home_p.exists() {
            return home_p;
        }
    }
    project_p
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
