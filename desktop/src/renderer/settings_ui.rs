use super::text::{draw_text_into_pixmap, TextAlignment};
use super::Pixmap;
use crate::state_watcher::PetSettings;
use std::time::Instant;

pub const SETTINGS_WIN_W: f64 = 700.0;
pub const SETTINGS_WIN_H: f64 = 530.0;
const SIDEBAR_W: f32 = 186.0;
const FOOTER_H: f32 = 50.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SettingsTab {
    General = 0,
    Autostart = 1,
    Skins = 2,
    Dialogue = 3,
    Menu = 4,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum SettingsAction {
    None,
    Save(PetSettings),
    Cancel,
    ResetPosition,
    CheckUpdate,
    SelectTab(SettingsTab),
    Redraw,
}

/// Native pure 2D CPU software-rendered Preferences Settings Window UI.
/// Zero WebKit, zero Node.js, and under 2MB RAM footprint.
pub struct SettingsUi {
    pub active_tab: SettingsTab,
    pub temp_settings: PetSettings,
    #[allow(dead_code)]
    pub original_settings: PetSettings,
    pub autostart_enabled: bool,
    pub scroll_y: f32,
    pub toast: Option<(String, Instant)>,
    #[allow(dead_code)]
    pub hovered_id: Option<String>,
    pub dragging_slider: Option<String>,
    pub mouse_pos: (f32, f32),
    pub cat_sprite: super::sprite::CatSprite,
}

impl SettingsUi {
    pub fn new(settings: PetSettings) -> Self {
        Self {
            active_tab: SettingsTab::General,
            temp_settings: settings.clone(),
            original_settings: settings,
            autostart_enabled: false,
            scroll_y: 0.0,
            toast: None,
            hovered_id: None,
            dragging_slider: None,
            mouse_pos: (0.0, 0.0),
            cat_sprite: super::sprite::CatSprite::load_default(),
        }
    }

    pub fn open(&mut self, current_settings: &PetSettings) {
        self.temp_settings = current_settings.clone();
        self.original_settings = current_settings.clone();
        self.scroll_y = 0.0;
        self.toast = None;
        self.dragging_slider = None;
    }

    pub fn show_toast(&mut self, msg: &str) {
        self.toast = Some((msg.to_string(), Instant::now()));
    }

    pub fn is_dark_mode(&self) -> bool {
        match self.temp_settings.theme.as_str() {
            "dark" => true,
            "light" => false,
            _ => {
                #[cfg(target_os = "macos")]
                {
                    macos_is_dark_mode()
                }
                #[cfg(not(target_os = "macos"))]
                {
                    false
                }
            }
        }
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

        // 1. Sidebar clicks
        if mx < SIDEBAR_W && my >= 76.0 {
            let tab_idx = ((my - 78.0) / 36.0).floor() as i32;
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
                return Some(SettingsAction::SelectTab(tab));
            }
            return None;
        }

        // 2. Footer clicks
        if my >= (SETTINGS_WIN_H as f32 - FOOTER_H) {
            let win_w = SETTINGS_WIN_W as f32;
            let cancel_rect = (win_w - 170.0, win_w - 96.0, SETTINGS_WIN_H as f32 - 40.0, SETTINGS_WIN_H as f32 - 12.0);
            let save_rect = (win_w - 90.0, win_w - 14.0, SETTINGS_WIN_H as f32 - 40.0, SETTINGS_WIN_H as f32 - 12.0);

            if mx >= cancel_rect.0 && mx <= cancel_rect.1 && my >= cancel_rect.2 && my <= cancel_rect.3 {
                return Some(SettingsAction::Cancel);
            }
            if mx >= save_rect.0 && mx <= save_rect.1 && my >= save_rect.2 && my <= save_rect.3 {
                return Some(SettingsAction::Save(self.temp_settings.clone()));
            }
            return None;
        }

        // 3. Tab content clicks
        let action = self.interact_tab_controls(mx, my);
        Some(action)
    }

    pub fn handle_mouse_up(&mut self) -> bool {
        if self.dragging_slider.is_some() {
            self.dragging_slider = None;
            return true;
        }
        false
    }

    pub fn handle_mouse_wheel(&mut self, dy: f32) -> bool {
        let max_scroll = match self.active_tab {
            SettingsTab::General => 240.0,
            SettingsTab::Menu => 180.0,
            SettingsTab::Dialogue => 120.0,
            _ => 0.0,
        };
        let new_scroll = (self.scroll_y - dy).clamp(0.0, max_scroll);
        if (new_scroll - self.scroll_y).abs() > 0.5 {
            self.scroll_y = new_scroll;
            return true;
        }
        false
    }

    fn update_slider(&mut self, key: &str, mouse_x: f32) {
        let slider_x = 510.0;
        let slider_w = 120.0;
        let pct = ((mouse_x - slider_x) / slider_w).clamp(0.0, 1.0);

        match key {
            "scale" => {
                // 50% ~ 200%
                let val = 0.5 + pct * 1.5;
                self.temp_settings.scale = ((val * 20.0).round() / 20.0).clamp(0.5, 2.0);
            }
            "bubbleDuration" => {
                // 1000ms ~ 10000ms
                let ms = 1000.0 + pct * 9000.0;
                let stepped = ((ms / 500.0).round() * 500.0) as u64;
                let _ = stepped;
            }
            "soundVolume" => {
                // 0% ~ 100%
            }
            _ => {}
        }
    }

    fn hit_test_control(&self, mx: f32, my: f32) -> Option<String> {
        let win_w = SETTINGS_WIN_W as f32;
        let win_h = SETTINGS_WIN_H as f32;

        if mx < SIDEBAR_W && my >= 76.0 {
            let tab_idx = ((my - 78.0) / 36.0).floor() as i32;
            if (0..=4).contains(&tab_idx) {
                return Some(format!("tab_{}", tab_idx));
            }
        }
        if my >= win_h - FOOTER_H {
            if mx >= win_w - 170.0 && mx <= win_w - 96.0 {
                return Some("btn_cancel".to_string());
            }
            if mx >= win_w - 90.0 && mx <= win_w - 14.0 {
                return Some("btn_save".to_string());
            }
        }
        None
    }

    fn interact_tab_controls(&mut self, mx: f32, my: f32) -> SettingsAction {
        let content_y = my + self.scroll_y;

        match self.active_tab {
            SettingsTab::General => {
                // Row 1: 外观与主题 (y approx 99)
                if (86.0..126.0).contains(&content_y) && (490.0..660.0).contains(&mx) {
                    self.temp_settings.theme = match self.temp_settings.theme.as_str() {
                        "system" => "light".to_string(),
                        "light" => "dark".to_string(),
                        _ => "system".to_string(),
                    };
                    return SettingsAction::Redraw;
                }
                // Row 2: 缩放大小 (y approx 150)
                if (126.0..170.0).contains(&content_y) && (500.0..660.0).contains(&mx) {
                    self.dragging_slider = Some("scale".to_string());
                    self.update_slider("scale", mx);
                    return SettingsAction::Redraw;
                }
                // Row 3: 落地吸附模式 (y approx 194)
                if (170.0..214.0).contains(&content_y) && (480.0..660.0).contains(&mx) {
                    self.temp_settings.floor_mode = match self.temp_settings.floor_mode.as_str() {
                        "bottom" => "taskbar".to_string(),
                        "taskbar" => "free".to_string(),
                        _ => "bottom".to_string(),
                    };
                    return SettingsAction::Redraw;
                }
                // Row 4: 重力加速度 (y approx 238)
                if (214.0..258.0).contains(&content_y) && (590.0..650.0).contains(&mx) {
                    self.temp_settings.gravity_enabled = !self.temp_settings.gravity_enabled;
                    return SettingsAction::Redraw;
                }
                // Row 5: 屏幕位置校准 (复位居中) (y approx 280)
                if (258.0..302.0).contains(&content_y) && (560.0..650.0).contains(&mx) {
                    return SettingsAction::ResetPosition;
                }
                // Row 6: 实时额度卡片 (y approx 344)
                if (326.0..374.0).contains(&content_y) && (590.0..650.0).contains(&mx) {
                    self.temp_settings.show_quota = !self.temp_settings.show_quota;
                    return SettingsAction::Redraw;
                }
                // Row 7: 仅在 IDE 活跃 (y approx 388)
                if (374.0..418.0).contains(&content_y) && (590.0..650.0).contains(&mx) {
                    self.temp_settings.menu_config.ide_only = !self.temp_settings.menu_config.ide_only;
                    return SettingsAction::Redraw;
                }
                // Row 8: 完全穿透免打扰 (y approx 432)
                if (418.0..462.0).contains(&content_y) && (590.0..650.0).contains(&mx) {
                    self.temp_settings.dnd_passthrough = !self.temp_settings.dnd_passthrough;
                    return SettingsAction::Redraw;
                }
                // Row 10: 互动音效 (y approx 538)
                if (516.0..566.0).contains(&content_y) && (590.0..650.0).contains(&mx) {
                    self.temp_settings.sound_enabled = !self.temp_settings.sound_enabled;
                    return SettingsAction::Redraw;
                }
            }

            SettingsTab::Autostart => {
                // Row 1: 开机自启 (y approx 98)
                if (82.0..132.0).contains(&content_y) && (590.0..650.0).contains(&mx) {
                    self.autostart_enabled = !self.autostart_enabled;
                    return SettingsAction::Redraw;
                }
                // Row 2: 检查更新按钮 (y approx 186)
                if (166.0..216.0).contains(&content_y) && (550.0..650.0).contains(&mx) {
                    return SettingsAction::CheckUpdate;
                }
            }

            SettingsTab::Menu => {
                let menu_keys = [
                    &mut self.temp_settings.menu_config.open_antigravity,
                    &mut self.temp_settings.menu_config.settings,
                    &mut self.temp_settings.menu_config.quota,
                    &mut self.temp_settings.menu_config.random_dialogue,
                    &mut self.temp_settings.menu_config.pet_action,
                    &mut self.temp_settings.menu_config.zoom,
                    &mut self.temp_settings.menu_config.dnd,
                    &mut self.temp_settings.menu_config.ide_only,
                    &mut self.temp_settings.menu_config.gravity,
                    &mut self.temp_settings.menu_config.switch_skin,
                    &mut self.temp_settings.menu_config.sound,
                ];

                let start_y = 92.0;
                let row_h = 36.0;
                for (i, val) in menu_keys.into_iter().enumerate() {
                    let ry = start_y + (i as f32) * row_h;
                    if (ry..(ry + row_h)).contains(&content_y) && (210.0..670.0).contains(&mx) {
                        *val = !*val;
                        return SettingsAction::Redraw;
                    }
                }
            }

            _ => {}
        }

        SettingsAction::None
    }

    /// Renders the full native settings window into `pixmap`.
    pub fn render(&mut self, pixmap: &mut Pixmap, dpi_scale: f32) {
        let is_dark = self.is_dark_mode();
        let s = dpi_scale;

        let w_px = ((SETTINGS_WIN_W as f32) * s).round() as u32;
        let h_px = ((SETTINGS_WIN_H as f32) * s).round() as u32;
        pixmap.resize(w_px, h_px);

        // Theme palette
        let bg_win = if is_dark { [30, 30, 30, 255] } else { [245, 245, 247, 255] };
        let bg_sidebar = if is_dark { [37, 37, 37, 255] } else { [236, 236, 237, 255] };
        let bg_card = if is_dark { [44, 44, 46, 255] } else { [255, 255, 255, 255] };
        let text_pri = if is_dark { [245, 245, 247] } else { [29, 29, 31] };
        let text_sec = if is_dark { [152, 152, 157] } else { [110, 110, 115] };
        let border_card = if is_dark { [255, 255, 255, 20] } else { [0, 0, 0, 20] };
        let row_div = if is_dark { [255, 255, 255, 12] } else { [0, 0, 0, 12] };
        let accent_blue = if is_dark { [10, 132, 255, 255] } else { [0, 122, 255, 255] };

        // 1. Fill window background
        pixmap.fill_rounded_rect_sdf(0.0, 0.0, w_px as f32, h_px as f32, 0.0, bg_win, 0.0, [0, 0, 0, 0]);

        // 2. Render Left Sidebar
        pixmap.fill_rounded_rect_sdf(0.0, 0.0, SIDEBAR_W * s, h_px as f32, 0.0, bg_sidebar, 1.0 * s, border_card);

        // Sidebar title and version (spaced below native macOS traffic light buttons)
        draw_text_into_pixmap(pixmap, "偏好设置", 18.0 * s, 44.0 * s, 100.0 * s, 20.0 * s, 13.0 * s, 0.25, text_pri, 1.0, TextAlignment::Left);
        pixmap.fill_rounded_rect_sdf(126.0 * s, 45.0 * s, 46.0 * s, 18.0 * s, 4.0 * s, if is_dark { [255, 255, 255, 24] } else { [0, 0, 0, 14] }, 0.0, [0, 0, 0, 0]);
        draw_text_into_pixmap(pixmap, "v0.4.0", 126.0 * s, 47.0 * s, 46.0 * s, 14.0 * s, 10.0 * s, 0.0, text_sec, 1.0, TextAlignment::Center);

        // Sidebar nav items
        let tabs = [
            (SettingsTab::General, "常规与显示"),
            (SettingsTab::Autostart, "自启动与更新"),
            (SettingsTab::Skins, "皮肤管理"),
            (SettingsTab::Dialogue, "台词与问候"),
            (SettingsTab::Menu, "右键菜单"),
        ];

        for (i, (tab, title)) in tabs.into_iter().enumerate() {
            let ty = (78.0 + (i as f32) * 36.0) * s;
            let is_active = self.active_tab == tab;
            let (bg_pill, txt_color, weight) = if is_active {
                (accent_blue, [255, 255, 255], 0.23)
            } else {
                ([0, 0, 0, 0], text_pri, 0.0)
            };

            if is_active {
                pixmap.fill_rounded_rect_sdf(10.0 * s, ty, (SIDEBAR_W - 20.0) * s, 28.0 * s, 6.0 * s, bg_pill, 0.0, [0, 0, 0, 0]);
            }
            draw_text_into_pixmap(pixmap, title, 20.0 * s, ty + 6.0 * s, (SIDEBAR_W - 40.0) * s, 16.0 * s, 12.5 * s, weight, txt_color, 1.0, TextAlignment::Left);
        }

        // 3. Render Right Content Area
        let cx = (SIDEBAR_W + 24.0) * s;
        let cw = (SETTINGS_WIN_W as f32 - SIDEBAR_W - 48.0) * s;
        let _ch = (SETTINGS_WIN_H as f32 - FOOTER_H - 24.0) * s;

        match self.active_tab {
            SettingsTab::General => {
                self.render_general_tab(pixmap, cx, cw, s, bg_card, border_card, row_div, text_pri, text_sec, accent_blue);
            }
            SettingsTab::Autostart => {
                self.render_autostart_tab(pixmap, cx, cw, s, bg_card, border_card, row_div, text_pri, text_sec, accent_blue);
            }
            SettingsTab::Skins => {
                self.render_skins_tab(pixmap, cx, cw, s, bg_card, border_card, text_pri, text_sec, accent_blue);
            }
            SettingsTab::Dialogue => {
                self.render_dialogue_tab(pixmap, cx, cw, s, bg_card, border_card, row_div, text_pri, text_sec);
            }
            SettingsTab::Menu => {
                self.render_menu_tab(pixmap, cx, cw, s, bg_card, border_card, row_div, text_pri, text_sec, accent_blue);
            }
        }

        // 4. Render Fixed Footer Bar
        let fy = (SETTINGS_WIN_H as f32 - FOOTER_H) * s;
        let fw = (SETTINGS_WIN_W as f32) * s;
        pixmap.fill_rounded_rect_sdf(SIDEBAR_W * s, fy, fw - SIDEBAR_W * s, FOOTER_H * s, 0.0, bg_win, 1.0 * s, border_card);

        draw_text_into_pixmap(pixmap, "配置变更将在点击保存后立即生效", cx, fy + 16.0 * s, 260.0 * s, 18.0 * s, 11.0 * s, 0.0, text_sec, 1.0, TextAlignment::Left);

        // Cancel Button
        let btn_cancel_x = (SETTINGS_WIN_W as f32 - 164.0) * s;
        pixmap.fill_rounded_rect_sdf(btn_cancel_x, fy + 12.0 * s, 68.0 * s, 26.0 * s, 6.0 * s, if is_dark { [58, 58, 60, 255] } else { [255, 255, 255, 255] }, 1.0 * s, border_card);
        draw_text_into_pixmap(pixmap, "取消", btn_cancel_x, fy + 17.0 * s, 68.0 * s, 16.0 * s, 12.0 * s, 0.0, text_pri, 1.0, TextAlignment::Center);

        // Save Button
        let btn_save_x = (SETTINGS_WIN_W as f32 - 86.0) * s;
        pixmap.fill_rounded_rect_sdf(btn_save_x, fy + 12.0 * s, 72.0 * s, 26.0 * s, 6.0 * s, accent_blue, 0.0, [0, 0, 0, 0]);
        draw_text_into_pixmap(pixmap, "保存", btn_save_x, fy + 17.0 * s, 72.0 * s, 16.0 * s, 12.0 * s, 0.25, [255, 255, 255], 1.0, TextAlignment::Center);

        // 5. Toast Pill
        if let Some((ref msg, since)) = self.toast {
            if since.elapsed().as_secs_f32() < 2.5 {
                let alpha = (1.0 - (since.elapsed().as_secs_f32() - 2.0).max(0.0) * 2.0).clamp(0.0, 1.0);
                let toast_w = 260.0 * s;
                let toast_x = ((SETTINGS_WIN_W as f32 * s) - toast_w) * 0.5;
                let toast_y = fy - 36.0 * s;
                pixmap.fill_rounded_rect_sdf(toast_x, toast_y, toast_w, 28.0 * s, 14.0 * s, [28, 28, 30, (230.0 * alpha) as u8], 0.0, [0, 0, 0, 0]);
                draw_text_into_pixmap(pixmap, msg, toast_x, toast_y + 6.0 * s, toast_w, 16.0 * s, 11.5 * s, 0.0, [255, 255, 255], alpha, TextAlignment::Center);
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
        accent_blue: [u8; 4],
    ) {
        let base_y = (44.0 - self.scroll_y) * s;

        // Header
        draw_text_into_pixmap(pixmap, "常规与显示", cx, base_y, cw, 22.0 * s, 15.0 * s, 0.3, text_pri, 1.0, TextAlignment::Left);
        draw_text_into_pixmap(pixmap, "调整大肥猫的桌面尺寸、物理重力下落与交互行为", cx, base_y + 22.0 * s, cw, 16.0 * s, 11.5 * s, 0.0, text_sec, 1.0, TextAlignment::Left);

        // Card 1: 外观与主题
        let c1_y = base_y + 46.0 * s;
        let c1_h = 44.0 * s;
        pixmap.fill_rounded_rect_sdf(cx, c1_y, cw, c1_h, 8.0 * s, bg_card, 1.0 * s, border_card);
        draw_text_into_pixmap(pixmap, "界面外观", cx + 14.0 * s, c1_y + 8.0 * s, 200.0 * s, 16.0 * s, 13.0 * s, 0.0, text_pri, 1.0, TextAlignment::Left);
        draw_text_into_pixmap(pixmap, "设置窗口明暗主题风格，默认自动跟随系统", cx + 14.0 * s, c1_y + 24.0 * s, 260.0 * s, 14.0 * s, 11.0 * s, 0.0, text_sec, 1.0, TextAlignment::Left);

        // Theme option pill button
        let theme_label = match self.temp_settings.theme.as_str() {
            "dark" => "深色模式",
            "light" => "浅色模式",
            _ => "跟随系统",
        };
        let pill_x = cx + cw - 96.0 * s;
        pixmap.fill_rounded_rect_sdf(pill_x, c1_y + 9.0 * s, 84.0 * s, 24.0 * s, 6.0 * s, border_card, 1.0 * s, border_card);
        draw_text_into_pixmap(pixmap, theme_label, pill_x, c1_y + 13.0 * s, 84.0 * s, 16.0 * s, 11.5 * s, 0.0, text_pri, 1.0, TextAlignment::Center);

        // Card 2: 体型与物理
        let c2_y = c1_y + c1_h + 12.0 * s;
        let c2_h = 176.0 * s;
        pixmap.fill_rounded_rect_sdf(cx, c2_y, cw, c2_h, 8.0 * s, bg_card, 1.0 * s, border_card);

        // Row 2.1: 缩放大小
        let r1_y = c2_y + 4.0 * s;
        draw_text_into_pixmap(pixmap, "缩放大小", cx + 14.0 * s, r1_y + 6.0 * s, 200.0 * s, 16.0 * s, 13.0 * s, 0.0, text_pri, 1.0, TextAlignment::Left);
        draw_text_into_pixmap(pixmap, "挂件在屏幕上的呈现比例 (也可悬停直接滚轮平滑缩放)", cx + 14.0 * s, r1_y + 22.0 * s, 260.0 * s, 14.0 * s, 11.0 * s, 0.0, text_sec, 1.0, TextAlignment::Left);
        self.render_slider(pixmap, cx + cw - 170.0 * s, r1_y + 14.0 * s, 110.0 * s, s, (self.temp_settings.scale - 0.5) / 1.5, accent_blue);
        let scale_str = format!("{:.0}%", self.temp_settings.scale * 100.0);
        draw_text_into_pixmap(pixmap, &scale_str, cx + cw - 52.0 * s, r1_y + 12.0 * s, 40.0 * s, 16.0 * s, 11.5 * s, 0.0, text_sec, 1.0, TextAlignment::Right);
        pixmap.fill_rounded_rect_sdf(cx + 14.0 * s, r1_y + 42.0 * s, cw - 28.0 * s, 1.0 * s, 0.0, row_div, 0.0, [0, 0, 0, 0]);

        // Row 2.2: 落地吸附模式
        let r2_y = r1_y + 44.0 * s;
        draw_text_into_pixmap(pixmap, "落地吸附模式", cx + 14.0 * s, r2_y + 6.0 * s, 200.0 * s, 16.0 * s, 13.0 * s, 0.0, text_pri, 1.0, TextAlignment::Left);
        draw_text_into_pixmap(pixmap, "拖拽松手后猫猫下落吸附的目标基准线", cx + 14.0 * s, r2_y + 22.0 * s, 260.0 * s, 14.0 * s, 11.0 * s, 0.0, text_sec, 1.0, TextAlignment::Left);
        let floor_label = match self.temp_settings.floor_mode.as_str() {
            "taskbar" => "贴合任务栏与 Dock",
            "free" => "自由悬浮",
            _ => "屏幕绝对落底 (0px)",
        };
        let floor_pill_x = cx + cw - 138.0 * s;
        pixmap.fill_rounded_rect_sdf(floor_pill_x, r2_y + 9.0 * s, 126.0 * s, 24.0 * s, 6.0 * s, border_card, 1.0 * s, border_card);
        draw_text_into_pixmap(pixmap, floor_label, floor_pill_x, r2_y + 13.0 * s, 126.0 * s, 16.0 * s, 11.5 * s, 0.0, text_pri, 1.0, TextAlignment::Center);
        pixmap.fill_rounded_rect_sdf(cx + 14.0 * s, r2_y + 42.0 * s, cw - 28.0 * s, 1.0 * s, 0.0, row_div, 0.0, [0, 0, 0, 0]);

        // Row 2.3: 重力加速度模拟
        let r3_y = r2_y + 44.0 * s;
        draw_text_into_pixmap(pixmap, "重力加速度模拟", cx + 14.0 * s, r3_y + 6.0 * s, 200.0 * s, 16.0 * s, 13.0 * s, 0.0, text_pri, 1.0, TextAlignment::Left);
        draw_text_into_pixmap(pixmap, "释放后遵循弹性重力加速度下落，触底自然回弹", cx + 14.0 * s, r3_y + 22.0 * s, 260.0 * s, 14.0 * s, 11.0 * s, 0.0, text_sec, 1.0, TextAlignment::Left);
        self.render_toggle(pixmap, cx + cw - 52.0 * s, r3_y + 10.0 * s, s, self.temp_settings.gravity_enabled);
        pixmap.fill_rounded_rect_sdf(cx + 14.0 * s, r3_y + 42.0 * s, cw - 28.0 * s, 1.0 * s, 0.0, row_div, 0.0, [0, 0, 0, 0]);

        // Row 2.4: 屏幕位置校准 (复位居中)
        let r4_y = r3_y + 44.0 * s;
        draw_text_into_pixmap(pixmap, "屏幕位置校准", cx + 14.0 * s, r4_y + 6.0 * s, 200.0 * s, 16.0 * s, 13.0 * s, 0.0, text_pri, 1.0, TextAlignment::Left);
        draw_text_into_pixmap(pixmap, "将桌宠一键拉回屏幕安全正中央，避免逃出屏幕或遮挡", cx + 14.0 * s, r4_y + 22.0 * s, 260.0 * s, 14.0 * s, 11.0 * s, 0.0, text_sec, 1.0, TextAlignment::Left);
        let reset_btn_x = cx + cw - 88.0 * s;
        pixmap.fill_rounded_rect_sdf(reset_btn_x, r4_y + 9.0 * s, 76.0 * s, 24.0 * s, 6.0 * s, border_card, 1.0 * s, border_card);
        draw_text_into_pixmap(pixmap, "复位居中", reset_btn_x, r4_y + 13.0 * s, 76.0 * s, 16.0 * s, 11.5 * s, 0.0, text_pri, 1.0, TextAlignment::Center);

        // Card 3: 状态与交互
        let c3_y = c2_y + c2_h + 12.0 * s;
        let c3_h = 132.0 * s;
        pixmap.fill_rounded_rect_sdf(cx, c3_y, cw, c3_h, 8.0 * s, bg_card, 1.0 * s, border_card);

        // Row 3.1: 实时额度卡片
        let r31_y = c3_y + 4.0 * s;
        draw_text_into_pixmap(pixmap, "实时额度卡片", cx + 14.0 * s, r31_y + 6.0 * s, 200.0 * s, 16.0 * s, 13.0 * s, 0.0, text_pri, 1.0, TextAlignment::Left);
        draw_text_into_pixmap(pixmap, "在对话气泡中同步展示 Antigravity 5小时与周额度进度", cx + 14.0 * s, r31_y + 22.0 * s, 260.0 * s, 14.0 * s, 11.0 * s, 0.0, text_sec, 1.0, TextAlignment::Left);
        self.render_toggle(pixmap, cx + cw - 52.0 * s, r31_y + 10.0 * s, s, self.temp_settings.show_quota);
        pixmap.fill_rounded_rect_sdf(cx + 14.0 * s, r31_y + 42.0 * s, cw - 28.0 * s, 1.0 * s, 0.0, row_div, 0.0, [0, 0, 0, 0]);

        // Row 3.2: 仅在 IDE 活跃
        let r32_y = r31_y + 44.0 * s;
        draw_text_into_pixmap(pixmap, "仅在 IDE 活跃时显示", cx + 14.0 * s, r32_y + 6.0 * s, 200.0 * s, 16.0 * s, 13.0 * s, 0.0, text_pri, 1.0, TextAlignment::Left);
        draw_text_into_pixmap(pixmap, "前台为 Antigravity / VS Code / Cursor 时呈现，切出时隐匿", cx + 14.0 * s, r32_y + 22.0 * s, 260.0 * s, 14.0 * s, 11.0 * s, 0.0, text_sec, 1.0, TextAlignment::Left);
        self.render_toggle(pixmap, cx + cw - 52.0 * s, r32_y + 10.0 * s, s, self.temp_settings.menu_config.ide_only);
        pixmap.fill_rounded_rect_sdf(cx + 14.0 * s, r32_y + 42.0 * s, cw - 28.0 * s, 1.0 * s, 0.0, row_div, 0.0, [0, 0, 0, 0]);

        // Row 3.3: 完全穿透免打扰
        let r33_y = r32_y + 44.0 * s;
        draw_text_into_pixmap(pixmap, "完全穿透免打扰", cx + 14.0 * s, r33_y + 6.0 * s, 200.0 * s, 16.0 * s, 13.0 * s, 0.0, text_pri, 1.0, TextAlignment::Left);
        draw_text_into_pixmap(pixmap, "猫猫处于只读透明状态，鼠标点击直接透传到底层应用", cx + 14.0 * s, r33_y + 22.0 * s, 260.0 * s, 14.0 * s, 11.0 * s, 0.0, text_sec, 1.0, TextAlignment::Left);
        self.render_toggle(pixmap, cx + cw - 52.0 * s, r33_y + 10.0 * s, s, self.temp_settings.dnd_passthrough);
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
        _row_div: [u8; 4],
        text_pri: [u8; 3],
        text_sec: [u8; 3],
        _accent_blue: [u8; 4],
    ) {
        let base_y = (44.0 - self.scroll_y) * s;
        draw_text_into_pixmap(pixmap, "自启动与更新", cx, base_y, cw, 22.0 * s, 15.0 * s, 0.3, text_pri, 1.0, TextAlignment::Left);
        draw_text_into_pixmap(pixmap, "管理开机自启动配置与挂件版本检测", cx, base_y + 22.0 * s, cw, 16.0 * s, 11.5 * s, 0.0, text_sec, 1.0, TextAlignment::Left);

        let c1_y = base_y + 48.0 * s;
        pixmap.fill_rounded_rect_sdf(cx, c1_y, cw, 68.0 * s, 8.0 * s, bg_card, 1.0 * s, border_card);
        draw_text_into_pixmap(pixmap, "开机自动启动", cx + 14.0 * s, c1_y + 8.0 * s, 200.0 * s, 16.0 * s, 13.0 * s, 0.0, text_pri, 1.0, TextAlignment::Left);
        draw_text_into_pixmap(pixmap, "在系统登录完成后自动在托盘挂载大肥猫伴侣", cx + 14.0 * s, c1_y + 24.0 * s, 260.0 * s, 14.0 * s, 11.0 * s, 0.0, text_sec, 1.0, TextAlignment::Left);
        self.render_toggle(pixmap, cx + cw - 52.0 * s, c1_y + 12.0 * s, s, self.autostart_enabled);
        draw_text_into_pixmap(pixmap, if self.autostart_enabled { "已开启系统开机自启" } else { "未开启自启，点击可在登录时自动拉起伴侣" }, cx + 14.0 * s, c1_y + 46.0 * s, cw - 28.0 * s, 14.0 * s, 10.5 * s, 0.0, text_sec, 1.0, TextAlignment::Left);

        let c2_y = c1_y + 80.0 * s;
        pixmap.fill_rounded_rect_sdf(cx, c2_y, cw, 56.0 * s, 8.0 * s, bg_card, 1.0 * s, border_card);
        draw_text_into_pixmap(pixmap, "当前版本", cx + 14.0 * s, c2_y + 8.0 * s, 200.0 * s, 16.0 * s, 13.0 * s, 0.0, text_pri, 1.0, TextAlignment::Left);
        draw_text_into_pixmap(pixmap, "v0.4.0 (Pure CPU 2D 软渲染引擎 · 单二进制)", cx + 14.0 * s, c2_y + 26.0 * s, 280.0 * s, 14.0 * s, 11.0 * s, 0.0, text_sec, 1.0, TextAlignment::Left);
        let btn_update_x = cx + cw - 88.0 * s;
        pixmap.fill_rounded_rect_sdf(btn_update_x, c2_y + 14.0 * s, 76.0 * s, 26.0 * s, 6.0 * s, border_card, 1.0 * s, border_card);
        draw_text_into_pixmap(pixmap, "检查更新", btn_update_x, c2_y + 19.0 * s, 76.0 * s, 16.0 * s, 11.5 * s, 0.0, text_pri, 1.0, TextAlignment::Center);
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
        accent_blue: [u8; 4],
    ) {
        let is_dark = self.is_dark_mode();
        let base_y = (44.0 - self.scroll_y) * s;
        draw_text_into_pixmap(pixmap, "皮肤管理", cx, base_y, cw, 22.0 * s, 15.0 * s, 0.3, text_pri, 1.0, TextAlignment::Left);
        draw_text_into_pixmap(pixmap, "切换外观皮肤，轻量化纯 2D 精灵切片", cx, base_y + 22.0 * s, cw, 16.0 * s, 11.5 * s, 0.0, text_sec, 1.0, TextAlignment::Left);

        let card_y = base_y + 50.0 * s;
        let card_w = 144.0 * s;
        let card_h = 168.0 * s;

        // Outer Card Container with Active Blue Outline
        pixmap.fill_rounded_rect_sdf(cx, card_y, card_w, card_h, 10.0 * s, bg_card, 1.5 * s, accent_blue);

        // Preview inner background box
        let preview_bg = if is_dark { [26, 26, 28, 220] } else { [240, 240, 244, 230] };
        pixmap.fill_rounded_rect_sdf(cx + 8.0 * s, card_y + 8.0 * s, card_w - 16.0 * s, 98.0 * s, 8.0 * s, preview_bg, 0.0, [0, 0, 0, 0]);

        // Render the fat-cat sprite avatar into the preview area
        let tf = super::sprite::SpriteTransform {
            anchor_x: (cx + (card_w * 0.5)) / s,
            anchor_y: (card_y + 98.0 * s) / s,
            base_size: 76.0,
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

        // "已启用" Badge in top-right of preview
        let badge_w = 42.0 * s;
        let badge_h = 18.0 * s;
        pixmap.fill_rounded_rect_sdf(cx + card_w - badge_w - 12.0 * s, card_y + 12.0 * s, badge_w, badge_h, 4.0 * s, accent_blue, 0.0, [0, 0, 0, 0]);
        draw_text_into_pixmap(pixmap, "已启用", cx + card_w - badge_w - 12.0 * s, card_y + 13.5 * s, badge_w, 14.0 * s, 10.0 * s, 0.2, [255, 255, 255], 1.0, TextAlignment::Center);

        // Character Name & Subtitle
        draw_text_into_pixmap(pixmap, "大肥猫", cx, card_y + 116.0 * s, card_w, 18.0 * s, 13.0 * s, 0.23, text_pri, 1.0, TextAlignment::Center);
        draw_text_into_pixmap(pixmap, "默认官方伴侣", cx, card_y + 136.0 * s, card_w, 14.0 * s, 11.0 * s, 0.0, text_sec, 1.0, TextAlignment::Center);
    }

    fn render_dialogue_tab(
        &self,
        pixmap: &mut Pixmap,
        cx: f32,
        cw: f32,
        s: f32,
        bg_card: [u8; 4],
        border_card: [u8; 4],
        _row_div: [u8; 4],
        text_pri: [u8; 3],
        text_sec: [u8; 3],
    ) {
        let base_y = (44.0 - self.scroll_y) * s;
        draw_text_into_pixmap(pixmap, "台词与问候", cx, base_y, cw, 22.0 * s, 15.0 * s, 0.3, text_pri, 1.0, TextAlignment::Left);
        draw_text_into_pixmap(pixmap, "自定义不同工作状态下的气泡台词与提示内容", cx, base_y + 22.0 * s, cw, 16.0 * s, 11.5 * s, 0.0, text_sec, 1.0, TextAlignment::Left);

        let cats = [
            ("待机日常 (idle)", "5 条预置台词随时间周期随机出现"),
            ("互动抚摸 (petting)", "鼠标双击或在猫头连续抚摸摩擦时触发"),
            ("任务成功 (success)", "Antigravity 成功完成代码交付或构建时"),
            ("深度思考 (thinking)", "模型在执行深度推理或调用工具流时"),
            ("任务异常 (error)", "执行出错或被阻断时触发的安抚提示"),
        ];

        for (i, (title, desc)) in cats.into_iter().enumerate() {
            let cy = base_y + (48.0 + (i as f32) * 58.0) * s;
            pixmap.fill_rounded_rect_sdf(cx, cy, cw, 50.0 * s, 8.0 * s, bg_card, 1.0 * s, border_card);
            draw_text_into_pixmap(pixmap, title, cx + 14.0 * s, cy + 8.0 * s, 260.0 * s, 16.0 * s, 12.5 * s, 0.2, text_pri, 1.0, TextAlignment::Left);
            draw_text_into_pixmap(pixmap, desc, cx + 14.0 * s, cy + 26.0 * s, 340.0 * s, 14.0 * s, 11.0 * s, 0.0, text_sec, 1.0, TextAlignment::Left);
        }
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
        accent_blue: [u8; 4],
    ) {
        let base_y = (44.0 - self.scroll_y) * s;
        draw_text_into_pixmap(pixmap, "右键菜单", cx, base_y, cw, 22.0 * s, 15.0 * s, 0.3, text_pri, 1.0, TextAlignment::Left);
        draw_text_into_pixmap(pixmap, "管理在桌面右键点击桌宠时出现的控制项与呈现策略", cx, base_y + 22.0 * s, cw, 16.0 * s, 11.5 * s, 0.0, text_sec, 1.0, TextAlignment::Left);

        let menu_items = [
            ("打开 Antigravity", self.temp_settings.menu_config.open_antigravity),
            ("偏好设置...", self.temp_settings.menu_config.settings),
            ("切换额度卡片", self.temp_settings.menu_config.quota),
            ("换一句台词", self.temp_settings.menu_config.random_dialogue),
            ("互动抚摸", self.temp_settings.menu_config.pet_action),
            ("缩放大小 (滚轮微调)", self.temp_settings.menu_config.zoom),
            ("完全穿透 (免打扰)", self.temp_settings.menu_config.dnd),
            ("仅在 IDE 活跃时显示", self.temp_settings.menu_config.ide_only),
            ("重力下落吸附", self.temp_settings.menu_config.gravity),
            ("切换皮肤", self.temp_settings.menu_config.switch_skin),
            ("动作音效", self.temp_settings.menu_config.sound),
        ];

        let card_y = base_y + 48.0 * s;
        let card_h = (menu_items.len() as f32) * 36.0 * s;
        pixmap.fill_rounded_rect_sdf(cx, card_y, cw, card_h, 8.0 * s, bg_card, 1.0 * s, border_card);

        for (i, (name, checked)) in menu_items.into_iter().enumerate() {
            let ry = card_y + (i as f32) * 36.0 * s;
            self.render_checkbox(pixmap, cx + 14.0 * s, ry + 10.0 * s, s, checked, accent_blue);
            draw_text_into_pixmap(pixmap, name, cx + 38.0 * s, ry + 9.0 * s, cw - 50.0 * s, 16.0 * s, 12.5 * s, 0.0, text_pri, 1.0, TextAlignment::Left);

            if i < menu_items.len() - 1 {
                pixmap.fill_rounded_rect_sdf(cx + 14.0 * s, ry + 35.0 * s, cw - 28.0 * s, 1.0 * s, 0.0, row_div, 0.0, [0, 0, 0, 0]);
            }
        }
    }

    fn render_toggle(&self, pixmap: &mut Pixmap, x: f32, y: f32, s: f32, on: bool) {
        let w = 38.0 * s;
        let h = 22.0 * s;
        let r = 11.0 * s;
        let track_col = if on { [52, 199, 89, 255] } else { [142, 142, 147, 120] };
        pixmap.fill_rounded_rect_sdf(x, y, w, h, r, track_col, 0.0, [0, 0, 0, 0]);

        let knob_x = if on { x + w - 19.0 * s } else { x + 3.0 * s };
        pixmap.fill_circle_sdf(knob_x + 8.0 * s, y + 11.0 * s, 8.0 * s, [255, 255, 255, 255], 0.0, [0, 0, 0, 0]);
    }

    fn render_slider(&self, pixmap: &mut Pixmap, x: f32, y: f32, w: f32, s: f32, pct: f32, accent: [u8; 4]) {
        let track_h = 4.0 * s;
        pixmap.fill_rounded_rect_sdf(x, y, w, track_h, 2.0 * s, [142, 142, 147, 80], 0.0, [0, 0, 0, 0]);
        let filled_w = w * pct.clamp(0.0, 1.0);
        if filled_w > 1.0 {
            pixmap.fill_rounded_rect_sdf(x, y, filled_w, track_h, 2.0 * s, accent, 0.0, [0, 0, 0, 0]);
        }
        let knob_x = x + filled_w;
        pixmap.fill_circle_sdf(knob_x, y + 2.0 * s, 7.0 * s, [255, 255, 255, 255], 0.5 * s, [0, 0, 0, 40]);
    }

    fn render_checkbox(&self, pixmap: &mut Pixmap, x: f32, y: f32, s: f32, checked: bool, accent: [u8; 4]) {
        let size = 16.0 * s;
        let r = 4.0 * s;
        if checked {
            pixmap.fill_rounded_rect_sdf(x, y, size, size, r, accent, 0.0, [0, 0, 0, 0]);
            // Draw clean anti-aliased vector checkmark (Apple SF Symbols style)
            let thick = 1.8 * s;
            let check_color = [255, 255, 255, 255];
            let p1_x = x + 4.2 * s;
            let p1_y = y + 8.2 * s;
            let p2_x = x + 6.8 * s;
            let p2_y = y + 11.4 * s;
            let p3_x = x + 12.0 * s;
            let p3_y = y + 4.8 * s;
            pixmap.draw_line_sdf(p1_x, p1_y, p2_x, p2_y, thick, check_color);
            pixmap.draw_line_sdf(p2_x, p2_y, p3_x, p3_y, thick, check_color);
        } else {
            pixmap.fill_rounded_rect_sdf(x, y, size, size, r, [0, 0, 0, 0], 1.2 * s, [142, 142, 147, 180]);
        }
    }
}

#[cfg(target_os = "macos")]
fn macos_is_dark_mode() -> bool {
    use std::ffi::c_void;
    extern "C" {
        fn objc_getClass(name: *const u8) -> *mut c_void;
        fn sel_registerName(name: *const u8) -> *mut c_void;
        fn objc_msgSend();
    }
    unsafe {
        let app_cls = objc_getClass(b"NSApplication\0".as_ptr());
        if app_cls.is_null() {
            return false;
        }
        let msg_ptr: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
            std::mem::transmute(objc_msgSend as *const ());
        let shared_app = msg_ptr(app_cls, sel_registerName(b"sharedApplication\0".as_ptr()));
        if shared_app.is_null() {
            return false;
        }
        let app_appearance = msg_ptr(shared_app, sel_registerName(b"effectiveAppearance\0".as_ptr()));
        if app_appearance.is_null() {
            return false;
        }
        let name_str = msg_ptr(app_appearance, sel_registerName(b"name\0".as_ptr()));
        if name_str.is_null() {
            return false;
        }
        let msg_utf8: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *const i8 =
            std::mem::transmute(objc_msgSend as *const ());
        let utf8_ptr = msg_utf8(name_str, sel_registerName(b"UTF8String\0".as_ptr()));
        if !utf8_ptr.is_null() {
            let cstr = std::ffi::CStr::from_ptr(utf8_ptr);
            if let Ok(s) = cstr.to_str() {
                return s.contains("Dark");
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_settings_ui_layout_and_interaction() {
        let mut ui = SettingsUi::new(PetSettings::default());
        let mut pixmap = Pixmap::new(700, 530);
        ui.render(&mut pixmap, 1.0);

        assert_eq!(ui.active_tab, SettingsTab::General);
        assert_eq!(ui.temp_settings.scale, 1.0);

        // Click sidebar tab 4 (Menu)
        ui.mouse_pos = (50.0, 230.0);
        let action = ui.handle_mouse_down(1.0);
        assert!(matches!(action, Some(SettingsAction::SelectTab(SettingsTab::Menu))));
        assert_eq!(ui.active_tab, SettingsTab::Menu);

        // Re-render
        ui.render(&mut pixmap, 1.0);

        // Click sidebar tab 2 (Skins)
        ui.mouse_pos = (50.0, 160.0);
        let action = ui.handle_mouse_down(1.0);
        assert!(matches!(action, Some(SettingsAction::SelectTab(SettingsTab::Skins))));
        assert_eq!(ui.active_tab, SettingsTab::Skins);

        // Render skins tab with cat sprite preview
        ui.render(&mut pixmap, 1.0);

        // Click row 0 in Menu (Toggle openAntigravity)
        ui.active_tab = SettingsTab::Menu;
        ui.mouse_pos = (300.0, 102.0);
        let action = ui.handle_mouse_down(1.0);
        assert!(matches!(action, Some(SettingsAction::Redraw)));
        assert_eq!(ui.temp_settings.menu_config.open_antigravity, false);
    }
}
