use muda::{CheckMenuItem, IsMenuItem, Menu, MenuId, MenuItem, PredefinedMenuItem, Submenu};
#[cfg(target_os = "macos")]
use muda::ContextMenu;
#[cfg(target_os = "windows")]
use muda::ContextMenu;
#[cfg(target_os = "macos")]
use std::ffi::c_void;
use std::path::PathBuf;

/// Embedded WAV sound effects for instant zero-dependency playback.
const WAV_POP_PRESS: &[u8] = include_bytes!("../../skins/fat-cat/audio/pop-press.wav");
const WAV_POP_RELEASE: &[u8] = include_bytes!("../../skins/fat-cat/audio/pop-release.wav");
const WAV_CLICK_PRESS: &[u8] = include_bytes!("../../skins/fat-cat/audio/click-press.wav");
const WAV_CLICK_RELEASE: &[u8] = include_bytes!("../../skins/fat-cat/audio/click-release.wav");

/// Tsundere Chinese dialogue corpus from `skins/fat-cat/dialogue.json`.
pub const IDLE_DIALOGUES: &[&str] = &[
    "我才不是美版豆包，哼！",
    "才、才不是特意陪你的！",
    "本喵可是 Gemini，记住啦。",
    "“10块钱一年”？本喵才不便宜！",
    "Token 可以省，夸夸不能省。",
    "本喵的聪明，可不按斤卖。",
    "怎么，又被本喵可爱到了？",
    "少熬夜！才不是担心你呢。",
    "叫声猫猫，说不定就帮你了。",
    "代码写累了？准你摸一下耳朵。",
    "看什么看，本喵只是在等你。",
    "哼，这么安静，是在憋大招？",
    "人类，今天也要记得喝水。",
    "本喵不是美版豆包，是限量星空猫猫。",
    "便宜归便宜，本喵的傲气可不打折。",
    "嘴上说随便，Prompt 倒是写了八百字。",
    "今天也在认真营业，绝对没有偷偷晒太阳。",
    "别叫我人工智障，叫我偶尔休息的天才。",
    "代码可以重构，猫猫的自尊不能回滚。",
    "本喵没摸鱼，只是在进行低功耗推理。",
    "先保存文件，再夸本喵，两件都很重要。",
    "今日份夸夸未到账，本喵保留催收权。",
];

pub const PETTING_DIALOGUES: &[&str] = &[
    "只准摸一下……再一下也行。",
    "哼，手法还算合格。",
    "谁、谁在呼噜？反正不是本喵。",
];

pub const DRAGGED_DIALOGUES: &[&str] = &[
    "本喵会走路！放下来啦。",
    "轻点拎，呆毛要乱了！",
];

pub const THINKING_DIALOGUES: &[&str] = &[
    "别催，在长脑子了。",
    "本喵认真起来，你可看好了。",
    "喵式推理中，先别揪耳朵。",
];

pub const SUCCESS_DIALOGUES: &[&str] = &[
    "又走完一步啦，夸本喵！",
    "小小一步，得意一下怎么了？",
    "这一步完成了，本喵记着呢。",
];

pub const ERROR_DIALOGUES: &[&str] = &[
    "这步没走通，陪你再看看。",
    "哼，报错而已，一起查就是了。",
    "本喵也会卡壳，不许笑！",
];

pub fn pick_random_line(lines: &[&'static str]) -> &'static str {
    if lines.is_empty() {
        return "本喵随时待命~";
    }
    let idx = fastrand::usize(..lines.len());
    lines[idx]
}

#[derive(Clone, Copy, Debug)]
pub enum SoundEffect {
    PopPress,
    PopRelease,
    ClickPress,
    ClickRelease,
}

/// Lightweight native audio player (extracts embedded WAV to `/tmp` once and plays asynchronously).
pub struct NativeAudioPlayer {
    pub enabled: bool,
    temp_dir: PathBuf,
}

impl NativeAudioPlayer {
    pub fn new(enabled: bool) -> Self {
        let temp_dir = std::env::temp_dir().join("anti-pet-audio-v030");
        let _ = std::fs::create_dir_all(&temp_dir);
        let _ = std::fs::write(temp_dir.join("pop-press.wav"), WAV_POP_PRESS);
        let _ = std::fs::write(temp_dir.join("pop-release.wav"), WAV_POP_RELEASE);
        let _ = std::fs::write(temp_dir.join("click-press.wav"), WAV_CLICK_PRESS);
        let _ = std::fs::write(temp_dir.join("click-release.wav"), WAV_CLICK_RELEASE);
        Self { enabled, temp_dir }
    }

    pub fn play(&self, sfx: SoundEffect) {
        if !self.enabled {
            return;
        }
        let filename = match sfx {
            SoundEffect::PopPress => "pop-press.wav",
            SoundEffect::PopRelease => "pop-release.wav",
            SoundEffect::ClickPress => "click-press.wav",
            SoundEffect::ClickRelease => "click-release.wav",
        };
        let path = self.temp_dir.join(filename);
        if !path.exists() {
            return;
        }

        #[cfg(target_os = "macos")]
        {
            let _ = std::process::Command::new("/usr/bin/afplay")
                .arg("-v")
                .arg("0.75")
                .arg(&path)
                .spawn();
        }
    }
}

/// High-level action decoded from either the Tray Menu or the Right-Click Context Menu.
#[derive(Clone, Debug, PartialEq)]
pub enum PetMenuAction {
    OpenSettings,
    ResetPosition,
    NextDialogue,
    PetHead,
    RefreshQuota,
    ToggleShowQuota,
    ToggleDndMode,
    ToggleGravity,
    SetFloorMode(&'static str),
    SetZoom(f32),
    ToggleSound,
    ToggleVisibility,
    OpenAntigravity,
    Quit,
}

#[cfg(target_os = "macos")]
pub fn is_antigravity_running() -> bool {
    let out = std::process::Command::new("/usr/bin/pgrep")
        .args(["-f", "Antigravity.app"])
        .output();
    if let Ok(res) = out {
        if res.status.success() {
            return true;
        }
    }
    let out2 = std::process::Command::new("/usr/bin/pgrep")
        .args(["-x", "Antigravity"])
        .output();
    if let Ok(res2) = out2 {
        if res2.status.success() {
            return true;
        }
    }
    false
}

#[cfg(not(target_os = "macos"))]
pub fn is_antigravity_running() -> bool {
    false
}

/// Native OS Menu Manager (`muda`) serving both the System Tray Menu and the Right-Click Context Menu.
pub struct NativeMenuManager {
    pub menu: Menu,
    item_settings: MenuItem,
    item_reset_center: MenuItem,
    item_next_dialogue: MenuItem,
    item_pet_head: MenuItem,
    item_refresh_quota: MenuItem,
    pub item_show_quota: CheckMenuItem,
    pub item_dnd: CheckMenuItem,
    pub item_gravity: CheckMenuItem,
    pub item_floor_bottom: CheckMenuItem,
    pub item_floor_dock: CheckMenuItem,
    pub item_floor_free: CheckMenuItem,
    floor_sub: Submenu,
    zoom_sub: Submenu,
    id_zoom_80: MenuId,
    id_zoom_100: MenuId,
    id_zoom_125: MenuId,
    id_zoom_150: MenuId,
    pub item_sound: CheckMenuItem,
    item_toggle_vis: MenuItem,
    item_open_agy: MenuItem,
    item_about: MenuItem,
    item_quit: MenuItem,
    id_settings: MenuId,
    id_reset_center: MenuId,
    id_next_dialogue: MenuId,
    id_pet_head: MenuId,
    id_refresh_quota: MenuId,
    id_toggle_vis: MenuId,
    id_open_agy: MenuId,
    id_quit: MenuId,
}

impl NativeMenuManager {
    pub fn new(settings: &crate::state_watcher::PetSettings) -> Self {
        let menu = Menu::new();

        let item_settings = MenuItem::new("偏好设置...", true, None);
        let item_reset_center = MenuItem::new("复位居中", true, None);

        let item_next_dialogue = MenuItem::new("换一句傲娇台词", true, None);
        let item_pet_head = MenuItem::new("摸摸猫猫头", true, None);
        let item_refresh_quota = MenuItem::new("立即刷新 Antigravity 额度", true, None);

        let item_show_quota = CheckMenuItem::new("气泡内显示额度条", true, settings.show_quota, None);
        let item_dnd = CheckMenuItem::new("免打扰模式 (右键猫咪可随时退出)", true, settings.dnd_passthrough, None);
        let item_gravity = CheckMenuItem::new("启用物理重力掉落", true, settings.gravity_enabled, None);
        let item_sound = CheckMenuItem::new("启用 Q 弹互动音效", true, settings.sound_enabled, None);

        // Floor mode submenu
        let floor_sub = Submenu::new("掉落落点位置", true);
        let item_floor_bottom = CheckMenuItem::new(
            "屏幕最底边 (贴底)",
            true,
            settings.floor_mode == "bottom",
            None,
        );
        let item_floor_dock = CheckMenuItem::new(
            "Mac 程序坞 / 任务栏上方",
            true,
            settings.floor_mode == "taskbar" || settings.floor_mode == "dock",
            None,
        );
        let item_floor_free = CheckMenuItem::new(
            "自由悬浮 (松手不掉落)",
            true,
            settings.floor_mode == "free",
            None,
        );
        let _ = floor_sub.append_items(&[&item_floor_bottom, &item_floor_dock, &item_floor_free]);

        // Zoom submenu
        let zoom_sub = Submenu::new("猫咪体型缩放", true);
        let item_zoom_80 = MenuItem::new("小巧 (80%)", true, None);
        let item_zoom_100 = MenuItem::new("标准 (100%)", true, None);
        let item_zoom_125 = MenuItem::new("圆润 (125%)", true, None);
        let item_zoom_150 = MenuItem::new("超大肥猫 (150%)", true, None);
        let _ = zoom_sub.append_items(&[
            &item_zoom_80,
            &item_zoom_100,
            &item_zoom_125,
            &item_zoom_150,
        ]);

        let item_toggle_vis = MenuItem::new("显示 / 隐藏 大肥猫", true, None);
        let item_open_agy = MenuItem::new("唤起 Antigravity IDE", true, None);
        let item_about = MenuItem::new("大肥猫桌宠 v0.3.10 (Pure CPU 2D)", false, None);
        let item_quit = MenuItem::new("退出", true, None);

        let id_settings = item_settings.id().clone();
        let id_reset_center = item_reset_center.id().clone();
        let id_next_dialogue = item_next_dialogue.id().clone();
        let id_pet_head = item_pet_head.id().clone();
        let id_refresh_quota = item_refresh_quota.id().clone();
        let id_zoom_80 = item_zoom_80.id().clone();
        let id_zoom_100 = item_zoom_100.id().clone();
        let id_zoom_125 = item_zoom_125.id().clone();
        let id_zoom_150 = item_zoom_150.id().clone();
        let id_toggle_vis = item_toggle_vis.id().clone();
        let id_open_agy = item_open_agy.id().clone();
        let id_quit = item_quit.id().clone();

        let mut mgr = Self {
            menu,
            item_settings,
            item_reset_center,
            item_next_dialogue,
            item_pet_head,
            item_refresh_quota,
            item_show_quota,
            item_dnd,
            item_gravity,
            item_floor_bottom,
            item_floor_dock,
            item_floor_free,
            floor_sub,
            zoom_sub,
            id_zoom_80,
            id_zoom_100,
            id_zoom_125,
            id_zoom_150,
            item_sound,
            item_toggle_vis,
            item_open_agy,
            item_about,
            item_quit,
            id_settings,
            id_reset_center,
            id_next_dialogue,
            id_pet_head,
            id_refresh_quota,
            id_toggle_vis,
            id_open_agy,
            id_quit,
        };
        mgr.rebuild_menu(settings);
        mgr
    }

    /// Rebuilds the menu hierarchy to strictly match user configuration (`menuConfig`).
    pub fn rebuild_menu(&mut self, settings: &crate::state_watcher::PetSettings) {
        while self.menu.remove_at(0).is_some() {}

        self.item_show_quota.set_checked(settings.show_quota);
        self.item_dnd.set_checked(settings.dnd_passthrough);
        self.item_gravity.set_checked(settings.gravity_enabled);
        self.item_floor_bottom.set_checked(settings.floor_mode == "bottom");
        self.item_floor_dock.set_checked(settings.floor_mode == "taskbar" || settings.floor_mode == "dock");
        self.item_floor_free.set_checked(settings.floor_mode == "free");
        self.item_sound.set_checked(settings.sound_enabled);

        let mut items: Vec<&dyn IsMenuItem> = Vec::new();

        if settings.menu_config.settings {
            items.push(&self.item_settings);
        }

        // 复位居中 (无条件提供，避免迷航)
        items.push(&self.item_reset_center);

        let sep0 = PredefinedMenuItem::separator();
        items.push(&sep0);

        if settings.menu_config.random_dialogue {
            items.push(&self.item_next_dialogue);
        }
        if settings.menu_config.pet_action {
            items.push(&self.item_pet_head);
        }
        if settings.menu_config.quota {
            items.push(&self.item_refresh_quota);
            items.push(&self.item_show_quota);
        }
        if settings.menu_config.dnd || settings.dnd_passthrough {
            items.push(&self.item_dnd);
        }
        if settings.menu_config.gravity {
            items.push(&self.item_gravity);
            items.push(&self.floor_sub);
        }
        if settings.menu_config.zoom {
            items.push(&self.zoom_sub);
        }
        if settings.menu_config.sound {
            items.push(&self.item_sound);
        }

        let sep1 = PredefinedMenuItem::separator();
        items.push(&sep1);

        items.push(&self.item_toggle_vis);

        if settings.menu_config.open_antigravity && !is_antigravity_running() {
            items.push(&self.item_open_agy);
        }

        items.push(&self.item_about);

        let sep2 = PredefinedMenuItem::separator();
        items.push(&sep2);

        items.push(&self.item_quit);

        let _ = self.menu.append_items(&items);
    }

    /// Pops up the native OS context menu right at the current cursor position on right-click!
    #[cfg(target_os = "macos")]
    pub fn show_context_menu_macos(&self, ns_view: *mut c_void, pos: Option<muda::dpi::LogicalPosition<f64>>) {
        if ns_view.is_null() {
            return;
        }
        unsafe {
            let pos_muda = pos.map(muda::dpi::Position::Logical);
            let _ = self.menu.show_context_menu_for_nsview(ns_view, pos_muda);
        }
    }

    #[cfg(target_os = "windows")]
    pub fn show_context_menu_windows(&self, hwnd: isize, pos: Option<muda::dpi::LogicalPosition<f64>>) {
        unsafe {
            let pos_muda = pos.map(muda::dpi::Position::Logical);
            let _ = self.menu.show_context_menu_for_hwnd(hwnd, pos_muda);
        }
    }

    pub fn resolve_event(&self, id: &MenuId) -> Option<PetMenuAction> {
        if *id == self.id_settings {
            Some(PetMenuAction::OpenSettings)
        } else if *id == self.id_reset_center {
            Some(PetMenuAction::ResetPosition)
        } else if *id == self.id_next_dialogue {
            Some(PetMenuAction::NextDialogue)
        } else if *id == self.id_pet_head {
            Some(PetMenuAction::PetHead)
        } else if *id == self.id_refresh_quota {
            Some(PetMenuAction::RefreshQuota)
        } else if *id == *self.item_show_quota.id() {
            Some(PetMenuAction::ToggleShowQuota)
        } else if *id == *self.item_dnd.id() {
            Some(PetMenuAction::ToggleDndMode)
        } else if *id == *self.item_gravity.id() {
            Some(PetMenuAction::ToggleGravity)
        } else if *id == *self.item_floor_bottom.id() {
            self.item_floor_bottom.set_checked(true);
            self.item_floor_dock.set_checked(false);
            self.item_floor_free.set_checked(false);
            Some(PetMenuAction::SetFloorMode("bottom"))
        } else if *id == *self.item_floor_dock.id() {
            self.item_floor_bottom.set_checked(false);
            self.item_floor_dock.set_checked(true);
            self.item_floor_free.set_checked(false);
            Some(PetMenuAction::SetFloorMode("taskbar"))
        } else if *id == *self.item_floor_free.id() {
            self.item_floor_bottom.set_checked(false);
            self.item_floor_dock.set_checked(false);
            self.item_floor_free.set_checked(true);
            Some(PetMenuAction::SetFloorMode("free"))
        } else if *id == self.id_zoom_80 {
            Some(PetMenuAction::SetZoom(0.8))
        } else if *id == self.id_zoom_100 {
            Some(PetMenuAction::SetZoom(1.0))
        } else if *id == self.id_zoom_125 {
            Some(PetMenuAction::SetZoom(1.25))
        } else if *id == self.id_zoom_150 {
            Some(PetMenuAction::SetZoom(1.5))
        } else if *id == *self.item_sound.id() {
            Some(PetMenuAction::ToggleSound)
        } else if *id == self.id_toggle_vis {
            Some(PetMenuAction::ToggleVisibility)
        } else if *id == self.id_open_agy {
            Some(PetMenuAction::OpenAntigravity)
        } else if *id == self.id_quit {
            Some(PetMenuAction::Quit)
        } else {
            None
        }
    }
}
