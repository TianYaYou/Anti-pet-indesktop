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

/// Native macOS AppKit Settings Window Controller.
/// Uses native `NSWindow` and `NSVisualEffectView` with `material = 7` (Sidebar Vibrancy)
/// for 100% authentic Apple aesthetics and zero bundle size.
pub struct MacosSettingsApp {
    pub store: Arc<Mutex<SettingsStore>>,
    ns_window: *mut c_void,
}

impl MacosSettingsApp {
    pub fn new(store: Arc<Mutex<SettingsStore>>) -> Self {
        let ns_window = unsafe { Self::create_native_window() };
        Self { store, ns_window }
    }

    unsafe fn create_native_window() -> *mut c_void {
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
        let color_cls = objc_getClass(b"NSColor\0".as_ptr());
        let str_cls = objc_getClass(b"NSString\0".as_ptr());

        let w = 700.0f64;
        let h = 500.0f64;
        let rect = [100.0, 100.0, w, h];

        // NSWindowStyleMaskTitled (1) | NSWindowStyleMaskClosable (2) | NSWindowStyleMaskMiniaturizable (4) | NSWindowStyleMaskFullSizeContentView (32768)
        let style_mask = 1 | 2 | 4 | (1 << 15);
        let win_alloc = msg_ptr(window_cls, sel(b"alloc\0"));
        let win = msg_init_window(win_alloc, sel(b"initWithContentRect:styleMask:backing:defer:\0"), rect, style_mask, 2, 0);

        if win.is_null() {
            return std::ptr::null_mut();
        }

        // Title
        let msg_str: unsafe extern "C" fn(*mut c_void, *mut c_void, *const u8) -> *mut c_void =
            std::mem::transmute(objc_msgSend as *const ());
        let title_str = msg_str(str_cls, sel(b"stringWithUTF8String:\0"), b"Anti-Pet \xE5\x81\x8F\xE5\xA5\xBD\xE8\xAE\xBE\xE7\xBD\xAE\0".as_ptr());
        msg_void_ptr(win, sel(b"setTitle:\0"), title_str);

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
        if !content_view.is_null() && !effect_cls.is_null() {
            let frame = [0.0, 0.0, w, h];
            let effect_alloc = msg_ptr(effect_cls, sel(b"alloc\0"));
            let effect_view = msg_init_frame(effect_alloc, sel(b"initWithFrame:\0"), frame);
            if !effect_view.is_null() {
                // NSVisualEffectMaterialSidebar = 7
                msg_void_isize(effect_view, sel(b"setMaterial:\0"), 7);
                // NSVisualEffectBlendingModeBehindWindow = 0
                msg_void_isize(effect_view, sel(b"setBlendingMode:\0"), 0);
                // NSVisualEffectStateActive = 1
                msg_void_isize(effect_view, sel(b"setState:\0"), 1);
                // NSViewWidthSizable (2) | NSViewHeightSizable (16) = 18
                msg_void_usize(effect_view, sel(b"setAutoresizingMask:\0"), 18);
                msg_subview(content_view, sel(b"addSubview:positioned:relativeTo:\0"), effect_view, -1, std::ptr::null_mut());
            }
        }

        msg_ptr(win, sel(b"center\0"));
        win
    }

    pub fn show(&self) {
        if self.ns_window.is_null() {
            return;
        }
        unsafe {
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
