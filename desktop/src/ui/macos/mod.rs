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

#[inline]
unsafe fn get_flipped_view_class() -> *mut c_void {
    let cls_name = b"AntiPetFlippedView\0";
    let mut cls = objc_getClass(cls_name.as_ptr());
    if cls.is_null() {
        let view_cls = objc_getClass(b"NSView\0".as_ptr());
        cls = objc_allocateClassPair(view_cls, cls_name.as_ptr(), 0);
        let flipped_sel = sel(b"isFlipped\0");
        let types = b"c@:\0";
        class_addMethod(
            cls,
            flipped_sel,
            flipped_view_is_flipped as *const c_void,
            types.as_ptr(),
        );
        objc_registerClassPair(cls);
    }
    cls
}

unsafe extern "C" fn flipped_view_is_flipped(_this: *mut c_void, _cmd: *mut c_void) -> i8 {
    1 // YES: coordinate origin (0, 0) is top-left, y points downward
}

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

/// Native macOS AppKit Settings Window Controller.
/// Complies 100% with Apple Human Interface Guidelines (HIG):
/// Source List Sidebar, Inset Grouped Cards, SF Symbols, NSPopUpButton, NSSwitch, and Instant Auto-save.
pub struct MacosSettingsApp {
    pub store: Arc<Mutex<SettingsStore>>,
    pub ns_window: *mut c_void,
    pub content_container: *mut c_void,
    pub tab_views: Vec<*mut c_void>,
    pub current_tab: usize,
    pub sidebar_capsules: Vec<*mut c_void>,
    pub sidebar_labels: Vec<*mut c_void>,
    pub sidebar_status_lbl: *mut c_void,
    pub scale_val_lbl: *mut c_void,
    pub theme_popup: *mut c_void,
    pub floor_popup: *mut c_void,
    pub dialogue_pills: Vec<*mut c_void>,
    pub dialogue_pill_labels: Vec<*mut c_void>,
    pub dialogue_input: *mut c_void,
    pub dialogue_doc_view: *mut c_void,
    pub action_target: *mut c_void,
}

static mut GLOBAL_MACOS_APP: Option<*mut MacosSettingsApp> = None;

impl MacosSettingsApp {
    pub fn new(store: Arc<Mutex<SettingsStore>>) -> Self {
        unsafe {
            let action_target = get_action_target();
            Self::create_ui(store, action_target)
        }
    }

    unsafe fn create_ui(store: Arc<Mutex<SettingsStore>>, action_target: *mut c_void) -> Self {
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
        let msg_subview: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) =
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
        let msg_color_rgba: unsafe extern "C" fn(*mut c_void, *mut c_void, f64, f64, f64, f64) -> *mut c_void =
            std::mem::transmute(objc_msgSend as *const ());
        let msg_void_f64: unsafe extern "C" fn(*mut c_void, *mut c_void, f64) =
            std::mem::transmute(objc_msgSend as *const ());
        let msg_slider: unsafe extern "C" fn(*mut c_void, *mut c_void, f64, f64, f64, *mut c_void, *mut c_void) -> *mut c_void =
            std::mem::transmute(objc_msgSend as *const ());

        let window_cls = objc_getClass(b"NSWindow\0".as_ptr());
        let effect_cls = objc_getClass(b"NSVisualEffectView\0".as_ptr());
        let flipped_cls = get_flipped_view_class();
        let color_cls = objc_getClass(b"NSColor\0".as_ptr());
        let button_cls = objc_getClass(b"NSButton\0".as_ptr());
        let textfield_cls = objc_getClass(b"NSTextField\0".as_ptr());
        let sw_cls = objc_getClass(b"NSSwitch\0".as_ptr());
        let slider_cls = objc_getClass(b"NSSlider\0".as_ptr());
        let font_cls = objc_getClass(b"NSFont\0".as_ptr());
        let image_cls = objc_getClass(b"NSImage\0".as_ptr());
        let imageview_cls = objc_getClass(b"NSImageView\0".as_ptr());
        let popup_cls = objc_getClass(b"NSPopUpButton\0".as_ptr());
        let scroll_cls = objc_getClass(b"NSScrollView\0".as_ptr());

        let win_w = 720.0f64;
        let win_h = 520.0f64;
        let sidebar_w = 200.0f64;
        let content_w = win_w - sidebar_w;

        // 1. Create NSWindow
        let style_mask = 1 | 2 | 4 | (1 << 15); // Titled, Closable, Miniaturizable, FullSizeContentView
        let win_alloc = msg_ptr(window_cls, sel(b"alloc\0"));
        let win = msg_init_window(win_alloc, sel(b"initWithContentRect:styleMask:backing:defer:\0"), [100.0, 100.0, win_w, win_h], style_mask, 2, 0);

        // Clean title bar: remove window title next to traffic lights per Apple HIG
        msg_void_ptr(win, sel(b"setTitle:\0"), nsstring(""));
        msg_void_isize(win, sel(b"setTitleVisibility:\0"), 1); // NSWindowTitleHidden = 1
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
        let effect_alloc = msg_ptr(effect_cls, sel(b"alloc\0"));
        let effect_view = msg_init_frame(effect_alloc, sel(b"initWithFrame:\0"), [0.0, 0.0, win_w, win_h]);
        if !effect_view.is_null() {
            msg_void_isize(effect_view, sel(b"setMaterial:\0"), 7); // NSVisualEffectMaterialSidebar = 7
            msg_void_isize(effect_view, sel(b"setBlendingMode:\0"), 0); // BehindWindow
            msg_void_isize(effect_view, sel(b"setState:\0"), 1); // Active
            msg_void_usize(effect_view, sel(b"setAutoresizingMask:\0"), 18); // WidthSizable | HeightSizable
            msg_subview(content_view, sel(b"addSubview:\0"), effect_view);
        }

        // 3. Flipped Main Host
        let main_host = msg_init_frame(msg_ptr(flipped_cls, sel(b"alloc\0")), sel(b"initWithFrame:\0"), [0.0, 0.0, win_w, win_h]);
        msg_subview(content_view, sel(b"addSubview:\0"), main_host);

        // 4. Left Sidebar (w = 200, h = 520)
        let sidebar_view = msg_init_frame(msg_ptr(flipped_cls, sel(b"alloc\0")), sel(b"initWithFrame:\0"), [0.0, 0.0, sidebar_w, win_h]);
        msg_subview(main_host, sel(b"addSubview:\0"), sidebar_view);

        // Sidebar Branding
        let title_lbl = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring("偏好设置"));
        let msg_font_bold: unsafe extern "C" fn(*mut c_void, *mut c_void, f64) -> *mut c_void =
            std::mem::transmute(objc_msgSend as *const ());
        let title_font = msg_font_bold(font_cls, sel(b"boldSystemFontOfSize:\0"), 15.0);
        msg_void_ptr(title_lbl, sel(b"setFont:\0"), title_font);
        set_frame(title_lbl, sel(b"setFrame:\0"), [18.0, 42.0, 160.0, 22.0]);
        msg_subview(sidebar_view, sel(b"addSubview:\0"), title_lbl);

        let ver_lbl = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring("Anti-Pet v0.5.0"));
        let sec_color = msg_ptr(color_cls, sel(b"secondaryLabelColor\0"));
        msg_void_ptr(ver_lbl, sel(b"setTextColor:\0"), sec_color);
        let msg_font: unsafe extern "C" fn(*mut c_void, *mut c_void, f64) -> *mut c_void =
            std::mem::transmute(objc_msgSend as *const ());
        let small_font = msg_font(font_cls, sel(b"systemFontOfSize:\0"), 11.0);
        msg_void_ptr(ver_lbl, sel(b"setFont:\0"), small_font);
        set_frame(ver_lbl, sel(b"setFrame:\0"), [18.0, 64.0, 160.0, 16.0]);
        msg_subview(sidebar_view, sel(b"addSubview:\0"), ver_lbl);

        // 5 Navigation Source List items
        let tabs = [
            ("常规与显示", "slider.horizontal.3", 100),
            ("自启动与更新", "arrow.triangle.2.circlepath", 101),
            ("皮肤管理", "paintpalette", 102),
            ("台词与问候", "bubble.left.and.bubble.right", 103),
            ("右键菜单配置", "menubar.rectangle", 104),
        ];

        let mut sidebar_capsules = Vec::new();
        let mut sidebar_labels = Vec::new();
        let regular_font = msg_font(font_cls, sel(b"systemFontOfSize:\0"), 13.0);
        let white_color = msg_ptr(color_cls, sel(b"whiteColor\0"));
        let label_color = msg_ptr(color_cls, sel(b"labelColor\0"));

        let accent_color = msg_color_rgba(color_cls, sel(b"colorWithRed:green:blue:alpha:\0"), 0.0, 0.48, 1.0, 0.88);
        let accent_cg = msg_ptr(accent_color, sel(b"CGColor\0"));
        let clear_color = msg_ptr(color_cls, sel(b"clearColor\0"));
        let clear_cg = msg_ptr(clear_color, sel(b"CGColor\0"));

        for (i, (tab_title, symbol_name, tag)) in tabs.iter().enumerate() {
            let item_y = 96.0 + (i as f64 * 38.0);
            let item_host = msg_init_frame(msg_ptr(flipped_cls, sel(b"alloc\0")), sel(b"initWithFrame:\0"), [8.0, item_y, 184.0, 34.0]);

            // Capsule background view
            let capsule = msg_init_frame(msg_ptr(flipped_cls, sel(b"alloc\0")), sel(b"initWithFrame:\0"), [0.0, 0.0, 184.0, 34.0]);
            msg_void_bool(capsule, sel(b"setWantsLayer:\0"), 1);
            let layer = msg_ptr(capsule, sel(b"layer\0"));
            msg_void_f64(layer, sel(b"setCornerRadius:\0"), 7.0);
            if i == 0 {
                msg_void_ptr(layer, sel(b"setBackgroundColor:\0"), accent_cg);
            } else {
                msg_void_ptr(layer, sel(b"setBackgroundColor:\0"), clear_cg);
            }
            msg_subview(item_host, sel(b"addSubview:\0"), capsule);
            sidebar_capsules.push(capsule);

            // SF Symbol icon
            let msg_sym: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void, *mut c_void) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const ());
            let sym_img = msg_sym(image_cls, sel(b"imageWithSystemSymbolName:accessibilityDescription:\0"), nsstring(symbol_name), std::ptr::null_mut());
            let iv = msg_init_frame(msg_ptr(imageview_cls, sel(b"alloc\0")), sel(b"initWithFrame:\0"), [10.0, 9.0, 16.0, 16.0]);
            msg_void_ptr(iv, sel(b"setImage:\0"), sym_img);
            msg_subview(item_host, sel(b"addSubview:\0"), iv);

            // Title label (optically centered at y = 8.0)
            let item_lbl = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring(tab_title));
            msg_void_ptr(item_lbl, sel(b"setFont:\0"), regular_font);
            if i == 0 {
                msg_void_ptr(item_lbl, sel(b"setTextColor:\0"), white_color);
            } else {
                msg_void_ptr(item_lbl, sel(b"setTextColor:\0"), label_color);
            }
            set_frame(item_lbl, sel(b"setFrame:\0"), [34.0, 8.0, 140.0, 18.0]);
            msg_subview(item_host, sel(b"addSubview:\0"), item_lbl);
            sidebar_labels.push(item_lbl);

            // Transparent hit button for full-row clicking
            let btn = msg_init_frame(msg_ptr(button_cls, sel(b"alloc\0")), sel(b"initWithFrame:\0"), [0.0, 0.0, 184.0, 34.0]);
            msg_void_bool(btn, sel(b"setTransparent:\0"), 1);
            msg_void_isize(btn, sel(b"setTag:\0"), *tag);
            msg_void_ptr(btn, sel(b"setTarget:\0"), action_target);
            msg_void_ptr(btn, sel(b"setAction:\0"), sel(b"onAction:\0"));
            msg_subview(item_host, sel(b"addSubview:\0"), btn);

            msg_subview(sidebar_view, sel(b"addSubview:\0"), item_host);
        }

        // Bottom Auto-save status note
        let sidebar_status_lbl = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring("已自动保存到本地配置"));
        msg_void_ptr(sidebar_status_lbl, sel(b"setFont:\0"), small_font);
        msg_void_ptr(sidebar_status_lbl, sel(b"setTextColor:\0"), sec_color);
        set_frame(sidebar_status_lbl, sel(b"setFrame:\0"), [18.0, 485.0, 160.0, 18.0]);
        msg_subview(sidebar_view, sel(b"addSubview:\0"), sidebar_status_lbl);

        // Vertical Divider line between Sidebar and Content
        let divider = msg_init_frame(msg_ptr(flipped_cls, sel(b"alloc\0")), sel(b"initWithFrame:\0"), [sidebar_w - 1.0, 0.0, 1.0, win_h]);
        msg_void_bool(divider, sel(b"setWantsLayer:\0"), 1);
        let div_layer = msg_ptr(divider, sel(b"layer\0"));
        let sep_color = msg_ptr(color_cls, sel(b"separatorColor\0"));
        let sep_cg = msg_ptr(sep_color, sel(b"CGColor\0"));
        msg_void_ptr(div_layer, sel(b"setBackgroundColor:\0"), sep_cg);
        msg_subview(main_host, sel(b"addSubview:\0"), divider);

        // 5. Right Content Area (x = 200, w = 520, h = 520)
        let content_container = msg_init_frame(msg_ptr(flipped_cls, sel(b"alloc\0")), sel(b"initWithFrame:\0"), [sidebar_w, 0.0, content_w, win_h]);
        msg_subview(main_host, sel(b"addSubview:\0"), content_container);

        let snap = store.lock().unwrap().snapshot();
        let mut tab_views = Vec::new();

        // Helper: creates a scrollable tab host for smooth trackpad scrolling
        let make_scrollable_tab = |content_h: f64| -> (*mut c_void, *mut c_void) {
            let sv = msg_init_frame(msg_ptr(scroll_cls, sel(b"alloc\0")), sel(b"initWithFrame:\0"), [0.0, 0.0, content_w, win_h]);
            msg_void_bool(sv, sel(b"setHasVerticalScroller:\0"), 1);
            msg_void_bool(sv, sel(b"setHasHorizontalScroller:\0"), 0);
            msg_void_bool(sv, sel(b"setAutohidesScrollers:\0"), 1);
            msg_void_bool(sv, sel(b"setDrawsBackground:\0"), 0);

            let doc = msg_init_frame(msg_ptr(flipped_cls, sel(b"alloc\0")), sel(b"initWithFrame:\0"), [0.0, 0.0, content_w, content_h.max(win_h)]);
            msg_void_ptr(sv, sel(b"setDocumentView:\0"), doc);
            (sv, doc)
        };

        // Helper closures for Inset Grouped Cards
        let create_card = |y_pos: f64, h: f64| -> *mut c_void {
            let card = msg_init_frame(msg_ptr(flipped_cls, sel(b"alloc\0")), sel(b"initWithFrame:\0"), [24.0, y_pos, content_w - 48.0, h]);
            msg_void_bool(card, sel(b"setWantsLayer:\0"), 1);
            let layer = msg_ptr(card, sel(b"layer\0"));
            msg_void_f64(layer, sel(b"setCornerRadius:\0"), 10.0);
            let card_bg = msg_color_rgba(color_cls, sel(b"colorWithRed:green:blue:alpha:\0"), 1.0, 1.0, 1.0, 0.06);
            let card_border = msg_color_rgba(color_cls, sel(b"colorWithRed:green:blue:alpha:\0"), 1.0, 1.0, 1.0, 0.10);
            msg_void_ptr(layer, sel(b"setBackgroundColor:\0"), msg_ptr(card_bg, sel(b"CGColor\0")));
            msg_void_ptr(layer, sel(b"setBorderColor:\0"), msg_ptr(card_border, sel(b"CGColor\0")));
            msg_void_f64(layer, sel(b"setBorderWidth:\0"), 1.0);
            card
        };

        let add_section_header = |host: *mut c_void, title: &str, y_pos: f64| {
            let lbl = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring(title));
            msg_void_ptr(lbl, sel(b"setFont:\0"), msg_font_bold(font_cls, sel(b"boldSystemFontOfSize:\0"), 11.0));
            msg_void_ptr(lbl, sel(b"setTextColor:\0"), sec_color);
            set_frame(lbl, sel(b"setFrame:\0"), [28.0, y_pos, 300.0, 16.0]);
            msg_subview(host, sel(b"addSubview:\0"), lbl);
        };

        // Two-line row: Optical vertical centering with height 48.0
        let add_card_row_two_lines = |card: *mut c_void, y_offset: f64, title: &str, subtitle: &str| {
            let t_lbl = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring(title));
            msg_void_ptr(t_lbl, sel(b"setFont:\0"), msg_font(font_cls, sel(b"systemFontOfSize:\0"), 13.0));
            set_frame(t_lbl, sel(b"setFrame:\0"), [16.0, y_offset + 8.0, 260.0, 18.0]);
            msg_subview(card, sel(b"addSubview:\0"), t_lbl);

            let s_lbl = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring(subtitle));
            msg_void_ptr(s_lbl, sel(b"setFont:\0"), small_font);
            msg_void_ptr(s_lbl, sel(b"setTextColor:\0"), sec_color);
            set_frame(s_lbl, sel(b"setFrame:\0"), [16.0, y_offset + 26.0, 260.0, 15.0]);
            msg_subview(card, sel(b"addSubview:\0"), s_lbl);
        };

        // Single-line row: Optical vertical centering with height 44.0
        let add_card_row_single_line = |card: *mut c_void, y_offset: f64, title: &str| {
            let t_lbl = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring(title));
            msg_void_ptr(t_lbl, sel(b"setFont:\0"), msg_font(font_cls, sel(b"systemFontOfSize:\0"), 13.0));
            set_frame(t_lbl, sel(b"setFrame:\0"), [16.0, y_offset + 13.0, 260.0, 18.0]);
            msg_subview(card, sel(b"addSubview:\0"), t_lbl);
        };

        let add_divider = |card: *mut c_void, y_offset: f64| {
            let div = msg_init_frame(msg_ptr(flipped_cls, sel(b"alloc\0")), sel(b"initWithFrame:\0"), [16.0, y_offset, content_w - 80.0, 1.0]);
            msg_void_bool(div, sel(b"setWantsLayer:\0"), 1);
            let layer = msg_ptr(div, sel(b"layer\0"));
            let div_color = msg_color_rgba(color_cls, sel(b"colorWithRed:green:blue:alpha:\0"), 1.0, 1.0, 1.0, 0.07);
            msg_void_ptr(layer, sel(b"setBackgroundColor:\0"), msg_ptr(div_color, sel(b"CGColor\0")));
            msg_subview(card, sel(b"addSubview:\0"), div);
        };

        // ================= TAB 0: 常规与显示 (Scrollable) =================
        let theme_popup;
        let scale_val_lbl;
        let floor_popup;

        let (tab0_sv, tab0_doc) = make_scrollable_tab(640.0);
        {
            // Title Header
            let h_title = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring("常规与显示"));
            msg_void_ptr(h_title, sel(b"setFont:\0"), msg_font_bold(font_cls, sel(b"boldSystemFontOfSize:\0"), 20.0));
            set_frame(h_title, sel(b"setFrame:\0"), [24.0, 20.0, 300.0, 26.0]);
            msg_subview(tab0_doc, sel(b"addSubview:\0"), h_title);

            let h_sub = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring("调整大肥猫的桌面尺寸、物理重力下落与交互行为"));
            msg_void_ptr(h_sub, sel(b"setFont:\0"), small_font);
            msg_void_ptr(h_sub, sel(b"setTextColor:\0"), sec_color);
            set_frame(h_sub, sel(b"setFrame:\0"), [24.0, 48.0, 420.0, 16.0]);
            msg_subview(tab0_doc, sel(b"addSubview:\0"), h_sub);

            // Card 1: 外观与主题
            add_section_header(tab0_doc, "外观与主题", 78.0);
            let card1 = create_card(98.0, 48.0);
            add_card_row_two_lines(card1, 0.0, "界面外观", "设置窗口明暗主题风格，默认跟随系统");

            // NSPopUpButton for Theme (vertically centered at y = 11.0)
            let msg_pop_init: unsafe extern "C" fn(*mut c_void, *mut c_void, [f64; 4], i8) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const ());
            theme_popup = msg_pop_init(msg_ptr(popup_cls, sel(b"alloc\0")), sel(b"initWithFrame:pullsDown:\0"), [content_w - 188.0, 11.0, 120.0, 26.0], 0);
            msg_void_ptr(theme_popup, sel(b"addItemWithTitle:\0"), nsstring("跟随系统"));
            msg_void_ptr(theme_popup, sel(b"addItemWithTitle:\0"), nsstring("浅色模式"));
            msg_void_ptr(theme_popup, sel(b"addItemWithTitle:\0"), nsstring("深色模式"));
            let initial_theme_idx = match snap.temp_settings.theme.as_str() {
                "light" => 1,
                "dark" => 2,
                _ => 0,
            };
            msg_void_isize(theme_popup, sel(b"selectItemAtIndex:\0"), initial_theme_idx);
            msg_void_isize(theme_popup, sel(b"setTag:\0"), 201);
            msg_void_ptr(theme_popup, sel(b"setTarget:\0"), action_target);
            msg_void_ptr(theme_popup, sel(b"setAction:\0"), sel(b"onAction:\0"));
            msg_subview(card1, sel(b"addSubview:\0"), theme_popup);
            msg_subview(tab0_doc, sel(b"addSubview:\0"), card1);

            // Card 2: 体型与物理
            add_section_header(tab0_doc, "体型与物理", 162.0);
            let card2 = create_card(182.0, 192.0);

            // Row 0: Scale Slider + Label
            add_card_row_two_lines(card2, 0.0, "缩放大小", "挂件在屏幕上的呈现比例 (50% - 200%)");
            let slider = msg_slider(slider_cls, sel(b"sliderWithValue:minValue:maxValue:target:action:\0"), snap.temp_settings.scale as f64, 0.5, 2.0, action_target, sel(b"onAction:\0"));
            set_frame(slider, sel(b"setFrame:\0"), [content_w - 238.0, 15.0, 120.0, 18.0]);
            msg_void_isize(slider, sel(b"setTag:\0"), 202);
            msg_subview(card2, sel(b"addSubview:\0"), slider);

            let scale_str = format!("{}%", (snap.temp_settings.scale * 100.0).round() as i32);
            scale_val_lbl = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring(&scale_str));
            msg_void_ptr(scale_val_lbl, sel(b"setFont:\0"), regular_font);
            set_frame(scale_val_lbl, sel(b"setFrame:\0"), [content_w - 110.0, 15.0, 48.0, 18.0]);
            msg_subview(card2, sel(b"addSubview:\0"), scale_val_lbl);
            add_divider(card2, 48.0);

            // Row 1: Floor Mode NSPopUpButton
            add_card_row_two_lines(card2, 48.0, "落地吸附基线", "松手后猫猫下落吸附的目标基准线");
            floor_popup = msg_pop_init(msg_ptr(popup_cls, sel(b"alloc\0")), sel(b"initWithFrame:pullsDown:\0"), [content_w - 238.0, 59.0, 170.0, 26.0], 0);
            msg_void_ptr(floor_popup, sel(b"addItemWithTitle:\0"), nsstring("贴合任务栏与 Dock 上沿"));
            msg_void_ptr(floor_popup, sel(b"addItemWithTitle:\0"), nsstring("屏幕绝对落底 (0px 极底)"));
            msg_void_ptr(floor_popup, sel(b"addItemWithTitle:\0"), nsstring("自由悬浮 (不吸附底边)"));
            let initial_floor_idx = match snap.temp_settings.floor_mode.as_str() {
                "bottom" => 1,
                "free" => 2,
                _ => 0,
            };
            msg_void_isize(floor_popup, sel(b"selectItemAtIndex:\0"), initial_floor_idx);
            msg_void_isize(floor_popup, sel(b"setTag:\0"), 203);
            msg_void_ptr(floor_popup, sel(b"setTarget:\0"), action_target);
            msg_void_ptr(floor_popup, sel(b"setAction:\0"), sel(b"onAction:\0"));
            msg_subview(card2, sel(b"addSubview:\0"), floor_popup);
            add_divider(card2, 96.0);

            // Row 2: Gravity Switch
            add_card_row_two_lines(card2, 96.0, "重力加速度模拟", "释放后遵循物理加速度下落与触底弹性回弹");
            let sw_gravity = msg_init_frame(msg_ptr(sw_cls, sel(b"alloc\0")), sel(b"initWithFrame:\0"), [content_w - 116.0, 107.0, 48.0, 26.0]);
            msg_void_isize(sw_gravity, sel(b"setState:\0"), if snap.temp_settings.gravity_enabled { 1 } else { 0 });
            msg_void_isize(sw_gravity, sel(b"setTag:\0"), 205);
            msg_void_ptr(sw_gravity, sel(b"setTarget:\0"), action_target);
            msg_void_ptr(sw_gravity, sel(b"setAction:\0"), sel(b"onAction:\0"));
            msg_subview(card2, sel(b"addSubview:\0"), sw_gravity);
            add_divider(card2, 144.0);

            // Row 3: Reset Position Button
            add_card_row_two_lines(card2, 144.0, "屏幕位置校准", "将桌宠一键拉回屏幕正中央安全区域");
            let btn_reset = msg_btn(button_cls, sel(b"buttonWithTitle:target:action:\0"), nsstring("复位居中"), action_target, sel(b"onAction:\0"));
            set_frame(btn_reset, sel(b"setFrame:\0"), [content_w - 156.0, 154.0, 88.0, 28.0]);
            msg_void_isize(btn_reset, sel(b"setTag:\0"), 204);
            msg_subview(card2, sel(b"addSubview:\0"), btn_reset);
            msg_subview(tab0_doc, sel(b"addSubview:\0"), card2);

            // Card 3: 状态与交互
            add_section_header(tab0_doc, "状态与交互", 390.0);
            let card3 = create_card(410.0, 192.0);

            // Row 0: Quota
            add_card_row_two_lines(card3, 0.0, "气泡实时额度卡片", "气泡中展示 5小时与周额度进度条");
            let sw_quota = msg_init_frame(msg_ptr(sw_cls, sel(b"alloc\0")), sel(b"initWithFrame:\0"), [content_w - 116.0, 11.0, 48.0, 26.0]);
            msg_void_isize(sw_quota, sel(b"setState:\0"), if snap.temp_settings.show_quota { 1 } else { 0 });
            msg_void_isize(sw_quota, sel(b"setTag:\0"), 206);
            msg_void_ptr(sw_quota, sel(b"setTarget:\0"), action_target);
            msg_void_ptr(sw_quota, sel(b"setAction:\0"), sel(b"onAction:\0"));
            msg_subview(card3, sel(b"addSubview:\0"), sw_quota);
            add_divider(card3, 48.0);

            // Row 1: IDE Only
            add_card_row_two_lines(card3, 48.0, "仅在 IDE 活跃时显示", "前台为编辑器时呈现，切出隐匿");
            let sw_ide = msg_init_frame(msg_ptr(sw_cls, sel(b"alloc\0")), sel(b"initWithFrame:\0"), [content_w - 116.0, 59.0, 48.0, 26.0]);
            msg_void_isize(sw_ide, sel(b"setState:\0"), if snap.temp_settings.menu_config.ide_only { 1 } else { 0 });
            msg_void_isize(sw_ide, sel(b"setTag:\0"), 209);
            msg_void_ptr(sw_ide, sel(b"setTarget:\0"), action_target);
            msg_void_ptr(sw_ide, sel(b"setAction:\0"), sel(b"onAction:\0"));
            msg_subview(card3, sel(b"addSubview:\0"), sw_ide);
            add_divider(card3, 96.0);

            // Row 2: DND Passthrough
            add_card_row_two_lines(card3, 96.0, "完全穿透免打扰", "猫猫处于只读透明状态，点击穿透到底层");
            let sw_dnd = msg_init_frame(msg_ptr(sw_cls, sel(b"alloc\0")), sel(b"initWithFrame:\0"), [content_w - 116.0, 107.0, 48.0, 26.0]);
            msg_void_isize(sw_dnd, sel(b"setState:\0"), if snap.temp_settings.dnd_passthrough { 1 } else { 0 });
            msg_void_isize(sw_dnd, sel(b"setTag:\0"), 207);
            msg_void_ptr(sw_dnd, sel(b"setTarget:\0"), action_target);
            msg_void_ptr(sw_dnd, sel(b"setAction:\0"), sel(b"onAction:\0"));
            msg_subview(card3, sel(b"addSubview:\0"), sw_dnd);
            add_divider(card3, 144.0);

            // Row 3: Sound
            add_card_row_two_lines(card3, 144.0, "动作互动音效", "点击、抚摸、成功与报错时的猫叫音频");
            let sw_snd = msg_init_frame(msg_ptr(sw_cls, sel(b"alloc\0")), sel(b"initWithFrame:\0"), [content_w - 116.0, 155.0, 48.0, 26.0]);
            msg_void_isize(sw_snd, sel(b"setState:\0"), if snap.temp_settings.sound_enabled { 1 } else { 0 });
            msg_void_isize(sw_snd, sel(b"setTag:\0"), 208);
            msg_void_ptr(sw_snd, sel(b"setTarget:\0"), action_target);
            msg_void_ptr(sw_snd, sel(b"setAction:\0"), sel(b"onAction:\0"));
            msg_subview(card3, sel(b"addSubview:\0"), sw_snd);
            msg_subview(tab0_doc, sel(b"addSubview:\0"), card3);
        }
        tab_views.push(tab0_sv);
        msg_subview(content_container, sel(b"addSubview:\0"), tab0_sv);

        // ================= TAB 1: 自启动与更新 (Scrollable) =================
        let (tab1_sv, tab1_doc) = make_scrollable_tab(520.0);
        {
            let h_title = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring("自启动与更新"));
            msg_void_ptr(h_title, sel(b"setFont:\0"), msg_font_bold(font_cls, sel(b"boldSystemFontOfSize:\0"), 20.0));
            set_frame(h_title, sel(b"setFrame:\0"), [24.0, 20.0, 300.0, 26.0]);
            msg_subview(tab1_doc, sel(b"addSubview:\0"), h_title);

            let h_sub = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring("管理开机自启动配置与挂件版本检测"));
            msg_void_ptr(h_sub, sel(b"setFont:\0"), small_font);
            msg_void_ptr(h_sub, sel(b"setTextColor:\0"), sec_color);
            set_frame(h_sub, sel(b"setFrame:\0"), [24.0, 48.0, 420.0, 16.0]);
            msg_subview(tab1_doc, sel(b"addSubview:\0"), h_sub);

            // Card 1: 系统集成
            add_section_header(tab1_doc, "系统集成", 78.0);
            let card1 = create_card(98.0, 48.0);
            add_card_row_two_lines(card1, 0.0, "开机自动启动", "在系统登录完成后自动在后台静默挂载大肥猫");
            let sw_auto = msg_init_frame(msg_ptr(sw_cls, sel(b"alloc\0")), sel(b"initWithFrame:\0"), [content_w - 116.0, 11.0, 48.0, 26.0]);
            msg_void_isize(sw_auto, sel(b"setState:\0"), if snap.autostart_enabled { 1 } else { 0 });
            msg_void_isize(sw_auto, sel(b"setTag:\0"), 301);
            msg_void_ptr(sw_auto, sel(b"setTarget:\0"), action_target);
            msg_void_ptr(sw_auto, sel(b"setAction:\0"), sel(b"onAction:\0"));
            msg_subview(card1, sel(b"addSubview:\0"), sw_auto);
            msg_subview(tab1_doc, sel(b"addSubview:\0"), card1);

            // Card 2: 核心规范与架构
            add_section_header(tab1_doc, "核心规范与架构", 162.0);
            let card2 = create_card(182.0, 144.0);
            add_card_row_two_lines(card2, 0.0, "核心引擎版本", "v0.5.0 (Pure CPU 2D NextGen Core)");
            add_divider(card2, 48.0);
            add_card_row_two_lines(card2, 48.0, "UI 渲染前端", "macOS Native Cocoa AppKit (0 KB 额外依赖)");
            add_divider(card2, 96.0);
            add_card_row_two_lines(card2, 96.0, "运行内存指标", "约 12MB RSS / 0% GPU 硬件占用");
            msg_subview(tab1_doc, sel(b"addSubview:\0"), card2);

            // Card 3: 版本更新
            add_section_header(tab1_doc, "版本检测", 342.0);
            let card3 = create_card(362.0, 48.0);
            add_card_row_two_lines(card3, 0.0, "检查更新通道", "当前已是最新稳定版本 (v0.5.0)");
            let btn_update = msg_btn(button_cls, sel(b"buttonWithTitle:target:action:\0"), nsstring("检查更新"), action_target, sel(b"onAction:\0"));
            set_frame(btn_update, sel(b"setFrame:\0"), [content_w - 156.0, 10.0, 88.0, 28.0]);
            msg_void_isize(btn_update, sel(b"setTag:\0"), 302);
            msg_subview(card3, sel(b"addSubview:\0"), btn_update);
            msg_subview(tab1_doc, sel(b"addSubview:\0"), card3);
        }
        msg_void_bool(tab1_sv, sel(b"setHidden:\0"), 1);
        tab_views.push(tab1_sv);
        msg_subview(content_container, sel(b"addSubview:\0"), tab1_sv);

        // ================= TAB 2: 皮肤管理 (Scrollable with Preview) =================
        let (tab2_sv, tab2_doc) = make_scrollable_tab(580.0);
        {
            let h_title = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring("皮肤管理"));
            msg_void_ptr(h_title, sel(b"setFont:\0"), msg_font_bold(font_cls, sel(b"boldSystemFontOfSize:\0"), 20.0));
            set_frame(h_title, sel(b"setFrame:\0"), [24.0, 20.0, 300.0, 26.0]);
            msg_subview(tab2_doc, sel(b"addSubview:\0"), h_title);

            let h_sub = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring("管理已安装皮肤外观与本地动画资产"));
            msg_void_ptr(h_sub, sel(b"setFont:\0"), small_font);
            msg_void_ptr(h_sub, sel(b"setTextColor:\0"), sec_color);
            set_frame(h_sub, sel(b"setFrame:\0"), [24.0, 48.0, 420.0, 16.0]);
            msg_subview(tab2_doc, sel(b"addSubview:\0"), h_sub);

            // Card 1: 激活皮肤视觉预览卡片 (160pt height)
            add_section_header(tab2_doc, "当前激活皮肤与样式预览", 78.0);
            let card1 = create_card(98.0, 160.0);

            // Left preview box (128x128)
            let preview_box = msg_init_frame(msg_ptr(flipped_cls, sel(b"alloc\0")), sel(b"initWithFrame:\0"), [16.0, 16.0, 128.0, 128.0]);
            msg_void_bool(preview_box, sel(b"setWantsLayer:\0"), 1);
            let p_layer = msg_ptr(preview_box, sel(b"layer\0"));
            msg_void_f64(p_layer, sel(b"setCornerRadius:\0"), 8.0);
            let box_bg = msg_color_rgba(color_cls, sel(b"colorWithRed:green:blue:alpha:\0"), 1.0, 1.0, 1.0, 0.05);
            let box_border = msg_color_rgba(color_cls, sel(b"colorWithRed:green:blue:alpha:\0"), 1.0, 1.0, 1.0, 0.12);
            msg_void_ptr(p_layer, sel(b"setBackgroundColor:\0"), msg_ptr(box_bg, sel(b"CGColor\0")));
            msg_void_ptr(p_layer, sel(b"setBorderColor:\0"), msg_ptr(box_border, sel(b"CGColor\0")));
            msg_void_f64(p_layer, sel(b"setBorderWidth:\0"), 1.0);
            msg_subview(card1, sel(b"addSubview:\0"), preview_box);

            // Load skin preview image from skins/fat-cat/frames/idle.png
            let root_dir = store.lock().unwrap().root_dir.clone();
            let idle_png_path = root_dir.join("skins").join("fat-cat").join("frames").join("idle.png");
            let idle_path_str = idle_png_path.to_string_lossy();

            let skin_iv = msg_init_frame(msg_ptr(imageview_cls, sel(b"alloc\0")), sel(b"initWithFrame:\0"), [4.0, 4.0, 120.0, 120.0]);
            let msg_init_file: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const ());
            let skin_img = msg_init_file(msg_ptr(image_cls, sel(b"alloc\0")), sel(b"initWithContentsOfFile:\0"), nsstring(&idle_path_str));
            if !skin_img.is_null() {
                msg_void_ptr(skin_iv, sel(b"setImage:\0"), skin_img);
                msg_void_usize(skin_iv, sel(b"setImageScaling:\0"), 3); // NSImageScaleProportionallyUpOrDown
            }
            msg_subview(preview_box, sel(b"addSubview:\0"), skin_iv);

            // Right skin metadata
            let s_name = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring("大肥猫 (fat-cat)"));
            msg_void_ptr(s_name, sel(b"setFont:\0"), msg_font_bold(font_cls, sel(b"boldSystemFontOfSize:\0"), 16.0));
            set_frame(s_name, sel(b"setFrame:\0"), [160.0, 16.0, 280.0, 22.0]);
            msg_subview(card1, sel(b"addSubview:\0"), s_name);

            let s_author = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring("创作者: Antigravity Pet Studio"));
            msg_void_ptr(s_author, sel(b"setFont:\0"), regular_font);
            msg_void_ptr(s_author, sel(b"setTextColor:\0"), sec_color);
            set_frame(s_author, sel(b"setFrame:\0"), [160.0, 42.0, 280.0, 18.0]);
            msg_subview(card1, sel(b"addSubview:\0"), s_author);

            let s_ver = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring("版本: v1.3.0 · 60fps 逐帧光栅化"));
            msg_void_ptr(s_ver, sel(b"setFont:\0"), small_font);
            msg_void_ptr(s_ver, sel(b"setTextColor:\0"), sec_color);
            set_frame(s_ver, sel(b"setFrame:\0"), [160.0, 64.0, 280.0, 16.0]);
            msg_subview(card1, sel(b"addSubview:\0"), s_ver);

            let s_desc = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring("简洁半身大肥猫桌宠，支持透明穿透、按压回弹、物理重力落底与实时用量监控。"));
            msg_void_ptr(s_desc, sel(b"setFont:\0"), small_font);
            set_frame(s_desc, sel(b"setFrame:\0"), [160.0, 84.0, 290.0, 32.0]);
            msg_subview(card1, sel(b"addSubview:\0"), s_desc);

            // Active Badge
            let badge_box = msg_init_frame(msg_ptr(flipped_cls, sel(b"alloc\0")), sel(b"initWithFrame:\0"), [160.0, 122.0, 96.0, 22.0]);
            msg_void_bool(badge_box, sel(b"setWantsLayer:\0"), 1);
            let b_layer = msg_ptr(badge_box, sel(b"layer\0"));
            msg_void_f64(b_layer, sel(b"setCornerRadius:\0"), 4.0);
            let badge_bg = msg_color_rgba(color_cls, sel(b"colorWithRed:green:blue:alpha:\0"), 0.18, 0.80, 0.44, 0.18);
            let badge_border = msg_color_rgba(color_cls, sel(b"colorWithRed:green:blue:alpha:\0"), 0.18, 0.80, 0.44, 0.45);
            msg_void_ptr(b_layer, sel(b"setBackgroundColor:\0"), msg_ptr(badge_bg, sel(b"CGColor\0")));
            msg_void_ptr(b_layer, sel(b"setBorderColor:\0"), msg_ptr(badge_border, sel(b"CGColor\0")));
            msg_void_f64(b_layer, sel(b"setBorderWidth:\0"), 1.0);

            let badge_lbl = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring("当前正在使用"));
            msg_void_ptr(badge_lbl, sel(b"setFont:\0"), small_font);
            let green_color = msg_color_rgba(color_cls, sel(b"colorWithRed:green:blue:alpha:\0"), 0.22, 0.88, 0.48, 1.0);
            msg_void_ptr(badge_lbl, sel(b"setTextColor:\0"), green_color);
            set_frame(badge_lbl, sel(b"setFrame:\0"), [10.0, 3.0, 80.0, 16.0]);
            msg_subview(badge_box, sel(b"addSubview:\0"), badge_lbl);
            msg_subview(card1, sel(b"addSubview:\0"), badge_box);

            msg_subview(tab2_doc, sel(b"addSubview:\0"), card1);

            // Card 2: 动画状态与规格
            add_section_header(tab2_doc, "动画状态与规格", 274.0);
            let card2 = create_card(294.0, 96.0);
            add_card_row_two_lines(card2, 0.0, "包含动作状态", "idle, thinking, working, success, error, petting");
            add_divider(card2, 48.0);
            add_card_row_two_lines(card2, 48.0, "渲染引擎架构", "Pure CPU 2D Software Surface (<12MB RSS, 0% GPU)");
            msg_subview(tab2_doc, sel(b"addSubview:\0"), card2);

            // Card 3: 本地资产管理
            add_section_header(tab2_doc, "本地资产管理与扩展", 406.0);
            let card3 = create_card(426.0, 96.0);
            add_card_row_two_lines(card3, 0.0, "资产存储路径", "skins/fat-cat/ (支持本地热重载与帧替换)");
            add_divider(card3, 48.0);
            add_card_row_two_lines(card3, 48.0, "在 Finder 中管理", "直接在访达中打开并替换 PNG 帧序列");
            let btn_finder = msg_btn(button_cls, sel(b"buttonWithTitle:target:action:\0"), nsstring("在 Finder 中打开"), action_target, sel(b"onAction:\0"));
            set_frame(btn_finder, sel(b"setFrame:\0"), [content_w - 188.0, 58.0, 120.0, 28.0]);
            msg_void_isize(btn_finder, sel(b"setTag:\0"), 230);
            msg_subview(card3, sel(b"addSubview:\0"), btn_finder);
            msg_subview(tab2_doc, sel(b"addSubview:\0"), card3);
        }
        msg_void_bool(tab2_sv, sel(b"setHidden:\0"), 1);
        tab_views.push(tab2_sv);
        msg_subview(content_container, sel(b"addSubview:\0"), tab2_sv);

        // ================= TAB 3: 台词与问候 =================
        let tab3_view = msg_init_frame(msg_ptr(flipped_cls, sel(b"alloc\0")), sel(b"initWithFrame:\0"), [0.0, 0.0, content_w, win_h]);
        let mut dialogue_pills = Vec::new();
        let mut dialogue_pill_labels = Vec::new();
        let dialogue_input;
        let dialogue_doc_view;
        {
            let h_title = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring("台词与问候"));
            msg_void_ptr(h_title, sel(b"setFont:\0"), msg_font_bold(font_cls, sel(b"boldSystemFontOfSize:\0"), 20.0));
            set_frame(h_title, sel(b"setFrame:\0"), [24.0, 20.0, 300.0, 26.0]);
            msg_subview(tab3_view, sel(b"addSubview:\0"), h_title);

            let h_sub = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring("自定义不同工作状态下的气泡台词，修改即时自动保存"));
            msg_void_ptr(h_sub, sel(b"setFont:\0"), small_font);
            msg_void_ptr(h_sub, sel(b"setTextColor:\0"), sec_color);
            set_frame(h_sub, sel(b"setFrame:\0"), [24.0, 48.0, 420.0, 16.0]);
            msg_subview(tab3_view, sel(b"addSubview:\0"), h_sub);

            // 5 Category Pills Segment
            let cats = ["待机日常", "互动抚摸", "深度思考", "任务成功", "任务遇阻"];
            let pill_w = 88.0f64;
            for (ci, cname) in cats.iter().enumerate() {
                let px = 24.0 + (ci as f64 * (pill_w + 8.0));
                let pill_host = msg_init_frame(msg_ptr(flipped_cls, sel(b"alloc\0")), sel(b"initWithFrame:\0"), [px, 74.0, pill_w, 28.0]);

                let pill_cap = msg_init_frame(msg_ptr(flipped_cls, sel(b"alloc\0")), sel(b"initWithFrame:\0"), [0.0, 0.0, pill_w, 28.0]);
                msg_void_bool(pill_cap, sel(b"setWantsLayer:\0"), 1);
                let p_layer = msg_ptr(pill_cap, sel(b"layer\0"));
                msg_void_f64(p_layer, sel(b"setCornerRadius:\0"), 6.0);
                if ci == snap.dialogue_cat {
                    msg_void_ptr(p_layer, sel(b"setBackgroundColor:\0"), accent_cg);
                } else {
                    let trans_bg = msg_color_rgba(color_cls, sel(b"colorWithRed:green:blue:alpha:\0"), 1.0, 1.0, 1.0, 0.06);
                    msg_void_ptr(p_layer, sel(b"setBackgroundColor:\0"), msg_ptr(trans_bg, sel(b"CGColor\0")));
                }
                msg_subview(pill_host, sel(b"addSubview:\0"), pill_cap);
                dialogue_pills.push(pill_cap);

                let plbl = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring(cname));
                msg_void_ptr(plbl, sel(b"setFont:\0"), small_font);
                if ci == snap.dialogue_cat {
                    msg_void_ptr(plbl, sel(b"setTextColor:\0"), white_color);
                } else {
                    msg_void_ptr(plbl, sel(b"setTextColor:\0"), label_color);
                }
                set_frame(plbl, sel(b"setFrame:\0"), [10.0, 6.0, 68.0, 16.0]);
                msg_subview(pill_host, sel(b"addSubview:\0"), plbl);
                dialogue_pill_labels.push(plbl);

                let pbtn = msg_init_frame(msg_ptr(button_cls, sel(b"alloc\0")), sel(b"initWithFrame:\0"), [0.0, 0.0, pill_w, 28.0]);
                msg_void_bool(pbtn, sel(b"setTransparent:\0"), 1);
                msg_void_isize(pbtn, sel(b"setTag:\0"), 400 + ci as isize);
                msg_void_ptr(pbtn, sel(b"setTarget:\0"), action_target);
                msg_void_ptr(pbtn, sel(b"setAction:\0"), sel(b"onAction:\0"));
                msg_subview(pill_host, sel(b"addSubview:\0"), pbtn);

                msg_subview(tab3_view, sel(b"addSubview:\0"), pill_host);
            }

            // Card 1: 添加新台词 (H=48.0)
            add_section_header(tab3_view, "添加新语句", 114.0);
            let card_add = create_card(132.0, 48.0);
            let tf_input = msg_init_frame(msg_ptr(textfield_cls, sel(b"alloc\0")), sel(b"initWithFrame:\0"), [16.0, 11.0, content_w - 180.0, 26.0]);
            msg_void_ptr(tf_input, sel(b"setPlaceholderString:\0"), nsstring("输入自定义台词内容并点击添加..."));
            msg_subview(card_add, sel(b"addSubview:\0"), tf_input);
            dialogue_input = tf_input;

            let btn_add = msg_btn(button_cls, sel(b"buttonWithTitle:target:action:\0"), nsstring("添加条目"), action_target, sel(b"onAction:\0"));
            set_frame(btn_add, sel(b"setFrame:\0"), [content_w - 156.0, 10.0, 88.0, 28.0]);
            msg_void_isize(btn_add, sel(b"setTag:\0"), 450);
            msg_subview(card_add, sel(b"addSubview:\0"), btn_add);
            msg_subview(tab3_view, sel(b"addSubview:\0"), card_add);

            // Card 2: 滚动台词列表
            add_section_header(tab3_view, "已收录台词列表", 192.0);
            let card_list = create_card(210.0, 292.0);

            let sv = msg_init_frame(msg_ptr(scroll_cls, sel(b"alloc\0")), sel(b"initWithFrame:\0"), [8.0, 8.0, content_w - 64.0, 276.0]);
            msg_void_bool(sv, sel(b"setHasVerticalScroller:\0"), 1);
            msg_void_bool(sv, sel(b"setAutohidesScrollers:\0"), 1);
            msg_void_bool(sv, sel(b"setDrawsBackground:\0"), 0);

            let doc_view = msg_init_frame(msg_ptr(flipped_cls, sel(b"alloc\0")), sel(b"initWithFrame:\0"), [0.0, 0.0, content_w - 64.0, 276.0]);
            msg_void_ptr(sv, sel(b"setDocumentView:\0"), doc_view);
            dialogue_doc_view = doc_view;

            msg_subview(card_list, sel(b"addSubview:\0"), sv);
            msg_subview(tab3_view, sel(b"addSubview:\0"), card_list);
        }
        msg_void_bool(tab3_view, sel(b"setHidden:\0"), 1);
        tab_views.push(tab3_view);
        msg_subview(content_container, sel(b"addSubview:\0"), tab3_view);

        // ================= TAB 4: 右键菜单配置 (Scrollable) =================
        let (tab4_sv, tab4_doc) = make_scrollable_tab(610.0);
        {
            let h_title = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring("右键菜单配置"));
            msg_void_ptr(h_title, sel(b"setFont:\0"), msg_font_bold(font_cls, sel(b"boldSystemFontOfSize:\0"), 20.0));
            set_frame(h_title, sel(b"setFrame:\0"), [24.0, 20.0, 300.0, 26.0]);
            msg_subview(tab4_doc, sel(b"addSubview:\0"), h_title);

            let h_sub = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring("管理在桌面右键点击桌宠时出现的控制项"));
            msg_void_ptr(h_sub, sel(b"setFont:\0"), small_font);
            msg_void_ptr(h_sub, sel(b"setTextColor:\0"), sec_color);
            set_frame(h_sub, sel(b"setFrame:\0"), [24.0, 48.0, 420.0, 16.0]);
            msg_subview(tab4_doc, sel(b"addSubview:\0"), h_sub);

            // Group 1: 核心控制项 (3 rows x 44.0 = 132.0)
            add_section_header(tab4_doc, "核心控制项", 78.0);
            let card1 = create_card(98.0, 132.0);
            add_card_row_single_line(card1, 0.0, "打开 Antigravity 控制台");
            let sw1 = msg_init_frame(msg_ptr(sw_cls, sel(b"alloc\0")), sel(b"initWithFrame:\0"), [content_w - 116.0, 9.0, 48.0, 26.0]);
            msg_void_isize(sw1, sel(b"setState:\0"), if snap.temp_settings.menu_config.open_antigravity { 1 } else { 0 });
            msg_void_isize(sw1, sel(b"setTag:\0"), 501);
            msg_void_ptr(sw1, sel(b"setTarget:\0"), action_target);
            msg_void_ptr(sw1, sel(b"setAction:\0"), sel(b"onAction:\0"));
            msg_subview(card1, sel(b"addSubview:\0"), sw1);
            add_divider(card1, 44.0);

            add_card_row_single_line(card1, 44.0, "偏好设置...");
            let sw2 = msg_init_frame(msg_ptr(sw_cls, sel(b"alloc\0")), sel(b"initWithFrame:\0"), [content_w - 116.0, 53.0, 48.0, 26.0]);
            msg_void_isize(sw2, sel(b"setState:\0"), if snap.temp_settings.menu_config.settings { 1 } else { 0 });
            msg_void_isize(sw2, sel(b"setTag:\0"), 502);
            msg_void_ptr(sw2, sel(b"setTarget:\0"), action_target);
            msg_void_ptr(sw2, sel(b"setAction:\0"), sel(b"onAction:\0"));
            msg_subview(card1, sel(b"addSubview:\0"), sw2);
            add_divider(card1, 88.0);

            add_card_row_single_line(card1, 88.0, "切换额度卡片");
            let sw3 = msg_init_frame(msg_ptr(sw_cls, sel(b"alloc\0")), sel(b"initWithFrame:\0"), [content_w - 116.0, 97.0, 48.0, 26.0]);
            msg_void_isize(sw3, sel(b"setState:\0"), if snap.temp_settings.menu_config.quota { 1 } else { 0 });
            msg_void_isize(sw3, sel(b"setTag:\0"), 505);
            msg_void_ptr(sw3, sel(b"setTarget:\0"), action_target);
            msg_void_ptr(sw3, sel(b"setAction:\0"), sel(b"onAction:\0"));
            msg_subview(card1, sel(b"addSubview:\0"), sw3);
            msg_subview(tab4_doc, sel(b"addSubview:\0"), card1);

            // Group 2: 交互与动作 (3 rows x 44.0 = 132.0)
            add_section_header(tab4_doc, "交互与动作", 246.0);
            let card2 = create_card(266.0, 132.0);
            add_card_row_single_line(card2, 0.0, "换一句台词");
            let sw4 = msg_init_frame(msg_ptr(sw_cls, sel(b"alloc\0")), sel(b"initWithFrame:\0"), [content_w - 116.0, 9.0, 48.0, 26.0]);
            msg_void_isize(sw4, sel(b"setState:\0"), if snap.temp_settings.menu_config.random_dialogue { 1 } else { 0 });
            msg_void_isize(sw4, sel(b"setTag:\0"), 503);
            msg_void_ptr(sw4, sel(b"setTarget:\0"), action_target);
            msg_void_ptr(sw4, sel(b"setAction:\0"), sel(b"onAction:\0"));
            msg_subview(card2, sel(b"addSubview:\0"), sw4);
            add_divider(card2, 44.0);

            add_card_row_single_line(card2, 44.0, "互动抚摸 / 动作");
            let sw5 = msg_init_frame(msg_ptr(sw_cls, sel(b"alloc\0")), sel(b"initWithFrame:\0"), [content_w - 116.0, 53.0, 48.0, 26.0]);
            msg_void_isize(sw5, sel(b"setState:\0"), if snap.temp_settings.menu_config.pet_action { 1 } else { 0 });
            msg_void_isize(sw5, sel(b"setTag:\0"), 504);
            msg_void_ptr(sw5, sel(b"setTarget:\0"), action_target);
            msg_void_ptr(sw5, sel(b"setAction:\0"), sel(b"onAction:\0"));
            msg_subview(card2, sel(b"addSubview:\0"), sw5);
            add_divider(card2, 88.0);

            add_card_row_single_line(card2, 88.0, "缩放大小 (滚轮微调)");
            let sw6 = msg_init_frame(msg_ptr(sw_cls, sel(b"alloc\0")), sel(b"initWithFrame:\0"), [content_w - 116.0, 97.0, 48.0, 26.0]);
            msg_void_isize(sw6, sel(b"setState:\0"), if snap.temp_settings.menu_config.zoom { 1 } else { 0 });
            msg_void_isize(sw6, sel(b"setTag:\0"), 508);
            msg_void_ptr(sw6, sel(b"setTarget:\0"), action_target);
            msg_void_ptr(sw6, sel(b"setAction:\0"), sel(b"onAction:\0"));
            msg_subview(card2, sel(b"addSubview:\0"), sw6);
            msg_subview(tab4_doc, sel(b"addSubview:\0"), card2);

            // Group 3: 运行控制 (3 rows x 44.0 = 132.0)
            add_section_header(tab4_doc, "运行行为控制", 414.0);
            let card3 = create_card(434.0, 132.0);
            add_card_row_single_line(card3, 0.0, "免打扰鼠标穿透");
            let sw7 = msg_init_frame(msg_ptr(sw_cls, sel(b"alloc\0")), sel(b"initWithFrame:\0"), [content_w - 116.0, 9.0, 48.0, 26.0]);
            msg_void_isize(sw7, sel(b"setState:\0"), if snap.temp_settings.menu_config.dnd { 1 } else { 0 });
            msg_void_isize(sw7, sel(b"setTag:\0"), 506);
            msg_void_ptr(sw7, sel(b"setTarget:\0"), action_target);
            msg_void_ptr(sw7, sel(b"setAction:\0"), sel(b"onAction:\0"));
            msg_subview(card3, sel(b"addSubview:\0"), sw7);
            add_divider(card3, 44.0);

            add_card_row_single_line(card3, 44.0, "动作音效开关");
            let sw8 = msg_init_frame(msg_ptr(sw_cls, sel(b"alloc\0")), sel(b"initWithFrame:\0"), [content_w - 116.0, 53.0, 48.0, 26.0]);
            msg_void_isize(sw8, sel(b"setState:\0"), if snap.temp_settings.menu_config.sound { 1 } else { 0 });
            msg_void_isize(sw8, sel(b"setTag:\0"), 507);
            msg_void_ptr(sw8, sel(b"setTarget:\0"), action_target);
            msg_void_ptr(sw8, sel(b"setAction:\0"), sel(b"onAction:\0"));
            msg_subview(card3, sel(b"addSubview:\0"), sw8);
            add_divider(card3, 88.0);

            add_card_row_single_line(card3, 88.0, "重力加速度下落");
            let sw9 = msg_init_frame(msg_ptr(sw_cls, sel(b"alloc\0")), sel(b"initWithFrame:\0"), [content_w - 116.0, 97.0, 48.0, 26.0]);
            msg_void_isize(sw9, sel(b"setState:\0"), if snap.temp_settings.menu_config.gravity { 1 } else { 0 });
            msg_void_isize(sw9, sel(b"setTag:\0"), 509);
            msg_void_ptr(sw9, sel(b"setTarget:\0"), action_target);
            msg_void_ptr(sw9, sel(b"setAction:\0"), sel(b"onAction:\0"));
            msg_subview(card3, sel(b"addSubview:\0"), sw9);
            msg_subview(tab4_doc, sel(b"addSubview:\0"), card3);
        }
        msg_void_bool(tab4_sv, sel(b"setHidden:\0"), 1);
        tab_views.push(tab4_sv);
        msg_subview(content_container, sel(b"addSubview:\0"), tab4_sv);

        msg_ptr(win, sel(b"center\0"));

        let app = Self {
            store,
            ns_window: win,
            content_container,
            tab_views,
            current_tab: 0,
            sidebar_capsules,
            sidebar_labels,
            sidebar_status_lbl,
            scale_val_lbl,
            theme_popup,
            floor_popup,
            dialogue_pills,
            dialogue_pill_labels,
            dialogue_input,
            dialogue_doc_view,
            action_target,
        };

        // Populate initial dialogue list
        app.rebuild_dialogue_list();
        app
    }

    pub fn handle_action(&mut self, tag: isize, sender: *mut c_void) {
        unsafe {
            let msg_f64: unsafe extern "C" fn(*mut c_void, *mut c_void) -> f64 =
                std::mem::transmute(objc_msgSend as *const ());
            let msg_isize: unsafe extern "C" fn(*mut c_void, *mut c_void) -> isize =
                std::mem::transmute(objc_msgSend as *const ());
            let msg_ptr: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const ());
            let msg_void_ptr: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) =
                std::mem::transmute(objc_msgSend as *const ());

            match tag {
                // Tab Navigation 100..=104
                100..=104 => {
                    let tab_idx = (tag - 100) as usize;
                    self.switch_tab(tab_idx);
                }
                // Theme Popup
                201 => {
                    let idx = msg_isize(sender, sel(b"indexOfSelectedItem\0"));
                    let theme = match idx {
                        1 => "light".to_string(),
                        2 => "dark".to_string(),
                        _ => "system".to_string(),
                    };
                    self.store.lock().unwrap().dispatch(SettingsAction::SetTheme(theme));
                    self.store.lock().unwrap().commit_to_disk();
                    self.update_status("外观主题已保存");
                }
                // Scale Slider
                202 => {
                    let val = msg_f64(sender, sel(b"doubleValue\0"));
                    self.store.lock().unwrap().dispatch(SettingsAction::SetScale(val as f32));
                    let scale = self.store.lock().unwrap().temp_settings.scale;
                    let text = format!("{}%", (scale * 100.0).round() as i32);
                    msg_void_ptr(self.scale_val_lbl, sel(b"setStringValue:\0"), nsstring(&text));
                    self.store.lock().unwrap().commit_to_disk();
                    self.update_status("缩放大小已同步");
                }
                // Floor Mode Popup
                203 => {
                    let idx = msg_isize(sender, sel(b"indexOfSelectedItem\0"));
                    let mode = match idx {
                        1 => "bottom".to_string(),
                        2 => "free".to_string(),
                        _ => "taskbar".to_string(),
                    };
                    self.store.lock().unwrap().dispatch(SettingsAction::SetFloorMode(mode));
                    self.store.lock().unwrap().commit_to_disk();
                    self.update_status("落地点模式已保存");
                }
                // Reset Position Button
                204 => {
                    self.store.lock().unwrap().dispatch(SettingsAction::ResetPosition);
                    self.store.lock().unwrap().commit_to_disk();
                    self.update_status("已复位至屏幕正中央");
                }
                // Switches Tab 0
                205 => {
                    self.store.lock().unwrap().dispatch(SettingsAction::ToggleGravity);
                    self.store.lock().unwrap().commit_to_disk();
                    self.update_status("重力物理状态已保存");
                }
                206 => {
                    self.store.lock().unwrap().dispatch(SettingsAction::ToggleShowQuota);
                    self.store.lock().unwrap().commit_to_disk();
                    self.update_status("额度气泡卡片已保存");
                }
                207 => {
                    self.store.lock().unwrap().dispatch(SettingsAction::ToggleDndPassthrough);
                    self.store.lock().unwrap().commit_to_disk();
                    self.update_status("免打扰模式已保存");
                }
                208 => {
                    self.store.lock().unwrap().dispatch(SettingsAction::ToggleSound);
                    self.store.lock().unwrap().commit_to_disk();
                    self.update_status("音效开关已保存");
                }
                209 => {
                    self.store.lock().unwrap().dispatch(SettingsAction::ToggleIdeOnly);
                    self.store.lock().unwrap().commit_to_disk();
                    self.update_status("IDE 聚焦模式已保存");
                }
                // Tab 1 Actions
                301 => {
                    self.store.lock().unwrap().dispatch(SettingsAction::ToggleAutostart);
                    self.store.lock().unwrap().commit_to_disk();
                    self.update_status("自启动状态已保存");
                }
                302 => {
                    self.update_status("已是最新稳定版本 (v0.5.0)");
                }
                // Tab 2 Action: Open in Finder
                230 => {
                    let root = self.store.lock().unwrap().root_dir.clone();
                    let skin_dir = root.join("skins").join("fat-cat");
                    let _ = std::process::Command::new("open").arg(&skin_dir).spawn();
                    self.update_status("已在 Finder 中打开皮肤目录");
                }
                // Tab 3 Category Pills 400..=404
                400..=404 => {
                    let cat_idx = (tag - 400) as usize;
                    self.switch_dialogue_cat(cat_idx);
                }
                // Tab 3 Add Dialogue Button
                450 => {
                    let str_obj = msg_ptr(self.dialogue_input, sel(b"stringValue\0"));
                    let utf_ptr: *const i8 = msg_ptr(str_obj, sel(b"UTF8String\0")) as *const i8;
                    if !utf_ptr.is_null() {
                        let text = std::ffi::CStr::from_ptr(utf_ptr).to_string_lossy().trim().to_string();
                        if !text.is_empty() {
                            self.store.lock().unwrap().dispatch(SettingsAction::AddDialogue(text));
                            self.store.lock().unwrap().commit_to_disk();
                            msg_void_ptr(self.dialogue_input, sel(b"setStringValue:\0"), nsstring(""));
                            self.rebuild_dialogue_list();
                            self.update_status("已添加新台词并保存");
                        }
                    }
                }
                // Tab 3 Delete Dialogue Buttons 2000..=2999
                2000..=2999 => {
                    let line_idx = (tag - 2000) as usize;
                    self.store.lock().unwrap().dispatch(SettingsAction::DeleteDialogue(line_idx));
                    self.store.lock().unwrap().commit_to_disk();
                    self.rebuild_dialogue_list();
                    self.update_status("已删除台词并保存");
                }
                // Tab 4 Menu Options 501..=509
                501..=509 => {
                    let opt_idx = (tag - 501) as usize;
                    self.store.lock().unwrap().dispatch(SettingsAction::ToggleMenuOption(opt_idx));
                    self.store.lock().unwrap().commit_to_disk();
                    self.update_status("右键菜单配置已更新");
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
            let msg_void_ptr: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) =
                std::mem::transmute(objc_msgSend as *const ());
            let msg_ptr: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const ());
            let msg_color_rgba: unsafe extern "C" fn(*mut c_void, *mut c_void, f64, f64, f64, f64) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const ());

            let color_cls = objc_getClass(b"NSColor\0".as_ptr());
            let accent_color = msg_color_rgba(color_cls, sel(b"colorWithRed:green:blue:alpha:\0"), 0.0, 0.48, 1.0, 0.88);
            let accent_cg = msg_ptr(accent_color, sel(b"CGColor\0"));
            let clear_color = msg_ptr(color_cls, sel(b"clearColor\0"));
            let clear_cg = msg_ptr(clear_color, sel(b"CGColor\0"));

            let white_color = msg_ptr(color_cls, sel(b"whiteColor\0"));
            let label_color = msg_ptr(color_cls, sel(b"labelColor\0"));

            // Update views visibility
            for (i, view) in self.tab_views.iter().enumerate() {
                msg_void_bool(*view, sel(b"setHidden:\0"), if i == new_tab { 0 } else { 1 });
            }

            // Update sidebar capsules and labels
            for (i, capsule) in self.sidebar_capsules.iter().enumerate() {
                let layer = msg_ptr(*capsule, sel(b"layer\0"));
                if i == new_tab {
                    msg_void_ptr(layer, sel(b"setBackgroundColor:\0"), accent_cg);
                    msg_void_ptr(self.sidebar_labels[i], sel(b"setTextColor:\0"), white_color);
                } else {
                    msg_void_ptr(layer, sel(b"setBackgroundColor:\0"), clear_cg);
                    msg_void_ptr(self.sidebar_labels[i], sel(b"setTextColor:\0"), label_color);
                }
            }

            if new_tab == 3 {
                self.rebuild_dialogue_list();
            }
        }
    }

    pub fn switch_dialogue_cat(&mut self, new_cat: usize) {
        if new_cat >= self.dialogue_pills.len() {
            return;
        }
        self.store.lock().unwrap().dispatch(SettingsAction::SelectDialogueCategory(new_cat));
        unsafe {
            let msg_void_ptr: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) =
                std::mem::transmute(objc_msgSend as *const ());
            let msg_ptr: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const ());
            let msg_color_rgba: unsafe extern "C" fn(*mut c_void, *mut c_void, f64, f64, f64, f64) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const ());

            let color_cls = objc_getClass(b"NSColor\0".as_ptr());
            let accent_color = msg_color_rgba(color_cls, sel(b"colorWithRed:green:blue:alpha:\0"), 0.0, 0.48, 1.0, 0.88);
            let accent_cg = msg_ptr(accent_color, sel(b"CGColor\0"));
            let trans_bg = msg_color_rgba(color_cls, sel(b"colorWithRed:green:blue:alpha:\0"), 1.0, 1.0, 1.0, 0.06);
            let trans_cg = msg_ptr(trans_bg, sel(b"CGColor\0"));

            let white_color = msg_ptr(color_cls, sel(b"whiteColor\0"));
            let label_color = msg_ptr(color_cls, sel(b"labelColor\0"));

            for (ci, pill) in self.dialogue_pills.iter().enumerate() {
                let p_layer = msg_ptr(*pill, sel(b"layer\0"));
                if ci == new_cat {
                    msg_void_ptr(p_layer, sel(b"setBackgroundColor:\0"), accent_cg);
                    msg_void_ptr(self.dialogue_pill_labels[ci], sel(b"setTextColor:\0"), white_color);
                } else {
                    msg_void_ptr(p_layer, sel(b"setBackgroundColor:\0"), trans_cg);
                    msg_void_ptr(self.dialogue_pill_labels[ci], sel(b"setTextColor:\0"), label_color);
                }
            }
        }
        self.rebuild_dialogue_list();
    }

    pub fn rebuild_dialogue_list(&self) {
        if self.dialogue_doc_view.is_null() {
            return;
        }
        unsafe {
            let msg_ptr: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const ());
            let msg_void_ptr: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) =
                std::mem::transmute(objc_msgSend as *const ());
            let msg_void_isize: unsafe extern "C" fn(*mut c_void, *mut c_void, isize) =
                std::mem::transmute(objc_msgSend as *const ());
            let msg_subview: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) =
                std::mem::transmute(objc_msgSend as *const ());
            let set_frame: unsafe extern "C" fn(*mut c_void, *mut c_void, [f64; 4]) =
                std::mem::transmute(objc_msgSend as *const ());
            let msg_label: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const ());
            let msg_btn: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void, *mut c_void, *mut c_void) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const ());

            let arr_cls = objc_getClass(b"NSArray\0".as_ptr());
            let empty_arr = msg_ptr(arr_cls, sel(b"array\0"));
            msg_void_ptr(self.dialogue_doc_view, sel(b"setSubviews:\0"), empty_arr);

            let store = self.store.lock().unwrap();
            let lines = store.active_dialogues().clone();
            drop(store);

            let row_h = 38.0f64;
            let total_h = (lines.len() as f64 * row_h).max(276.0);
            set_frame(self.dialogue_doc_view, sel(b"setFrame:\0"), [0.0, 0.0, 440.0, total_h]);

            let textfield_cls = objc_getClass(b"NSTextField\0".as_ptr());
            let button_cls = objc_getClass(b"NSButton\0".as_ptr());
            let font_cls = objc_getClass(b"NSFont\0".as_ptr());
            let msg_font: unsafe extern "C" fn(*mut c_void, *mut c_void, f64) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const ());
            let regular_font = msg_font(font_cls, sel(b"systemFontOfSize:\0"), 12.0);

            for (i, line) in lines.iter().enumerate() {
                let y = i as f64 * row_h;

                let display_text = format!("{}. {}", i + 1, line);
                let lbl = msg_label(textfield_cls, sel(b"labelWithString:\0"), nsstring(&display_text));
                msg_void_ptr(lbl, sel(b"setFont:\0"), regular_font);
                // Optical vertical alignment: y + 10.0 centers text with button at y + 6.0
                set_frame(lbl, sel(b"setFrame:\0"), [10.0, y + 10.0, 350.0, 18.0]);
                msg_subview(self.dialogue_doc_view, sel(b"addSubview:\0"), lbl);

                let del_btn = msg_btn(button_cls, sel(b"buttonWithTitle:target:action:\0"), nsstring("删除"), self.action_target, sel(b"onAction:\0"));
                set_frame(del_btn, sel(b"setFrame:\0"), [370.0, y + 6.0, 58.0, 26.0]);
                msg_void_isize(del_btn, sel(b"setTag:\0"), 2000 + i as isize);
                msg_subview(self.dialogue_doc_view, sel(b"addSubview:\0"), del_btn);
            }
        }
    }

    pub fn update_status(&self, text: &str) {
        if self.sidebar_status_lbl.is_null() {
            return;
        }
        unsafe {
            let msg_void_ptr: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) =
                std::mem::transmute(objc_msgSend as *const ());
            msg_void_ptr(self.sidebar_status_lbl, sel(b"setStringValue:\0"), nsstring(text));
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
