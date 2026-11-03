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

        // 2. Cycle Theme
        {
            let store = store.clone();
            let win = win.clone();
            self.window.on_cycle_theme(move || {
                store.lock().unwrap().dispatch(SettingsAction::CycleTheme);
                let theme = store.lock().unwrap().temp_settings.theme.clone();
                if let Some(w) = win.upgrade() {
                    w.set_theme_mode(SharedString::from(theme));
                }
            });
        }

        // 3. Scale Changed
        {
            let store = store.clone();
            let win = win.clone();
            self.window.on_scale_changed(move |scale| {
                store.lock().unwrap().dispatch(SettingsAction::SetScale(scale));
                let cur = store.lock().unwrap().temp_settings.scale;
                if let Some(w) = win.upgrade() {
                    w.set_pet_scale(cur);
                }
            });
        }

        // 4. Cycle Floor Mode
        {
            let store = store.clone();
            let win = win.clone();
            self.window.on_cycle_floor_mode(move || {
                store.lock().unwrap().dispatch(SettingsAction::CycleFloorMode);
                let mode = store.lock().unwrap().temp_settings.floor_mode.clone();
                if let Some(w) = win.upgrade() {
                    w.set_floor_mode(SharedString::from(mode));
                }
            });
        }

        // 5. Toggle Gravity
        {
            let store = store.clone();
            let win = win.clone();
            self.window.on_toggle_gravity(move || {
                store.lock().unwrap().dispatch(SettingsAction::ToggleGravity);
                let val = store.lock().unwrap().temp_settings.gravity_enabled;
                if let Some(w) = win.upgrade() {
                    w.set_gravity_enabled(val);
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

        // 7. Toggle Quota
        {
            let store = store.clone();
            let win = win.clone();
            self.window.on_toggle_quota(move || {
                store.lock().unwrap().dispatch(SettingsAction::ToggleShowQuota);
                let val = store.lock().unwrap().temp_settings.show_quota;
                if let Some(w) = win.upgrade() {
                    w.set_show_quota(val);
                }
            });
        }

        // 8. Toggle IDE Only
        {
            let store = store.clone();
            let win = win.clone();
            self.window.on_toggle_ide_only(move || {
                store.lock().unwrap().dispatch(SettingsAction::ToggleIdeOnly);
                let val = store.lock().unwrap().temp_settings.menu_config.ide_only;
                if let Some(w) = win.upgrade() {
                    w.set_ide_only(val);
                }
            });
        }

        // 9. Toggle DND
        {
            let store = store.clone();
            let win = win.clone();
            self.window.on_toggle_dnd(move || {
                store.lock().unwrap().dispatch(SettingsAction::ToggleDndPassthrough);
                let val = store.lock().unwrap().temp_settings.dnd_passthrough;
                if let Some(w) = win.upgrade() {
                    w.set_dnd_mode(val);
                }
            });
        }

        // 10. Toggle Sound
        {
            let store = store.clone();
            let win = win.clone();
            self.window.on_toggle_sound(move || {
                store.lock().unwrap().dispatch(SettingsAction::ToggleSound);
                let val = store.lock().unwrap().temp_settings.sound_enabled;
                if let Some(w) = win.upgrade() {
                    w.set_sound_enabled(val);
                }
            });
        }

        // 11. Toggle Autostart
        {
            let store = store.clone();
            let win = win.clone();
            self.window.on_toggle_autostart(move || {
                let ev = store.lock().unwrap().dispatch(SettingsAction::ToggleAutostart);
                let enabled = store.lock().unwrap().autostart_enabled;
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
                    w.set_toast_message(SharedString::from("当前已是最新版本 (v0.5.0)"));
                }
            });
        }

        // 13. Menu Options
        {
            let store = store.clone();
            self.window.on_toggle_menu_option(move |idx| {
                store.lock().unwrap().dispatch(SettingsAction::ToggleMenuOption(idx as usize));
            });
        }

        // 14. Dialogue Category
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

        // 15. Add Dialogue
        {
            let store = store.clone();
            let win = win.clone();
            self.window.on_add_dialogue(move |text| {
                store.lock().unwrap().dispatch(SettingsAction::AddDialogue(text.to_string()));
                if let Some(w) = win.upgrade() {
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
                    w.set_toast_message(SharedString::from("已添加新台词"));
                }
            });
        }

        // 16. Delete Dialogue
        {
            let store = store.clone();
            let win = win.clone();
            self.window.on_delete_dialogue(move |id| {
                store.lock().unwrap().dispatch(SettingsAction::DeleteDialogue(id as usize));
                if let Some(w) = win.upgrade() {
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
                    w.set_toast_message(SharedString::from("已删除台词"));
                }
            });
        }

        // 17. Save Settings
        {
            let store = store.clone();
            let win = win.clone();
            self.window.on_save_settings(move || {
                store.lock().unwrap().dispatch(SettingsAction::Save);
                if let Some(w) = win.upgrade() {
                    w.set_toast_message(SharedString::from("设置已成功保存"));
                    let _ = w.hide();
                }
            });
        }

        // 18. Cancel Settings
        {
            let store = store.clone();
            let win = win.clone();
            self.window.on_cancel_settings(move || {
                store.lock().unwrap().dispatch(SettingsAction::Cancel);
                if let Some(w) = win.upgrade() {
                    let _ = w.hide();
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
