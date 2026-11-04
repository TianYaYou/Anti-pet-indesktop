use crate::menu_audio::DialogueBank;
use crate::state_watcher::PetSettings;
use std::path::PathBuf;

/// Settings navigation tabs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsTab {
    General,
    Autostart,
    Skin,
    Dialogue,
    Menu,
}

/// Domain-level actions triggered by any frontend adapter (AppKit, Slint, or legacy).
#[derive(Clone, Debug, PartialEq)]
pub enum SettingsAction {
    SelectTab(SettingsTab),
    SetScale(f32),
    StepScale(f32),
    SetTheme(String),
    CycleTheme,
    SetFloorMode(String),
    CycleFloorMode,
    ToggleGravity,
    ToggleShowQuota,
    ToggleIdeOnly,
    ToggleDndPassthrough,
    ToggleSound,
    ToggleAutostart,
    ToggleMenuOption(usize),
    ResetPosition,
    SelectDialogueCategory(usize),
    AddDialogue(String),
    UpdateDialogue(usize, String),
    DeleteDialogue(usize),
    Save,
    Cancel,
    CheckUpdate,
}

/// Side-effects returned by the store to the windowing/platform host.
#[derive(Clone, Debug, PartialEq)]
pub enum StoreEvent {
    None,
    Redraw,
    CloseWindow,
    Toast(String),
    SettingsSaved(PetSettings),
    ResetPositionRequested,
    CheckUpdateRequested,
}

/// Immutable snapshot consumed by UI views to render controls.
#[derive(Clone, Debug, PartialEq)]
pub struct SettingsSnapshot {
    pub temp_settings: PetSettings,
    pub original_settings: PetSettings,
    pub autostart_enabled: bool,
    pub active_tab: SettingsTab,
    pub dialogue_bank: DialogueBank,
    pub dialogue_cat: usize,
    pub is_dirty: bool,
}

/// Headless domain state machine and persistence store for Anti-Pet settings.
pub struct SettingsStore {
    pub root_dir: PathBuf,
    pub temp_settings: PetSettings,
    pub original_settings: PetSettings,
    pub autostart_enabled: bool,
    pub active_tab: SettingsTab,
    pub dialogue_bank: DialogueBank,
    pub original_dialogue_bank: DialogueBank,
    pub dialogue_cat: usize,
}

impl SettingsStore {
    pub fn new(root_dir: PathBuf, settings: PetSettings) -> Self {
        let dialogue_bank = DialogueBank::load_from_root(&root_dir, "fat-cat");
        Self {
            root_dir,
            temp_settings: settings.clone(),
            original_settings: settings,
            autostart_enabled: false,
            active_tab: SettingsTab::General,
            dialogue_bank: dialogue_bank.clone(),
            original_dialogue_bank: dialogue_bank,
            dialogue_cat: 0,
        }
    }

    /// Prepares store state when opening the settings window.
    pub fn open(&mut self, current_settings: &PetSettings) {
        self.temp_settings = current_settings.clone();
        self.original_settings = current_settings.clone();
        self.reload_dialogues();
    }

    pub fn commit_to_disk(&mut self) {
        let watcher = crate::state_watcher::NativeStateWatcher::new();
        watcher.save_settings(&self.temp_settings);
        let _ = self.dialogue_bank.save_to_root(&self.root_dir, "fat-cat");
        self.original_settings = self.temp_settings.clone();
        self.original_dialogue_bank = self.dialogue_bank.clone();
    }

    pub fn reload_dialogues(&mut self) {
        let bank = DialogueBank::load_from_root(&self.root_dir, "fat-cat");
        self.dialogue_bank = bank.clone();
        self.original_dialogue_bank = bank;
    }

    pub fn is_dirty(&self) -> bool {
        self.temp_settings != self.original_settings
            || self.dialogue_bank != self.original_dialogue_bank
    }

    pub fn snapshot(&self) -> SettingsSnapshot {
        SettingsSnapshot {
            temp_settings: self.temp_settings.clone(),
            original_settings: self.original_settings.clone(),
            autostart_enabled: self.autostart_enabled,
            active_tab: self.active_tab,
            dialogue_bank: self.dialogue_bank.clone(),
            dialogue_cat: self.dialogue_cat,
            is_dirty: self.is_dirty(),
        }
    }

    pub fn active_dialogues(&self) -> &Vec<String> {
        match self.dialogue_cat {
            1 => &self.dialogue_bank.petting,
            2 => &self.dialogue_bank.thinking,
            3 => &self.dialogue_bank.success,
            4 => &self.dialogue_bank.error,
            _ => &self.dialogue_bank.idle,
        }
    }

    pub fn active_dialogues_mut(&mut self) -> &mut Vec<String> {
        match self.dialogue_cat {
            1 => &mut self.dialogue_bank.petting,
            2 => &mut self.dialogue_bank.thinking,
            3 => &mut self.dialogue_bank.success,
            4 => &mut self.dialogue_bank.error,
            _ => &mut self.dialogue_bank.idle,
        }
    }

    /// Dispatches a domain action, validates state mutations, and outputs platform events.
    pub fn dispatch(&mut self, action: SettingsAction) -> StoreEvent {
        match action {
            SettingsAction::SelectTab(tab) => {
                self.active_tab = tab;
                StoreEvent::Redraw
            }
            SettingsAction::SetScale(val) => {
                let clamped = val.clamp(0.5, 2.0);
                let rounded = (clamped * 20.0).round() / 20.0;
                self.temp_settings.scale = rounded;
                StoreEvent::Redraw
            }
            SettingsAction::StepScale(delta) => {
                let new_scale = self.temp_settings.scale + delta;
                let clamped = new_scale.clamp(0.5, 2.0);
                let rounded = (clamped * 20.0).round() / 20.0;
                self.temp_settings.scale = rounded;
                StoreEvent::Redraw
            }
            SettingsAction::SetTheme(theme) => {
                self.temp_settings.theme = theme;
                StoreEvent::Redraw
            }
            SettingsAction::CycleTheme => {
                self.temp_settings.theme = match self.temp_settings.theme.as_str() {
                    "dark" => "light".to_string(),
                    "light" => "system".to_string(),
                    _ => "dark".to_string(),
                };
                StoreEvent::Redraw
            }
            SettingsAction::SetFloorMode(mode) => {
                self.temp_settings.floor_mode = mode;
                StoreEvent::Redraw
            }
            SettingsAction::CycleFloorMode => {
                self.temp_settings.floor_mode = if self.temp_settings.floor_mode == "taskbar" {
                    "bottom".to_string()
                } else {
                    "taskbar".to_string()
                };
                StoreEvent::Redraw
            }
            SettingsAction::ToggleGravity => {
                self.temp_settings.gravity_enabled = !self.temp_settings.gravity_enabled;
                StoreEvent::Redraw
            }
            SettingsAction::ToggleShowQuota => {
                self.temp_settings.show_quota = !self.temp_settings.show_quota;
                StoreEvent::Redraw
            }
            SettingsAction::ToggleIdeOnly => {
                self.temp_settings.menu_config.ide_only = !self.temp_settings.menu_config.ide_only;
                StoreEvent::Redraw
            }
            SettingsAction::ToggleDndPassthrough => {
                self.temp_settings.dnd_passthrough = !self.temp_settings.dnd_passthrough;
                StoreEvent::Redraw
            }
            SettingsAction::ToggleSound => {
                self.temp_settings.sound_enabled = !self.temp_settings.sound_enabled;
                StoreEvent::Redraw
            }
            SettingsAction::ToggleAutostart => {
                self.autostart_enabled = !self.autostart_enabled;
                let msg = if self.autostart_enabled {
                    "已开启开机自动启动"
                } else {
                    "已关闭开机自动启动"
                };
                StoreEvent::Toast(msg.to_string())
            }
            SettingsAction::ToggleMenuOption(idx) => {
                match idx {
                    0 => self.temp_settings.menu_config.open_antigravity = !self.temp_settings.menu_config.open_antigravity,
                    1 => self.temp_settings.menu_config.settings = !self.temp_settings.menu_config.settings,
                    2 => self.temp_settings.menu_config.random_dialogue = !self.temp_settings.menu_config.random_dialogue,
                    3 => self.temp_settings.menu_config.pet_action = !self.temp_settings.menu_config.pet_action,
                    4 => self.temp_settings.menu_config.quota = !self.temp_settings.menu_config.quota,
                    5 => self.temp_settings.menu_config.dnd = !self.temp_settings.menu_config.dnd,
                    6 => self.temp_settings.menu_config.sound = !self.temp_settings.menu_config.sound,
                    7 => self.temp_settings.menu_config.zoom = !self.temp_settings.menu_config.zoom,
                    8 => self.temp_settings.menu_config.gravity = !self.temp_settings.menu_config.gravity,
                    _ => {}
                }
                StoreEvent::Redraw
            }
            SettingsAction::ResetPosition => {
                self.temp_settings.pos_x = None;
                self.temp_settings.pos_y = None;
                let seq = self.temp_settings.reset_seq.unwrap_or(0) + 1;
                self.temp_settings.reset_seq = Some(seq);
                StoreEvent::ResetPositionRequested
            }
            SettingsAction::SelectDialogueCategory(idx) => {
                self.dialogue_cat = idx.min(4);
                StoreEvent::Redraw
            }
            SettingsAction::AddDialogue(text) => {
                let trimmed = text.trim().to_string();
                if !trimmed.is_empty() {
                    let list = self.active_dialogues_mut();
                    list.push(trimmed);
                }
                StoreEvent::Redraw
            }
            SettingsAction::UpdateDialogue(idx, text) => {
                let trimmed = text.trim().to_string();
                let list = self.active_dialogues_mut();
                if idx < list.len() {
                    if trimmed.is_empty() {
                        list.remove(idx);
                    } else {
                        list[idx] = trimmed;
                    }
                }
                StoreEvent::Redraw
            }
            SettingsAction::DeleteDialogue(idx) => {
                let list = self.active_dialogues_mut();
                if idx < list.len() {
                    list.remove(idx);
                }
                StoreEvent::Redraw
            }
            SettingsAction::Save => {
                self.commit_to_disk();
                StoreEvent::SettingsSaved(self.original_settings.clone())
            }
            SettingsAction::Cancel => {
                self.temp_settings = self.original_settings.clone();
                self.dialogue_bank = self.original_dialogue_bank.clone();
                StoreEvent::CloseWindow
            }
            SettingsAction::CheckUpdate => {
                StoreEvent::CheckUpdateRequested
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_scale_clamping_and_rounding() {
        let mut store = SettingsStore::new(PathBuf::from("."), PetSettings::default());

        store.dispatch(SettingsAction::SetScale(3.5));
        assert_eq!(store.temp_settings.scale, 2.0);

        store.dispatch(SettingsAction::SetScale(0.1));
        assert_eq!(store.temp_settings.scale, 0.5);

        store.dispatch(SettingsAction::SetScale(1.3333));
        assert_eq!(store.temp_settings.scale, 1.35);

        store.dispatch(SettingsAction::StepScale(0.1));
        assert_eq!(store.temp_settings.scale, 1.45);
    }

    #[test]
    fn test_store_theme_cycling() {
        let mut store = SettingsStore::new(PathBuf::from("."), PetSettings::default());
        assert_eq!(store.temp_settings.theme, "system");

        store.dispatch(SettingsAction::CycleTheme);
        assert_eq!(store.temp_settings.theme, "dark");

        store.dispatch(SettingsAction::CycleTheme);
        assert_eq!(store.temp_settings.theme, "light");

        store.dispatch(SettingsAction::CycleTheme);
        assert_eq!(store.temp_settings.theme, "system");
    }

    #[test]
    fn test_store_floor_mode_cycling() {
        let mut store = SettingsStore::new(PathBuf::from("."), PetSettings::default());
        assert_eq!(store.temp_settings.floor_mode, "bottom");

        store.dispatch(SettingsAction::CycleFloorMode);
        assert_eq!(store.temp_settings.floor_mode, "taskbar");

        store.dispatch(SettingsAction::CycleFloorMode);
        assert_eq!(store.temp_settings.floor_mode, "bottom");
    }

    #[test]
    fn test_store_dirty_tracking_and_cancel_revert() {
        let mut store = SettingsStore::new(PathBuf::from("."), PetSettings::default());
        assert!(!store.is_dirty());

        store.dispatch(SettingsAction::ToggleSound);
        assert!(store.is_dirty());

        let ev = store.dispatch(SettingsAction::Cancel);
        assert_eq!(ev, StoreEvent::CloseWindow);
        assert!(!store.is_dirty());
        assert_eq!(store.temp_settings.sound_enabled, store.original_settings.sound_enabled);
    }

    #[test]
    fn test_store_dialogue_add_edit_delete() {
        let mut store = SettingsStore::new(PathBuf::from("."), PetSettings::default());
        store.dispatch(SettingsAction::SelectDialogueCategory(0));
        let initial_count = store.active_dialogues().len();

        store.dispatch(SettingsAction::AddDialogue("测试台词一号".to_string()));
        assert_eq!(store.active_dialogues().len(), initial_count + 1);
        assert!(store.is_dirty());

        store.dispatch(SettingsAction::UpdateDialogue(initial_count, "测试台词改版".to_string()));
        assert_eq!(store.active_dialogues()[initial_count], "测试台词改版");

        store.dispatch(SettingsAction::DeleteDialogue(initial_count));
        assert_eq!(store.active_dialogues().len(), initial_count);
    }
}
