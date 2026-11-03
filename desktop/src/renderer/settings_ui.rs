use std::time::Instant;
use crate::state_watcher::PetSettings;
use super::Pixmap;
use super::text::{draw_text_into_pixmap, TextAlignment, VerticalAlignment};

pub const SETTINGS_WIN_W: f64 = 780.0;
pub const SETTINGS_WIN_H: f64 = 520.0;
const SIDEBAR_W: f32 = 180.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsTab {
    General,
    Autostart,
    Skins,
    Dialogue,
    Menu,
}

#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub enum SettingsAction {
    None,
    Save(PetSettings),
    Cancel,
    ResetPosition,
    CheckUpdate,
    SelectTab(SettingsTab),
    Redraw,
    OpenDialogueConfig,
    BeginTitleDrag,
    MinimizeWindow,
}

/// Native pure 2D CPU software-rendered Preferences Settings Window UI.
/// Supports native WinUI 3 (Windows 11 Fluent) and macOS HIG styling.
/// Zero WebKit, zero Node.js, and under 2MB RAM footprint.
pub struct SettingsUi {
    pub active_tab: SettingsTab,
    pub temp_settings: PetSettings,
    pub original_settings: PetSettings,
    pub autostart_enabled: bool,
    pub scroll_y: f32,
    pub general_scroll: f32,
    pub toast: Option<(String, Instant)>,
    pub hovered_id: Option<String>,
    pub pressed_id: Option<String>,
    pub dragging_slider: Option<String>,
    pub mouse_pos: (f32, f32),
    pub cat_sprite: super::sprite::CatSprite,
    pub dialogue_bank: crate::menu_audio::DialogueBank,
    // Direct in-place dialogue editing state
    pub dialogue_cat: usize,
    pub dialogue_scroll: f32,
    pub editing_dialogue_idx: Option<usize>,
    pub editing_buffer: String,
}

impl SettingsUi {
    pub fn new(settings: PetSettings) -> Self {
        let cat_sprite = super::sprite::CatSprite::load_default();
        let root_dir = crate::state_watcher::resolve_project_root();
        let dialogue_bank = crate::menu_audio::DialogueBank::load_from_root(&root_dir, "fat-cat");
        Self {
            active_tab: SettingsTab::General,
            temp_settings: settings.clone(),
            original_settings: settings,
            autostart_enabled: false,
            scroll_y: 0.0,
            general_scroll: 0.0,
            toast: None,
            hovered_id: None,
            pressed_id: None,
            dragging_slider: None,
            mouse_pos: (0.0, 0.0),
            cat_sprite,
            dialogue_bank,
            dialogue_cat: 0,
            dialogue_scroll: 0.0,
            editing_dialogue_idx: None,
            editing_buffer: String::new(),
        }
    }

    pub fn reload_dialogues(&mut self) {
        let root_dir = crate::state_watcher::resolve_project_root();
        self.dialogue_bank = crate::menu_audio::DialogueBank::load_from_root(&root_dir, "fat-cat");
    }

    pub fn open(&mut self, current_settings: &PetSettings) {
        self.temp_settings = current_settings.clone();
        self.original_settings = current_settings.clone();
        self.scroll_y = 0.0;
        self.general_scroll = 0.0;
        self.dialogue_scroll = 0.0;
        self.editing_dialogue_idx = None;
        self.editing_buffer.clear();
        self.toast = None;
        self.hovered_id = None;
        self.pressed_id = None;
        self.dragging_slider = None;
        self.reload_dialogues();
    }

    pub fn show_toast(&mut self, msg: &str) {
        self.toast = Some((msg.to_string(), Instant::now()));
    }

    pub fn is_dark_mode(&self) -> bool {
        super::is_dark_mode_active(&self.temp_settings.theme)
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

    pub fn handle_char(&mut self, ch: char) -> bool {
        if let Some(idx) = self.editing_dialogue_idx {
            if ch == '\r' || ch == '\n' {
                let text = self.editing_buffer.clone();
                let list = self.active_dialogues_mut();
                if idx < list.len() {
                    list[idx] = text;
                }
                self.editing_dialogue_idx = None;
                return true;
            } else if ch == '\x08' {
                self.editing_buffer.pop();
                let text = self.editing_buffer.clone();
                let list = self.active_dialogues_mut();
                if idx < list.len() {
                    list[idx] = text;
                }
                return true;
            } else if !ch.is_control() {
                self.editing_buffer.push(ch);
                let text = self.editing_buffer.clone();
                let list = self.active_dialogues_mut();
                if idx < list.len() {
                    list[idx] = text;
                }
                return true;
            }
        }
        false
    }

    pub fn handle_key_event(&mut self, event: &tao::event::KeyEvent) -> bool {
        if event.state != tao::event::ElementState::Pressed {
            return false;
        }
        if self.editing_dialogue_idx.is_some() {
            if event.physical_key == tao::keyboard::KeyCode::Escape {
                self.cancel_editing();
                return true;
            } else if event.physical_key == tao::keyboard::KeyCode::Enter {
                return self.handle_char('\n');
            } else if event.physical_key == tao::keyboard::KeyCode::Backspace {
                return self.handle_char('\x08');
            } else if let Some(text) = event.text {
                let mut changed = false;
                for ch in text.chars() {
                    if !ch.is_control() {
                        self.handle_char(ch);
                        changed = true;
                    }
                }
                return changed;
            }
        }
        false
    }

    pub fn handle_ime_text(&mut self, text: &str) -> bool {
        if self.editing_dialogue_idx.is_some() {
            let mut changed = false;
            for ch in text.chars() {
                if !ch.is_control() {
                    self.handle_char(ch);
                    changed = true;
                }
            }
            return changed;
        }
        false
    }

    pub fn cancel_editing(&mut self) {
        self.editing_dialogue_idx = None;
        self.editing_buffer.clear();
    }

    pub fn handle_cursor_move(&mut self, x: f64, y: f64, _scale: f32) -> bool {
        let (lx, ly) = (x as f32, y as f32);
        self.mouse_pos = (lx, ly);

        if let Some(slider_key) = self.dragging_slider.clone() {
            self.update_slider(&slider_key, lx);
            return true;
        }

        // Hit-test hover targets
        let old_hover = self.hovered_id.clone();
        self.hovered_id = self.hit_test_control(lx, ly);
        old_hover != self.hovered_id
    }

    pub fn handle_mouse_down(&mut self, _scale: f32) -> Option<SettingsAction> {
        let (mx, my) = self.mouse_pos;
        self.pressed_id = self.hit_test_control(mx, my);

        // Skip macOS native traffic lights (top-left 0..72 x 0..36) ONLY on macOS
        #[cfg(target_os = "macos")]
        if mx < 72.0 && my < 36.0 {
            return None;
        }

        // macOS titlebar drag area (top 0..48 outside traffic lights)
        #[cfg(target_os = "macos")]
        if my <= 48.0 {
            return Some(SettingsAction::BeginTitleDrag);
        }

        // Windows Caption Controls
        #[cfg(target_os = "windows")]
        if my <= 32.0 {
            let win_w = SETTINGS_WIN_W as f32;
            if mx >= win_w - 46.0 && mx <= win_w {
                return Some(SettingsAction::Cancel);
            }
            if mx >= win_w - 92.0 && mx < win_w - 46.0 {
                return Some(SettingsAction::MinimizeWindow);
            }
            return Some(SettingsAction::BeginTitleDrag);
        }

        // Bottom Action Footer Buttons
        if my >= (SETTINGS_WIN_H as f32 - 48.0) {
            let win_w = SETTINGS_WIN_W as f32;
            let save_x = win_w - 88.0;
            if (save_x..save_x + 76.0).contains(&mx) {
                return Some(SettingsAction::Save(self.temp_settings.clone()));
            }
            let cancel_x = win_w - 176.0;
            if (cancel_x..cancel_x + 76.0).contains(&mx) {
                return Some(SettingsAction::Cancel);
            }
            return None;
        }

        // 1. Sidebar clicks (tabs start at y = 58.0 with 36.0 spacing, 5 tabs end at 238.0)
        if mx < SIDEBAR_W {
            if (58.0..238.0).contains(&my) {
                let tab_idx = ((my - 58.0) / 36.0).floor() as i32;
                let target_tab = match tab_idx {
                    0 => Some(SettingsTab::General),
                    1 => Some(SettingsTab::Autostart),
                    2 => Some(SettingsTab::Skins),
                    3 => Some(SettingsTab::Dialogue),
                    4 => Some(SettingsTab::Menu),
                    _ => None,
                };
                if let Some(tab) = target_tab {
                    self.active_tab = tab;
                    self.scroll_y = 0.0;
                    self.general_scroll = 0.0;
                    self.dialogue_scroll = 0.0;
                    self.cancel_editing();
                    return Some(SettingsAction::SelectTab(tab));
                }
            }
            // Clicking empty sidebar area below tabs allows moving window
            if my > 238.0 && my < (SETTINGS_WIN_H as f32 - 48.0) {
                return Some(SettingsAction::BeginTitleDrag);
            }
            return None;
        }

        // 2. Tab content clicks
        let action = self.interact_tab_controls(mx, my);
        match action {
            SettingsAction::None => None,
            other => Some(other),
        }
    }

    pub fn handle_mouse_up(&mut self) -> Option<SettingsAction> {
        let had_press = self.pressed_id.take().is_some();
        if self.dragging_slider.is_some() {
            self.dragging_slider = None;
            return Some(SettingsAction::Redraw);
        }
        if had_press {
            return Some(SettingsAction::Redraw);
        }
        None
    }

    pub fn handle_mouse_wheel(&mut self, dy: f32) -> bool {
        match self.active_tab {
            SettingsTab::General => {
                let (mx, my) = self.mouse_pos;
                let cw_right = SETTINGS_WIN_W as f32 - 28.0;
                let rel_y = my + self.general_scroll;
                // Allow wheel over the scale slider row to smoothly adjust scale
                if (134.0..166.0).contains(&rel_y) && (cw_right - 180.0..cw_right - 40.0).contains(&mx) {
                    let step = if dy > 0.0 { 0.05 } else { -0.05 };
                    let new_scale = ((self.temp_settings.scale + step) * 20.0).round() / 20.0;
                    let clamped = new_scale.clamp(0.5, 2.0);
                    if (clamped - self.temp_settings.scale).abs() > 0.001 {
                        self.temp_settings.scale = clamped;
                        return true;
                    }
                }
                // Smoothly scroll General tab content
                let max_scroll = 56.0f32;
                let new_scroll = (self.general_scroll - dy).clamp(0.0, max_scroll);
                if (new_scroll - self.general_scroll).abs() > 0.01 {
                    self.general_scroll = new_scroll;
                    return true;
                }
                false
            }
            SettingsTab::Dialogue => {
                let count = self.active_dialogues().len();
                let max_scroll = ((count as f32 * 42.0) - 280.0).max(0.0);
                let new_scroll = (self.dialogue_scroll - dy).clamp(0.0, max_scroll);
                if (new_scroll - self.dialogue_scroll).abs() > 0.01 {
                    self.dialogue_scroll = new_scroll;
                    return true;
                }
                false
            }
            SettingsTab::Menu => {
                let max_scroll = 56.0f32;
                let new_scroll = (self.scroll_y - dy).clamp(0.0, max_scroll);
                if (new_scroll - self.scroll_y).abs() > 0.01 {
                    self.scroll_y = new_scroll;
                    return true;
                }
                false
            }
            _ => false,
        }
    }

    fn update_slider(&mut self, key: &str, mouse_x: f32) {
        let slider_x = (SETTINGS_WIN_W as f32) - 28.0 - 170.0;
        let slider_w = 110.0;
        let pct = ((mouse_x - slider_x) / slider_w).clamp(0.0, 1.0);

        match key {
            "scale" => {
                let val = 0.5 + pct * 1.5;
                self.temp_settings.scale = ((val * 20.0).round() / 20.0).clamp(0.5, 2.0);
            }
            _ => {}
        }
    }

    fn hit_test_control(&self, mx: f32, my: f32) -> Option<String> {
        let is_win = cfg!(target_os = "windows");
        if is_win && my <= 32.0 {
            let win_w = SETTINGS_WIN_W as f32;
            if mx >= win_w - 46.0 && mx <= win_w {
                return Some("win_close".to_string());
            }
            if mx >= win_w - 92.0 && mx < win_w - 46.0 {
                return Some("win_minimize".to_string());
            }
        }

        // Action Footer buttons
        if my >= (SETTINGS_WIN_H as f32 - 48.0) {
            let win_w = SETTINGS_WIN_W as f32;
            let save_x = win_w - 88.0;
            if (save_x..save_x + 76.0).contains(&mx) {
                return Some("btn_save".to_string());
            }
            let cancel_x = win_w - 176.0;
            if (cancel_x..cancel_x + 76.0).contains(&mx) {
                return Some("btn_cancel".to_string());
            }
        }

        if mx < SIDEBAR_W && (58.0..238.0).contains(&my) {
            let tab_idx = ((my - 58.0) / 36.0).floor() as i32;
            if (0..=4).contains(&tab_idx) {
                return Some(format!("tab_{}", tab_idx));
            }
        }

        let cw_left = SIDEBAR_W + 28.0;
        let cw_right = SETTINGS_WIN_W as f32 - 28.0;

        match self.active_tab {
            SettingsTab::General => {
                if my < 70.0 || my > (SETTINGS_WIN_H as f32 - 48.0) {
                    return None;
                }
                let rel_y = my + self.general_scroll;
                if (74.0..120.0).contains(&rel_y) && (cw_right - 96.0..cw_right).contains(&mx) {
                    return Some("theme".to_string());
                }
                if (126.0..170.0).contains(&rel_y) && (cw_right - 180.0..cw_right).contains(&mx) {
                    return Some("scale".to_string());
                }
                if (170.0..214.0).contains(&rel_y) && (cw_right - 160.0..cw_right).contains(&mx) {
                    return Some("floor_mode".to_string());
                }
                if (214.0..258.0).contains(&rel_y) && (cw_right - 60.0..cw_right).contains(&mx) {
                    return Some("gravity".to_string());
                }
                if (258.0..302.0).contains(&rel_y) && (cw_right - 88.0..cw_right).contains(&mx) {
                    return Some("reset_pos".to_string());
                }
                if (310.0..348.0).contains(&rel_y) && (cw_right - 60.0..cw_right).contains(&mx) {
                    return Some("show_quota".to_string());
                }
                if (348.0..386.0).contains(&rel_y) && (cw_right - 60.0..cw_right).contains(&mx) {
                    return Some("ide_only".to_string());
                }
                if (386.0..424.0).contains(&rel_y) && (cw_right - 60.0..cw_right).contains(&mx) {
                    return Some("dnd".to_string());
                }
                if (424.0..462.0).contains(&rel_y) && (cw_right - 60.0..cw_right).contains(&mx) {
                    return Some("sound".to_string());
                }
            }
            SettingsTab::Autostart => {
                if (80.0..126.0).contains(&my) && (cw_right - 60.0..cw_right).contains(&mx) {
                    return Some("autostart".to_string());
                }
                if (138.0..182.0).contains(&my) && (cw_right - 96.0..cw_right).contains(&mx) {
                    return Some("check_update".to_string());
                }
            }
            SettingsTab::Skins => {
                let card_w = 180.0;
                if (80.0..240.0).contains(&my) && (cw_left..cw_left + card_w).contains(&mx) {
                    return Some("skin_0".to_string());
                }
            }
            SettingsTab::Dialogue => {
                // Category Pills (y: 82.0 .. 114.0)
                if (82.0..114.0).contains(&my) && (cw_left..cw_right).contains(&mx) {
                    let pill_w = (cw_right - cw_left) / 5.0;
                    let idx = ((mx - cw_left) / pill_w).floor() as usize;
                    if idx < 5 {
                        return Some(format!("diag_cat_{}", idx));
                    }
                }
                // Add dialogue button (y: 126.0 .. 154.0, right-aligned)
                let btn_w = 92.0;
                if (126.0..154.0).contains(&my) && (cw_right - btn_w..cw_right).contains(&mx) {
                    return Some("add_dialogue".to_string());
                }
                // Dialogue items list (ONLY inside viewport y: 160.0 .. SETTINGS_WIN_H - 48.0)
                if my >= 160.0 && my <= (SETTINGS_WIN_H as f32 - 48.0) {
                    let rel_y = my - 160.0 + self.dialogue_scroll;
                    let row_idx = (rel_y / 42.0).floor() as i32;
                    if row_idx >= 0 {
                        let uidx = row_idx as usize;
                        let count = self.active_dialogues().len();
                        if uidx < count {
                            if mx >= cw_right - 56.0 && mx <= cw_right - 6.0 {
                                return Some(format!("del_diag_{}", uidx));
                            } else if mx >= cw_left && mx < cw_right - 60.0 {
                                return Some(format!("edit_diag_{}", uidx));
                            }
                        }
                    }
                }
            }
            SettingsTab::Menu => {
                if my >= 74.0 && my <= (SETTINGS_WIN_H as f32 - 48.0) {
                    let content_y = my - 74.0 + self.scroll_y;
                    let row_idx = (content_y / 44.0).floor() as i32;
                    if (0..=8).contains(&row_idx) && (cw_right - 60.0..cw_right).contains(&mx) {
                        return Some(format!("menu_{}", row_idx));
                    }
                }
            }
        }

        None
    }

    fn interact_tab_controls(&mut self, mx: f32, my: f32) -> SettingsAction {
        let cw_left = SIDEBAR_W + 28.0;
        let cw_right = SETTINGS_WIN_W as f32 - 28.0;

        match self.active_tab {
            SettingsTab::General => {
                if my < 70.0 || my > (SETTINGS_WIN_H as f32 - 48.0) {
                    return SettingsAction::None;
                }
                let rel_y = my + self.general_scroll;
                if (74.0..120.0).contains(&rel_y) && (cw_right - 96.0..cw_right).contains(&mx) {
                    self.temp_settings.theme = match self.temp_settings.theme.as_str() {
                        "system" => "light".to_string(),
                        "light" => "dark".to_string(),
                        _ => "system".to_string(),
                    };
                    return SettingsAction::Redraw;
                }
                if (126.0..170.0).contains(&rel_y) && (cw_right - 180.0..cw_right).contains(&mx) {
                    self.dragging_slider = Some("scale".to_string());
                    self.update_slider("scale", mx);
                    return SettingsAction::Redraw;
                }
                if (170.0..214.0).contains(&rel_y) && (cw_right - 160.0..cw_right).contains(&mx) {
                    self.temp_settings.floor_mode = match self.temp_settings.floor_mode.as_str() {
                        "taskbar" | "dock" => "bottom".to_string(),
                        "bottom" => "auto".to_string(),
                        _ => "taskbar".to_string(),
                    };
                    return SettingsAction::Redraw;
                }
                if (214.0..258.0).contains(&rel_y) && (cw_right - 60.0..cw_right).contains(&mx) {
                    self.temp_settings.gravity_enabled = !self.temp_settings.gravity_enabled;
                    return SettingsAction::Redraw;
                }
                if (258.0..302.0).contains(&rel_y) && (cw_right - 88.0..cw_right).contains(&mx) {
                    return SettingsAction::ResetPosition;
                }
                if (310.0..348.0).contains(&rel_y) && (cw_right - 60.0..cw_right).contains(&mx) {
                    self.temp_settings.show_quota = !self.temp_settings.show_quota;
                    return SettingsAction::Redraw;
                }
                if (348.0..386.0).contains(&rel_y) && (cw_right - 60.0..cw_right).contains(&mx) {
                    self.temp_settings.menu_config.ide_only = !self.temp_settings.menu_config.ide_only;
                    return SettingsAction::Redraw;
                }
                if (386.0..424.0).contains(&rel_y) && (cw_right - 60.0..cw_right).contains(&mx) {
                    self.temp_settings.dnd_passthrough = !self.temp_settings.dnd_passthrough;
                    return SettingsAction::Redraw;
                }
                if (424.0..462.0).contains(&rel_y) && (cw_right - 60.0..cw_right).contains(&mx) {
                    self.temp_settings.sound_enabled = !self.temp_settings.sound_enabled;
                    return SettingsAction::Redraw;
                }
            }
            SettingsTab::Autostart => {
                if (80.0..126.0).contains(&my) && (cw_right - 60.0..cw_right).contains(&mx) {
                    self.autostart_enabled = !self.autostart_enabled;
                    let msg = if self.autostart_enabled { "已开启开机自动启动" } else { "已关闭开机自动启动" };
                    self.show_toast(msg);
                    return SettingsAction::Redraw;
                }
                if (138.0..182.0).contains(&my) && (cw_right - 96.0..cw_right).contains(&mx) {
                    return SettingsAction::CheckUpdate;
                }
            }
            SettingsTab::Skins => {
                let card_w = 180.0;
                if (80.0..240.0).contains(&my) && (cw_left..cw_left + card_w).contains(&mx) {
                    self.show_toast("当前经典紫毛猫已生效");
                    return SettingsAction::Redraw;
                }
            }
            SettingsTab::Dialogue => {
                // Category Pills (y: 82.0 .. 114.0)
                if (82.0..114.0).contains(&my) && (cw_left..cw_right).contains(&mx) {
                    let pill_w = (cw_right - cw_left) / 5.0;
                    let idx = ((mx - cw_left) / pill_w).floor() as usize;
                    if idx < 5 {
                        self.dialogue_cat = idx;
                        self.dialogue_scroll = 0.0;
                        self.cancel_editing();
                        return SettingsAction::Redraw;
                    }
                }
                // Add button (y: 126.0 .. 154.0)
                let btn_w = 92.0;
                if (126.0..154.0).contains(&my) && (cw_right - btn_w..cw_right).contains(&mx) {
                    let total = {
                        let list = self.active_dialogues_mut();
                        list.push("新台词条目".to_string());
                        list.len()
                    };
                    let new_idx = total - 1;
                    self.editing_dialogue_idx = Some(new_idx);
                    self.editing_buffer = "新台词条目".to_string();
                    self.dialogue_scroll = ((total as f32 * 42.0) - 280.0).max(0.0);
                    return SettingsAction::Redraw;
                }
                // Dialogue items (ONLY inside viewport y: 160.0 .. SETTINGS_WIN_H - 48.0)
                if my >= 160.0 && my <= (SETTINGS_WIN_H as f32 - 48.0) {
                    let rel_y = my - 160.0 + self.dialogue_scroll;
                    let row_idx = (rel_y / 42.0).floor() as i32;
                    if row_idx >= 0 {
                        let uidx = row_idx as usize;
                        let count = self.active_dialogues().len();
                        if uidx < count {
                            if mx >= cw_right - 56.0 && mx <= cw_right - 6.0 {
                                let list = self.active_dialogues_mut();
                                list.remove(uidx);
                                if self.editing_dialogue_idx == Some(uidx) {
                                    self.editing_dialogue_idx = None;
                                } else if let Some(ref mut cur) = self.editing_dialogue_idx {
                                    if *cur > uidx {
                                        *cur -= 1;
                                    }
                                }
                                return SettingsAction::Redraw;
                            } else if mx >= cw_left && mx < cw_right - 60.0 {
                                let line_text = self.active_dialogues()[uidx].clone();
                                self.editing_dialogue_idx = Some(uidx);
                                self.editing_buffer = line_text;
                                return SettingsAction::Redraw;
                            }
                        }
                    }
                }
            }
            SettingsTab::Menu => {
                if my >= 74.0 && my <= (SETTINGS_WIN_H as f32 - 48.0) {
                    let content_y = my - 74.0 + self.scroll_y;
                    let row_h = 44.0;
                    let row_idx = (content_y / row_h).floor() as i32;
                    if (0..=8).contains(&row_idx) && (cw_right - 60.0..cw_right).contains(&mx) {
                        match row_idx {
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
                        return SettingsAction::Redraw;
                    }
                }
            }
        }

        SettingsAction::None
    }

    /// Renders the full native settings window into `pixmap`.
    pub fn render(&mut self, pixmap: &mut Pixmap, dpi_scale: f32) {
        let is_dark = self.is_dark_mode();
        let is_win = cfg!(target_os = "windows");
        let s = dpi_scale;

        let w_px = ((SETTINGS_WIN_W as f32) * s).round() as u32;
        let h_px = ((SETTINGS_WIN_H as f32) * s).round() as u32;
        pixmap.resize(w_px, h_px);
        pixmap.clear();

        // Theme palette: WinUI 3 (Fluent) vs macOS HIG
        let accent_color = if is_win {
            if is_dark { [96, 205, 255, 255] } else { [0, 103, 192, 255] }
        } else {
            if is_dark { [10, 132, 255, 255] } else { [0, 122, 255, 255] }
        };

        // Fully opaque background on Windows to prevent any transparent terminal bleed-through
        let bg_sidebar = if is_win {
            if is_dark { [32, 32, 32, 255] } else { [243, 243, 243, 255] }
        } else {
            if is_dark { [18, 18, 20, 38] } else { [255, 255, 255, 55] }
        };

        let bg_content = if is_win {
            if is_dark { [39, 39, 39, 255] } else { [249, 249, 249, 255] }
        } else {
            if is_dark { [30, 30, 32, 255] } else { [246, 246, 248, 255] }
        };

        let bg_card = if is_win {
            if is_dark { [45, 45, 45, 255] } else { [255, 255, 255, 255] }
        } else {
            if is_dark { [44, 44, 46, 255] } else { [255, 255, 255, 255] }
        };

        let text_pri = if is_dark { [255, 255, 255] } else { [26, 26, 26] };
        let text_sec = if is_dark { [160, 160, 160] } else { [93, 93, 93] };

        let border_card = if is_win {
            if is_dark { [255, 255, 255, 20] } else { [0, 0, 0, 18] }
        } else {
            if is_dark { [255, 255, 255, 14] } else { [0, 0, 0, 12] }
        };

        let row_div = if is_win {
            if is_dark { [255, 255, 255, 15] } else { [0, 0, 0, 14] }
        } else {
            if is_dark { [255, 255, 255, 12] } else { [0, 0, 0, 10] }
        };

        let card_radius = if is_win { 4.0 * s } else { 10.0 * s };

        // 1. Fill sidebar with solid background
        pixmap.fill_rounded_rect_sdf(0.0, 0.0, SIDEBAR_W * s, h_px as f32, 0.0, bg_sidebar, 0.0, [0, 0, 0, 0]);

        // Sidebar Header: App Icon Thumbnail + Titles (Only on Windows where there are no native traffic lights)
        if is_win {
            let icon_box_x = 16.0 * s;
            let icon_box_y = 14.0 * s;
            let icon_box_size = 36.0 * s;
            pixmap.fill_rounded_rect_sdf(
                icon_box_x,
                icon_box_y,
                icon_box_size,
                icon_box_size,
                6.0 * s,
                bg_card,
                1.0 * s,
                border_card,
            );
            let tf_avatar = super::sprite::SpriteTransform {
                anchor_x: (icon_box_x + 18.0 * s) / s,
                anchor_y: (icon_box_y + 32.0 * s) / s,
                base_size: 28.0,
                scale_x: 1.0,
                scale_y: 1.0,
                flip_x: 1.0,
                rotation_rad: 0.0,
                gaze_offset_x: 0.0,
                gaze_offset_y: 0.0,
                opacity: 1.0,
                draw_ground_shadow: false,
            };
            let _ = self.cat_sprite.draw(pixmap, &tf_avatar, s);

            let header_txt_x = icon_box_x + icon_box_size + 10.0 * s;
            draw_text_into_pixmap(pixmap, "Anti-Pet", header_txt_x, icon_box_y + 3.0 * s, (SIDEBAR_W - 68.0) * s, 16.0 * s, 13.5 * s, 0.35, text_pri, 1.0, TextAlignment::Left, VerticalAlignment::Middle);
            draw_text_into_pixmap(pixmap, "伴侣偏好设置", header_txt_x, icon_box_y + 19.0 * s, (SIDEBAR_W - 68.0) * s, 14.0 * s, 10.5 * s, 0.0, text_sec, 0.85, TextAlignment::Left, VerticalAlignment::Middle);
        }

        // 2. Fill content area with solid background
        let content_x = SIDEBAR_W * s;
        let content_w = (w_px as f32) - content_x;
        pixmap.fill_rounded_rect_sdf(content_x, 0.0, content_w, h_px as f32, 0.0, bg_content, 0.0, [0, 0, 0, 0]);

        // 3. Hairline vertical divider between sidebar and content
        let div_col = if is_dark { [255, 255, 255, 18] } else { [0, 0, 0, 18] };
        pixmap.fill_rounded_rect_sdf(content_x, 0.0, 1.0 * s, h_px as f32, 0.0, div_col, 0.0, [0, 0, 0, 0]);

        // Windows Caption Controls (Minimize and Close buttons at top right)
        if is_win {
            let win_w = SETTINGS_WIN_W as f32 * s;
            let close_x = win_w - 46.0 * s;
            let is_close_hover = self.hovered_id.as_deref() == Some("win_close");
            let is_close_press = self.pressed_id.as_deref() == Some("win_close");

            let close_bg = if is_close_press {
                [176, 32, 18, 255]
            } else if is_close_hover {
                [196, 43, 28, 255]
            } else {
                [0, 0, 0, 0]
            };
            if close_bg[3] > 0 {
                pixmap.fill_rounded_rect_sdf(close_x, 0.0, 46.0 * s, 32.0 * s, 0.0, close_bg, 0.0, [0, 0, 0, 0]);
            }
            let close_icon_col = if is_close_hover || is_close_press {
                [255, 255, 255, 255]
            } else {
                [text_pri[0], text_pri[1], text_pri[2], 220]
            };
            // Draw close cross
            let cx_c = close_x + 23.0 * s;
            let cy_c = 16.0 * s;
            let d = 4.5 * s;
            pixmap.draw_line_sdf(cx_c - d, cy_c - d, cx_c + d, cy_c + d, 1.2 * s, close_icon_col);
            pixmap.draw_line_sdf(cx_c + d, cy_c - d, cx_c - d, cy_c + d, 1.2 * s, close_icon_col);

            // Minimize '─'
            let min_x = close_x - 46.0 * s;
            let is_min_hover = self.hovered_id.as_deref() == Some("win_minimize");
            let is_min_press = self.pressed_id.as_deref() == Some("win_minimize");

            let min_bg = if is_min_press {
                if is_dark { [255, 255, 255, 26] } else { [0, 0, 0, 20] }
            } else if is_min_hover {
                if is_dark { [255, 255, 255, 14] } else { [0, 0, 0, 10] }
            } else {
                [0, 0, 0, 0]
            };
            if min_bg[3] > 0 {
                pixmap.fill_rounded_rect_sdf(min_x, 0.0, 46.0 * s, 32.0 * s, 0.0, min_bg, 0.0, [0, 0, 0, 0]);
            }
            let min_icon_col = [text_pri[0], text_pri[1], text_pri[2], 220];
            let mx_c = min_x + 23.0 * s;
            pixmap.draw_line_sdf(mx_c - 5.0 * s, cy_c, mx_c + 5.0 * s, cy_c, 1.2 * s, min_icon_col);
        }

        // 4. Sidebar nav items
        let tabs = [
            (SettingsTab::General, "常规与显示"),
            (SettingsTab::Autostart, "自启动与更新"),
            (SettingsTab::Skins, "皮肤管理"),
            (SettingsTab::Dialogue, "台词与问候"),
            (SettingsTab::Menu, "右键菜单"),
        ];

        for (i, (tab, title)) in tabs.into_iter().enumerate() {
            let ty = (58.0 + (i as f32) * 36.0) * s;
            let is_active = self.active_tab == tab;
            let tab_id = format!("tab_{}", i);
            let is_hovered = self.hovered_id.as_deref() == Some(&tab_id);
            let is_pressed = self.pressed_id.as_deref() == Some(&tab_id);

            let (bg_pill, txt_col) = if is_win {
                let bg = if is_pressed {
                    if is_dark { [255, 255, 255, 24] } else { [0, 0, 0, 18] }
                } else if is_active {
                    if is_dark { [255, 255, 255, 16] } else { [0, 0, 0, 12] }
                } else if is_hovered {
                    if is_dark { [255, 255, 255, 10] } else { [0, 0, 0, 7] }
                } else {
                    [0, 0, 0, 0]
                };
                (bg, text_pri)
            } else if is_active {
                (accent_color, [255, 255, 255])
            } else if is_pressed {
                (if is_dark { [255, 255, 255, 26] } else { [0, 0, 0, 18] }, text_pri)
            } else if is_hovered {
                (if is_dark { [255, 255, 255, 18] } else { [0, 0, 0, 12] }, text_pri)
            } else {
                ([0, 0, 0, 0], text_pri)
            };

            let pill_x = if is_win { 10.0 * s } else { 12.0 * s };
            let pill_w = if is_win { (SIDEBAR_W - 20.0) * s } else { (SIDEBAR_W - 24.0) * s };
            let pill_r = if is_win { 4.0 * s } else { 6.0 * s };

            if bg_pill[3] > 0 {
                pixmap.fill_rounded_rect_sdf(pill_x, ty, pill_w, 32.0 * s, pill_r, bg_pill, 0.0, [0, 0, 0, 0]);
            }

            // WinUI 3 left vertical selection indicator
            if is_win && is_active {
                let ind_h = if is_pressed { 12.0 * s } else { 16.0 * s };
                let ind_y = ty + (32.0 * s - ind_h) * 0.5;
                pixmap.fill_rounded_rect_sdf(10.0 * s, ind_y, 3.0 * s, ind_h, 1.5 * s, accent_color, 0.0, [0, 0, 0, 0]);
            }

            // Clean text-only navigation (no extra icons, precisely vertically centered)
            let text_x = if is_win { 22.0 * s } else { 24.0 * s };
            let weight = if is_active { 0.3 } else { 0.0 };
            draw_text_into_pixmap(pixmap, title, text_x, ty, (SIDEBAR_W - 36.0) * s, 32.0 * s, 12.5 * s, weight, txt_col, 1.0, TextAlignment::Left, VerticalAlignment::Middle);
        }

        // Version badge at the bottom of the sidebar
        let version_y = (SETTINGS_WIN_H as f32 - 68.0) * s;
        draw_text_into_pixmap(pixmap, "Anti-Pet v0.4.1", 22.0 * s, version_y, (SIDEBAR_W - 44.0) * s, 16.0 * s, 10.5 * s, 0.0, text_sec, 0.85, TextAlignment::Left, VerticalAlignment::Middle);

        // 5. Render Right Content Area
        let cx = (SIDEBAR_W + 28.0) * s;
        let cw = (SETTINGS_WIN_W as f32 - SIDEBAR_W - 56.0) * s;

        match self.active_tab {
            SettingsTab::General => {
                self.render_general_tab(pixmap, cx, cw, s, bg_card, border_card, row_div, text_pri, text_sec, accent_color, card_radius, is_win, is_dark);
            }
            SettingsTab::Autostart => {
                self.render_autostart_tab(pixmap, cx, cw, s, bg_card, border_card, row_div, text_pri, text_sec, accent_color, card_radius, is_win, is_dark);
            }
            SettingsTab::Skins => {
                self.render_skins_tab(pixmap, cx, cw, s, bg_card, border_card, text_pri, text_sec, accent_color, card_radius, is_win);
            }
            SettingsTab::Dialogue => {
                self.render_dialogue_tab(pixmap, cx, cw, s, bg_card, border_card, text_pri, text_sec, accent_color, card_radius, is_win, is_dark);
            }
            SettingsTab::Menu => {
                self.render_menu_tab(pixmap, cx, cw, s, bg_card, border_card, row_div, text_pri, text_sec, accent_color, card_radius, is_win, is_dark);
            }
        }

        // 6. Action Footer Across All Tabs
        let footer_y = (SETTINGS_WIN_H as f32 - 48.0) * s;
        let footer_h = 48.0 * s;
        let bg_footer = if is_win {
            if is_dark { [32, 32, 32, 255] } else { [240, 240, 240, 255] }
        } else {
            if is_dark { [24, 24, 26, 255] } else { [242, 242, 244, 255] }
        };
        pixmap.fill_rounded_rect_sdf(0.0, footer_y, w_px as f32, footer_h, 0.0, bg_footer, 0.0, [0, 0, 0, 0]);
        pixmap.fill_rounded_rect_sdf(0.0, footer_y, w_px as f32, 1.0 * s, 0.0, div_col, 0.0, [0, 0, 0, 0]);

        draw_text_into_pixmap(pixmap, "配置与台词变更将在点击确定后立即保存生效", 22.0 * s, footer_y, 360.0 * s, footer_h, 11.0 * s, 0.0, text_sec, 0.85, TextAlignment::Left, VerticalAlignment::Middle);

        // Cancel Button
        let cancel_w = 76.0 * s;
        let cancel_h = 30.0 * s;
        let cancel_x = (SETTINGS_WIN_W as f32 - 176.0) * s;
        let cancel_y = footer_y + 9.0 * s;
        self.render_button(pixmap, cancel_x, cancel_y, cancel_w, cancel_h, "取消", s, is_win, is_dark, border_card, text_pri, "btn_cancel");

        // Save / Confirm Button
        let save_w = 76.0 * s;
        let save_h = 30.0 * s;
        let save_x = (SETTINGS_WIN_W as f32 - 88.0) * s;
        let save_y = footer_y + 9.0 * s;
        self.render_primary_button(pixmap, save_x, save_y, save_w, save_h, "确定", s, accent_color, "btn_save");

        // 7. Toast Notification Pill
        if let Some((ref msg, since)) = self.toast {
            if since.elapsed().as_secs_f32() < 2.5 {
                let alpha = (1.0 - (since.elapsed().as_secs_f32() - 2.0).max(0.0) * 2.0).clamp(0.0, 1.0);
                let toast_w = 260.0 * s;
                let toast_x = ((SETTINGS_WIN_W as f32 * s) - toast_w) * 0.5;
                let toast_y = (SETTINGS_WIN_H as f32 - 44.0) * s;
                let toast_r = if is_win { 6.0 * s } else { 14.0 * s };
                let toast_bg = if is_win {
                    if is_dark { [45, 45, 45, (240.0 * alpha) as u8] } else { [255, 255, 255, (240.0 * alpha) as u8] }
                } else {
                    [28, 28, 30, (230.0 * alpha) as u8]
                };
                let toast_txt = if is_win && !is_dark { [26, 26, 26] } else { [255, 255, 255] };
                let toast_border = if is_win {
                    if is_dark { [255, 255, 255, (25.0 * alpha) as u8] } else { [0, 0, 0, (25.0 * alpha) as u8] }
                } else {
                    [0, 0, 0, 0]
                };
                pixmap.fill_rounded_rect_sdf(toast_x, toast_y, toast_w, 28.0 * s, toast_r, toast_bg, 1.0 * s, toast_border);
                draw_text_into_pixmap(pixmap, msg, toast_x, toast_y, toast_w, 28.0 * s, 11.5 * s, 0.0, toast_txt, alpha, TextAlignment::Center, VerticalAlignment::Middle);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_general_tab(
        &self,
        pixmap: &mut Pixmap,
        cx: f32,
        cw: f32,
        s: f32,
        bg_card: [u8; 4],
        border_card: [u8; 4],
        row_div: [u8; 4],
        text_pri: [u8; 3],
        text_sec: [u8; 3],
        accent: [u8; 4],
        card_radius: f32,
        is_win: bool,
        is_dark: bool,
    ) {
        let base_y = 28.0 * s;

        // Header (Fixed)
        draw_text_into_pixmap(pixmap, "常规与显示", cx, base_y, cw, 22.0 * s, 16.0 * s, 0.35, text_pri, 1.0, TextAlignment::Left, VerticalAlignment::Middle);
        draw_text_into_pixmap(pixmap, "调整大肥猫的桌面尺寸、物理重力下落与交互行为", cx, base_y + 24.0 * s, cw, 16.0 * s, 11.5 * s, 0.0, text_sec, 1.0, TextAlignment::Left, VerticalAlignment::Middle);

        // Viewport for scrolling content
        let vp_y = 70.0 * s;
        let vp_bottom = (SETTINGS_WIN_H as f32 - 52.0) * s;
        let vp_h = vp_bottom - vp_y;
        pixmap.set_clip_rect(cx - 2.0 * s, vp_y, cw + 4.0 * s, vp_h);

        // Card 1: 界面外观
        let c1_y = 74.0 * s - self.general_scroll * s;
        let c1_h = 44.0 * s;
        pixmap.fill_rounded_rect_sdf(cx, c1_y, cw, c1_h, card_radius, bg_card, 1.0 * s, border_card);
        draw_text_into_pixmap(pixmap, "界面外观", cx + 14.0 * s, c1_y + 4.0 * s, 200.0 * s, 18.0 * s, 13.0 * s, 0.0, text_pri, 1.0, TextAlignment::Left, VerticalAlignment::Middle);
        draw_text_into_pixmap(pixmap, "设置窗口明暗主题风格，默认自动跟随系统", cx + 14.0 * s, c1_y + 22.0 * s, 260.0 * s, 16.0 * s, 11.0 * s, 0.0, text_sec, 1.0, TextAlignment::Left, VerticalAlignment::Middle);

        let theme_label = match self.temp_settings.theme.as_str() {
            "dark" => "深色模式",
            "light" => "浅色模式",
            _ => "跟随系统",
        };
        let pill_x = cx + cw - 96.0 * s;
        self.render_button(pixmap, pill_x, c1_y + 10.0 * s, 84.0 * s, 24.0 * s, theme_label, s, is_win, is_dark, border_card, text_pri, "theme");

        // Card 2: 体型与物理 (4 rows)
        let c2_y = c1_y + c1_h + 8.0 * s;
        let c2_h = 176.0 * s;
        pixmap.fill_rounded_rect_sdf(cx, c2_y, cw, c2_h, card_radius, bg_card, 1.0 * s, border_card);

        // Row 2.1: 缩放大小
        let r1_y = c2_y;
        draw_text_into_pixmap(pixmap, "缩放大小", cx + 14.0 * s, r1_y + 4.0 * s, 200.0 * s, 18.0 * s, 13.0 * s, 0.0, text_pri, 1.0, TextAlignment::Left, VerticalAlignment::Middle);
        draw_text_into_pixmap(pixmap, "挂件在屏幕上的呈现比例 (也可悬停直接滚轮平滑缩放)", cx + 14.0 * s, r1_y + 22.0 * s, 260.0 * s, 16.0 * s, 11.0 * s, 0.0, text_sec, 1.0, TextAlignment::Left, VerticalAlignment::Middle);
        self.render_slider(pixmap, cx + cw - 170.0 * s, r1_y + 18.0 * s, 110.0 * s, s, (self.temp_settings.scale - 0.5) / 1.5, accent, is_win, is_dark);
        let scale_str = format!("{:.0}%", self.temp_settings.scale * 100.0);
        draw_text_into_pixmap(pixmap, &scale_str, cx + cw - 52.0 * s, r1_y + 13.0 * s, 40.0 * s, 18.0 * s, 11.5 * s, 0.0, text_sec, 1.0, TextAlignment::Right, VerticalAlignment::Middle);
        pixmap.fill_rounded_rect_sdf(cx + 14.0 * s, r1_y + 43.0 * s, cw - 28.0 * s, 1.0 * s, 0.0, row_div, 0.0, [0, 0, 0, 0]);

        // Row 2.2: 落地吸附模式 (Restored)
        let r2_y = r1_y + 44.0 * s;
        draw_text_into_pixmap(pixmap, "落地吸附模式", cx + 14.0 * s, r2_y + 4.0 * s, 200.0 * s, 18.0 * s, 13.0 * s, 0.0, text_pri, 1.0, TextAlignment::Left, VerticalAlignment::Middle);
        draw_text_into_pixmap(pixmap, "拖拽松手后猫猫下落吸附的目标基准线", cx + 14.0 * s, r2_y + 22.0 * s, 260.0 * s, 16.0 * s, 11.0 * s, 0.0, text_sec, 1.0, TextAlignment::Left, VerticalAlignment::Middle);
        let floor_label = match self.temp_settings.floor_mode.as_str() {
            "bottom" => "屏幕绝对落底 (0px)",
            "dock" | "taskbar" => "任务栏/程序坞上方",
            _ => "智能自适应",
        };
        let floor_btn_x = cx + cw - 148.0 * s;
        self.render_button(pixmap, floor_btn_x, r2_y + 10.0 * s, 136.0 * s, 24.0 * s, floor_label, s, is_win, is_dark, border_card, text_pri, "floor_mode");
        pixmap.fill_rounded_rect_sdf(cx + 14.0 * s, r2_y + 43.0 * s, cw - 28.0 * s, 1.0 * s, 0.0, row_div, 0.0, [0, 0, 0, 0]);

        // Row 2.3: 重力加速度模拟
        let r3_y = r2_y + 44.0 * s;
        draw_text_into_pixmap(pixmap, "重力加速度模拟", cx + 14.0 * s, r3_y + 4.0 * s, 200.0 * s, 18.0 * s, 13.0 * s, 0.0, text_pri, 1.0, TextAlignment::Left, VerticalAlignment::Middle);
        draw_text_into_pixmap(pixmap, "释放后遵循弹性重力吸附到任务栏或底边，触底自然回弹", cx + 14.0 * s, r3_y + 22.0 * s, 260.0 * s, 16.0 * s, 11.0 * s, 0.0, text_sec, 1.0, TextAlignment::Left, VerticalAlignment::Middle);
        self.render_toggle(pixmap, cx + cw - 52.0 * s, r3_y + 11.0 * s, s, self.temp_settings.gravity_enabled, is_win, is_dark, accent, "gravity");
        pixmap.fill_rounded_rect_sdf(cx + 14.0 * s, r3_y + 43.0 * s, cw - 28.0 * s, 1.0 * s, 0.0, row_div, 0.0, [0, 0, 0, 0]);

        // Row 2.4: 屏幕位置校准
        let r4_y = r3_y + 44.0 * s;
        draw_text_into_pixmap(pixmap, "屏幕位置校准", cx + 14.0 * s, r4_y + 4.0 * s, 200.0 * s, 18.0 * s, 13.0 * s, 0.0, text_pri, 1.0, TextAlignment::Left, VerticalAlignment::Middle);
        draw_text_into_pixmap(pixmap, "将桌宠一键拉回屏幕安全正中央，避免逃出屏幕或遮挡", cx + 14.0 * s, r4_y + 22.0 * s, 260.0 * s, 16.0 * s, 11.0 * s, 0.0, text_sec, 1.0, TextAlignment::Left, VerticalAlignment::Middle);
        let reset_btn_x = cx + cw - 88.0 * s;
        self.render_button(pixmap, reset_btn_x, r4_y + 10.0 * s, 76.0 * s, 24.0 * s, "复位居中", s, is_win, is_dark, border_card, text_pri, "reset_pos");

        // Card 3: 状态与交互 (4 rows)
        let c3_y = c2_y + c2_h + 8.0 * s;
        let c3_h = 152.0 * s;
        pixmap.fill_rounded_rect_sdf(cx, c3_y, cw, c3_h, card_radius, bg_card, 1.0 * s, border_card);

        // Row 3.1: 实时额度卡片
        let r31_y = c3_y;
        draw_text_into_pixmap(pixmap, "实时额度卡片", cx + 14.0 * s, r31_y + 2.0 * s, 200.0 * s, 17.0 * s, 13.0 * s, 0.0, text_pri, 1.0, TextAlignment::Left, VerticalAlignment::Middle);
        draw_text_into_pixmap(pixmap, "在对话气泡中同步展示 Antigravity 5小时与周额度进度", cx + 14.0 * s, r31_y + 19.0 * s, 260.0 * s, 15.0 * s, 11.0 * s, 0.0, text_sec, 1.0, TextAlignment::Left, VerticalAlignment::Middle);
        self.render_toggle(pixmap, cx + cw - 52.0 * s, r31_y + 8.0 * s, s, self.temp_settings.show_quota, is_win, is_dark, accent, "show_quota");
        pixmap.fill_rounded_rect_sdf(cx + 14.0 * s, r31_y + 37.0 * s, cw - 28.0 * s, 1.0 * s, 0.0, row_div, 0.0, [0, 0, 0, 0]);

        // Row 3.2: 仅在 IDE 活跃
        let r32_y = r31_y + 38.0 * s;
        draw_text_into_pixmap(pixmap, "仅在 IDE 活跃时显示", cx + 14.0 * s, r32_y + 2.0 * s, 200.0 * s, 17.0 * s, 13.0 * s, 0.0, text_pri, 1.0, TextAlignment::Left, VerticalAlignment::Middle);
        draw_text_into_pixmap(pixmap, "前台为 Antigravity / VS Code / Cursor 时呈现，切出时隐匿", cx + 14.0 * s, r32_y + 19.0 * s, 260.0 * s, 15.0 * s, 11.0 * s, 0.0, text_sec, 1.0, TextAlignment::Left, VerticalAlignment::Middle);
        self.render_toggle(pixmap, cx + cw - 52.0 * s, r32_y + 8.0 * s, s, self.temp_settings.menu_config.ide_only, is_win, is_dark, accent, "ide_only");
        pixmap.fill_rounded_rect_sdf(cx + 14.0 * s, r32_y + 37.0 * s, cw - 28.0 * s, 1.0 * s, 0.0, row_div, 0.0, [0, 0, 0, 0]);

        // Row 3.3: 完全穿透免打扰
        let r33_y = r32_y + 38.0 * s;
        draw_text_into_pixmap(pixmap, "完全穿透免打扰", cx + 14.0 * s, r33_y + 2.0 * s, 200.0 * s, 17.0 * s, 13.0 * s, 0.0, text_pri, 1.0, TextAlignment::Left, VerticalAlignment::Middle);
        draw_text_into_pixmap(pixmap, "猫猫处于只读透明状态，鼠标点击直接透传到底层应用", cx + 14.0 * s, r33_y + 19.0 * s, 260.0 * s, 15.0 * s, 11.0 * s, 0.0, text_sec, 1.0, TextAlignment::Left, VerticalAlignment::Middle);
        self.render_toggle(pixmap, cx + cw - 52.0 * s, r33_y + 8.0 * s, s, self.temp_settings.dnd_passthrough, is_win, is_dark, accent, "dnd");
        pixmap.fill_rounded_rect_sdf(cx + 14.0 * s, r33_y + 37.0 * s, cw - 28.0 * s, 1.0 * s, 0.0, row_div, 0.0, [0, 0, 0, 0]);

        // Row 3.4: 动作音效
        let r34_y = r33_y + 38.0 * s;
        draw_text_into_pixmap(pixmap, "动作音效", cx + 14.0 * s, r34_y + 2.0 * s, 200.0 * s, 17.0 * s, 13.0 * s, 0.0, text_pri, 1.0, TextAlignment::Left, VerticalAlignment::Middle);
        draw_text_into_pixmap(pixmap, "播放挂件拖拽释放、抚摸互动与提示音效", cx + 14.0 * s, r34_y + 19.0 * s, 260.0 * s, 15.0 * s, 11.0 * s, 0.0, text_sec, 1.0, TextAlignment::Left, VerticalAlignment::Middle);
        self.render_toggle(pixmap, cx + cw - 52.0 * s, r34_y + 8.0 * s, s, self.temp_settings.sound_enabled, is_win, is_dark, accent, "sound");

        // Clear clip rect
        pixmap.clear_clip_rect();

        // Native floating scrollbar
        self.render_native_scrollbar(pixmap, cx + cw - 3.5 * s, vp_y, vp_h, 450.0 * s, self.general_scroll * s, s, is_dark);
    }

    #[allow(clippy::too_many_arguments)]
    fn render_autostart_tab(
        &self,
        pixmap: &mut Pixmap,
        cx: f32,
        cw: f32,
        s: f32,
        bg_card: [u8; 4],
        border_card: [u8; 4],
        row_div: [u8; 4],
        text_pri: [u8; 3],
        text_sec: [u8; 3],
        accent: [u8; 4],
        card_radius: f32,
        is_win: bool,
        is_dark: bool,
    ) {
        let base_y = 28.0 * s;
        draw_text_into_pixmap(pixmap, "自启动与更新", cx, base_y, cw, 22.0 * s, 16.0 * s, 0.35, text_pri, 1.0, TextAlignment::Left, VerticalAlignment::Middle);
        draw_text_into_pixmap(pixmap, "管理系统登录自启动服务与检查最新桌面伴侣版本", cx, base_y + 24.0 * s, cw, 16.0 * s, 11.5 * s, 0.0, text_sec, 1.0, TextAlignment::Left, VerticalAlignment::Middle);

        let c1_y = 80.0 * s;
        let c1_h = 92.0 * s;
        pixmap.fill_rounded_rect_sdf(cx, c1_y, cw, c1_h, card_radius, bg_card, 1.0 * s, border_card);

        // Row 1: 开机自启动
        let r1_y = c1_y + 4.0 * s;
        draw_text_into_pixmap(pixmap, "开机时自动启动", cx + 14.0 * s, r1_y + 4.0 * s, 200.0 * s, 18.0 * s, 13.0 * s, 0.0, text_pri, 1.0, TextAlignment::Left, VerticalAlignment::Middle);
        draw_text_into_pixmap(pixmap, "在系统开机登录后自动在托盘区唤起桌宠，陪伴编码", cx + 14.0 * s, r1_y + 22.0 * s, 260.0 * s, 16.0 * s, 11.0 * s, 0.0, text_sec, 1.0, TextAlignment::Left, VerticalAlignment::Middle);
        self.render_toggle(pixmap, cx + cw - 52.0 * s, r1_y + 11.0 * s, s, self.autostart_enabled, is_win, is_dark, accent, "autostart");
        pixmap.fill_rounded_rect_sdf(cx + 14.0 * s, r1_y + 43.0 * s, cw - 28.0 * s, 1.0 * s, 0.0, row_div, 0.0, [0, 0, 0, 0]);

        // Row 2: 检查更新
        let r2_y = r1_y + 44.0 * s;
        draw_text_into_pixmap(pixmap, "软件版本检查", cx + 14.0 * s, r2_y + 4.0 * s, 200.0 * s, 18.0 * s, 13.0 * s, 0.0, text_pri, 1.0, TextAlignment::Left, VerticalAlignment::Middle);
        draw_text_into_pixmap(pixmap, "当前版本：v0.4.1", cx + 14.0 * s, r2_y + 22.0 * s, 260.0 * s, 16.0 * s, 11.0 * s, 0.0, text_sec, 1.0, TextAlignment::Left, VerticalAlignment::Middle);
        let btn_x = cx + cw - 88.0 * s;
        self.render_button(pixmap, btn_x, r2_y + 10.0 * s, 76.0 * s, 24.0 * s, "检查更新", s, is_win, is_dark, border_card, text_pri, "check_update");
    }

    #[allow(clippy::too_many_arguments)]
    fn render_skins_tab(
        &self,
        pixmap: &mut Pixmap,
        cx: f32,
        cw: f32,
        s: f32,
        bg_card: [u8; 4],
        _border_card: [u8; 4],
        text_pri: [u8; 3],
        text_sec: [u8; 3],
        accent: [u8; 4],
        card_radius: f32,
        _is_win: bool,
    ) {
        let base_y = 28.0 * s;
        draw_text_into_pixmap(pixmap, "皮肤管理", cx, base_y, cw, 22.0 * s, 16.0 * s, 0.35, text_pri, 1.0, TextAlignment::Left, VerticalAlignment::Middle);
        draw_text_into_pixmap(pixmap, "选择个性化猫咪伴侣皮肤与动作外观", cx, base_y + 24.0 * s, cw, 16.0 * s, 11.5 * s, 0.0, text_sec, 1.0, TextAlignment::Left, VerticalAlignment::Middle);

        let card_w = 180.0 * s;
        let card_h = 160.0 * s;
        let card_y = 80.0 * s;
        let card_x = cx;

        let skin_id = "skin_0";
        let is_press = self.pressed_id.as_deref() == Some(skin_id);
        let border_c = if is_press { accent } else { accent };

        pixmap.fill_rounded_rect_sdf(card_x, card_y, card_w, card_h, card_radius, bg_card, 2.0 * s, border_c);

        let tf = super::sprite::SpriteTransform {
            anchor_x: (card_x + (card_w * 0.5)) / s,
            anchor_y: (card_y + 86.0 * s) / s,
            base_size: 68.0,
            scale_x: 1.0,
            scale_y: 1.0,
            flip_x: 1.0,
            rotation_rad: 0.0,
            gaze_offset_x: 0.0,
            gaze_offset_y: 0.0,
            opacity: 1.0,
            draw_ground_shadow: true,
        };
        let _ = self.cat_sprite.draw(pixmap, &tf, s);

        draw_text_into_pixmap(pixmap, "经典紫毛猫", card_x + 8.0 * s, card_y + 92.0 * s, card_w - 16.0 * s, 18.0 * s, 13.0 * s, 0.2, text_pri, 1.0, TextAlignment::Center, VerticalAlignment::Middle);
        draw_text_into_pixmap(pixmap, "原装傲娇大肥猫", card_x + 8.0 * s, card_y + 114.0 * s, card_w - 16.0 * s, 16.0 * s, 10.5 * s, 0.0, text_sec, 0.85, TextAlignment::Center, VerticalAlignment::Middle);
        draw_text_into_pixmap(pixmap, "当前使用中", card_x + 8.0 * s, card_y + 134.0 * s, card_w - 16.0 * s, 16.0 * s, 10.5 * s, 0.3, [accent[0], accent[1], accent[2]], 1.0, TextAlignment::Center, VerticalAlignment::Middle);
    }

    #[allow(clippy::too_many_arguments)]
    fn render_dialogue_tab(
        &self,
        pixmap: &mut Pixmap,
        cx: f32,
        cw: f32,
        s: f32,
        bg_card: [u8; 4],
        border_card: [u8; 4],
        text_pri: [u8; 3],
        text_sec: [u8; 3],
        accent: [u8; 4],
        card_radius: f32,
        is_win: bool,
        is_dark: bool,
    ) {
        let base_y = 24.0 * s;
        // Header Title
        draw_text_into_pixmap(pixmap, "台词与问候", cx, base_y, cw, 22.0 * s, 16.0 * s, 0.35, text_pri, 1.0, TextAlignment::Left, VerticalAlignment::Middle);
        // Header Subtitle (Ends at y = 64.0 * s)
        draw_text_into_pixmap(pixmap, "绑定 Antigravity 模型状态与桌面交互台词库，支持直接增删修改", cx, base_y + 24.0 * s, cw, 16.0 * s, 11.5 * s, 0.0, text_sec, 1.0, TextAlignment::Left, VerticalAlignment::Middle);

        // 1. Five Category Pills Navigation (Segmented container at y = 82.0 * s, h = 32.0 * s)
        // Safe clear gap of 18px between subtitle (64.0) and pills (82.0) to eliminate any overlap
        let pill_y = 82.0 * s;
        let pill_h = 32.0 * s;
        let pill_w = cw / 5.0;

        // Container track for segmented control
        let seg_bg = if is_win {
            if is_dark { [32, 32, 32, 255] } else { [240, 240, 240, 255] }
        } else {
            if is_dark { [36, 36, 38, 255] } else { [236, 236, 238, 255] }
        };
        pixmap.fill_rounded_rect_sdf(cx, pill_y, cw, pill_h, 6.0 * s, seg_bg, 1.0 * s, border_card);

        let cats = [
            ("待机日常", self.dialogue_bank.idle.len()),
            ("互动抚摸", self.dialogue_bank.petting.len()),
            ("深度思考", self.dialogue_bank.thinking.len()),
            ("任务成功", self.dialogue_bank.success.len()),
            ("任务遇阻", self.dialogue_bank.error.len()),
        ];

        for (i, (_tag, count)) in cats.iter().enumerate() {
            let px = cx + (i as f32) * pill_w;
            let is_sel = self.dialogue_cat == i;
            let cat_id = format!("diag_cat_{}", i);
            let is_hover = self.hovered_id.as_deref() == Some(&cat_id);

            let pill_bg = if is_sel {
                if is_win {
                    if is_dark { [52, 52, 52, 255] } else { [255, 255, 255, 255] }
                } else {
                    accent
                }
            } else if is_hover {
                if is_dark { [44, 44, 44, 255] } else { [248, 248, 248, 255] }
            } else {
                [0, 0, 0, 0]
            };

            let stroke = if is_sel && is_win {
                accent
            } else {
                [0, 0, 0, 0]
            };

            if pill_bg[3] > 0 || stroke[3] > 0 {
                pixmap.fill_rounded_rect_sdf(px + 2.0 * s, pill_y + 2.0 * s, pill_w - 4.0 * s, pill_h - 4.0 * s, 4.0 * s, pill_bg, if is_sel && is_win { 1.2 * s } else { 0.0 }, stroke);
            }

            let label = match i {
                0 => format!("待机 ({})", count),
                1 => format!("抚摸 ({})", count),
                2 => format!("思考 ({})", count),
                3 => format!("成功 ({})", count),
                _ => format!("遇阻 ({})", count),
            };
            let txt_c = if is_sel {
                if is_win { [accent[0], accent[1], accent[2]] } else { [255, 255, 255] }
            } else {
                text_sec
            };
            draw_text_into_pixmap(pixmap, &label, px + 2.0 * s, pill_y, pill_w - 4.0 * s, pill_h, 11.5 * s, if is_sel { 0.3 } else { 0.0 }, txt_c, 1.0, TextAlignment::Center, VerticalAlignment::Middle);
        }

        // 2. Action Bar: Subtitle on left, "+ 添加台词" button on right (y = 126.0 * s, h = 28.0 * s)
        let bar_y = 126.0 * s;
        let bar_h = 28.0 * s;
        let cat_desc = match self.dialogue_cat {
            0 => "空闲时随时间周期性随机说出的台词",
            1 => "鼠标双击或在猫头连续抚摸摩擦时触发",
            2 => "模型在执行深度推理或调用工具流时",
            3 => "Antigravity 成功完成编译、测试或代码交付时",
            _ => "代码构建失败或运行异常时安慰用户的提示",
        };
        draw_text_into_pixmap(pixmap, cat_desc, cx, bar_y, cw - 100.0 * s, bar_h, 11.5 * s, 0.0, text_sec, 0.9, TextAlignment::Left, VerticalAlignment::Middle);

        let add_btn_w = 92.0 * s;
        let add_btn_h = bar_h;
        let add_btn_x = cx + cw - add_btn_w;
        self.render_button(pixmap, add_btn_x, bar_y, add_btn_w, add_btn_h, "+ 添加台词", s, is_win, is_dark, border_card, text_pri, "add_dialogue");

        // 3. Scrollable List of Dialogue Lines (strictly viewport-clipped)
        let list_y = 160.0 * s;
        let list_bottom = (SETTINGS_WIN_H as f32 - 52.0) * s;
        let vp_h = list_bottom - list_y;
        let lines = self.active_dialogues();

        pixmap.set_clip_rect(cx - 2.0 * s, list_y, cw + 4.0 * s, vp_h);

        for (i, line) in lines.iter().enumerate() {
            let row_y = list_y + (i as f32 * 42.0 * s) - self.dialogue_scroll * s;
            if row_y + 34.0 * s < list_y || row_y > list_bottom {
                continue; // completely off-screen
            }

            let is_editing = self.editing_dialogue_idx == Some(i);
            let edit_id = format!("edit_diag_{}", i);
            let del_id = format!("del_diag_{}", i);

            let is_hover_row = self.hovered_id.as_deref() == Some(&edit_id);
            let row_bg = if is_editing {
                if is_dark { [52, 52, 52, 255] } else { [255, 255, 255, 255] }
            } else if is_hover_row {
                if is_dark { [48, 48, 48, 255] } else { [250, 250, 250, 255] }
            } else {
                bg_card
            };

            let row_stroke = if is_editing {
                accent
            } else {
                border_card
            };

            // Card row
            pixmap.fill_rounded_rect_sdf(cx, row_y, cw, 34.0 * s, card_radius, row_bg, if is_editing { 1.5 * s } else { 1.0 * s }, row_stroke);

            // Display text - exactly centered vertically with the row and delete button
            let text_val = if is_editing {
                format!("{}|", self.editing_buffer)
            } else {
                line.clone()
            };
            let text_color = if is_editing {
                if is_dark { [255, 255, 255] } else { [0, 0, 0] }
            } else {
                text_pri
            };
            draw_text_into_pixmap(pixmap, &text_val, cx + 12.0 * s, row_y, cw - 66.0 * s, 34.0 * s, 12.0 * s, 0.0, text_color, 1.0, TextAlignment::Left, VerticalAlignment::Middle);

            // "删除" button - aligned on the exact same horizontal baseline
            let del_w = 46.0 * s;
            let del_h = 24.0 * s;
            let del_x = cx + cw - 52.0 * s;
            let del_y = row_y + 5.0 * s;

            let is_del_hover = self.hovered_id.as_deref() == Some(&del_id);
            let is_del_press = self.pressed_id.as_deref() == Some(&del_id);
            let del_bg = if is_del_press {
                [176, 32, 18, 40]
            } else if is_del_hover {
                [196, 43, 28, 25]
            } else {
                [0, 0, 0, 0]
            };
            if del_bg[3] > 0 {
                pixmap.fill_rounded_rect_sdf(del_x, del_y, del_w, del_h, 3.0 * s, del_bg, 0.0, [0, 0, 0, 0]);
            }
            let del_txt_c = if is_del_hover || is_del_press {
                [220, 60, 60]
            } else {
                text_sec
            };
            draw_text_into_pixmap(pixmap, "删除", del_x, del_y, del_w, del_h, 10.5 * s, 0.0, del_txt_c, 0.9, TextAlignment::Center, VerticalAlignment::Middle);
        }

        pixmap.clear_clip_rect();

        let total_content_h = (lines.len() as f32 * 42.0 * s).max(vp_h);
        self.render_native_scrollbar(pixmap, cx + cw - 3.5 * s, list_y, vp_h, total_content_h, self.dialogue_scroll * s, s, is_dark);
    }

    #[allow(clippy::too_many_arguments)]
    fn render_menu_tab(
        &self,
        pixmap: &mut Pixmap,
        cx: f32,
        cw: f32,
        s: f32,
        bg_card: [u8; 4],
        border_card: [u8; 4],
        row_div: [u8; 4],
        text_pri: [u8; 3],
        text_sec: [u8; 3],
        accent: [u8; 4],
        card_radius: f32,
        is_win: bool,
        is_dark: bool,
    ) {
        let base_y = 28.0 * s;
        // Fixed Header
        draw_text_into_pixmap(pixmap, "右键菜单配置", cx, base_y, cw, 22.0 * s, 16.0 * s, 0.35, text_pri, 1.0, TextAlignment::Left, VerticalAlignment::Middle);
        draw_text_into_pixmap(pixmap, "勾选或隐藏桌宠右键控制面板中的快捷功能项", cx, base_y + 24.0 * s, cw, 16.0 * s, 11.5 * s, 0.0, text_sec, 1.0, TextAlignment::Left, VerticalAlignment::Middle);

        let menu_items = [
            ("打开 Antigravity", self.temp_settings.menu_config.open_antigravity),
            ("偏好设置...", self.temp_settings.menu_config.settings),
            ("换一句傲娇台词", self.temp_settings.menu_config.random_dialogue),
            ("摸摸猫猫头", self.temp_settings.menu_config.pet_action),
            ("额度使用卡片", self.temp_settings.menu_config.quota),
            ("免打扰模式", self.temp_settings.menu_config.dnd),
            ("互动音效", self.temp_settings.menu_config.sound),
            ("体型缩放调节", self.temp_settings.menu_config.zoom),
            ("重力下落吸附", self.temp_settings.menu_config.gravity),
        ];

        let vp_y = 74.0 * s;
        let vp_bottom = (SETTINGS_WIN_H as f32 - 52.0) * s;
        let vp_h = vp_bottom - vp_y;

        pixmap.set_clip_rect(cx - 2.0 * s, vp_y, cw + 4.0 * s, vp_h);

        let c1_y = vp_y + 2.0 * s - self.scroll_y * s;
        let c1_h = 398.0 * s;
        pixmap.fill_rounded_rect_sdf(cx, c1_y, cw, c1_h, card_radius, bg_card, 1.0 * s, border_card);

        for (i, (title, checked)) in menu_items.into_iter().enumerate() {
            let row_y = c1_y + 4.0 * s + (i as f32) * 44.0 * s;
            draw_text_into_pixmap(pixmap, title, cx + 14.0 * s, row_y, cw - 80.0 * s, 44.0 * s, 12.5 * s, 0.0, text_pri, 1.0, TextAlignment::Left, VerticalAlignment::Middle);
            self.render_checkbox(pixmap, cx + cw - 38.0 * s, row_y + 13.0 * s, s, checked, accent, is_win, is_dark);
            if i < 8 {
                pixmap.fill_rounded_rect_sdf(cx + 14.0 * s, row_y + 43.0 * s, cw - 28.0 * s, 1.0 * s, 0.0, row_div, 0.0, [0, 0, 0, 0]);
            }
        }

        pixmap.clear_clip_rect();

        let total_content_h = c1_h + 12.0 * s;
        self.render_native_scrollbar(pixmap, cx + cw - 3.5 * s, vp_y, vp_h, total_content_h, self.scroll_y * s, s, is_dark);
    }

    #[allow(clippy::too_many_arguments)]
    fn render_native_scrollbar(
        &self,
        pixmap: &mut Pixmap,
        track_x: f32,
        vp_y: f32,
        vp_h: f32,
        content_h: f32,
        scroll_y: f32,
        s: f32,
        is_dark: bool,
    ) {
        if content_h <= vp_h || vp_h <= 0.0 {
            return;
        }
        let max_scroll = (content_h - vp_h).max(1.0);
        let pct = (scroll_y / max_scroll).clamp(0.0, 1.0);
        let thumb_h = ((vp_h / content_h) * vp_h).clamp(28.0 * s, vp_h - 12.0 * s);
        let available_track = vp_h - thumb_h;
        let thumb_y = vp_y + pct * available_track;
        let thumb_w = 3.5 * s;
        let thumb_r = 1.75 * s;

        let thumb_color = if is_dark {
            [255, 255, 255, 75]
        } else {
            [0, 0, 0, 75]
        };
        pixmap.fill_rounded_rect_sdf(track_x, thumb_y, thumb_w, thumb_h, thumb_r, thumb_color, 0.0, [0, 0, 0, 0]);
    }

    #[allow(clippy::too_many_arguments)]
    fn render_button(
        &self,
        pixmap: &mut Pixmap,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        text: &str,
        s: f32,
        is_win: bool,
        is_dark: bool,
        border_card: [u8; 4],
        text_pri: [u8; 3],
        button_id: &str,
    ) {
        let r = if is_win { 4.0 * s } else { 6.0 * s };
        let is_hovered = self.hovered_id.as_deref() == Some(button_id);
        let is_pressed = self.pressed_id.as_deref() == Some(button_id);

        let bg = if is_win {
            if is_pressed {
                if is_dark { [65, 65, 65, 255] } else { [234, 235, 237, 255] }
            } else if is_hovered {
                if is_dark { [52, 52, 52, 255] } else { [246, 247, 249, 255] }
            } else {
                if is_dark { [43, 43, 43, 255] } else { [251, 252, 253, 255] }
            }
        } else if is_pressed {
            if is_dark { [70, 70, 74, 255] } else { [220, 220, 225, 255] }
        } else if is_hovered {
            if is_dark { [58, 58, 60, 255] } else { [235, 235, 240, 255] }
        } else {
            border_card
        };

        let stroke = if is_win {
            if is_pressed {
                if is_dark { [255, 255, 255, 45] } else { [180, 180, 180, 255] }
            } else if is_hovered {
                if is_dark { [255, 255, 255, 32] } else { [195, 195, 195, 255] }
            } else {
                if is_dark { [255, 255, 255, 22] } else { [209, 209, 209, 255] }
            }
        } else {
            border_card
        };

        let press_offset = if is_pressed { 1.0 * s } else { 0.0 };
        pixmap.fill_rounded_rect_sdf(x, y, w, h, r, bg, 1.0 * s, stroke);
        draw_text_into_pixmap(pixmap, text, x, y + press_offset, w, h, 11.5 * s, 0.0, text_pri, 1.0, TextAlignment::Center, VerticalAlignment::Middle);
    }

    #[allow(clippy::too_many_arguments)]
    fn render_primary_button(
        &self,
        pixmap: &mut Pixmap,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        text: &str,
        s: f32,
        accent: [u8; 4],
        button_id: &str,
    ) {
        let r = 4.0 * s;
        let is_hovered = self.hovered_id.as_deref() == Some(button_id);
        let is_pressed = self.pressed_id.as_deref() == Some(button_id);

        let bg = if is_pressed {
            let mut c = accent;
            c[0] = (c[0] as f32 * 0.85) as u8;
            c[1] = (c[1] as f32 * 0.85) as u8;
            c[2] = (c[2] as f32 * 0.85) as u8;
            c
        } else if is_hovered {
            let mut c = accent;
            c[0] = (c[0] as f32 * 1.1).min(255.0) as u8;
            c[1] = (c[1] as f32 * 1.1).min(255.0) as u8;
            c[2] = (c[2] as f32 * 1.1).min(255.0) as u8;
            c
        } else {
            accent
        };

        let press_offset = if is_pressed { 1.0 * s } else { 0.0 };
        pixmap.fill_rounded_rect_sdf(x, y, w, h, r, bg, 0.0, [0, 0, 0, 0]);
        draw_text_into_pixmap(pixmap, text, x, y + press_offset, w, h, 12.0 * s, 0.25, [255, 255, 255], 1.0, TextAlignment::Center, VerticalAlignment::Middle);
    }

    #[allow(clippy::too_many_arguments)]
    fn render_toggle(
        &self,
        pixmap: &mut Pixmap,
        x: f32,
        y: f32,
        s: f32,
        on: bool,
        is_win: bool,
        is_dark: bool,
        accent: [u8; 4],
        toggle_id: &str,
    ) {
        let is_pressed = self.pressed_id.as_deref() == Some(toggle_id);
        let is_hovered = self.hovered_id.as_deref() == Some(toggle_id);

        if is_win {
            let w = 40.0 * s;
            let h = 20.0 * s;
            let r = 10.0 * s;

            if on {
                let track_bg = if is_pressed {
                    let mut col = accent;
                    col[0] = (col[0] as f32 * 0.85) as u8;
                    col[1] = (col[1] as f32 * 0.85) as u8;
                    col[2] = (col[2] as f32 * 0.85) as u8;
                    col
                } else if is_hovered {
                    let mut col = accent;
                    col[0] = (col[0] as f32 * 1.08).min(255.0) as u8;
                    col[1] = (col[1] as f32 * 1.08).min(255.0) as u8;
                    col[2] = (col[2] as f32 * 1.08).min(255.0) as u8;
                    col
                } else {
                    accent
                };
                pixmap.fill_rounded_rect_sdf(x, y, w, h, r, track_bg, 0.0, [0, 0, 0, 0]);
                let knob_color = if is_dark { [0, 0, 0, 255] } else { [255, 255, 255, 255] };
                let knob_r = if is_pressed { 6.8 * s } else { 6.0 * s };
                let knob_x = x + w - 10.0 * s;
                pixmap.fill_circle_sdf(knob_x, y + 10.0 * s, knob_r, knob_color, 0.0, [0, 0, 0, 0]);
            } else {
                let track_bg = if is_pressed {
                    if is_dark { [48, 48, 48, 255] } else { [232, 232, 232, 255] }
                } else if is_hovered {
                    if is_dark { [44, 44, 44, 255] } else { [238, 238, 238, 255] }
                } else {
                    if is_dark { [40, 40, 40, 255] } else { [243, 243, 243, 255] }
                };
                let stroke_col = if is_dark { [158, 158, 158, 220] } else { [134, 134, 134, 220] };
                pixmap.fill_rounded_rect_sdf(x, y, w, h, r, track_bg, 1.2 * s, stroke_col);
                let knob_color = if is_dark { [206, 206, 206, 255] } else { [93, 93, 93, 255] };
                let knob_r = if is_pressed { 5.8 * s } else { 5.0 * s };
                let knob_x = x + 10.0 * s;
                pixmap.fill_circle_sdf(knob_x, y + 10.0 * s, knob_r, knob_color, 0.0, [0, 0, 0, 0]);
            }
        } else {
            let w = 38.0 * s;
            let h = 22.0 * s;
            let r = 11.0 * s;
            let track_col = if on { [52, 199, 89, 255] } else { [142, 142, 147, 120] };
            pixmap.fill_rounded_rect_sdf(x, y, w, h, r, track_col, 0.0, [0, 0, 0, 0]);
            let knob_x = if on { x + w - 11.0 * s } else { x + 11.0 * s };
            let knob_r = if is_pressed { 8.8 * s } else { 8.0 * s };
            pixmap.fill_circle_sdf(knob_x, y + 11.0 * s, knob_r, [255, 255, 255, 255], 0.0, [0, 0, 0, 0]);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_slider(&self, pixmap: &mut Pixmap, x: f32, y: f32, w: f32, s: f32, pct: f32, accent: [u8; 4], is_win: bool, is_dark: bool) {
        let track_h = 4.0 * s;
        let track_bg = if is_win {
            if is_dark { [255, 255, 255, 50] } else { [0, 0, 0, 50] }
        } else {
            [142, 142, 147, 80]
        };
        pixmap.fill_rounded_rect_sdf(x, y, w, track_h, 2.0 * s, track_bg, 0.0, [0, 0, 0, 0]);
        let filled_w = w * pct.clamp(0.0, 1.0);
        if filled_w > 1.0 {
            pixmap.fill_rounded_rect_sdf(x, y, filled_w, track_h, 2.0 * s, accent, 0.0, [0, 0, 0, 0]);
        }
        let knob_x = x + filled_w;
        if is_win {
            pixmap.fill_circle_sdf(knob_x, y + 2.0 * s, 7.5 * s, if is_dark { [32, 32, 32, 255] } else { [255, 255, 255, 255] }, 2.0 * s, accent);
        } else {
            pixmap.fill_circle_sdf(knob_x, y + 2.0 * s, 7.0 * s, [255, 255, 255, 255], 0.5 * s, [0, 0, 0, 40]);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_checkbox(&self, pixmap: &mut Pixmap, x: f32, y: f32, s: f32, checked: bool, accent: [u8; 4], is_win: bool, is_dark: bool) {
        let size = 18.0 * s;
        let r = 4.0 * s;
        if checked {
            pixmap.fill_rounded_rect_sdf(x, y, size, size, r, accent, 0.0, [0, 0, 0, 0]);
            let thick = 1.8 * s;
            let check_color = if is_win && is_dark { [0, 0, 0, 255] } else { [255, 255, 255, 255] };
            let p1_x = x + 4.5 * s;
            let p1_y = y + 9.2 * s;
            let p2_x = x + 7.4 * s;
            let p2_y = y + 12.8 * s;
            let p3_x = x + 13.5 * s;
            let p3_y = y + 5.2 * s;
            pixmap.draw_line_sdf(p1_x, p1_y, p2_x, p2_y, thick, check_color);
            pixmap.draw_line_sdf(p2_x, p2_y, p3_x, p3_y, thick, check_color);
        } else {
            let stroke_col = if is_win {
                if is_dark { [158, 158, 158, 200] } else { [134, 134, 134, 200] }
            } else {
                [142, 142, 147, 180]
            };
            pixmap.fill_rounded_rect_sdf(x, y, size, size, r, [0, 0, 0, 0], 1.2 * s, stroke_col);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_ui_layout_and_interaction() {
        let mut ui = SettingsUi::new(PetSettings::default());
        let mut pixmap = Pixmap::new(SETTINGS_WIN_W as u32, SETTINGS_WIN_H as u32);
        ui.render(&mut pixmap, 1.0);

        assert_eq!(ui.active_tab, SettingsTab::General);
        assert_eq!(ui.temp_settings.scale, 1.0);

        // Test floor mode cycle in General tab
        let cw_right = SETTINGS_WIN_W as f32 - 28.0;
        ui.mouse_pos = (cw_right - 40.0, 190.0);
        let floor_act = ui.handle_mouse_down(1.0);
        assert_eq!(floor_act, Some(SettingsAction::Redraw));
        assert_eq!(ui.temp_settings.floor_mode, "auto");

        // Test smooth mouse wheel over scale slider
        ui.mouse_pos = (cw_right - 80.0, 145.0);
        assert!(ui.handle_mouse_wheel(1.0));
        assert!((ui.temp_settings.scale - 1.05).abs() < 0.01);

        // Click sidebar tab 4 (Menu)
        ui.mouse_pos = (50.0, 210.0);
        let action = ui.handle_mouse_down(1.0);
        assert!(matches!(action, Some(SettingsAction::SelectTab(SettingsTab::Menu))));
        assert_eq!(ui.active_tab, SettingsTab::Menu);

        // Click sidebar tab 2 (Skins)
        ui.mouse_pos = (50.0, 138.0);
        let action = ui.handle_mouse_down(1.0);
        assert!(matches!(action, Some(SettingsAction::SelectTab(SettingsTab::Skins))));
        assert_eq!(ui.active_tab, SettingsTab::Skins);

        // Test Dialogue Tab direct in-place editing
        ui.active_tab = SettingsTab::Dialogue;
        ui.render(&mut pixmap, 1.0);

        // Switch to category 1 (petting) via pill click (pills are at y = 82.0..114.0)
        let cw_left = SIDEBAR_W + 28.0;
        let cw_right = SETTINGS_WIN_W as f32 - 28.0;
        let pill_w = (cw_right - cw_left) / 5.0;
        ui.mouse_pos = (cw_left + pill_w * 1.5, 95.0);
        let cat_action = ui.handle_mouse_down(1.0);
        assert_eq!(cat_action, Some(SettingsAction::Redraw));
        assert_eq!(ui.dialogue_cat, 1);

        // Click "+ 添加台词" (action bar is at y = 126.0..154.0)
        ui.mouse_pos = (cw_right - 40.0, 138.0);
        let add_action = ui.handle_mouse_down(1.0);
        assert_eq!(add_action, Some(SettingsAction::Redraw));
        assert!(ui.editing_dialogue_idx.is_some());

        // Type characters
        assert!(ui.handle_char('!'));
        assert_eq!(ui.editing_buffer, "新台词条目!");

        // Test bottom footer "取消" and "确定"
        ui.mouse_pos = (SETTINGS_WIN_W as f32 - 140.0, SETTINGS_WIN_H as f32 - 20.0);
        let cancel_act = ui.handle_mouse_down(1.0);
        assert_eq!(cancel_act, Some(SettingsAction::Cancel));

        ui.mouse_pos = (SETTINGS_WIN_W as f32 - 50.0, SETTINGS_WIN_H as f32 - 20.0);
        let save_act = ui.handle_mouse_down(1.0);
        assert!(matches!(save_act, Some(SettingsAction::Save(_))));

        #[cfg(target_os = "macos")]
        {
            // Test macOS traffic lights area (0..72 x 0..36) returns None to preserve native close/min/zoom
            ui.mouse_pos = (30.0, 20.0);
            assert_eq!(ui.handle_mouse_down(1.0), None);

            // Test macOS title drag area (outside traffic lights)
            ui.mouse_pos = (300.0, 20.0);
            assert_eq!(ui.handle_mouse_down(1.0), Some(SettingsAction::BeginTitleDrag));
        }

        #[cfg(target_os = "windows")]
        {
            // Test Windows close button
            ui.mouse_pos = (760.0, 15.0);
            let close_act = ui.handle_mouse_down(1.0);
            assert_eq!(close_act, Some(SettingsAction::Cancel));

            // Test Windows minimize button
            ui.mouse_pos = (710.0, 15.0);
            let min_act = ui.handle_mouse_down(1.0);
            assert_eq!(min_act, Some(SettingsAction::MinimizeWindow));

            // Test Windows title drag area
            ui.mouse_pos = (300.0, 15.0);
            let title_act = ui.handle_mouse_down(1.0);
            assert_eq!(title_act, Some(SettingsAction::BeginTitleDrag));
        }
    }

    #[test]
    fn test_winui_theme_and_toggle_rendering() {
        let mut ui = SettingsUi::new(PetSettings::default());
        let mut pixmap = Pixmap::new(SETTINGS_WIN_W as u32, SETTINGS_WIN_H as u32);

        // Test light mode render
        ui.temp_settings.theme = "light".to_string();
        ui.render(&mut pixmap, 1.0);
        assert!(!ui.is_dark_mode());

        // Test dark mode render
        ui.temp_settings.theme = "dark".to_string();
        ui.render(&mut pixmap, 1.0);
        assert!(ui.is_dark_mode());
    }
}
