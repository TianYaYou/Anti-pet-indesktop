#[cfg(not(feature = "legacy-webkit"))]
mod hit_test;
#[cfg(not(feature = "legacy-webkit"))]
mod menu_audio;
#[cfg(not(feature = "legacy-webkit"))]
mod physics;
#[cfg(not(feature = "legacy-webkit"))]
mod quota;
#[cfg(not(feature = "legacy-webkit"))]
mod renderer;
#[cfg(not(feature = "legacy-webkit"))]
mod state_watcher;

#[cfg(feature = "legacy-webkit")]
mod legacy_webkit;

#[cfg(feature = "legacy-webkit")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    legacy_webkit::run_legacy_webkit()
}

#[cfg(not(feature = "legacy-webkit"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    cpu_2d_app::run_cpu_2d()
}

#[cfg(not(feature = "legacy-webkit"))]
mod cpu_2d_app {
    #[cfg(target_os = "macos")]
    use std::ffi::c_void;
    use std::{
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        },
        thread,
        time::{Duration, Instant},
    };
    use muda::MenuEvent;
    use tao::{
        dpi::{LogicalPosition, LogicalSize},
        event::{ElementState, Event, MouseButton as TaoMouseButton, StartCause, WindowEvent},
        event_loop::{ControlFlow, EventLoopBuilder},
        window::WindowBuilder,
    };
    #[cfg(target_os = "macos")]
    use tao::platform::macos::{WindowBuilderExtMacOS, WindowExtMacOS};
    #[cfg(target_os = "windows")]
    use tao::platform::windows::{WindowBuilderExtWindows, WindowExtWindows};
    use tray_icon::{
        Icon as TrayIcon, MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent,
    };

    use crate::{
        hit_test::PixelHitTester,
        menu_audio::{
            pick_random_line, NativeAudioPlayer, NativeMenuManager, PetMenuAction, SoundEffect,
            DRAGGED_DIALOGUES, ERROR_DIALOGUES, IDLE_DIALOGUES, PETTING_DIALOGUES,
            SUCCESS_DIALOGUES, THINKING_DIALOGUES,
        },
        physics::{PetMotionState, PetPhysics, COMPACT_WIN_H, COMPACT_WIN_W},
        quota::{NativeQuotaProber, QuotaSnapshot},
        renderer::{
            bubble::ThoughtBubble,
            is_dark_mode_active,
            settings_ui::{SettingsAction, SettingsUi, SETTINGS_WIN_H, SETTINGS_WIN_W},
            sprite::CatSprite,
            Pixmap, SoftwareSurface,
        },
        state_watcher::{HookStateUpdate, NativeStateWatcher},
    };

    #[derive(Debug, Clone)]
    enum AppEvent {
        TickFrame,
        QuotaUpdated(QuotaSnapshot),
        HookStateChanged(HookStateUpdate),
        #[allow(dead_code)]
        CloseSettings,
        #[allow(dead_code)]
        ReloadSettings,
        ResetPosition,
    }


    #[cfg(target_os = "macos")]
    mod macos_window {
        use super::c_void;

        #[repr(C)]
        #[derive(Clone, Copy, Debug, Default)]
        pub struct NSPoint {
            pub x: f64,
            pub y: f64,
        }

        #[repr(C)]
        #[derive(Clone, Copy, Debug, Default)]
        pub struct NSSize {
            pub width: f64,
            pub height: f64,
        }

        #[repr(C)]
        #[derive(Clone, Copy, Debug, Default)]
        pub struct NSRect {
            pub origin: NSPoint,
            pub size: NSSize,
        }

        #[derive(Clone, Copy, Debug, Default)]
        pub struct DockInsets {
            pub bottom: f64,
            pub left: f64,
            pub right: f64,
        }

        extern "C" {
            fn objc_getClass(name: *const u8) -> *mut c_void;
            fn sel_registerName(name: *const u8) -> *mut c_void;
            fn objc_msgSend();
            #[cfg(target_arch = "x86_64")]
            fn objc_msgSend_stret();
            fn CGEventSourceSecondsSinceLastEventType(state_id: i32, event_type: u32) -> f64;
        }

        #[inline]
        unsafe fn sel(name: &[u8]) -> *mut c_void {
            sel_registerName(name.as_ptr())
        }

        #[cfg(target_arch = "aarch64")]
        unsafe fn msg_send_rect(receiver: *mut c_void, selector: *mut c_void) -> NSRect {
            let f: unsafe extern "C" fn(*mut c_void, *mut c_void) -> NSRect =
                std::mem::transmute(objc_msgSend as *const ());
            f(receiver, selector)
        }

        #[cfg(target_arch = "x86_64")]
        unsafe fn msg_send_rect(receiver: *mut c_void, selector: *mut c_void) -> NSRect {
            let mut out = NSRect::default();
            let f: unsafe extern "C" fn(*mut NSRect, *mut c_void, *mut c_void) =
                std::mem::transmute(objc_msgSend_stret as *const ());
            f(&mut out, receiver, selector);
            out
        }

        #[cfg(target_arch = "aarch64")]
        unsafe fn msg_send_point(receiver: *mut c_void, selector: *mut c_void) -> NSPoint {
            let f: unsafe extern "C" fn(*mut c_void, *mut c_void) -> NSPoint =
                std::mem::transmute(objc_msgSend as *const ());
            f(receiver, selector)
        }

        #[cfg(target_arch = "x86_64")]
        unsafe fn msg_send_point(receiver: *mut c_void, selector: *mut c_void) -> NSPoint {
            let f: unsafe extern "C" fn(*mut c_void, *mut c_void) -> NSPoint =
                std::mem::transmute(objc_msgSend as *const ());
            f(receiver, selector)
        }

        pub fn is_user_typing_globally() -> bool {
            unsafe {
                let down_secs = CGEventSourceSecondsSinceLastEventType(0, 10);
                let up_secs = CGEventSourceSecondsSinceLastEventType(0, 11);
                (down_secs >= 0.0 && down_secs < 1.2) || (up_secs >= 0.0 && up_secs < 1.2)
            }
        }

        /// Returns `(screen_x, screen_y_top_down, local_x, local_y)`
        pub fn get_cursor_state(ns_window: *mut c_void, mon_h: f64) -> Option<(f64, f64, f64, f64)> {
            if ns_window.is_null() {
                return None;
            }
            unsafe {
                let event_cls = objc_getClass(b"NSEvent\0".as_ptr());
                if event_cls.is_null() {
                    return None;
                }
                let mouse_loc = msg_send_point(event_cls, sel(b"mouseLocation\0"));
                let frame = msg_send_rect(ns_window, sel(b"frame\0"));
                let local_x = mouse_loc.x - frame.origin.x;
                let local_y = (frame.origin.y + frame.size.height) - mouse_loc.y;
                let screen_y_top_down = mon_h - mouse_loc.y;
                Some((mouse_loc.x, screen_y_top_down, local_x, local_y))
            }
        }

        pub fn set_accessory_activation_policy() {
            unsafe {
                let app_cls = objc_getClass(b"NSApplication\0".as_ptr());
                if app_cls.is_null() {
                    return;
                }
                let msg_ptr: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                    std::mem::transmute(objc_msgSend as *const ());
                let shared_app = msg_ptr(app_cls, sel(b"sharedApplication\0"));
                if shared_app.is_null() {
                    return;
                }
                let msg_set_policy: unsafe extern "C" fn(*mut c_void, *mut c_void, isize) -> bool =
                    std::mem::transmute(objc_msgSend as *const ());
                msg_set_policy(shared_app, sel(b"setActivationPolicy:\0"), 1);
            }
        }

        pub fn enforce_topmost(ns_window: *mut c_void) {
            if ns_window.is_null() {
                return;
            }
            unsafe {
                // Level 100 floats above the macOS Dock (level 20), trash can, status items (25),
                // while staying strictly below native context menus (NSPopUpMenuWindowLevel: 101)
                let set_level: unsafe extern "C" fn(*mut c_void, *mut c_void, isize) =
                    std::mem::transmute(objc_msgSend as *const ());
                set_level(ns_window, sel(b"setLevel:\0"), 100);

                // CanJoinAllSpaces (1<<0) | Stationary (1<<4) | IgnoresCycle (1<<6) | FullScreenAuxiliary (1<<8)
                let behavior: usize = (1 << 0) | (1 << 4) | (1 << 6) | (1 << 8);
                let set_behavior: unsafe extern "C" fn(*mut c_void, *mut c_void, usize) =
                    std::mem::transmute(objc_msgSend as *const ());
                set_behavior(ns_window, sel(b"setCollectionBehavior:\0"), behavior);

                let order_front: unsafe extern "C" fn(*mut c_void, *mut c_void) =
                    std::mem::transmute(objc_msgSend as *const ());
                order_front(ns_window, sel(b"orderFrontRegardless\0"));
            }
        }

        pub fn activate_ignoring_other_apps(ns_window: *mut c_void) {
            unsafe {
                let app_cls = objc_getClass(b"NSApplication\0".as_ptr());
                if !app_cls.is_null() {
                    let msg_ptr: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                        std::mem::transmute(objc_msgSend as *const ());
                    let shared_app = msg_ptr(app_cls, sel(b"sharedApplication\0"));
                    if !shared_app.is_null() {
                        let msg_void_bool: unsafe extern "C" fn(*mut c_void, *mut c_void, i8) =
                            std::mem::transmute(objc_msgSend as *const ());
                        msg_void_bool(shared_app, sel(b"activateIgnoringOtherApps:\0"), 1);
                    }
                }
                if !ns_window.is_null() {
                    let msg_void: unsafe extern "C" fn(*mut c_void, *mut c_void) =
                        std::mem::transmute(objc_msgSend as *const ());
                    msg_void(ns_window, sel(b"makeKeyWindow\0"));
                    msg_void(ns_window, sel(b"orderFrontRegardless\0"));
                }
            }
        }

        pub fn show_settings_window(ns_window: *mut c_void) {
            if ns_window.is_null() {
                return;
            }
            unsafe {
                // Settings dialog window level: NSFloatingWindowLevel (3)
                let set_level: unsafe extern "C" fn(*mut c_void, *mut c_void, isize) =
                    std::mem::transmute(objc_msgSend as *const ());
                set_level(ns_window, sel(b"setLevel:\0"), 3);

                // MoveToActiveSpace (1<<1) | FullScreenNone (1<<9)
                // Prevents corrupting or invading full-screen spaces of other applications
                let behavior: usize = (1 << 1) | (1 << 9);
                let set_behavior: unsafe extern "C" fn(*mut c_void, *mut c_void, usize) =
                    std::mem::transmute(objc_msgSend as *const ());
                set_behavior(ns_window, sel(b"setCollectionBehavior:\0"), behavior);

                let center_win: unsafe extern "C" fn(*mut c_void, *mut c_void) =
                    std::mem::transmute(objc_msgSend as *const ());
                center_win(ns_window, sel(b"center\0"));

                let make_key_and_order: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) =
                    std::mem::transmute(objc_msgSend as *const ());
                make_key_and_order(ns_window, sel(b"makeKeyAndOrderFront:\0"), std::ptr::null_mut());

                let app_cls = objc_getClass(b"NSApplication\0".as_ptr());
                if !app_cls.is_null() {
                    let msg_ptr: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                        std::mem::transmute(objc_msgSend as *const ());
                    let shared_app = msg_ptr(app_cls, sel(b"sharedApplication\0"));
                    if !shared_app.is_null() {
                        let msg_void_bool: unsafe extern "C" fn(*mut c_void, *mut c_void, i8) =
                            std::mem::transmute(objc_msgSend as *const ());
                        msg_void_bool(shared_app, sel(b"activateIgnoringOtherApps:\0"), 1);
                    }
                }
            }
        }

        pub fn query_dock_insets_global() -> DockInsets {
            unsafe {
                let cls = objc_getClass(b"NSScreen\0".as_ptr());
                if cls.is_null() {
                    return DockInsets::default();
                }
                let msg_ptr: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                    std::mem::transmute(objc_msgSend as *const ());
                let screen = msg_ptr(cls, sel(b"mainScreen\0"));
                if screen.is_null() {
                    return DockInsets::default();
                }

                let full_frame = msg_send_rect(screen, sel(b"frame\0"));
                let vis_frame = msg_send_rect(screen, sel(b"visibleFrame\0"));

                DockInsets {
                    bottom: (vis_frame.origin.y - full_frame.origin.y).max(0.0),
                    left: (vis_frame.origin.x - full_frame.origin.x).max(0.0),
                    right: ((full_frame.origin.x + full_frame.size.width)
                        - (vis_frame.origin.x + vis_frame.size.width))
                    .max(0.0),
                }
            }
        }
    }

    #[allow(dead_code)]
    fn ensure_backend_server() {
        let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 4199));
        let server_alive = std::net::TcpStream::connect_timeout(&addr, Duration::from_millis(150)).is_ok();
        if !server_alive {
            let mut server_path = std::path::PathBuf::from("server/index.js");
            if !server_path.exists() {
                if let Ok(exe) = std::env::current_exe() {
                    let bundle_path = exe.parent().and_then(|p| p.parent()).map(|p| p.join("Resources/server/index.js"));
                    if let Some(bp) = bundle_path {
                        if bp.exists() {
                            server_path = bp;
                        }
                    }
                }
            }
            if server_path.exists() {
                let _ = std::process::Command::new("node")
                    .arg(server_path)
                    .spawn();
                std::thread::sleep(Duration::from_millis(400));
            }
        }
    }



    #[cfg(target_os = "windows")]
    mod windows_window {
        use windows::Win32::Foundation::{HWND, POINT, WPARAM, LPARAM};
        use windows::Win32::Graphics::Gdi::{
            CreateCompatibleBitmap, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject,
            GetMonitorInfoW, MonitorFromWindow, ScreenToClient, BITMAPINFO, BITMAPINFOHEADER,
            BI_RGB, DIB_RGB_COLORS, MONITORINFO, MONITOR_DEFAULTTOPRIMARY,
        };
        use windows::Win32::Graphics::Dwm::{
            DwmSetWindowAttribute, DWMWA_BORDER_COLOR, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWA_USE_IMMERSIVE_DARK_MODE,
        };
        use windows::Win32::UI::WindowsAndMessaging::{
            CreateIconIndirect, GetCursorPos, LoadImageW, SendMessageW, SetClassLongPtrW,
            SetWindowPos, GCLP_HICON, GCLP_HICONSM, HICON, HWND_TOPMOST, ICONINFO, IMAGE_ICON,
            LR_DEFAULTSIZE, LR_LOADFROMFILE, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, WM_SETICON,
        };
        use windows::core::PCWSTR;

        #[derive(Clone, Copy, Debug, Default)]
        pub struct TaskbarInsets {
            pub bottom: f64,
            pub left: f64,
            pub right: f64,
        }

        pub fn set_window_icon(hwnd_isize: isize, tray_rgba: &[u8]) {
            unsafe {
                let hwnd = HWND(hwnd_isize as *mut _);
                let mut loaded_hicon: Option<HICON> = None;
                let candidate_rel_paths = [
                    "assets/icon.ico",
                    "../assets/icon.ico",
                    "../../assets/icon.ico",
                ];
                if let Ok(mut dir) = std::env::current_exe() {
                    for _ in 0..4 {
                        dir.pop();
                        for rel in &candidate_rel_paths {
                            let p = dir.join(rel);
                            if p.exists() {
                                let wpath: Vec<u16> = p.to_string_lossy().encode_utf16().chain(Some(0)).collect();
                                if let Ok(handle) = LoadImageW(
                                    None,
                                    PCWSTR(wpath.as_ptr()),
                                    IMAGE_ICON,
                                    0,
                                    0,
                                    LR_LOADFROMFILE | LR_DEFAULTSIZE,
                                ) {
                                    if !handle.0.is_null() {
                                        loaded_hicon = Some(HICON(handle.0));
                                        break;
                                    }
                                }
                            }
                        }
                        if loaded_hicon.is_some() {
                            break;
                        }
                    }
                }

                let hicon = if let Some(h) = loaded_hicon {
                    h
                } else {
                    let hdc = CreateCompatibleDC(None);
                    let bmi = BITMAPINFO {
                        bmiHeader: BITMAPINFOHEADER {
                            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                            biWidth: 32,
                            biHeight: -32,
                            biPlanes: 1,
                            biBitCount: 32,
                            biCompression: BI_RGB.0,
                            ..Default::default()
                        },
                        ..Default::default()
                    };
                    let mut bits_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
                    if let Ok(dib) = CreateDIBSection(Some(hdc), &bmi, DIB_RGB_COLORS, &mut bits_ptr, None, 0) {
                        if !bits_ptr.is_null() && tray_rgba.len() >= 32 * 32 * 4 {
                            let dst = std::slice::from_raw_parts_mut(bits_ptr as *mut u8, 32 * 32 * 4);
                            for i in 0..1024 {
                                let o = i * 4;
                                dst[o] = tray_rgba[o + 2];
                                dst[o + 1] = tray_rgba[o + 1];
                                dst[o + 2] = tray_rgba[o];
                                dst[o + 3] = tray_rgba[o + 3];
                            }
                        }
                        let hmask = CreateCompatibleBitmap(hdc, 32, 32);
                        let ii = ICONINFO {
                            fIcon: true.into(),
                            xHotspot: 0,
                            yHotspot: 0,
                            hbmMask: hmask,
                            hbmColor: dib,
                        };
                        let icon = CreateIconIndirect(&ii).unwrap_or(HICON(std::ptr::null_mut()));
                        let _ = DeleteObject(hmask.into());
                        let _ = DeleteObject(dib.into());
                        let _ = DeleteDC(hdc);
                        icon
                    } else {
                        let _ = DeleteDC(hdc);
                        HICON(std::ptr::null_mut())
                    }
                };

                if !hicon.0.is_null() {
                    let _ = SendMessageW(hwnd, WM_SETICON, Some(WPARAM(0)), Some(LPARAM(hicon.0 as isize)));
                    let _ = SendMessageW(hwnd, WM_SETICON, Some(WPARAM(1)), Some(LPARAM(hicon.0 as isize)));
                    let _ = SetClassLongPtrW(hwnd, GCLP_HICON, hicon.0 as isize);
                    let _ = SetClassLongPtrW(hwnd, GCLP_HICONSM, hicon.0 as isize);
                }
            }
        }

        pub fn apply_settings_window_style(hwnd_isize: isize, is_dark: bool) {
            unsafe {
                let hwnd = HWND(hwnd_isize as *mut _);
                let corner_pref: u32 = 2; // DWMWCP_ROUND (Windows 11 rounded corners)
                let _ = DwmSetWindowAttribute(
                    hwnd,
                    DWMWA_WINDOW_CORNER_PREFERENCE,
                    &corner_pref as *const _ as *const _,
                    std::mem::size_of::<u32>() as u32,
                );
                let dark_val: i32 = if is_dark { 1 } else { 0 };
                let _ = DwmSetWindowAttribute(
                    hwnd,
                    DWMWA_USE_IMMERSIVE_DARK_MODE,
                    &dark_val as *const _ as *const _,
                    std::mem::size_of::<i32>() as u32,
                );
            }
        }

        pub fn strip_window_decorations(hwnd_isize: isize) {
            unsafe {
                let hwnd = HWND(hwnd_isize as *mut _);
                let corner_pref: u32 = 1; // DWMWCP_DONOTROUND
                let _ = DwmSetWindowAttribute(
                    hwnd,
                    DWMWA_WINDOW_CORNER_PREFERENCE,
                    &corner_pref as *const _ as *const _,
                    std::mem::size_of::<u32>() as u32,
                );
                let border_color: u32 = 0xFFFFFFFE; // DWMWA_COLOR_NONE
                let _ = DwmSetWindowAttribute(
                    hwnd,
                    DWMWA_BORDER_COLOR,
                    &border_color as *const _ as *const _,
                    std::mem::size_of::<u32>() as u32,
                );
            }
        }

        pub fn enforce_topmost(hwnd_isize: isize) {
            unsafe {
                let hwnd = HWND(hwnd_isize as *mut _);
                let _ = SetWindowPos(
                    hwnd,
                    Some(HWND_TOPMOST),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
        }

        pub fn query_taskbar_insets(hwnd_isize: isize, scale_factor: f64) -> TaskbarInsets {
            unsafe {
                let hwnd = HWND(hwnd_isize as *mut _);
                let mut mi = MONITORINFO {
                    cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                    ..Default::default()
                };
                let hmon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTOPRIMARY);
                if GetMonitorInfoW(hmon, &mut mi).as_bool() {
                    let rc_mon = mi.rcMonitor;
                    let rc_work = mi.rcWork;

                    let scale = if scale_factor > 0.1 { scale_factor } else { 1.0 };
                    let bottom = ((rc_mon.bottom - rc_work.bottom).max(0) as f64) / scale;
                    let left = ((rc_work.left - rc_mon.left).max(0) as f64) / scale;
                    let right = ((rc_mon.right - rc_work.right).max(0) as f64) / scale;

                    TaskbarInsets {
                        bottom,
                        left,
                        right,
                    }
                } else {
                    TaskbarInsets::default()
                }
            }
        }

        pub fn is_user_typing_globally() -> bool {
            false
        }

        pub fn get_cursor_state(hwnd_isize: isize, scale_factor: f64) -> Option<(f64, f64, f64, f64)> {
            unsafe {
                let hwnd = HWND(hwnd_isize as *mut _);
                let mut pt = POINT::default();
                if GetCursorPos(&mut pt).is_ok() {
                    let mut client_pt = pt;
                    let _ = ScreenToClient(hwnd, &mut client_pt);
                    let scale = if scale_factor > 0.1 { scale_factor } else { 1.0 };
                    let screen_x = (pt.x as f64) / scale;
                    let screen_y = (pt.y as f64) / scale;
                    let local_x = (client_pt.x as f64) / scale;
                    let local_y = (client_pt.y as f64) / scale;
                    Some((screen_x, screen_y, local_x, local_y))
                } else {
                    None
                }
            }
        }
    }

    pub fn run_cpu_2d() -> Result<(), Box<dyn std::error::Error>> {
        let mut state_watcher = NativeStateWatcher::new();
        let mut settings = state_watcher.load_settings();

        let event_loop = EventLoopBuilder::<AppEvent>::with_user_event().build();
        let proxy = event_loop.create_proxy();

        let (mon_width, mon_height, _mon_scale) =
            if let Some(monitor) = event_loop.primary_monitor() {
                let scale = monitor.scale_factor();
                let size = monitor.size().to_logical::<f64>(scale);
                (size.width.max(360.0), size.height.max(400.0), scale)
            } else {
                (1440.0, 900.0, 2.0)
            };

        #[cfg(target_os = "macos")]
        let init_dock = {
            let d = macos_window::query_dock_insets_global();
            (d.bottom, d.left, d.right)
        };
        #[cfg(target_os = "windows")]
        let init_dock = {
            let insets = windows_window::query_taskbar_insets(0, _mon_scale);
            (insets.bottom, insets.left, insets.right)
        };
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        let init_dock = (0.0_f64, 0.0_f64, 0.0_f64);

        let safe_dock_bottom = init_dock.0.max(0.0);
        let safe_dock_left = init_dock.1.max(0.0);
        let safe_dock_right = init_dock.2.max(0.0);

        let floor_baseline_y = if safe_dock_bottom <= 2.0 {
            (mon_height - COMPACT_WIN_H).max(0.0)
        } else {
            (mon_height - COMPACT_WIN_H - safe_dock_bottom).max(0.0)
        };

        let min_safe_x = safe_dock_left;
        let max_safe_x = (mon_width - safe_dock_right - COMPACT_WIN_W).max(min_safe_x);

        let default_x = match settings.pos_x {
            Some(x) => x.clamp(min_safe_x, max_safe_x),
            None => (max_safe_x - 24.0).max(min_safe_x),
        };
        let default_y = match settings.pos_y {
            Some(y) => {
                if !settings.gravity_enabled {
                    y.clamp(0.0, floor_baseline_y)
                } else {
                    y.min(floor_baseline_y).max(0.0)
                }
            }
            None => floor_baseline_y,
        };

        settings.pos_x = Some(default_x);
        settings.pos_y = Some(default_y);

        let tray_rgba = include_bytes!("../../assets/tray.rgba");
        let win_icon = tao::window::Icon::from_rgba(tray_rgba.to_vec(), 32, 32).ok();

        #[allow(unused_mut)]
        let mut win_builder = WindowBuilder::new()
            .with_title("Anti-Pet CPU 2D")
            .with_transparent(true)
            .with_decorations(false)
            .with_always_on_top(true)
            .with_visible_on_all_workspaces(true)
            .with_resizable(false)
            .with_inner_size(LogicalSize::new(COMPACT_WIN_W, COMPACT_WIN_H))
            .with_position(LogicalPosition::new(default_x, default_y));

        if let Some(ref icon) = win_icon {
            win_builder = win_builder.with_window_icon(Some(icon.clone()));
        }

        #[cfg(target_os = "macos")]
        {
            win_builder = win_builder
                .with_has_shadow(false)
                .with_titlebar_transparent(true)
                .with_title_hidden(true)
                .with_titlebar_hidden(true)
                .with_titlebar_buttons_hidden(true)
                .with_fullsize_content_view(true)
                .with_movable_by_window_background(false);
        }

        #[cfg(target_os = "windows")]
        {
            win_builder = win_builder
                .with_undecorated_shadow(false)
                .with_skip_taskbar(true);
        }

        let window = win_builder.build(&event_loop)?;
        #[cfg(target_os = "windows")]
        {
            windows_window::strip_window_decorations(window.hwnd());
            windows_window::enforce_topmost(window.hwnd());
        }
        let _ = window.set_ignore_cursor_events(true);

        let mut settings_win_builder = WindowBuilder::new()
            .with_title("偏好设置")
            .with_inner_size(LogicalSize::new(SETTINGS_WIN_W, SETTINGS_WIN_H))
            .with_resizable(false)
            .with_maximizable(false)
            .with_visible(false);

        if let Some(ref icon) = win_icon {
            settings_win_builder = settings_win_builder.with_window_icon(Some(icon.clone()));
        }

        #[cfg(target_os = "windows")]
        {
            settings_win_builder = settings_win_builder
                .with_decorations(false)
                .with_transparent(true);
        }

        #[cfg(target_os = "macos")]
        {
            settings_win_builder = settings_win_builder
                .with_has_shadow(true)
                .with_titlebar_transparent(true)
                .with_fullsize_content_view(true)
                .with_title_hidden(true)
                .with_traffic_light_inset(LogicalPosition::new(20.0, 20.0))
                .with_movable_by_window_background(false);
        }

        let settings_window = settings_win_builder.build(&event_loop)?;

        #[cfg(target_os = "windows")]
        {
            windows_window::set_window_icon(settings_window.hwnd(), tray_rgba);
            windows_window::set_window_icon(window.hwnd(), tray_rgba);
        }

        #[cfg(target_os = "macos")]
        let settings_surface = SoftwareSurface::new_macos_settings(
            settings_window.ns_window(),
            SETTINGS_WIN_W,
            SETTINGS_WIN_H,
        );
        #[cfg(target_os = "windows")]
        let settings_surface = SoftwareSurface::new_windows(settings_window.hwnd());
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        let settings_surface = SoftwareSurface::new_unix();

        let mut settings_pixmap = Pixmap::new(SETTINGS_WIN_W as u32, SETTINGS_WIN_H as u32);
        let mut settings_ui = SettingsUi::new(settings.clone());
        #[cfg(target_os = "windows")]
        windows_window::apply_settings_window_style(settings_window.hwnd(), settings_ui.is_dark_mode());
        let mut is_settings_open = false;

        let mut pending_settings_save: Option<Instant> = None;


        #[cfg(target_os = "macos")]
        let surface = {
            macos_window::set_accessory_activation_policy();
            macos_window::enforce_topmost(window.ns_window());
            SoftwareSurface::new_macos(window.ns_window())
        };

        #[cfg(target_os = "windows")]
        let surface = {
            windows_window::enforce_topmost(window.hwnd());
            SoftwareSurface::new_windows(window.hwnd())
        };

        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        let surface = SoftwareSurface::new_unix();

        // Initialize CPU 2D software framebuffer, sprite, physics, bubble, and hit-tester
        let dpi_scale = window.scale_factor() as f32;
        let phys_w = ((COMPACT_WIN_W as f32) * dpi_scale).round() as u32;
        let phys_h = ((COMPACT_WIN_H as f32) * dpi_scale).round() as u32;
        let mut pixmap = Pixmap::new(phys_w, phys_h);

        let custom_skin_path = state_watcher
            .root_dir()
            .join("skins/fat-cat/frames/idle.png");
        let cat_sprite = CatSprite::load_from_path(&custom_skin_path);
        settings_ui.cat_sprite = cat_sprite.clone();

        let mut physics = PetPhysics::new(default_x, default_y);
        physics.zoom_scale = settings.scale;
        physics.gravity_enabled = settings.gravity_enabled;

        let initial_quota = crate::quota::read_state_file_quota()
            .or_else(crate::quota::read_fallback_ledger_quota);
        let mut bubble = ThoughtBubble {
            visible: !settings.dnd_passthrough,
            text: pick_random_line(IDLE_DIALOGUES).to_string(),
            show_quota: settings.show_quota,
            five_hour_pct: initial_quota.as_ref().map(|q| q.five_hour).unwrap_or(100.0),
            weekly_pct: initial_quota.as_ref().map(|q| q.weekly).unwrap_or(100.0),
            quota_meta: initial_quota
                .map(|q| q.meta)
                .unwrap_or_else(|| "正在同步 Antigravity 额度...".to_string()),
            opacity: if settings.dnd_passthrough { 0.0 } else { 1.0 },
            is_dark: is_dark_mode_active(&settings.theme),
        };
        let mut bubble_hide_deadline: Option<Instant> =
            Some(Instant::now() + Duration::from_millis(5000));

        let mut hit_tester = PixelHitTester::default();
        hit_tester.cat_rect = Some(physics.cat_bounding_rect());
        let mut last_cursor_local = (150.0_f64, 320.0_f64);
        let mut audio = NativeAudioPlayer::new(settings.sound_enabled);
        let mut menu_mgr = NativeMenuManager::new(&settings);

        let tray_icon = TrayIcon::from_rgba(tray_rgba.to_vec(), 32, 32).ok();
        let mut tray_builder = TrayIconBuilder::new()
            .with_tooltip("大肥猫桌宠 v0.4.0 (Pure CPU 2D)")
            .with_menu(Box::new(menu_mgr.menu.clone()));
        if let Some(icon) = tray_icon {
            tray_builder = tray_builder.with_icon(icon);
        }
        let _tray = tray_builder.build()?;

        // 1. High-precision 60 FPS physics and frame ticker (strictly never blocked)
        let ticker_proxy = proxy.clone();
        let force_quota_flag = Arc::new(AtomicBool::new(false));
        let force_quota_bg = force_quota_flag.clone();
        thread::spawn(move || {
            let mut hook_watcher = NativeStateWatcher::new();
            let mut tick_count: u64 = 0;
            let frame_duration = Duration::from_micros(16_667); // 60.0 FPS
            let mut next_tick = Instant::now();

            loop {
                let now = Instant::now();
                if now < next_tick {
                    thread::sleep(next_tick - now);
                }
                next_tick += frame_duration;
                if next_tick < Instant::now() {
                    next_tick = Instant::now() + frame_duration;
                }
                tick_count = tick_count.wrapping_add(1);

                if ticker_proxy.send_event(AppEvent::TickFrame).is_err() {
                    break;
                }

                // Every ~250ms (15 frames at 60 FPS): check if Antigravity hooks updated `.antigravity-pet-state.json`
                if tick_count % 15 == 0 {
                    if let Some(update) = hook_watcher.poll_state_change() {
                        let _ = ticker_proxy.send_event(AppEvent::HookStateChanged(update));
                        force_quota_bg.store(true, Ordering::Relaxed);
                    }
                }
            }
        });

        // 2. Dedicated asynchronous quota probe worker (eliminates startup GUI freeze)
        let quota_proxy = proxy.clone();
        let force_quota_worker = force_quota_flag.clone();
        thread::spawn(move || {
            let mut prober = NativeQuotaProber::default();
            // Stagger initial probe by 600ms so window is fully mounted, placed on taskbar and smooth first
            thread::sleep(Duration::from_millis(600));
            loop {
                let snap = prober.fetch_live_quota();
                if quota_proxy.send_event(AppEvent::QuotaUpdated(snap)).is_err() {
                    break;
                }

                // Sleep up to 12s, or wake early if user explicitly refreshed quota or hook triggered
                for _ in 0..120 {
                    thread::sleep(Duration::from_millis(100));
                    if force_quota_worker.swap(false, Ordering::Relaxed) {
                        break;
                    }
                }
            }
        });

        let mut is_window_visible = true;
        let mut currently_ignoring_cursor = true;
        let mut pointer_down = false;
        let mut is_dragging = false;
        let mut drag_last_screen = (0.0_f64, 0.0_f64);
        let mut drag_total_dist = 0.0_f64;
        let mut last_dock = (0.0_f64, 0.0_f64, 0.0_f64);
        let mut last_over_cat = false;
        let mut cursor_left_since: Option<Instant> = None;
        let mut dirty_frame = true;

        println!("Anti-Pet v0.4.0 Pure CPU 2D 软渲染引擎已启动 (Zero WebKit / Zero Node.js)");

        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;

            match event {
                Event::NewEvents(StartCause::Init) => {
                    #[cfg(target_os = "macos")]
                    macos_window::enforce_topmost(window.ns_window());
                    #[cfg(target_os = "windows")]
                    windows_window::enforce_topmost(window.hwnd());
                    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
                    window.set_always_on_top(true);

                    window.set_visible_on_all_workspaces(true);
                    dirty_frame = true;
                }

                Event::UserEvent(app_evt) => match app_evt {
                    AppEvent::QuotaUpdated(snap) => {
                        if (bubble.five_hour_pct - snap.five_hour).abs() > 0.1
                            || (bubble.weekly_pct - snap.weekly).abs() > 0.1
                            || bubble.quota_meta != snap.meta
                        {
                            bubble.five_hour_pct = snap.five_hour;
                            bubble.weekly_pct = snap.weekly;
                            bubble.quota_meta = snap.meta;
                            dirty_frame = true;
                        }
                    }

                    AppEvent::HookStateChanged(upd) => {
                        physics.motion = upd.motion;
                        physics.motion_until =
                            Some(Instant::now() + Duration::from_millis(upd.duration_ms.max(1800)));
                        if let Some(q) = upd.embedded_quota {
                            bubble.five_hour_pct = q.five_hour;
                            bubble.weekly_pct = q.weekly;
                            bubble.quota_meta = q.meta;
                        }
                        if !settings.dnd_passthrough {
                            bubble.text = upd.bubble_text.unwrap_or_else(|| {
                                match upd.motion {
                                    PetMotionState::Thinking => pick_random_line(THINKING_DIALOGUES),
                                    PetMotionState::Success => pick_random_line(SUCCESS_DIALOGUES),
                                    PetMotionState::Error => pick_random_line(ERROR_DIALOGUES),
                                    _ => pick_random_line(IDLE_DIALOGUES),
                                }
                                .to_string()
                            });
                            bubble.visible = true;
                            bubble.opacity = 1.0;
                            bubble_hide_deadline = Some(
                                Instant::now() + Duration::from_millis(upd.duration_ms.max(4000)),
                            );
                        }
                        dirty_frame = true;
                    }

                    AppEvent::CloseSettings => {
                        settings_window.set_visible(false);
                        is_settings_open = false;
                        let fresh = state_watcher.load_settings();
                        if (fresh.scale - settings.scale).abs() >= 0.01 {
                            physics.zoom_scale = fresh.scale;
                        }
                        physics.gravity_enabled = fresh.gravity_enabled;
                        audio.enabled = fresh.sound_enabled;
                        bubble.show_quota = fresh.show_quota;
                        menu_mgr.rebuild_menu(&fresh);
                        settings = fresh;
                        dirty_frame = true;
                    }

                    AppEvent::ReloadSettings => {
                        let fresh = state_watcher.load_settings();
                        if (fresh.scale - settings.scale).abs() >= 0.01 {
                            physics.zoom_scale = fresh.scale;
                        }
                        physics.gravity_enabled = fresh.gravity_enabled;
                        audio.enabled = fresh.sound_enabled;
                        bubble.show_quota = fresh.show_quota;
                        menu_mgr.rebuild_menu(&fresh);
                        settings = fresh;
                        dirty_frame = true;
                    }

                    AppEvent::ResetPosition => {
                        let center_x = ((mon_width - COMPACT_WIN_W) * 0.5).max(0.0);
                        let center_y = ((mon_height - COMPACT_WIN_H) * 0.5).max(0.0);

                        physics.win_x = center_x;
                        physics.win_y = center_y;
                        physics.target_floor_y = center_y;
                        physics.falling = false;
                        physics.vx = 0.0;
                        physics.vy = 0.0;
                        physics.edge_dock = None;
                        physics.trigger_duang_rebound(420);

                        window.set_outer_position(LogicalPosition::new(center_x, center_y));
                        #[cfg(target_os = "macos")]
                        macos_window::enforce_topmost(window.ns_window());

                        settings.pos_x = Some(center_x);
                        settings.pos_y = Some(center_y);
                        state_watcher.save_settings(&settings);

                        audio.play(SoundEffect::PopRelease);
                        bubble.text = "已将本喵复位至屏幕中央啦~".to_string();
                        bubble.visible = true;
                        bubble.opacity = 1.0;
                        bubble_hide_deadline = Some(Instant::now() + Duration::from_millis(3500));
                        dirty_frame = true;
                    }

                    AppEvent::TickFrame => {


                        // 1. Drain native context/tray menu events
                        while let Ok(menu_evt) = MenuEvent::receiver().try_recv() {
                            hit_tester.is_menu_open = false;
                            if let Some(action) = menu_mgr.resolve_event(&menu_evt.id) {
                                match action {
                                    PetMenuAction::OpenSettings => {
                                        audio.play(SoundEffect::ClickRelease);
                                        bubble.text = "已为你打开偏好设置~".to_string();
                                        bubble.visible = !settings.dnd_passthrough;
                                        bubble.opacity = 1.0;
                                        bubble_hide_deadline =
                                            Some(Instant::now() + Duration::from_millis(3500));
                                        settings_ui.open(&settings);
                                        is_settings_open = true;
                                        settings_window.set_visible(true);
                                        settings_window.set_focus();
                                        #[cfg(target_os = "macos")]
                                        macos_window::show_settings_window(settings_window.ns_window());
                                        let s_scale = settings_window.scale_factor() as f32;
                                        settings_ui.render(&mut settings_pixmap, s_scale);
                                        settings_surface.present(&settings_pixmap, s_scale as f64);
                                        dirty_frame = true;
                                    }

                                    PetMenuAction::ResetPosition => {
                                        let _ = proxy.send_event(AppEvent::ResetPosition);
                                    }

                                    PetMenuAction::NextDialogue => {
                                        audio.play(SoundEffect::ClickRelease);
                                        bubble.text = pick_random_line(IDLE_DIALOGUES).to_string();
                                        bubble.visible = true;
                                        bubble.opacity = 1.0;
                                        bubble_hide_deadline =
                                            Some(Instant::now() + Duration::from_millis(5000));
                                        physics.trigger_duang_rebound(420);
                                        dirty_frame = true;
                                    }
                                    PetMenuAction::PetHead => {
                                        audio.play(SoundEffect::PopRelease);
                                        physics.motion = PetMotionState::Petting;
                                        physics.motion_until =
                                            Some(Instant::now() + Duration::from_millis(2400));
                                        bubble.text =
                                            pick_random_line(PETTING_DIALOGUES).to_string();
                                        bubble.visible = !settings.dnd_passthrough;
                                        bubble.opacity = 1.0;
                                        bubble_hide_deadline =
                                            Some(Instant::now() + Duration::from_millis(4200));
                                        dirty_frame = true;
                                    }
                                    PetMenuAction::RefreshQuota => {
                                        force_quota_flag.store(true, Ordering::Relaxed);
                                        bubble.visible = !settings.dnd_passthrough;
                                        bubble.opacity = 1.0;
                                        bubble_hide_deadline =
                                            Some(Instant::now() + Duration::from_millis(4500));
                                        dirty_frame = true;
                                    }
                                    PetMenuAction::ToggleShowQuota => {
                                        settings.show_quota = !settings.show_quota;
                                        menu_mgr.item_show_quota.set_checked(settings.show_quota);
                                        bubble.show_quota = settings.show_quota;
                                        state_watcher.save_settings(&settings);
                                        dirty_frame = true;
                                    }
                                    PetMenuAction::ToggleDndMode => {
                                        settings.dnd_passthrough = !settings.dnd_passthrough;
                                        menu_mgr.item_dnd.set_checked(settings.dnd_passthrough);
                                        if settings.dnd_passthrough {
                                            bubble.visible = false;
                                        } else {
                                            bubble.text = "本喵回来啦！随时待命~".to_string();
                                            bubble.visible = true;
                                            bubble.opacity = 1.0;
                                            bubble_hide_deadline =
                                                Some(Instant::now() + Duration::from_millis(4000));
                                        }
                                        state_watcher.save_settings(&settings);
                                        dirty_frame = true;
                                    }
                                    PetMenuAction::ToggleGravity => {
                                        settings.gravity_enabled = !settings.gravity_enabled;
                                        physics.gravity_enabled = settings.gravity_enabled;
                                        menu_mgr.item_gravity.set_checked(settings.gravity_enabled);
                                        if settings.gravity_enabled {
                                            physics.on_drag_release(
                                                mon_width,
                                                mon_height,
                                                last_dock.0,
                                                last_dock.1,
                                                last_dock.2,
                                            );
                                        }
                                        state_watcher.save_settings(&settings);
                                        dirty_frame = true;
                                    }
                                    PetMenuAction::SetZoom(z) => {
                                        settings.scale = z;
                                        physics.zoom_scale = z;
                                        state_watcher.save_settings(&settings);
                                        dirty_frame = true;
                                    }
                                    PetMenuAction::ToggleSound => {
                                        settings.sound_enabled = !settings.sound_enabled;
                                        audio.enabled = settings.sound_enabled;
                                        menu_mgr.item_sound.set_checked(settings.sound_enabled);
                                        state_watcher.save_settings(&settings);
                                    }
                                    PetMenuAction::ToggleVisibility => {
                                        is_window_visible = !is_window_visible;
                                        window.set_visible(is_window_visible);
                                    }
                                    PetMenuAction::OpenAntigravity => {
                                        #[cfg(target_os = "macos")]
                                        {
                                            let _ = std::process::Command::new("open")
                                                .args(["-a", "Antigravity"])
                                                .spawn();
                                        }
                                        #[cfg(target_os = "windows")]
                                        {
                                            let _ = std::process::Command::new("cmd")
                                                .args(["/C", "start", "", "antigravity:"])
                                                .spawn();
                                        }
                                    }
                                    PetMenuAction::Quit => {
                                        *control_flow = ControlFlow::Exit;
                                        return;
                                    }
                                }
                            }
                        }

                        // 2. Drain system tray icon left-click toggle
                        while let Ok(tray_evt) = TrayIconEvent::receiver().try_recv() {
                            if let TrayIconEvent::Click {
                                button: MouseButton::Left,
                                button_state: MouseButtonState::Up,
                                ..
                            } = tray_evt
                            {
                                is_window_visible = !is_window_visible;
                                window.set_visible(is_window_visible);
                            }
                        }

                        // 2.1 Check if user changed settings from Settings window or file
                        if let Some(new_settings) = state_watcher.poll_settings_change() {
                            if (new_settings.scale - settings.scale).abs() >= 0.01 {
                                physics.zoom_scale = new_settings.scale;
                            }
                            physics.gravity_enabled = new_settings.gravity_enabled;
                            audio.enabled = new_settings.sound_enabled;
                            bubble.show_quota = new_settings.show_quota;

                            let should_reset = (new_settings.reset_seq.is_some()
                                && new_settings.reset_seq != settings.reset_seq)
                                || (new_settings.pos_x.is_none()
                                    && new_settings.pos_y.is_none()
                                    && (settings.pos_x.is_some() || settings.pos_y.is_some()));

                            if should_reset {
                                let _ = proxy.send_event(AppEvent::ResetPosition);
                            }

                            menu_mgr.rebuild_menu(&new_settings);
                            settings = new_settings;
                            dirty_frame = true;
                        }

                        // 2.2 Debounced settings persistence
                        if let Some(save_at) = pending_settings_save {
                            if Instant::now() >= save_at {
                                pending_settings_save = None;
                                state_watcher.save_settings(&settings);
                            }
                        }

                        if !is_window_visible {
                            return;
                        }


                        // 3. Query macOS Dock insets, global typing state, and cursor position
                        #[cfg(target_os = "macos")]
                        let (dock, is_typing, cursor_state) = {
                            let d = macos_window::query_dock_insets_global();
                            let typing = macos_window::is_user_typing_globally();
                            let cur = macos_window::get_cursor_state(window.ns_window(), mon_height);
                            ((d.bottom, d.left, d.right), typing, cur)
                        };

                        #[cfg(target_os = "windows")]
                        let (dock, is_typing, cursor_state) = {
                            let insets = windows_window::query_taskbar_insets(window.hwnd(), window.scale_factor());
                            let typing = windows_window::is_user_typing_globally();
                            let cur = windows_window::get_cursor_state(window.hwnd(), window.scale_factor());
                            ((insets.bottom, insets.left, insets.right), typing, cur)
                        };

                        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
                        let (dock, is_typing, cursor_state) =
                            ((0.0, 0.0, 0.0), false, None::<(f64, f64, f64, f64)>);

                        last_dock = dock;

                        // Auto-hide thought bubble immediately when user starts typing
                        if is_typing && bubble.visible {
                            bubble.visible = false;
                            dirty_frame = true;
                        }

                        // Auto-hide thought bubble when deadline expires and cursor isn't hovering
                        if let Some(deadline) = bubble_hide_deadline {
                            if Instant::now() >= deadline && !last_over_cat && bubble.visible {
                                bubble.visible = false;
                                bubble_hide_deadline = None;
                                dirty_frame = true;
                            }
                        }

                        // 4. Cursor tracking, drag handling & < 1μs pixel-level alpha hit-testing
                        if let Some((screen_x, screen_y, local_x, local_y)) = cursor_state {
                            last_cursor_local = (local_x, local_y);
                            if pointer_down {
                                let dx = screen_x - drag_last_screen.0;
                                let dy = screen_y - drag_last_screen.1;
                                let step_dist = dx.hypot(dy);
                                drag_total_dist += step_dist;

                                if drag_total_dist > 5.0 {
                                    if !is_dragging {
                                        is_dragging = true;
                                        if !settings.dnd_passthrough {
                                            bubble.text =
                                                pick_random_line(DRAGGED_DIALOGUES).to_string();
                                            bubble.visible = true;
                                            bubble.opacity = 1.0;
                                        }
                                    }
                                    if step_dist > 0.5 {
                                        drag_last_screen = (screen_x, screen_y);
                                        physics.on_drag_move(dx, dy, mon_width, mon_height, last_dock.0);
                                        window.set_outer_position(LogicalPosition::new(
                                            physics.win_x,
                                            physics.win_y,
                                        ));
                                        dirty_frame = true;
                                    }
                                }
                            } else {
                                let scale_f64 = window.scale_factor();
                                let over_interactive = hit_tester.is_interactive_at(
                                    &pixmap,
                                    local_x,
                                    local_y,
                                    scale_f64,
                                    is_dragging,
                                    settings.dnd_passthrough,
                                );

                                let should_ignore = !over_interactive;
                                if currently_ignoring_cursor != should_ignore {
                                    currently_ignoring_cursor = should_ignore;
                                    let _ = window.set_ignore_cursor_events(should_ignore);
                                }

                                if over_interactive {
                                    cursor_left_since = None;
                                    if !last_over_cat && !settings.dnd_passthrough && !is_typing {
                                        bubble.visible = true;
                                        bubble.opacity = 1.0;
                                        bubble_hide_deadline =
                                            Some(Instant::now() + Duration::from_millis(4500));
                                        dirty_frame = true;
                                    }
                                } else if last_over_cat {
                                    cursor_left_since = Some(Instant::now());
                                }

                                if let Some(left_at) = cursor_left_since {
                                    if left_at.elapsed() >= Duration::from_millis(1600)
                                        && bubble.visible
                                    {
                                        bubble.visible = false;
                                        cursor_left_since = None;
                                        dirty_frame = true;
                                    }
                                }

                                last_over_cat = over_interactive;

                                if physics.on_cursor_moved(local_x, local_y, over_interactive) {
                                    // Petting gesture triggered!
                                    audio.play(SoundEffect::PopRelease);
                                    if !settings.dnd_passthrough {
                                        bubble.text =
                                            pick_random_line(PETTING_DIALOGUES).to_string();
                                        bubble.visible = true;
                                        bubble.opacity = 1.0;
                                        bubble_hide_deadline =
                                            Some(Instant::now() + Duration::from_millis(3800));
                                    }
                                    dirty_frame = true;
                                }
                            }
                        }

                        // 5. Step physics engine & redraw software framebuffer if needed
                        let (win_moved, _anim_active) =
                            physics.step_frame(mon_width, dock.1, dock.2);
                        if win_moved {
                            window.set_outer_position(LogicalPosition::new(
                                physics.win_x,
                                physics.win_y,
                            ));
                            settings.pos_x = Some(physics.win_x);
                            settings.pos_y = Some(physics.win_y);
                        }

                        // Redraw at 60 FPS whenever window is visible: fluid breathing, smooth gaze, zero stutter
                        if is_window_visible {
                            let _ = dirty_frame;
                            dirty_frame = false;
                            let scale = window.scale_factor() as f32;
                            let pw = ((COMPACT_WIN_W as f32) * scale).round() as u32;
                            let ph = ((COMPACT_WIN_H as f32) * scale).round() as u32;
                            pixmap.resize(pw, ph);
                            pixmap.clear();

                            let tf = physics.current_sprite_transform(settings.dnd_passthrough);
                            let (_, min_py, _, _) = cat_sprite.draw(&mut pixmap, &tf, scale);
                            let cat_top_logical_y = (min_py as f32) / scale.max(1.0);

                            bubble.is_dark = is_dark_mode_active(&settings.theme);
                            hit_tester.cat_rect = Some(physics.cat_bounding_rect());
                            hit_tester.bubble_rect = bubble.draw(
                                &mut pixmap,
                                tf.anchor_x,
                                cat_top_logical_y,
                                physics.win_x,
                                physics.win_y,
                                mon_width,
                                scale,
                            );

                            surface.present(&pixmap, scale as f64);
                        }

                        if is_settings_open && settings_ui.toast.is_some() {
                            let s_scale = settings_window.scale_factor() as f32;
                            settings_ui.render(&mut settings_pixmap, s_scale);
                            settings_surface.present(&settings_pixmap, s_scale as f64);
                        }
                    }
                },

                Event::WindowEvent { window_id, event, .. } => {
                    if window_id == settings_window.id() {
                        match event {
                            WindowEvent::CloseRequested => {
                                settings_window.set_visible(false);
                                is_settings_open = false;
                            }
                            WindowEvent::CursorMoved { position, .. } => {
                                let s_scale = settings_window.scale_factor() as f32;
                                let lx = position.x / (s_scale as f64);
                                let ly = position.y / (s_scale as f64);
                                if settings_ui.handle_cursor_move(lx, ly, s_scale) {
                                    if settings_ui.dragging_slider.is_some() {
                                        physics.zoom_scale = settings_ui.temp_settings.scale;
                                        dirty_frame = true;
                                    }
                                    settings_ui.render(&mut settings_pixmap, s_scale);
                                    settings_surface.present(&settings_pixmap, s_scale as f64);
                                }
                            }
                            WindowEvent::MouseInput {
                                state,
                                button: TaoMouseButton::Left,
                                ..
                            } => {
                                let s_scale = settings_window.scale_factor() as f32;
                                if state == ElementState::Pressed {
                                    if let Some(action) = settings_ui.handle_mouse_down(s_scale) {
                                        match action {
                                            SettingsAction::Save(new_settings) => {
                                                state_watcher.save_settings(&new_settings);
                                                settings = new_settings;
                                                physics.zoom_scale = settings.scale;
                                                physics.gravity_enabled = settings.gravity_enabled;
                                                audio.enabled = settings.sound_enabled;
                                                bubble.show_quota = settings.show_quota;
                                                menu_mgr.rebuild_menu(&settings);
                                                dirty_frame = true;
                                                settings_ui.render(&mut settings_pixmap, s_scale);
                                                settings_surface.present(&settings_pixmap, s_scale as f64);
                                            }
                                            SettingsAction::Cancel => {
                                                settings_window.set_visible(false);
                                                is_settings_open = false;
                                            }
                                            SettingsAction::MinimizeWindow => {
                                                settings_window.set_minimized(true);
                                            }
                                            SettingsAction::ResetPosition => {
                                                let _ = proxy.send_event(AppEvent::ResetPosition);
                                                settings_ui.show_toast("已将桌宠复位至屏幕中央");
                                                settings_ui.render(&mut settings_pixmap, s_scale);
                                                settings_surface.present(&settings_pixmap, s_scale as f64);
                                            }
                                            SettingsAction::CheckUpdate => {
                                                settings_ui.show_toast("当前已是最新版本 (v0.4.0)");
                                                settings_ui.render(&mut settings_pixmap, s_scale);
                                                settings_surface.present(&settings_pixmap, s_scale as f64);
                                            }
                                            SettingsAction::SelectTab(_) | SettingsAction::Redraw => {
                                                settings_ui.render(&mut settings_pixmap, s_scale);
                                                settings_surface.present(&settings_pixmap, s_scale as f64);
                                            }
                                            SettingsAction::DragWindow => {
                                                let _ = settings_window.drag_window();
                                            }
                                            SettingsAction::None => {}
                                        }
                                    }
                                } else if let Some(action) = settings_ui.handle_mouse_up() {
                                    match action {
                                        SettingsAction::Save(new_settings) => {
                                            state_watcher.save_settings(&new_settings);
                                            settings = new_settings;
                                            physics.zoom_scale = settings.scale;
                                            physics.gravity_enabled = settings.gravity_enabled;
                                            audio.enabled = settings.sound_enabled;
                                            bubble.show_quota = settings.show_quota;
                                            menu_mgr.rebuild_menu(&settings);
                                            dirty_frame = true;
                                            settings_ui.render(&mut settings_pixmap, s_scale);
                                            settings_surface.present(&settings_pixmap, s_scale as f64);
                                        }
                                        _ => {
                                            settings_ui.render(&mut settings_pixmap, s_scale);
                                            settings_surface.present(&settings_pixmap, s_scale as f64);
                                        }
                                    }
                                }
                            }
                            WindowEvent::MouseWheel { delta, .. } => {
                                let dy = match delta {
                                    tao::event::MouseScrollDelta::LineDelta(_, y) => y * 28.0,
                                    tao::event::MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
                                    _ => 0.0,
                                };
                                let s_scale = settings_window.scale_factor() as f32;
                                if settings_ui.handle_mouse_wheel(dy) {
                                    settings_ui.render(&mut settings_pixmap, s_scale);
                                    settings_surface.present(&settings_pixmap, s_scale as f64);
                                }
                            }
                            _ => {}
                        }
                        return;
                    }

                    match event {
                        WindowEvent::CloseRequested => {
                            if window_id == window.id() {
                                is_window_visible = false;
                                window.set_visible(false);
                            }
                        }

                    WindowEvent::Focused(false) => {
                        hit_tester.is_menu_open = false;
                        if window_id == window.id() && bubble.visible && !last_over_cat {
                            bubble.visible = false;
                            dirty_frame = true;
                        }
                    }

                    WindowEvent::MouseWheel { delta, .. } => {
                        if window_id != window.id() || settings.dnd_passthrough {
                            return;
                        }
                        if last_over_cat {
                            let delta_y = match delta {
                                tao::event::MouseScrollDelta::LineDelta(_, y) => (y as f64) * 16.0,
                                tao::event::MouseScrollDelta::PixelDelta(pos) => pos.y,
                                _ => 0.0,
                            };
                            if delta_y.abs() > 0.1 {
                                // In Cocoa/Tao on macOS, positive delta_y is scrolling UP (natural zoom IN)
                                let zoom_factor = (delta_y * 0.002).exp() as f32;
                                let new_scale = (physics.zoom_scale * zoom_factor).clamp(0.50, 2.00);
                                let rounded_scale = (new_scale * 100.0).round() / 100.0;
                                if (rounded_scale - physics.zoom_scale).abs() >= 0.01 {
                                    physics.zoom_scale = rounded_scale;
                                    settings.scale = rounded_scale;
                                    bubble.text = format!("体型缩放: {:.0}%", rounded_scale * 100.0);
                                    bubble.visible = true;
                                    bubble.opacity = 1.0;
                                    bubble_hide_deadline =
                                        Some(Instant::now() + Duration::from_millis(2400));
                                    pending_settings_save =
                                        Some(Instant::now() + Duration::from_millis(400));
                                    dirty_frame = true;
                                }
                            }
                        }
                    }

                    WindowEvent::MouseInput { state, button, .. } => {
                        if window_id != window.id() {
                            return;
                        }
                        match button {
                            TaoMouseButton::Right => {
                                if state == ElementState::Pressed {
                                    pointer_down = false;
                                    is_dragging = false;
                                    audio.play(SoundEffect::ClickPress);

                                    // Always reload fresh settings before rebuilding context menu
                                    settings = state_watcher.load_settings();
                                    menu_mgr.rebuild_menu(&settings);
                                    hit_tester.is_menu_open = true;
                                    currently_ignoring_cursor = false;
                                    let _ = window.set_ignore_cursor_events(false);

                                    #[cfg(target_os = "macos")]
                                    {
                                        macos_window::activate_ignoring_other_apps(window.ns_window());
                                        let pos = muda::dpi::LogicalPosition::new(
                                            last_cursor_local.0.clamp(10.0, (COMPACT_WIN_W - 10.0) as f64),
                                            last_cursor_local.1.clamp(10.0, (COMPACT_WIN_H - 10.0) as f64),
                                        );
                                        menu_mgr.show_context_menu_macos(window.ns_view(), Some(pos));
                                        hit_tester.is_menu_open = false;
                                    }
                                    #[cfg(target_os = "windows")]
                                    {
                                        let pos = muda::dpi::LogicalPosition::new(
                                            last_cursor_local.0.clamp(10.0, (COMPACT_WIN_W - 10.0) as f64),
                                            last_cursor_local.1.clamp(10.0, (COMPACT_WIN_H - 10.0) as f64),
                                        );
                                        menu_mgr.show_context_menu_windows(window.hwnd(), Some(pos));
                                        hit_tester.is_menu_open = false;
                                    }
                                }
                            }

                            TaoMouseButton::Left => {
                                if settings.dnd_passthrough {
                                    // In DND mode, only right-click opens the context menu; left-click is ignored
                                    return;
                                }
                                if state == ElementState::Pressed {
                                    pointer_down = true;
                                    is_dragging = false;
                                    drag_total_dist = 0.0;
                                    #[cfg(target_os = "macos")]
                                    if let Some((sx, sy, _, _)) =
                                        macos_window::get_cursor_state(window.ns_window(), mon_height)
                                    {
                                        drag_last_screen = (sx, sy);
                                    }
                                    #[cfg(target_os = "windows")]
                                    if let Some((sx, sy, _, _)) =
                                        windows_window::get_cursor_state(window.hwnd(), window.scale_factor())
                                    {
                                        drag_last_screen = (sx, sy);
                                    }
                                    audio.play(SoundEffect::PopPress);
                                    physics.on_pointer_down();
                                    dirty_frame = true;
                                } else if pointer_down {
                                    pointer_down = false;
                                    if is_dragging {
                                        is_dragging = false;
                                        audio.play(SoundEffect::PopRelease);
                                        physics.on_drag_release(
                                            mon_width,
                                            mon_height,
                                            last_dock.0,
                                            last_dock.1,
                                            last_dock.2,
                                        );
                                        window.set_outer_position(LogicalPosition::new(
                                            physics.win_x,
                                            physics.win_y,
                                        ));
                                        #[cfg(target_os = "macos")]
                                        macos_window::enforce_topmost(window.ns_window());
                                        #[cfg(target_os = "windows")]
                                        windows_window::enforce_topmost(window.hwnd());

                                        settings.pos_x = Some(physics.win_x);
                                        settings.pos_y = Some(physics.win_y);
                                        state_watcher.save_settings(&settings);
                                    } else {
                                        // Short click -> Duang rebound + next tsundere dialogue!
                                        audio.play(SoundEffect::ClickRelease);
                                        physics.trigger_duang_rebound(460);
                                        bubble.text = pick_random_line(IDLE_DIALOGUES).to_string();
                                        bubble.visible = true;
                                        bubble.opacity = 1.0;
                                        bubble_hide_deadline =
                                            Some(Instant::now() + Duration::from_millis(5000));
                                    }
                                    dirty_frame = true;
                                }
                            }
                            _ => (),
                        }
                    }
                    _ => (),
                }
            }

            _ => (),
        }
        });
    }
}
