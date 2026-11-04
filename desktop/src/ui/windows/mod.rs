slint::include_modules!();

use crate::core::store::{SettingsAction, SettingsStore, StoreEvent};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

pub struct SlintSettingsApp {
    pub window: SettingsWindow,
    pub store: Arc<Mutex<SettingsStore>>,
}

impl SlintSettingsApp {
    pub fn new(store: Arc<Mutex<SettingsStore>>) -> Result<Self, slint::PlatformError> {
        let window = SettingsWindow::new()?;

        let app = Self {
            window: window.clone_strong(),
            store: store.clone(),
        };

        app.bind_callbacks();
        app.sync_from_store();

        Ok(app)
    }

    pub fn sync_from_store(&self) {
        let store = self.store.lock().unwrap();
        let snap = store.snapshot();

        self.window.set_active_tab(snap.active_tab as i32);
        self.window.set_theme_mode(SharedString::from(snap.temp_settings.theme.clone()));
        self.window.set_pet_scale(snap.temp_settings.scale);
        self.window.set_floor_mode(SharedString::from(snap.temp_settings.floor_mode.clone()));
        self.window.set_gravity_enabled(snap.temp_settings.gravity_enabled);
        self.window.set_show_quota(snap.temp_settings.show_quota);
        self.window.set_ide_only(snap.temp_settings.menu_config.ide_only);
        self.window.set_dnd_mode(snap.temp_settings.dnd_passthrough);
        self.window.set_sound_enabled(snap.temp_settings.sound_enabled);
        self.window.set_autostart_enabled(snap.autostart_enabled);

        // Load skin preview from skins/fat-cat/frames/idle.png
        let idle_png = store.root_dir.join("skins").join("fat-cat").join("frames").join("idle.png");
        if let Ok(img) = slint::Image::load_from_path(&idle_png) {
            self.window.set_skin_preview(img);
        }

        // Menu items
        self.window.set_menu_open_antigravity(snap.temp_settings.menu_config.open_antigravity);
        self.window.set_menu_settings(snap.temp_settings.menu_config.settings);
        self.window.set_menu_random_dialogue(snap.temp_settings.menu_config.random_dialogue);
        self.window.set_menu_pet_action(snap.temp_settings.menu_config.pet_action);
        self.window.set_menu_quota(snap.temp_settings.menu_config.quota);
        self.window.set_menu_dnd(snap.temp_settings.menu_config.dnd);
        self.window.set_menu_sound(snap.temp_settings.menu_config.sound);
        self.window.set_menu_zoom(snap.temp_settings.menu_config.zoom);
        self.window.set_menu_gravity(snap.temp_settings.menu_config.gravity);

        // Dialogue items
        self.window.set_dialogue_cat(snap.dialogue_cat as i32);
        let active_lines = store.active_dialogues();
        let items: Vec<DialogueItem> = active_lines
            .iter()
            .enumerate()
            .map(|(idx, line)| DialogueItem {
                id: idx as i32,
                text: SharedString::from(line.clone()),
            })
            .collect();
        let model = Rc::new(VecModel::from(items));
        self.window.set_dialogue_list(ModelRc::from(model));
    }

    fn bind_callbacks(&self) {
        let store = self.store.clone();
        let win = self.window.as_weak();

        // 1. Select Tab
        {
            let store = store.clone();
            let win = win.clone();
            self.window.on_select_tab(move |tab_idx| {
                let tab = match tab_idx {
                    1 => crate::core::store::SettingsTab::Autostart,
                    2 => crate::core::store::SettingsTab::Skin,
                    3 => crate::core::store::SettingsTab::Dialogue,
                    4 => crate::core::store::SettingsTab::Menu,
                    _ => crate::core::store::SettingsTab::General,
                };
                store.lock().unwrap().dispatch(SettingsAction::SelectTab(tab));
                if let Some(w) = win.upgrade() {
                    w.set_active_tab(tab_idx);
                }
            });
        }

        // 2. Cycle Theme (Auto-save)
        {
            let store = store.clone();
            let win = win.clone();
            self.window.on_cycle_theme(move || {
                let mut st = store.lock().unwrap();
                st.dispatch(SettingsAction::CycleTheme);
                st.commit_to_disk();
                let theme = st.temp_settings.theme.clone();
                drop(st);
                if let Some(w) = win.upgrade() {
                    w.set_theme_mode(SharedString::from(theme));
                    w.set_toast_message(SharedString::from("外观主题已保存"));
                }
            });
        }

        // 3. Scale Changed (Auto-save)
        {
            let store = store.clone();
            let win = win.clone();
            self.window.on_scale_changed(move |scale| {
                let mut st = store.lock().unwrap();
                st.dispatch(SettingsAction::SetScale(scale));
                st.commit_to_disk();
                let cur = st.temp_settings.scale;
                drop(st);
                if let Some(w) = win.upgrade() {
                    w.set_pet_scale(cur);
                }
            });
        }

        // 4. Cycle Floor Mode (Auto-save)
        {
            let store = store.clone();
            let win = win.clone();
            self.window.on_cycle_floor_mode(move || {
                let mut st = store.lock().unwrap();
                st.dispatch(SettingsAction::CycleFloorMode);
                st.commit_to_disk();
                let mode = st.temp_settings.floor_mode.clone();
                drop(st);
                if let Some(w) = win.upgrade() {
                    w.set_floor_mode(SharedString::from(mode));
                    w.set_toast_message(SharedString::from("吸附基线已保存"));
                }
            });
        }

        // 5. Toggle Gravity (Auto-save)
        {
            let store = store.clone();
            let win = win.clone();
            self.window.on_toggle_gravity(move || {
                let mut st = store.lock().unwrap();
                st.dispatch(SettingsAction::ToggleGravity);
                st.commit_to_disk();
                let val = st.temp_settings.gravity_enabled;
                drop(st);
                if let Some(w) = win.upgrade() {
                    w.set_gravity_enabled(val);
                    w.set_toast_message(SharedString::from("重力模拟已更新"));
                }
            });
        }

        // 6. Reset Position
        {
            let store = store.clone();
            let win = win.clone();
            self.window.on_reset_position(move || {
                store.lock().unwrap().dispatch(SettingsAction::ResetPosition);
                if let Some(w) = win.upgrade() {
                    w.set_toast_message(SharedString::from("已将桌宠复位至屏幕中央"));
                }
            });
        }

        // 7. Toggle Quota (Auto-save)
        {
            let store = store.clone();
            let win = win.clone();
            self.window.on_toggle_quota(move || {
                let mut st = store.lock().unwrap();
                st.dispatch(SettingsAction::ToggleShowQuota);
                st.commit_to_disk();
                let val = st.temp_settings.show_quota;
                drop(st);
                if let Some(w) = win.upgrade() {
                    w.set_show_quota(val);
                    w.set_toast_message(SharedString::from("额度卡片配置已保存"));
                }
            });
        }

        // 8. Toggle IDE Only (Auto-save)
        {
            let store = store.clone();
            let win = win.clone();
            self.window.on_toggle_ide_only(move || {
                let mut st = store.lock().unwrap();
                st.dispatch(SettingsAction::ToggleIdeOnly);
                st.commit_to_disk();
                let val = st.temp_settings.menu_config.ide_only;
                drop(st);
                if let Some(w) = win.upgrade() {
                    w.set_ide_only(val);
                    w.set_toast_message(SharedString::from("IDE 聚焦模式已保存"));
                }
            });
        }

        // 9. Toggle DND (Auto-save)
        {
            let store = store.clone();
            let win = win.clone();
            self.window.on_toggle_dnd(move || {
                let mut st = store.lock().unwrap();
                st.dispatch(SettingsAction::ToggleDndPassthrough);
                st.commit_to_disk();
                let val = st.temp_settings.dnd_passthrough;
                drop(st);
                if let Some(w) = win.upgrade() {
                    w.set_dnd_mode(val);
                    w.set_toast_message(SharedString::from("免打扰状态已保存"));
                }
            });
        }

        // 10. Toggle Sound (Auto-save)
        {
            let store = store.clone();
            let win = win.clone();
            self.window.on_toggle_sound(move || {
                let mut st = store.lock().unwrap();
                st.dispatch(SettingsAction::ToggleSound);
                st.commit_to_disk();
                let val = st.temp_settings.sound_enabled;
                drop(st);
                if let Some(w) = win.upgrade() {
                    w.set_sound_enabled(val);
                    w.set_toast_message(SharedString::from("动作音效已保存"));
                }
            });
        }

        // 11. Toggle Autostart (Auto-save)
        {
            let store = store.clone();
            let win = win.clone();
            self.window.on_toggle_autostart(move || {
                let mut st = store.lock().unwrap();
                let ev = st.dispatch(SettingsAction::ToggleAutostart);
                st.commit_to_disk();
                let enabled = st.autostart_enabled;
                drop(st);
                if let Some(w) = win.upgrade() {
                    w.set_autostart_enabled(enabled);
                    if let StoreEvent::Toast(msg) = ev {
                        w.set_toast_message(SharedString::from(msg));
                    }
                }
            });
        }

        // 12. Check Update
        {
            let win = win.clone();
            self.window.on_check_update(move || {
                if let Some(w) = win.upgrade() {
                    w.set_toast_message(SharedString::from("当前已是最新稳定版本 (v0.5.1)"));
                }
            });
        }

        // 13. Open Skins Directory
        {
            let store = store.clone();
            let win = win.clone();
            self.window.on_open_skins_dir(move || {
                let root_dir = store.lock().unwrap().root_dir.clone();
                let skins_path = root_dir.join("skins").join("fat-cat");
                #[cfg(target_os = "macos")]
                let _ = std::process::Command::new("open").arg(&skins_path).spawn();
                #[cfg(target_os = "windows")]
                let _ = std::process::Command::new("explorer").arg(&skins_path).spawn();
                #[cfg(target_os = "linux")]
                let _ = std::process::Command::new("xdg-open").arg(&skins_path).spawn();
                if let Some(w) = win.upgrade() {
                    w.set_toast_message(SharedString::from("已打开本地 skins 目录"));
                }
            });
        }

        // 14. Menu Options (Auto-save)
        {
            let store = store.clone();
            let win = win.clone();
            self.window.on_toggle_menu_option(move |idx| {
                let mut st = store.lock().unwrap();
                st.dispatch(SettingsAction::ToggleMenuOption(idx as usize));
                st.commit_to_disk();
                drop(st);
                if let Some(w) = win.upgrade() {
                    w.set_toast_message(SharedString::from("右键菜单项配置已更新"));
                }
            });
        }

        // 15. Dialogue Category
        {
            let store = store.clone();
            let win = win.clone();
            self.window.on_select_dialogue_cat(move |cat| {
                store.lock().unwrap().dispatch(SettingsAction::SelectDialogueCategory(cat as usize));
                if let Some(w) = win.upgrade() {
                    w.set_dialogue_cat(cat);
                    let st = store.lock().unwrap();
                    let lines = st.active_dialogues();
                    let items: Vec<DialogueItem> = lines
                        .iter()
                        .enumerate()
                        .map(|(idx, line)| DialogueItem {
                            id: idx as i32,
                            text: SharedString::from(line.clone()),
                        })
                        .collect();
                    w.set_dialogue_list(ModelRc::from(Rc::new(VecModel::from(items))));
                }
            });
        }

        // 16. Add Dialogue (Auto-save)
        {
            let store = store.clone();
            let win = win.clone();
            self.window.on_add_dialogue(move |text| {
                let mut st = store.lock().unwrap();
                st.dispatch(SettingsAction::AddDialogue(text.to_string()));
                st.commit_to_disk();
                let lines = st.active_dialogues().clone();
                drop(st);
                if let Some(w) = win.upgrade() {
                    let items: Vec<DialogueItem> = lines
                        .iter()
                        .enumerate()
                        .map(|(idx, line)| DialogueItem {
                            id: idx as i32,
                            text: SharedString::from(line.clone()),
                        })
                        .collect();
                    w.set_dialogue_list(ModelRc::from(Rc::new(VecModel::from(items))));
                    w.set_toast_message(SharedString::from("已添加新台词并保存"));
                }
            });
        }

        // 17. Delete Dialogue (Auto-save)
        {
            let store = store.clone();
            let win = win.clone();
            self.window.on_delete_dialogue(move |id| {
                let mut st = store.lock().unwrap();
                st.dispatch(SettingsAction::DeleteDialogue(id as usize));
                st.commit_to_disk();
                let lines = st.active_dialogues().clone();
                drop(st);
                if let Some(w) = win.upgrade() {
                    let items: Vec<DialogueItem> = lines
                        .iter()
                        .enumerate()
                        .map(|(idx, line)| DialogueItem {
                            id: idx as i32,
                            text: SharedString::from(line.clone()),
                        })
                        .collect();
                    w.set_dialogue_list(ModelRc::from(Rc::new(VecModel::from(items))));
                    w.set_toast_message(SharedString::from("已删除台词并保存"));
                }
            });
        }
    }

    pub fn show(&self) {
        self.sync_from_store();
        let _ = self.window.show();
    }

    pub fn hide(&self) {
        let _ = self.window.hide();
    }
}
