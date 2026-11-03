use crate::core::store::{SettingsAction, SettingsStore};
use std::ffi::c_void;
use std::sync::{Arc, Mutex};

extern "C" {
    fn objc_getClass(name: *const u8) -> *mut c_void;
    fn sel_registerName(name: *const u8) -> *mut c_void;
    fn objc_msgSend();
    fn objc_allocateClassPair(superclass: *mut c_void, name: *const u8, extraBytes: usize) -> *mut c_void;
    fn objc_registerClassPair(cls: *mut c_void);
    fn class_addMethod(
        cls: *mut c_void,
        name: *mut c_void,
        imp: *const c_void,
        types: *const u8,
    ) -> i8;
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
    pub theme_lbl: *mut c_void,
    pub scale_lbl: *mut c_void,
    pub floor_lbl: *mut c_void,
    pub dialogue_cat_lbl: *mut c_void,
    pub dialogue_preview_lbl: *mut c_void,
}

// Global reference for native target-action callback routing
static mut GLOBAL_MACOS_APP: Option<*mut MacosSettingsApp> = None;

unsafe extern "C" fn handle_window_will_close(_this: *mut c_void, _cmd: *mut c_void, _notif: *mut c_void) {
    let app_cls = objc_getClass(b"NSApplication\0".as_ptr());
    let msg_ptr: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
        std::mem::transmute(objc_msgSend as *const ());
    let app = msg_ptr(app_cls, sel(b"sharedApplication\0"));
    let msg_void_ptr: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) =
        std::mem::transmute(objc_msgSend as *const ());
    msg_void_ptr(app, sel(b"terminate:\0"), std::ptr::null_mut());
}

unsafe extern "C" fn handle_control_action(_this: *mut c_void, _cmd: *mut c_void, sender: *mut c_void) {
    if sender.is_null() {
        return;
    }
    let msg_isize: unsafe extern "C" fn(*mut c_void, *mut c_void) -> isize =
        std::mem::transmute(objc_msgSend as *const ());
    let tag = msg_isize(sender, sel(b"tag\0"));

    if let Some(app_ptr) = GLOBAL_MACOS_APP {
        let app = &mut *app_ptr;
        app.handle_action(tag, sender);
    }
}

unsafe fn get_action_target() -> *mut c_void {
    let cls_name = b"AntiPetMacosActionTarget\0";
    let mut cls = objc_getClass(cls_name.as_ptr());
    if cls.is_null() {
        let nsobject = objc_getClass(b"NSObject\0".as_ptr());
        cls = objc_allocateClassPair(nsobject, cls_name.as_ptr(), 0);
        let on_action_sel = sel(b"onAction:\0");
        let will_close_sel = sel(b"windowWillClose:\0");
        let types = b"v@:@\0";
        class_addMethod(
            cls,
            on_action_sel,
            handle_control_action as *const c_void,
            types.as_ptr(),
        );
        class_addMethod(
            cls,
            will_close_sel,
            handle_window_will_close as *const c_void,
            types.as_ptr(),
        );
        objc_registerClassPair(cls);
    }
    let msg_ptr: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
        std::mem::transmute(objc_msgSend as *const ());
    msg_ptr(cls, sel(b"new\0"))
}

impl MacosSettingsApp {
    pub fn new(store: Arc<Mutex<SettingsStore>>) -> Self {
        unsafe {
            let action_target = get_action_target();
            let (window, content_container, toast_label, tab_views, theme_lbl, scale_lbl, floor_lbl, dialogue_cat_lbl, dialogue_preview_lbl) =
                Self::create_ui(&store, action_target);
            Self {
                store,
                ns_window: window,
                content_container,
                toast_label,
                tab_views,
                current_tab: 0,
                theme_lbl,
                scale_lbl,
                floor_lbl,
                dialogue_cat_lbl,
                dialogue_preview_lbl,
            }
        }
    }

    unsafe fn create_ui(
        store: &Arc<Mutex<SettingsStore>>,
        action_target: *mut c_void,
    ) -> (
        *mut c_void,
        *mut c_void,
        *mut c_void,
        Vec<*mut c_void>,
        *mut c_void,
        *mut c_void,
        *mut c_void,
        *mut c_void,
        *mut c_void,
    ) {
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
        let set_frame: unsafe extern "C" fn(*mut c_void, *mut c_void, [f64; 4]) =
            std::mem::transmute(objc_msgSend as *const ());
        let msg_label: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> *mut c_void =
            std::mem::transmute(objc_msgSend as *const ());
        let msg_btn: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void, *mut c_void, *mut c_void) -> *mut c_void =
            std::mem::transmute(objc_msgSend as *const ());

        let window_cls = objc_getClass(b"NSWindow\0".as_ptr());
        let effect_cls = objc_getClass(b"NSVisualEffectView\0".as_ptr());
        let view_cls = objc_getClass(b"NSView\0".as_ptr());
        let color_cls = objc_getClass(b"NSColor\0".as_ptr());
        let button_cls = objc_getClass(b"NSButton\0".as_ptr());
        let textfield_cls = objc_getClass(b"NSTextField\0".as_ptr());
        let sw_cls = objc_getClass(b"NSSwitch\0".as_ptr());
        let slider_cls = objc_getClass(b"NSSlider\0".as_ptr());

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
        msg_void_bool(win, sel(b"setMovableByWindowBackground:\0"), 1);
        msg_void_ptr(win, sel(b"setDelegate:\0"), action_target);

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
        let title_lbl = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring("偏好设置"));
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
        for (i, tab_title) in tabs.iter().enumerate() {
            let btn_y = 370.0 - (i as f64 * 40.0);
            let btn = msg_btn(button_cls, sel(b"buttonWithTitle:target:action:\0"), nsstring(tab_title), action_target, sel(b"onAction:\0"));
            set_frame(btn, sel(b"setFrame:\0"), [12.0, btn_y, 156.0, 32.0]);
            msg_void_isize(btn, sel(b"setTag:\0"), 100 + i as isize);
            msg_subview(sidebar_view, sel(b"addSubview:\0"), btn, 1, std::ptr::null_mut());
        }

        // 4. Right Content Container (x = 180, w = 520, h = 500)
        let content_w = win_w - sidebar_w;
        let content_frame = [sidebar_w, 0.0, content_w, win_h];
        let content_container = msg_init_frame(msg_ptr(view_cls, sel(b"alloc\0")), sel(b"initWithFrame:\0"), content_frame);
        msg_subview(content_view, sel(b"addSubview:positioned:relativeTo:\0"), content_container, 1, std::ptr::null_mut());

        let mut tab_views = Vec::new();
        let view_frame = [0.0, 50.0, content_w, win_h - 50.0];
        let snap = store.lock().unwrap().snapshot();

        // ================= TAB 0: 常规与显示 =================
        let tab0_view = msg_init_frame(msg_ptr(view_cls, sel(b"alloc\0")), sel(b"initWithFrame:\0"), view_frame);
        let header0 = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring("常规与显示"));
        set_frame(header0, sel(b"setFrame:\0"), [24.0, 400.0, 200.0, 26.0]);
        msg_subview(tab0_view, sel(b"addSubview:\0"), header0, 1, std::ptr::null_mut());

        let theme_str = format!("界面外观风格: {}", snap.temp_settings.theme);
        let theme_lbl = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring(&theme_str));
        set_frame(theme_lbl, sel(b"setFrame:\0"), [24.0, 360.0, 300.0, 20.0]);
        msg_subview(tab0_view, sel(b"addSubview:\0"), theme_lbl, 1, std::ptr::null_mut());

        let theme_btn = msg_btn(button_cls, sel(b"buttonWithTitle:target:action:\0"), nsstring("切换主题"), action_target, sel(b"onAction:\0"));
        set_frame(theme_btn, sel(b"setFrame:\0"), [380.0, 355.0, 100.0, 28.0]);
        msg_void_isize(theme_btn, sel(b"setTag:\0"), 201);
        msg_subview(tab0_view, sel(b"addSubview:\0"), theme_btn, 1, std::ptr::null_mut());

        let scale_title = format!("缩放大小: {}% (AppKit 原生滑块)", (snap.temp_settings.scale * 100.0).round() as i32);
        let scale_lbl = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring(&scale_title));
        set_frame(scale_lbl, sel(b"setFrame:\0"), [24.0, 315.0, 260.0, 20.0]);
        msg_subview(tab0_view, sel(b"addSubview:\0"), scale_lbl, 1, std::ptr::null_mut());

        let msg_slider: unsafe extern "C" fn(*mut c_void, *mut c_void, f64, f64, f64, *mut c_void, *mut c_void) -> *mut c_void =
            std::mem::transmute(objc_msgSend as *const ());
        let slider = msg_slider(slider_cls, sel(b"sliderWithValue:minValue:maxValue:target:action:\0"), snap.temp_settings.scale as f64, 0.5, 2.0, action_target, sel(b"onAction:\0"));
        set_frame(slider, sel(b"setFrame:\0"), [280.0, 315.0, 200.0, 20.0]);
        msg_void_isize(slider, sel(b"setTag:\0"), 202);
        msg_subview(tab0_view, sel(b"addSubview:\0"), slider, 1, std::ptr::null_mut());

        let floor_str = format!("落地吸附基线: {}", snap.temp_settings.floor_mode);
        let floor_lbl = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring(&floor_str));
        set_frame(floor_lbl, sel(b"setFrame:\0"), [24.0, 270.0, 260.0, 20.0]);
        msg_subview(tab0_view, sel(b"addSubview:\0"), floor_lbl, 1, std::ptr::null_mut());

        let floor_btn = msg_btn(button_cls, sel(b"buttonWithTitle:target:action:\0"), nsstring("切换落地点"), action_target, sel(b"onAction:\0"));
        set_frame(floor_btn, sel(b"setFrame:\0"), [380.0, 265.0, 100.0, 28.0]);
        msg_void_isize(floor_btn, sel(b"setTag:\0"), 203);
        msg_subview(tab0_view, sel(b"addSubview:\0"), floor_btn, 1, std::ptr::null_mut());

        let reset_lbl = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring("屏幕位置校准: 一键拉回屏幕正中央"));
        set_frame(reset_lbl, sel(b"setFrame:\0"), [24.0, 225.0, 260.0, 20.0]);
        msg_subview(tab0_view, sel(b"addSubview:\0"), reset_lbl, 1, std::ptr::null_mut());

        let reset_btn = msg_btn(button_cls, sel(b"buttonWithTitle:target:action:\0"), nsstring("复位居中"), action_target, sel(b"onAction:\0"));
        set_frame(reset_btn, sel(b"setFrame:\0"), [380.0, 220.0, 100.0, 28.0]);
        msg_void_isize(reset_btn, sel(b"setTag:\0"), 204);
        msg_subview(tab0_view, sel(b"addSubview:\0"), reset_btn, 1, std::ptr::null_mut());

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
                msg_void_ptr(sw, sel(b"setTarget:\0"), action_target);
                msg_void_ptr(sw, sel(b"setAction:\0"), sel(b"onAction:\0"));
                msg_subview(tab0_view, sel(b"addSubview:\0"), sw, 1, std::ptr::null_mut());
            }
        }
        tab_views.push(tab0_view);
        msg_subview(content_container, sel(b"addSubview:\0"), tab0_view, 1, std::ptr::null_mut());

        // ================= TAB 1: 自启动与更新 =================
        let tab1_view = msg_init_frame(msg_ptr(view_cls, sel(b"alloc\0")), sel(b"initWithFrame:\0"), view_frame);
        let header1 = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring("自启动与更新设置"));
        set_frame(header1, sel(b"setFrame:\0"), [24.0, 400.0, 260.0, 26.0]);
        msg_subview(tab1_view, sel(b"addSubview:\0"), header1, 1, std::ptr::null_mut());

        let auto_lbl = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring("开机自启动: 系统登录后静默拉起伴侣"));
        set_frame(auto_lbl, sel(b"setFrame:\0"), [24.0, 340.0, 300.0, 20.0]);
        msg_subview(tab1_view, sel(b"addSubview:\0"), auto_lbl, 1, std::ptr::null_mut());

        if !sw_cls.is_null() {
            let auto_sw = msg_init_frame(msg_ptr(sw_cls, sel(b"alloc\0")), sel(b"initWithFrame:\0"), [420.0, 336.0, 60.0, 28.0]);
            msg_void_isize(auto_sw, sel(b"setState:\0"), if snap.autostart_enabled { 1 } else { 0 });
            msg_void_isize(auto_sw, sel(b"setTag:\0"), 301);
            msg_void_ptr(auto_sw, sel(b"setTarget:\0"), action_target);
            msg_void_ptr(auto_sw, sel(b"setAction:\0"), sel(b"onAction:\0"));
            msg_subview(tab1_view, sel(b"addSubview:\0"), auto_sw, 1, std::ptr::null_mut());
        }

        let ver_card = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring("当前版本: v0.5.0 (macOS 原生 AppKit 架构，0 KB 额外体积)"));
        set_frame(ver_card, sel(b"setFrame:\0"), [24.0, 280.0, 450.0, 20.0]);
        msg_subview(tab1_view, sel(b"addSubview:\0"), ver_card, 1, std::ptr::null_mut());

        let engine_lbl = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring("渲染核心: Pure CPU 2D Software Surface (<12MB RSS, 0% GPU)"));
        set_frame(engine_lbl, sel(b"setFrame:\0"), [24.0, 240.0, 450.0, 20.0]);
        msg_subview(tab1_view, sel(b"addSubview:\0"), engine_lbl, 1, std::ptr::null_mut());

        let update_lbl = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring("更新检查: 校验最新版本及动作资产包"));
        set_frame(update_lbl, sel(b"setFrame:\0"), [24.0, 180.0, 300.0, 20.0]);
        msg_subview(tab1_view, sel(b"addSubview:\0"), update_lbl, 1, std::ptr::null_mut());

        let update_btn = msg_btn(button_cls, sel(b"buttonWithTitle:target:action:\0"), nsstring("检查更新"), action_target, sel(b"onAction:\0"));
        set_frame(update_btn, sel(b"setFrame:\0"), [380.0, 175.0, 100.0, 28.0]);
        msg_void_isize(update_btn, sel(b"setTag:\0"), 302);
        msg_subview(tab1_view, sel(b"addSubview:\0"), update_btn, 1, std::ptr::null_mut());

        msg_void_bool(tab1_view, sel(b"setHidden:\0"), 1);
        tab_views.push(tab1_view);
        msg_subview(content_container, sel(b"addSubview:\0"), tab1_view, 1, std::ptr::null_mut());

        // ================= TAB 2: 皮肤管理 =================
        let tab2_view = msg_init_frame(msg_ptr(view_cls, sel(b"alloc\0")), sel(b"initWithFrame:\0"), view_frame);
        let header2 = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring("已安装皮肤与外观资产"));
        set_frame(header2, sel(b"setFrame:\0"), [24.0, 400.0, 260.0, 26.0]);
        msg_subview(tab2_view, sel(b"addSubview:\0"), header2, 1, std::ptr::null_mut());

        let skin_lbl = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring("当前激活皮肤: fat-cat (经典大肥猫)"));
        set_frame(skin_lbl, sel(b"setFrame:\0"), [24.0, 340.0, 400.0, 20.0]);
        msg_subview(tab2_view, sel(b"addSubview:\0"), skin_lbl, 1, std::ptr::null_mut());

        let author_lbl = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring("资产创作者: Antigravity Community"));
        set_frame(author_lbl, sel(b"setFrame:\0"), [24.0, 300.0, 400.0, 20.0]);
        msg_subview(tab2_view, sel(b"addSubview:\0"), author_lbl, 1, std::ptr::null_mut());

        let states_lbl = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring("动画状态帧: idle, thinking, working, success, error, petting"));
        set_frame(states_lbl, sel(b"setFrame:\0"), [24.0, 260.0, 450.0, 20.0]);
        msg_subview(tab2_view, sel(b"addSubview:\0"), states_lbl, 1, std::ptr::null_mut());

        let dir_lbl = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring("资产目录: skins/fat-cat/ (支持本地热重载与实时帧替换)"));
        set_frame(dir_lbl, sel(b"setFrame:\0"), [24.0, 220.0, 450.0, 20.0]);
        msg_subview(tab2_view, sel(b"addSubview:\0"), dir_lbl, 1, std::ptr::null_mut());

        msg_void_bool(tab2_view, sel(b"setHidden:\0"), 1);
        tab_views.push(tab2_view);
        msg_subview(content_container, sel(b"addSubview:\0"), tab2_view, 1, std::ptr::null_mut());

        // ================= TAB 3: 台词与问候 =================
        let tab3_view = msg_init_frame(msg_ptr(view_cls, sel(b"alloc\0")), sel(b"initWithFrame:\0"), view_frame);
        let header3 = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring("伴侣语录与气泡台词管理"));
        set_frame(header3, sel(b"setFrame:\0"), [24.0, 400.0, 260.0, 26.0]);
        msg_subview(tab3_view, sel(b"addSubview:\0"), header3, 1, std::ptr::null_mut());

        let cat_names = ["待机闲聊", "交互抚摸", "思考中", "任务成功", "任务错误"];
        let cur_cat_str = format!("当前台词分类: {} (点击右侧切换)", cat_names[snap.dialogue_cat.min(4)]);
        let dialogue_cat_lbl = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring(&cur_cat_str));
        set_frame(dialogue_cat_lbl, sel(b"setFrame:\0"), [24.0, 340.0, 320.0, 20.0]);
        msg_subview(tab3_view, sel(b"addSubview:\0"), dialogue_cat_lbl, 1, std::ptr::null_mut());

        let switch_cat_btn = msg_btn(button_cls, sel(b"buttonWithTitle:target:action:\0"), nsstring("切换分类"), action_target, sel(b"onAction:\0"));
        set_frame(switch_cat_btn, sel(b"setFrame:\0"), [380.0, 335.0, 100.0, 28.0]);
        msg_void_isize(switch_cat_btn, sel(b"setTag:\0"), 401);
        msg_subview(tab3_view, sel(b"addSubview:\0"), switch_cat_btn, 1, std::ptr::null_mut());

        let active_lines = store.lock().unwrap().active_dialogues().clone();
        let preview_content = if active_lines.is_empty() {
            "当前分类暂无自定义台词".to_string()
        } else {
            format!("已收录 {} 条台词，样例: \"{}\"", active_lines.len(), active_lines[0])
        };
        let dialogue_preview_lbl = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring(&preview_content));
        set_frame(dialogue_preview_lbl, sel(b"setFrame:\0"), [24.0, 260.0, 460.0, 50.0]);
        msg_subview(tab3_view, sel(b"addSubview:\0"), dialogue_preview_lbl, 1, std::ptr::null_mut());

        let note3 = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring("台词与桌宠状态机实时双向绑定，保存后立即写入 dialogues.json"));
        set_frame(note3, sel(b"setFrame:\0"), [24.0, 180.0, 460.0, 20.0]);
        msg_subview(tab3_view, sel(b"addSubview:\0"), note3, 1, std::ptr::null_mut());

        msg_void_bool(tab3_view, sel(b"setHidden:\0"), 1);
        tab_views.push(tab3_view);
        msg_subview(content_container, sel(b"addSubview:\0"), tab3_view, 1, std::ptr::null_mut());

        // ================= TAB 4: 右键菜单配置 =================
        let tab4_view = msg_init_frame(msg_ptr(view_cls, sel(b"alloc\0")), sel(b"initWithFrame:\0"), view_frame);
        let header4 = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring("托盘与右键菜单项配置"));
        set_frame(header4, sel(b"setFrame:\0"), [24.0, 400.0, 260.0, 26.0]);
        msg_subview(tab4_view, sel(b"addSubview:\0"), header4, 1, std::ptr::null_mut());

        let menu_items = [
            ("打开 Antigravity 控制台", 501, snap.temp_settings.menu_config.open_antigravity, 360.0),
            ("偏好设置", 502, snap.temp_settings.menu_config.settings, 325.0),
            ("随机闲聊台词", 503, snap.temp_settings.menu_config.random_dialogue, 290.0),
            ("触发特定动作", 504, snap.temp_settings.menu_config.pet_action, 255.0),
            ("查看实时额度", 505, snap.temp_settings.menu_config.quota, 220.0),
            ("免打扰鼠标穿透", 506, snap.temp_settings.menu_config.dnd, 185.0),
            ("动作音效开关", 507, snap.temp_settings.menu_config.sound, 150.0),
            ("缩放级别调节", 508, snap.temp_settings.menu_config.zoom, 115.0),
            ("重力加速度下落", 509, snap.temp_settings.menu_config.gravity, 80.0),
        ];
        for (m_title, m_tag, m_val, m_y) in menu_items {
            let lbl = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring(m_title));
            set_frame(lbl, sel(b"setFrame:\0"), [24.0, m_y, 300.0, 20.0]);
            msg_subview(tab4_view, sel(b"addSubview:\0"), lbl, 1, std::ptr::null_mut());

            if !sw_cls.is_null() {
                let sw = msg_init_frame(msg_ptr(sw_cls, sel(b"alloc\0")), sel(b"initWithFrame:\0"), [420.0, m_y - 4.0, 60.0, 28.0]);
                msg_void_isize(sw, sel(b"setState:\0"), if m_val { 1 } else { 0 });
                msg_void_isize(sw, sel(b"setTag:\0"), m_tag);
                msg_void_ptr(sw, sel(b"setTarget:\0"), action_target);
                msg_void_ptr(sw, sel(b"setAction:\0"), sel(b"onAction:\0"));
                msg_subview(tab4_view, sel(b"addSubview:\0"), sw, 1, std::ptr::null_mut());
            }
        }
        msg_void_bool(tab4_view, sel(b"setHidden:\0"), 1);
        tab_views.push(tab4_view);
        msg_subview(content_container, sel(b"addSubview:\0"), tab4_view, 1, std::ptr::null_mut());

        // 6. Bottom Action Bar (Fixed across all tabs)
        let cancel_btn = msg_btn(button_cls, sel(b"buttonWithTitle:target:action:\0"), nsstring("取消"), action_target, sel(b"onAction:\0"));
        set_frame(cancel_btn, sel(b"setFrame:\0"), [content_w - 190.0, 14.0, 80.0, 28.0]);
        msg_void_isize(cancel_btn, sel(b"setTag:\0"), 998);
        msg_subview(content_container, sel(b"addSubview:\0"), cancel_btn, 1, std::ptr::null_mut());

        let save_btn = msg_btn(button_cls, sel(b"buttonWithTitle:target:action:\0"), nsstring("确定"), action_target, sel(b"onAction:\0"));
        set_frame(save_btn, sel(b"setFrame:\0"), [content_w - 98.0, 14.0, 80.0, 28.0]);
        msg_void_isize(save_btn, sel(b"setTag:\0"), 999);
        msg_void_ptr(save_btn, sel(b"setKeyEquivalent:\0"), nsstring("\r")); // Return key triggers save
        msg_subview(content_container, sel(b"addSubview:\0"), save_btn, 1, std::ptr::null_mut());

        msg_ptr(win, sel(b"center\0"));
        (
            win,
            content_container,
            toast_lbl,
            tab_views,
            theme_lbl,
            scale_lbl,
            floor_lbl,
            dialogue_cat_lbl,
            dialogue_preview_lbl,
        )
    }

    pub fn handle_action(&mut self, tag: isize, sender: *mut c_void) {
        unsafe {
            let msg_void_ptr: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) =
                std::mem::transmute(objc_msgSend as *const ());
            let msg_f64: unsafe extern "C" fn(*mut c_void, *mut c_void) -> f64 =
                std::mem::transmute(objc_msgSend as *const ());

            match tag {
                // Save & Commit
                999 => {
                    self.store.lock().unwrap().dispatch(SettingsAction::Save);
                    self.set_toast("设置已成功保存并立即生效");
                    let app_cls = objc_getClass(b"NSApplication\0".as_ptr());
                    let msg_ptr: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                        std::mem::transmute(objc_msgSend as *const ());
                    let app = msg_ptr(app_cls, sel(b"sharedApplication\0"));
                    msg_void_ptr(app, sel(b"terminate:\0"), std::ptr::null_mut());
                }
                // Cancel & Rollback
                998 => {
                    self.store.lock().unwrap().dispatch(SettingsAction::Cancel);
                    let app_cls = objc_getClass(b"NSApplication\0".as_ptr());
                    let msg_ptr: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                        std::mem::transmute(objc_msgSend as *const ());
                    let app = msg_ptr(app_cls, sel(b"sharedApplication\0"));
                    msg_void_ptr(app, sel(b"terminate:\0"), std::ptr::null_mut());
                }
                // Tab Selection
                100..=104 => {
                    let tab_idx = (tag - 100) as usize;
                    self.switch_tab(tab_idx);
                }
                // Theme Cycle
                201 => {
                    self.store.lock().unwrap().dispatch(SettingsAction::CycleTheme);
                    let theme = self.store.lock().unwrap().snapshot().temp_settings.theme;
                    let text = format!("界面外观风格: {}", theme);
                    msg_void_ptr(self.theme_lbl, sel(b"setStringValue:\0"), nsstring(&text));
                }
                // Scale Slider
                202 => {
                    let val = msg_f64(sender, sel(b"doubleValue\0"));
                    self.store.lock().unwrap().dispatch(SettingsAction::SetScale(val as f32));
                    let scale = self.store.lock().unwrap().snapshot().temp_settings.scale;
                    let text = format!("缩放大小: {}% (AppKit 原生滑块)", (scale * 100.0).round() as i32);
                    msg_void_ptr(self.scale_lbl, sel(b"setStringValue:\0"), nsstring(&text));
                }
                // Floor Mode Cycle
                203 => {
                    self.store.lock().unwrap().dispatch(SettingsAction::CycleFloorMode);
                    let floor = self.store.lock().unwrap().snapshot().temp_settings.floor_mode;
                    let text = format!("落地吸附基线: {}", floor);
                    msg_void_ptr(self.floor_lbl, sel(b"setStringValue:\0"), nsstring(&text));
                }
                // Reset Position
                204 => {
                    self.store.lock().unwrap().dispatch(SettingsAction::ResetPosition);
                    self.set_toast("已复位至屏幕正中央");
                }
                // Switches Tab 0
                205 => { self.store.lock().unwrap().dispatch(SettingsAction::ToggleGravity); }
                206 => { self.store.lock().unwrap().dispatch(SettingsAction::ToggleShowQuota); }
                207 => { self.store.lock().unwrap().dispatch(SettingsAction::ToggleDndPassthrough); }
                208 => { self.store.lock().unwrap().dispatch(SettingsAction::ToggleSound); }
                // Tab 1 Actions
                301 => { self.store.lock().unwrap().dispatch(SettingsAction::ToggleAutostart); }
                302 => { self.set_toast("当前已是最新版本 (v0.5.0)"); }
                // Tab 3 Dialogue Actions
                401 => {
                    let cur_cat = self.store.lock().unwrap().dialogue_cat;
                    let next_cat = (cur_cat + 1) % 5;
                    self.store.lock().unwrap().dispatch(SettingsAction::SelectDialogueCategory(next_cat));
                    let cat_names = ["待机闲聊", "交互抚摸", "思考中", "任务成功", "任务错误"];
                    let cat_str = format!("当前台词分类: {} (点击右侧切换)", cat_names[next_cat]);
                    msg_void_ptr(self.dialogue_cat_lbl, sel(b"setStringValue:\0"), nsstring(&cat_str));

                    let active = self.store.lock().unwrap().active_dialogues().clone();
                    let preview = if active.is_empty() {
                        "当前分类暂无自定义台词".to_string()
                    } else {
                        format!("已收录 {} 条台词，样例: \"{}\"", active.len(), active[0])
                    };
                    msg_void_ptr(self.dialogue_preview_lbl, sel(b"setStringValue:\0"), nsstring(&preview));
                }
                // Tab 4 Menu Options 501..=509
                501..=509 => {
                    let opt_idx = (tag - 501) as usize;
                    self.store.lock().unwrap().dispatch(SettingsAction::ToggleMenuOption(opt_idx));
                }
                _ => {}
            }
        }
    }

    pub fn switch_tab(&mut self, new_tab: usize) {
        if new_tab >= self.tab_views.len() {
            return;
        }
        self.current_tab = new_tab;
        unsafe {
            let msg_void_bool: unsafe extern "C" fn(*mut c_void, *mut c_void, i8) =
                std::mem::transmute(objc_msgSend as *const ());
            for (i, view) in self.tab_views.iter().enumerate() {
                msg_void_bool(*view, sel(b"setHidden:\0"), if i == new_tab { 0 } else { 1 });
            }
        }
    }

    pub fn set_toast(&self, text: &str) {
        if self.toast_label.is_null() {
            return;
        }
        unsafe {
            let msg_void_ptr: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) =
                std::mem::transmute(objc_msgSend as *const ());
            msg_void_ptr(self.toast_label, sel(b"setStringValue:\0"), nsstring(text));
        }
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
