use crate::core::store::SettingsStore;
use std::ffi::c_void;
use std::sync::{Arc, Mutex};

extern "C" {
    fn objc_getClass(name: *const u8) -> *mut c_void;
    fn sel_registerName(name: *const u8) -> *mut c_void;
    fn objc_msgSend();
}

#[inline]
unsafe fn sel(name: &[u8]) -> *mut c_void {
    sel_registerName(name.as_ptr())
}

#[inline]
unsafe fn nsstring(s: &str) -> *mut c_void {
    let str_cls = objc_getClass(b"NSString\0".as_ptr());
    let msg_str: unsafe extern "C" fn(*mut c_void, *mut c_void, *const u8, usize, usize) -> *mut c_void =
        std::mem::transmute(objc_msgSend as *const ());
    msg_str(str_cls, sel(b"stringWithBytes:length:encoding:\0"), s.as_ptr(), s.len(), 4) // 4 = NSUTF8StringEncoding
}

/// Native macOS AppKit Settings Window Controller.
/// Uses 100% pure Cocoa `NSWindow`, `NSVisualEffectView`, `NSButton`, `NSSwitch`, `NSSlider`,
/// and `NSTextField` for genuine Apple system aesthetics with 0 KB extra bundle size.
pub struct MacosSettingsApp {
    pub store: Arc<Mutex<SettingsStore>>,
    pub ns_window: *mut c_void,
    pub content_container: *mut c_void,
    pub toast_label: *mut c_void,
    pub tab_views: Vec<*mut c_void>,
    pub current_tab: usize,
}

// Global reference for native target-action callback routing
static mut GLOBAL_MACOS_APP: Option<*mut MacosSettingsApp> = None;

impl MacosSettingsApp {
    pub fn new(store: Arc<Mutex<SettingsStore>>) -> Self {
        unsafe {
            let (window, content_container, toast_label, tab_views) = Self::create_ui(&store);
            Self {
                store,
                ns_window: window,
                content_container,
                toast_label,
                tab_views,
                current_tab: 0,
            }
        }
    }

    unsafe fn create_ui(store: &Arc<Mutex<SettingsStore>>) -> (*mut c_void, *mut c_void, *mut c_void, Vec<*mut c_void>) {
        let msg_ptr: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
            std::mem::transmute(objc_msgSend as *const ());
        let msg_void_bool: unsafe extern "C" fn(*mut c_void, *mut c_void, i8) =
            std::mem::transmute(objc_msgSend as *const ());
        let msg_void_isize: unsafe extern "C" fn(*mut c_void, *mut c_void, isize) =
            std::mem::transmute(objc_msgSend as *const ());
        let msg_void_usize: unsafe extern "C" fn(*mut c_void, *mut c_void, usize) =
            std::mem::transmute(objc_msgSend as *const ());
        let msg_void_ptr: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) =
            std::mem::transmute(objc_msgSend as *const ());
        let msg_subview: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void, isize, *mut c_void) =
            std::mem::transmute(objc_msgSend as *const ());
        let msg_init_window: unsafe extern "C" fn(*mut c_void, *mut c_void, [f64; 4], usize, usize, i8) -> *mut c_void =
            std::mem::transmute(objc_msgSend as *const ());
        let msg_init_frame: unsafe extern "C" fn(*mut c_void, *mut c_void, [f64; 4]) -> *mut c_void =
            std::mem::transmute(objc_msgSend as *const ());

        let window_cls = objc_getClass(b"NSWindow\0".as_ptr());
        let effect_cls = objc_getClass(b"NSVisualEffectView\0".as_ptr());
        let view_cls = objc_getClass(b"NSView\0".as_ptr());
        let color_cls = objc_getClass(b"NSColor\0".as_ptr());
        let button_cls = objc_getClass(b"NSButton\0".as_ptr());
        let textfield_cls = objc_getClass(b"NSTextField\0".as_ptr());

        let win_w = 700.0f64;
        let win_h = 500.0f64;
        let rect = [100.0, 100.0, win_w, win_h];

        // 1. Create NSWindow
        let style_mask = 1 | 2 | 4 | (1 << 15); // Titled, Closable, Miniaturizable, FullSizeContentView
        let win_alloc = msg_ptr(window_cls, sel(b"alloc\0"));
        let win = msg_init_window(win_alloc, sel(b"initWithContentRect:styleMask:backing:defer:\0"), rect, style_mask, 2, 0);

        msg_void_ptr(win, sel(b"setTitle:\0"), nsstring("Anti-Pet 偏好设置 (macOS AppKit 原生)"));
        msg_void_bool(win, sel(b"setOpaque:\0"), 0);
        msg_void_bool(win, sel(b"setTitlebarAppearsTransparent:\0"), 1);
        msg_void_bool(win, sel(b"setIsMovableByWindowBackground:\0"), 1);

        if !color_cls.is_null() {
            let clear_color = msg_ptr(color_cls, sel(b"clearColor\0"));
            if !clear_color.is_null() {
                msg_void_ptr(win, sel(b"setBackgroundColor:\0"), clear_color);
            }
        }

        let content_view = msg_ptr(win, sel(b"contentView\0"));

        // 2. Full-Window NSVisualEffectView (Genuine Apple frosted glass blur)
        let full_frame = [0.0, 0.0, win_w, win_h];
        let effect_alloc = msg_ptr(effect_cls, sel(b"alloc\0"));
        let effect_view = msg_init_frame(effect_alloc, sel(b"initWithFrame:\0"), full_frame);
        if !effect_view.is_null() {
            msg_void_isize(effect_view, sel(b"setMaterial:\0"), 7); // NSVisualEffectMaterialSidebar = 7
            msg_void_isize(effect_view, sel(b"setBlendingMode:\0"), 0); // BehindWindow
            msg_void_isize(effect_view, sel(b"setState:\0"), 1); // Active
            msg_void_usize(effect_view, sel(b"setAutoresizingMask:\0"), 18); // WidthSizable | HeightSizable
            msg_subview(content_view, sel(b"addSubview:positioned:relativeTo:\0"), effect_view, -1, std::ptr::null_mut());
        }

        // 3. Left Sidebar (w = 180, h = 500)
        let sidebar_w = 180.0f64;
        let sidebar_frame = [0.0, 0.0, sidebar_w, win_h];
        let sidebar_view = msg_init_frame(msg_ptr(view_cls, sel(b"alloc\0")), sel(b"initWithFrame:\0"), sidebar_frame);
        msg_subview(content_view, sel(b"addSubview:positioned:relativeTo:\0"), sidebar_view, 1, std::ptr::null_mut());

        // Sidebar Title
        let msg_label: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> *mut c_void =
            std::mem::transmute(objc_msgSend as *const ());
        let title_lbl = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring("偏好设置"));
        let set_frame: unsafe extern "C" fn(*mut c_void, *mut c_void, [f64; 4]) =
            std::mem::transmute(objc_msgSend as *const ());
        set_frame(title_lbl, sel(b"setFrame:\0"), [18.0, 440.0, 140.0, 24.0]);
        msg_subview(sidebar_view, sel(b"addSubview:\0"), title_lbl, 1, std::ptr::null_mut());

        let ver_lbl = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring("Anti-Pet v0.5.0 AppKit"));
        set_frame(ver_lbl, sel(b"setFrame:\0"), [18.0, 422.0, 140.0, 16.0]);
        msg_subview(sidebar_view, sel(b"addSubview:\0"), ver_lbl, 1, std::ptr::null_mut());

        // Toast label at bottom of sidebar
        let toast_lbl = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring(""));
        set_frame(toast_lbl, sel(b"setFrame:\0"), [18.0, 16.0, 140.0, 32.0]);
        msg_subview(sidebar_view, sel(b"addSubview:\0"), toast_lbl, 1, std::ptr::null_mut());

        // Sidebar Tab Buttons
        let tabs = ["常规与显示", "自启动与更新", "皮肤管理", "台词与问候", "右键菜单配置"];
        let msg_btn: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void, *mut c_void, *mut c_void) -> *mut c_void =
            std::mem::transmute(objc_msgSend as *const ());
        for (i, tab_title) in tabs.iter().enumerate() {
            let btn_y = 370.0 - (i as f64 * 40.0);
            let btn = msg_btn(button_cls, sel(b"buttonWithTitle:target:action:\0"), nsstring(tab_title), std::ptr::null_mut(), std::ptr::null_mut());
            set_frame(btn, sel(b"setFrame:\0"), [12.0, btn_y, 156.0, 32.0]);
            msg_void_isize(btn, sel(b"setTag:\0"), 100 + i as isize);
            msg_subview(sidebar_view, sel(b"addSubview:\0"), btn, 1, std::ptr::null_mut());
        }

        // 4. Right Content Container (x = 180, w = 520, h = 500)
        let content_w = win_w - sidebar_w;
        let content_frame = [sidebar_w, 0.0, content_w, win_h];
        let content_container = msg_init_frame(msg_ptr(view_cls, sel(b"alloc\0")), sel(b"initWithFrame:\0"), content_frame);
        msg_subview(content_view, sel(b"addSubview:positioned:relativeTo:\0"), content_container, 1, std::ptr::null_mut());

        // 5. Create Tab Views
        let mut tab_views = Vec::new();
        let view_frame = [0.0, 50.0, content_w, win_h - 50.0];

        // --- TAB 0: General Tab View ---
        let snap = store.lock().unwrap().snapshot();
        let tab0_view = msg_init_frame(msg_ptr(view_cls, sel(b"alloc\0")), sel(b"initWithFrame:\0"), view_frame);
        {
            let header = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring("常规与显示"));
            set_frame(header, sel(b"setFrame:\0"), [24.0, 400.0, 200.0, 26.0]);
            msg_subview(tab0_view, sel(b"addSubview:\0"), header, 1, std::ptr::null_mut());

            // 1. Theme
            let theme_lbl = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring("界面外观风格: 跟随系统 / 深色 / 浅色"));
            set_frame(theme_lbl, sel(b"setFrame:\0"), [24.0, 360.0, 300.0, 20.0]);
            msg_subview(tab0_view, sel(b"addSubview:\0"), theme_lbl, 1, std::ptr::null_mut());

            let theme_btn = msg_btn(button_cls, sel(b"buttonWithTitle:target:action:\0"), nsstring("切换主题"), std::ptr::null_mut(), std::ptr::null_mut());
            set_frame(theme_btn, sel(b"setFrame:\0"), [380.0, 355.0, 100.0, 28.0]);
            msg_void_isize(theme_btn, sel(b"setTag:\0"), 201);
            msg_subview(tab0_view, sel(b"addSubview:\0"), theme_btn, 1, std::ptr::null_mut());

            // 2. Scale
            let scale_title = format!("缩放大小: {}% (AppKit 原生滑块)", (snap.temp_settings.scale * 100.0).round() as i32);
            let scale_lbl = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring(&scale_title));
            set_frame(scale_lbl, sel(b"setFrame:\0"), [24.0, 315.0, 260.0, 20.0]);
            msg_subview(tab0_view, sel(b"addSubview:\0"), scale_lbl, 1, std::ptr::null_mut());

            let slider_cls = objc_getClass(b"NSSlider\0".as_ptr());
            let msg_slider: unsafe extern "C" fn(*mut c_void, *mut c_void, f64, f64, f64, *mut c_void, *mut c_void) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const ());
            let slider = msg_slider(slider_cls, sel(b"sliderWithValue:minValue:maxValue:target:action:\0"), snap.temp_settings.scale as f64, 0.5, 2.0, std::ptr::null_mut(), std::ptr::null_mut());
            set_frame(slider, sel(b"setFrame:\0"), [280.0, 315.0, 200.0, 20.0]);
            msg_void_isize(slider, sel(b"setTag:\0"), 202);
            msg_subview(tab0_view, sel(b"addSubview:\0"), slider, 1, std::ptr::null_mut());

            // 3. Floor Mode
            let floor_lbl = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring("落地吸附基线: 任务栏 vs 屏幕底边"));
            set_frame(floor_lbl, sel(b"setFrame:\0"), [24.0, 270.0, 260.0, 20.0]);
            msg_subview(tab0_view, sel(b"addSubview:\0"), floor_lbl, 1, std::ptr::null_mut());

            let floor_btn = msg_btn(button_cls, sel(b"buttonWithTitle:target:action:\0"), nsstring("切换落地点"), std::ptr::null_mut(), std::ptr::null_mut());
            set_frame(floor_btn, sel(b"setFrame:\0"), [380.0, 265.0, 100.0, 28.0]);
            msg_void_isize(floor_btn, sel(b"setTag:\0"), 203);
            msg_subview(tab0_view, sel(b"addSubview:\0"), floor_btn, 1, std::ptr::null_mut());

            // 4. Reset Position
            let reset_lbl = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring("屏幕位置校准: 一键拉回屏幕正中央"));
            set_frame(reset_lbl, sel(b"setFrame:\0"), [24.0, 225.0, 260.0, 20.0]);
            msg_subview(tab0_view, sel(b"addSubview:\0"), reset_lbl, 1, std::ptr::null_mut());

            let reset_btn = msg_btn(button_cls, sel(b"buttonWithTitle:target:action:\0"), nsstring("复位居中"), std::ptr::null_mut(), std::ptr::null_mut());
            set_frame(reset_btn, sel(b"setFrame:\0"), [380.0, 220.0, 100.0, 28.0]);
            msg_void_isize(reset_btn, sel(b"setTag:\0"), 204);
            msg_subview(tab0_view, sel(b"addSubview:\0"), reset_btn, 1, std::ptr::null_mut());

            // 5. Switches: Gravity, Quota, Sound, DND
            let sw_cls = objc_getClass(b"NSSwitch\0".as_ptr());
            let toggles = [
                ("重力加速度物理下落", 205, snap.temp_settings.gravity_enabled, 175.0),
                ("气泡实时额度卡片", 206, snap.temp_settings.show_quota, 135.0),
                ("完全穿透免打扰", 207, snap.temp_settings.dnd_passthrough, 95.0),
                ("动作音效 (挂件交互音频)", 208, snap.temp_settings.sound_enabled, 55.0),
            ];
            for (title, tag, is_on, y_pos) in toggles {
                let lbl = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring(title));
                set_frame(lbl, sel(b"setFrame:\0"), [24.0, y_pos, 300.0, 20.0]);
                msg_subview(tab0_view, sel(b"addSubview:\0"), lbl, 1, std::ptr::null_mut());

                if !sw_cls.is_null() {
                    let sw = msg_init_frame(msg_ptr(sw_cls, sel(b"alloc\0")), sel(b"initWithFrame:\0"), [420.0, y_pos - 4.0, 60.0, 28.0]);
                    msg_void_isize(sw, sel(b"setState:\0"), if is_on { 1 } else { 0 });
                    msg_void_isize(sw, sel(b"setTag:\0"), tag);
                    msg_subview(tab0_view, sel(b"addSubview:\0"), sw, 1, std::ptr::null_mut());
                }
            }
        }
        tab_views.push(tab0_view);
        msg_subview(content_container, sel(b"addSubview:\0"), tab0_view, 1, std::ptr::null_mut());

        // 6. Bottom Action Bar (Fixed across all tabs)
        let cancel_btn = msg_btn(button_cls, sel(b"buttonWithTitle:target:action:\0"), nsstring("取消"), std::ptr::null_mut(), std::ptr::null_mut());
        set_frame(cancel_btn, sel(b"setFrame:\0"), [content_w - 190.0, 14.0, 80.0, 28.0]);
        msg_void_isize(cancel_btn, sel(b"setTag:\0"), 998);
        msg_subview(content_container, sel(b"addSubview:\0"), cancel_btn, 1, std::ptr::null_mut());

        let save_btn = msg_btn(button_cls, sel(b"buttonWithTitle:target:action:\0"), nsstring("确定"), std::ptr::null_mut(), std::ptr::null_mut());
        set_frame(save_btn, sel(b"setFrame:\0"), [content_w - 98.0, 14.0, 80.0, 28.0]);
        msg_void_isize(save_btn, sel(b"setTag:\0"), 999);
        msg_void_ptr(save_btn, sel(b"setKeyEquivalent:\0"), nsstring("\r")); // Return key triggers save
        msg_subview(content_container, sel(b"addSubview:\0"), save_btn, 1, std::ptr::null_mut());

        msg_ptr(win, sel(b"center\0"));
        (win, content_container, toast_lbl, tab_views)
    }

    pub fn show(&mut self) {
        if self.ns_window.is_null() {
            return;
        }
        unsafe {
            GLOBAL_MACOS_APP = Some(self as *mut Self);
            let msg_void_ptr: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) =
                std::mem::transmute(objc_msgSend as *const ());
            msg_void_ptr(self.ns_window, sel(b"makeKeyAndOrderFront:\0"), std::ptr::null_mut());
        }
    }

    pub fn hide(&self) {
        if self.ns_window.is_null() {
            return;
        }
        unsafe {
            let msg_ptr: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const ());
            msg_ptr(self.ns_window, sel(b"orderOut:\0"));
        }
    }
}

/// Runs the standalone macOS AppKit native settings window with Cocoa runloop.
pub fn run_macos_appkit_settings() -> Result<(), Box<dyn std::error::Error>> {
    use crate::state_watcher::{resolve_project_root, NativeStateWatcher};

    unsafe {
        let app_cls = objc_getClass(b"NSApplication\0".as_ptr());
        let msg_ptr: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
            std::mem::transmute(objc_msgSend as *const ());
        let msg_void_isize: unsafe extern "C" fn(*mut c_void, *mut c_void, isize) =
            std::mem::transmute(objc_msgSend as *const ());
        let app = msg_ptr(app_cls, sel(b"sharedApplication\0"));
        // NSApplicationActivationPolicyRegular = 0
        msg_void_isize(app, sel(b"setActivationPolicy:\0"), 0);

        let watcher = NativeStateWatcher::new();
        let settings = watcher.load_settings();
        let root_dir = resolve_project_root();

        let store = Arc::new(Mutex::new(SettingsStore::new(root_dir, settings)));
        let mut settings_app = MacosSettingsApp::new(store);
        settings_app.show();

        msg_ptr(app, sel(b"run\0"));
    }
    Ok(())
}
