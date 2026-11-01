#[cfg(target_os = "macos")]
use std::ffi::c_void;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};
use muda::{Menu, MenuItem, PredefinedMenuItem, MenuEvent};
use tao::{
    dpi::{LogicalPosition, LogicalSize},
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::WindowBuilder,
};
#[cfg(not(target_os = "macos"))]
use tao::dpi::PhysicalPosition;
#[cfg(target_os = "macos")]
use tao::platform::macos::{WindowBuilderExtMacOS, WindowExtMacOS};
#[cfg(target_os = "windows")]
use tao::platform::windows::{WindowBuilderExtWindows, WindowExtWindows};
use tray_icon::{
    Icon as TrayIcon, MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent,
};
use wry::WebViewBuilder;

/// Interactive bounding rectangle in logical window coordinates
#[derive(Clone, Copy, Debug)]
struct HitRect {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

impl HitRect {
    fn contains(&self, px: f64, py: f64) -> bool {
        px >= self.x && px <= self.x + self.w && py >= self.y && py <= self.y + self.h
    }
}

/// Custom events sent from WebView IPC or background ticker to the main Window thread
#[derive(Debug, Clone)]
enum UserEvent {
    SetIgnoreCursorEvents(bool),
    UpdateHitRects(Vec<HitRect>),
    SetDragging(bool),
    SetDndPassthrough(bool, Option<HitRect>),
    SetAlwaysOnTop(bool),
    SetVisible(bool),
    DragWindow,
    TickCursor,
}

#[cfg(target_os = "macos")]
mod macos_native {
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
        // CoreGraphics global keyboard idle timer (no Accessibility permission required)
        fn CGEventSourceSecondsSinceLastEventType(state_id: i32, event_type: u32) -> f64;
    }

    /// Returns true if the user pressed or released a key anywhere in macOS within the last 1.2s
    pub fn is_user_typing_globally() -> bool {
        unsafe {
            // state_id = 0 (kCGEventSourceStateCombinedSessionState)
            // event_type = 10 (kCGEventKeyDown), 11 (kCGEventKeyUp)
            let down_secs = CGEventSourceSecondsSinceLastEventType(0, 10);
            let up_secs = CGEventSourceSecondsSinceLastEventType(0, 11);
            (down_secs >= 0.0 && down_secs < 1.2) || (up_secs >= 0.0 && up_secs < 1.2)
        }
    }

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

    /// Retrieve the cursor position in window logical coordinates directly from Cocoa.
    /// This bypasses Tao's cursor_position Retina pixel-height bug.
    pub fn get_mouse_location_in_window(ns_window: *mut c_void) -> Option<(f64, f64)> {
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
            Some((local_x, local_y))
        }
    }

    /// Returns true if Antigravity, VS Code, Cursor, or the Desktop Pet is the frontmost application.
    pub fn is_ide_frontmost() -> bool {
        unsafe {
            let ws_cls = objc_getClass(b"NSWorkspace\0".as_ptr());
            if ws_cls.is_null() {
                return false;
            }
            let msg_ptr: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const ());
            let shared_ws = msg_ptr(ws_cls, sel(b"sharedWorkspace\0"));
            if shared_ws.is_null() {
                return false;
            }
            let front_app = msg_ptr(shared_ws, sel(b"frontmostApplication\0"));
            if front_app.is_null() {
                return false;
            }
            let name_nsstring = msg_ptr(front_app, sel(b"localizedName\0"));
            if name_nsstring.is_null() {
                return false;
            }
            let utf8_ptr: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *const i8 =
                std::mem::transmute(objc_msgSend as *const ());
            let cstr = utf8_ptr(name_nsstring, sel(b"UTF8String\0"));
            if cstr.is_null() {
                return false;
            }
            let name = std::ffi::CStr::from_ptr(cstr).to_string_lossy();
            let name_lower = name.to_lowercase();
            name_lower.contains("antigravity")
                || name_lower.contains("code")
                || name_lower.contains("cursor")
                || name_lower.contains("anti-pet")
        }
    }

    /// Elevate NSWindow above macOS Dock & Fullscreen Spaces, force full-screen frame (0px bottom),
    /// and return current Mac Dock insets (bottom, left, right) in logical points.
    pub fn sync_fullscreen_overlay_and_query_dock(
        ns_window: *mut c_void,
        enforce_frame: bool,
    ) -> DockInsets {
        if ns_window.is_null() {
            return DockInsets::default();
        }
        unsafe {
            let set_level: unsafe extern "C" fn(*mut c_void, *mut c_void, isize) =
                std::mem::transmute(objc_msgSend as *const ());
            set_level(ns_window, sel(b"setLevel:\0"), 25);

            let behavior: usize = (1 << 0) | (1 << 4) | (1 << 6) | (1 << 8);
            let set_behavior: unsafe extern "C" fn(*mut c_void, *mut c_void, usize) =
                std::mem::transmute(objc_msgSend as *const ());
            set_behavior(ns_window, sel(b"setCollectionBehavior:\0"), behavior);

            let msg_ptr: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const ());
            let mut screen = msg_ptr(ns_window, sel(b"screen\0"));
            if screen.is_null() {
                let cls = objc_getClass(b"NSScreen\0".as_ptr());
                if !cls.is_null() {
                    screen = msg_ptr(cls, sel(b"mainScreen\0"));
                }
            }
            if screen.is_null() {
                return DockInsets::default();
            }

            let full_frame = msg_send_rect(screen, sel(b"frame\0"));
            let vis_frame = msg_send_rect(screen, sel(b"visibleFrame\0"));

            if enforce_frame && full_frame.size.width > 100.0 && full_frame.size.height > 100.0 {
                let cur_frame = msg_send_rect(ns_window, sel(b"frame\0"));
                if (cur_frame.origin.x - full_frame.origin.x).abs() > 0.5
                    || (cur_frame.origin.y - full_frame.origin.y).abs() > 0.5
                    || (cur_frame.size.width - full_frame.size.width).abs() > 0.5
                    || (cur_frame.size.height - full_frame.size.height).abs() > 0.5
                {
                    let set_frame: unsafe extern "C" fn(*mut c_void, *mut c_void, NSRect, i8) =
                        std::mem::transmute(objc_msgSend as *const ());
                    set_frame(ns_window, sel(b"setFrame:display:\0"), full_frame, 1);
                }
            }

            let app_cls = objc_getClass(b"NSApplication\0".as_ptr());
            let mut is_fullscreen_space = false;
            if !app_cls.is_null() {
                let shared_app = msg_ptr(app_cls, sel(b"sharedApplication\0"));
                if !shared_app.is_null() {
                    let msg_usize: unsafe extern "C" fn(*mut c_void, *mut c_void) -> usize =
                        std::mem::transmute(objc_msgSend as *const ());
                    let pres_opts =
                        msg_usize(shared_app, sel(b"currentSystemPresentationOptions\0"));
                    if (pres_opts & ((1 << 0) | (1 << 1) | (1 << 10))) != 0 {
                        is_fullscreen_space = true;
                    }
                }
            }

            let top_menu_inset = (full_frame.origin.y + full_frame.size.height)
                - (vis_frame.origin.y + vis_frame.size.height);
            if top_menu_inset < 1.0 {
                is_fullscreen_space = true;
            }

            if is_fullscreen_space {
                DockInsets::default()
            } else {
                let bottom = (vis_frame.origin.y - full_frame.origin.y).max(0.0);
                let left = (vis_frame.origin.x - full_frame.origin.x).max(0.0);
                let right = ((full_frame.origin.x + full_frame.size.width)
                    - (vis_frame.origin.x + vis_frame.size.width))
                .max(0.0);
                DockInsets {
                    bottom,
                    left,
                    right,
                }
            }
        }
    }

    /// Sets the macOS Dock application tile icon to the provided image bytes.
    pub fn set_application_dock_icon(png_bytes: &[u8]) {
        unsafe {
            let data_cls = objc_getClass(b"NSData\0".as_ptr());
            let img_cls = objc_getClass(b"NSImage\0".as_ptr());
            let app_cls = objc_getClass(b"NSApplication\0".as_ptr());
            if data_cls.is_null() || img_cls.is_null() || app_cls.is_null() {
                return;
            }

            let msg_data: unsafe extern "C" fn(*mut c_void, *mut c_void, *const u8, usize) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const ());
            let ns_data = msg_data(
                data_cls,
                sel(b"dataWithBytes:length:\0"),
                png_bytes.as_ptr(),
                png_bytes.len(),
            );
            if ns_data.is_null() {
                return;
            }

            let msg_ptr: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const ());
            let img_alloc = msg_ptr(img_cls, sel(b"alloc\0"));
            if img_alloc.is_null() {
                return;
            }

            let msg_init: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) -> *mut c_void =
                std::mem::transmute(objc_msgSend as *const ());
            let ns_img = msg_init(img_alloc, sel(b"initWithData:\0"), ns_data);
            if ns_img.is_null() {
                return;
            }

            let shared_app = msg_ptr(app_cls, sel(b"sharedApplication\0"));
            if shared_app.is_null() {
                return;
            }

            let msg_set_icon: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) =
                std::mem::transmute(objc_msgSend as *const ());
            msg_set_icon(shared_app, sel(b"setApplicationIconImage:\0"), ns_img);
        }
    }
}


#[cfg(target_os = "windows")]
mod windows_native {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Gdi::{
        GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTOPRIMARY,
    };
    use windows::Win32::Graphics::Dwm::{
        DwmSetWindowAttribute, DWMWA_BORDER_COLOR, DWMWA_WINDOW_CORNER_PREFERENCE,
    };

    #[derive(Clone, Copy, Debug, Default)]
    pub struct TaskbarInsets {
        pub bottom: f64,
        pub left: f64,
        pub right: f64,
    }

    pub fn strip_window_decorations(hwnd_isize: isize) {
        unsafe {
            let hwnd = HWND(hwnd_isize as *mut _);

            // 1. Disable Windows 11 rounded window corners (DWMWCP_DONOTROUND = 1)
            let corner_pref: u32 = 1;
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_WINDOW_CORNER_PREFERENCE,
                &corner_pref as *const _ as *const _,
                std::mem::size_of::<u32>() as u32,
            );

            // 2. Disable window border line (DWMWA_COLOR_NONE = 0xFFFFFFFE)
            let border_color: u32 = 0xFFFFFFFE;
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_BORDER_COLOR,
                &border_color as *const _ as *const _,
                std::mem::size_of::<u32>() as u32,
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
}

fn parse_ipc_command(msg: &str) -> Option<UserEvent> {
    let trimmed = msg.trim();
    if trimmed == "drag_window" {
        return Some(UserEvent::DragWindow);
    }
    if let Some(val) = trimmed.strip_prefix("set_ignore_cursor_events:") {
        return Some(UserEvent::SetIgnoreCursorEvents(val.trim() == "true"));
    }
    if let Some(val) = trimmed.strip_prefix("set_dragging:") {
        return Some(UserEvent::SetDragging(val.trim() == "true"));
    }
    if let Some(val) = trimmed.strip_prefix("set_always_on_top:") {
        return Some(UserEvent::SetAlwaysOnTop(val.trim() == "true"));
    }
    if let Some(val) = trimmed.strip_prefix("set_visible:") {
        return Some(UserEvent::SetVisible(val.trim() == "true"));
    }
    if let Some(rest) = trimmed.strip_prefix("set_dnd:") {
        let mut parts = rest.splitn(2, ':');
        let enabled = parts.next().unwrap_or("false").trim() == "true";
        let wake_rect = parts.next().and_then(parse_single_rect);
        return Some(UserEvent::SetDndPassthrough(enabled, wake_rect));
    }
    if let Some(rects_str) = trimmed.strip_prefix("hit_rects:") {
        let rects: Vec<HitRect> = rects_str
            .split(';')
            .filter(|s| !s.trim().is_empty())
            .filter_map(parse_single_rect)
            .collect();
        return Some(UserEvent::UpdateHitRects(rects));
    }
    None
}

fn parse_single_rect(s: &str) -> Option<HitRect> {
    let nums: Vec<f64> = s
        .split(',')
        .filter_map(|n| n.trim().parse::<f64>().ok())
        .collect();
    if nums.len() == 4 {
        Some(HitRect {
            x: nums[0],
            y: nums[1],
            w: nums[2],
            h: nums[3],
        })
    } else {
        None
    }
}

#[allow(dead_code)]
const APP_ICON_PNG: &[u8] = include_bytes!("../../assets/icon.png");

/// Ensures the local node server at 127.0.0.1:4199 is running.
/// If not responding, attempts to auto-spawn `node server/index.js`.
fn ensure_local_server_running() {
    use std::net::{SocketAddr, TcpStream};
    use std::path::PathBuf;
    use std::process::Command;

    let addr: SocketAddr = "127.0.0.1:4199".parse().unwrap();
    if TcpStream::connect_timeout(&addr, Duration::from_millis(150)).is_ok() {
        return;
    }

    let mut candidate_paths = vec![
        PathBuf::from("server/index.js"),
        PathBuf::from("../server/index.js"),
    ];

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            candidate_paths.push(exe_dir.join("server/index.js"));
            candidate_paths.push(exe_dir.join("../server/index.js"));
            candidate_paths.push(exe_dir.join("../../server/index.js"));
            candidate_paths.push(exe_dir.join("../Resources/server/index.js"));
        }
    }

    for path in candidate_paths {
        if path.exists() {
            let working_dir = path.parent().and_then(|p| p.parent()).unwrap_or(&path);
            let mut cmd = Command::new("node");
            cmd.arg(&path);
            cmd.current_dir(working_dir);

            #[cfg(target_os = "windows")]
            {
                use std::os::windows::process::CommandExt;
                const CREATE_NO_WINDOW: u32 = 0x08000000;
                cmd.creation_flags(CREATE_NO_WINDOW);
            }

            if let Ok(mut child) = cmd.spawn() {
                println!("Auto-spawned background server from {:?}", path);
                std::thread::spawn(move || {
                    let _ = child.wait();
                });
                for _ in 0..25 {
                    thread::sleep(Duration::from_millis(80));
                    if TcpStream::connect_timeout(&addr, Duration::from_millis(80)).is_ok() {
                        println!("Anti-Pet local server (127.0.0.1:4199) is ready");
                        break;
                    }
                }
                return;
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    ensure_local_server_running();

    let args: Vec<String> = std::env::args().collect();
    let start_compact = args.iter().any(|a| a == "--compact" || a == "-c");

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    let (mon_width, mon_height, mon_scale) = if let Some(monitor) = event_loop.primary_monitor() {
        let scale = monitor.scale_factor();
        let size = monitor.size().to_logical::<f64>(scale);
        (size.width.max(360.0), size.height.max(400.0), scale)
    } else {
        (1440.0, 900.0, 1.0)
    };

    let tray_rgba = include_bytes!("../../assets/tray.rgba");
    let win_icon = tao::window::Icon::from_rgba(tray_rgba.to_vec(), 32, 32).ok();

    let (win_width, win_height, win_x, win_y) = if start_compact {
        let cw = 340.0;
        let ch = 400.0;
        let cx = (mon_width - cw - 24.0).max(0.0);
        let cy = (mon_height - 48.0 - ch).max(0.0);
        (cw, ch, cx, cy)
    } else {
        (mon_width, mon_height, 0.0, 0.0)
    };

    #[allow(unused_mut)]

    let mut win_builder = WindowBuilder::new()
        .with_title("Anti-Pet In Desktop")
        .with_transparent(true)
        .with_decorations(false)
        .with_always_on_top(true)
        .with_visible_on_all_workspaces(true)
        .with_resizable(false)
        .with_inner_size(LogicalSize::new(win_width, win_height))
        .with_position(LogicalPosition::new(win_x, win_y));

    #[cfg(target_os = "windows")]
    {
        win_builder = win_builder
            .with_undecorated_shadow(false)
            .with_skip_taskbar(true);
    }

    if let Some(icon) = win_icon {
        win_builder = win_builder.with_window_icon(Some(icon));
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

    let window = win_builder.build(&event_loop)?;
    let _ = window.set_ignore_cursor_events(true);

    #[cfg(target_os = "windows")]
    {
        windows_native::strip_window_decorations(window.hwnd());
    }

    #[cfg(target_os = "macos")]
    {
        macos_native::set_application_dock_icon(APP_ICON_PNG);
        let _ = macos_native::sync_fullscreen_overlay_and_query_dock(window.ns_window(), true);
    }

    let initial_rect = HitRect {
        x: (win_width - 200.0).max(0.0),
        y: (win_height - 260.0).max(0.0),
        w: 190.0,
        h: 260.0,
    };

    let hit_rects = Arc::new(Mutex::new(vec![initial_rect]));
    let is_dragging = Arc::new(AtomicBool::new(false));
    let dnd_passthrough = Arc::new(AtomicBool::new(false));
    let dnd_wake_rect: Arc<Mutex<Option<HitRect>>> = Arc::new(Mutex::new(None));
    let currently_ignoring = Arc::new(AtomicBool::new(true));
    let frontend_pixel_transparent = Arc::new(AtomicBool::new(false));

    let ipc_proxy = proxy.clone();
    let webview = WebViewBuilder::new()
        .with_transparent(true)
        .with_accept_first_mouse(true)
        .with_initialization_script(
            r#"
            window.__ANTI_PET_DESKTOP__ = true;
            "#,
        )
        .with_ipc_handler(move |req| {
            let body = req.body();
            if let Some(evt) = parse_ipc_command(body) {
                let _ = ipc_proxy.send_event(evt);
            }
        })
        .with_url("http://127.0.0.1:4199")
        .build(&window)?;

    let ticker_proxy = proxy.clone();
    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(25));
        if ticker_proxy.send_event(UserEvent::TickCursor).is_err() {
            break;
        }
    });

    println!("Anti-Pet 桌面全透明悬浮穿透引擎已挂载");

    // Initialize System Tray Menu and Icon
    let tray_menu = Menu::new();
    let item_settings = MenuItem::new("偏好设置...", true, None);
    let sep0 = PredefinedMenuItem::separator();
    let item_toggle = MenuItem::new("显示 / 隐藏 大肥猫", true, None);
    let item_toggle_mode = MenuItem::new(
        if start_compact { "切换为: 全屏画布模式" } else { "切换为: 紧凑挂件模式 (省资源)" },
        true,
        None,
    );
    let item_dnd = MenuItem::new("完全穿透 (免打扰模式)", true, None);
    let item_reset_zoom = MenuItem::new("重置缩放 (100%)", true, None);
    let item_switch_skin = MenuItem::new("切换皮肤", true, None);
    let sep1 = PredefinedMenuItem::separator();
    let item_about = MenuItem::new("关于 大肥猫桌宠挂件 v0.1.1", false, None);
    let sep2 = PredefinedMenuItem::separator();
    let item_quit = MenuItem::new("退出", true, None);

    let _ = tray_menu.append_items(&[
        &item_settings,
        &sep0,
        &item_toggle,
        &item_toggle_mode,
        &item_dnd,
        &item_reset_zoom,
        &item_switch_skin,
        &sep1,
        &item_about,
        &sep2,
        &item_quit,
    ]);

    let tray_rgba = include_bytes!("../../assets/tray.rgba");
    let tray_icon = TrayIcon::from_rgba(tray_rgba.to_vec(), 32, 32).ok();

    let mut tray_builder = TrayIconBuilder::new()
        .with_tooltip("大肥猫桌宠挂件 v0.1.1")
        .with_menu(Box::new(tray_menu));

    if let Some(icon) = tray_icon {
        tray_builder = tray_builder.with_icon(icon);
    }

    let _tray = tray_builder.build()?;
    let is_window_visible = Arc::new(AtomicBool::new(true));

    let mut last_cursor_local = (-1.0_f64, -1.0_f64);
    let mut tick_counter: u32 = 0;
    let mut last_dock_sig = (-1.0_f64, -1.0_f64, -1.0_f64);
    let mut last_typing_state = false;
    let mut last_ide_state = false;
    let mut is_compact_mode = start_compact;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(StartCause::Init) => {
                window.set_always_on_top(true);
                window.set_visible_on_all_workspaces(true);
                #[cfg(target_os = "macos")]
                {
                    let _ = macos_native::sync_fullscreen_overlay_and_query_dock(
                        window.ns_window(),
                        true,
                    );
                }
            }
            Event::WindowEvent {
                event: WindowEvent::Focused(false),
                ..
            } => {
                let _ = webview.evaluate_script(
                    "if(window.__antiPetOnWindowBlur)window.__antiPetOnWindowBlur();",
                );
            }
            Event::UserEvent(user_evt) => match user_evt {
                UserEvent::SetIgnoreCursorEvents(ignore) => {
                    frontend_pixel_transparent.store(ignore, Ordering::Relaxed);
                    let effective_ignore = if is_dragging.load(Ordering::Relaxed) {
                        false
                    } else if dnd_passthrough.load(Ordering::Relaxed) {
                        true
                    } else {
                        ignore
                    };
                    if currently_ignoring.swap(effective_ignore, Ordering::Relaxed)
                        != effective_ignore
                    {
                        let _ = window.set_ignore_cursor_events(effective_ignore);
                    }
                }
                UserEvent::UpdateHitRects(rects) => {
                    if let Ok(mut guard) = hit_rects.lock() {
                        *guard = rects;
                    }
                }
                UserEvent::SetDragging(dragging) => {
                    is_dragging.store(dragging, Ordering::Relaxed);
                    if dragging && currently_ignoring.swap(false, Ordering::Relaxed) {
                        let _ = window.set_ignore_cursor_events(false);
                    }
                }
                UserEvent::SetDndPassthrough(enabled, wake_rect) => {
                    dnd_passthrough.store(enabled, Ordering::Relaxed);
                    if let Ok(mut guard) = dnd_wake_rect.lock() {
                        *guard = wake_rect;
                    }
                    if enabled && !currently_ignoring.swap(true, Ordering::Relaxed) {
                        let _ = window.set_ignore_cursor_events(true);
                    }
                }
                UserEvent::SetAlwaysOnTop(on_top) => {
                    window.set_always_on_top(on_top);
                }
                UserEvent::SetVisible(visible) => {
                    is_window_visible.store(visible, Ordering::Relaxed);
                    window.set_visible(visible);
                }
                UserEvent::DragWindow => {
                    let _ = window.drag_window();
                }
                UserEvent::TickCursor => {
                    tick_counter = tick_counter.wrapping_add(1);

                    // Drain System Tray Menu Click Events
                    while let Ok(menu_event) = MenuEvent::receiver().try_recv() {
                        if menu_event.id == item_settings.id() {
                            if !is_window_visible.load(Ordering::Relaxed) {
                                is_window_visible.store(true, Ordering::Relaxed);
                                window.set_visible(true);
                            }
                            let _ = webview.evaluate_script("if(window.__openSettings)window.__openSettings();");
                        } else if menu_event.id == item_toggle.id() {
                            let vis = !is_window_visible.load(Ordering::Relaxed);
                            is_window_visible.store(vis, Ordering::Relaxed);
                            window.set_visible(vis);
                        } else if menu_event.id == item_toggle_mode.id() {
                            is_compact_mode = !is_compact_mode;
                            #[allow(unused_variables)]
                            let (mw, mh, sc) = if let Some(m) = window.current_monitor() {
                                let s = m.scale_factor();
                                let sz = m.size().to_logical::<f64>(s);
                                (sz.width, sz.height, s)
                            } else {
                                (mon_width, mon_height, mon_scale)
                            };

                            if is_compact_mode {
                                #[cfg(target_os = "windows")]
                                let tb = windows_native::query_taskbar_insets(window.hwnd(), sc).bottom;
                                #[cfg(not(target_os = "windows"))]
                                let tb = 0.0;

                                let cw = 340.0;
                                let ch = 400.0;
                                let cx = (mw - cw - 24.0).max(0.0);
                                let cy = (mh - tb - ch).max(0.0);

                                window.set_inner_size(LogicalSize::new(cw, ch));
                                window.set_outer_position(LogicalPosition::new(cx, cy));
                                #[cfg(target_os = "windows")]
                                windows_native::strip_window_decorations(window.hwnd());
                                let _ = item_toggle_mode.set_text("切换为: 全屏画布模式");
                                let _ = webview.evaluate_script("if(window.__antiPetOnModeChange)window.__antiPetOnModeChange('compact');");
                            } else {
                                window.set_inner_size(LogicalSize::new(mw, mh));
                                window.set_outer_position(LogicalPosition::new(0.0, 0.0));
                                #[cfg(target_os = "windows")]
                                windows_native::strip_window_decorations(window.hwnd());
                                let _ = item_toggle_mode.set_text("切换为: 紧凑挂件模式 (省资源)");
                                let _ = webview.evaluate_script("if(window.__antiPetOnModeChange)window.__antiPetOnModeChange('fullscreen');");
                            }
                        } else if menu_event.id == item_dnd.id() {
                            let _ = webview.evaluate_script(
                                "if(window.__antiPetToggleDnd)window.__antiPetToggleDnd();",
                            );
                        } else if menu_event.id == item_reset_zoom.id() {
                            let _ = webview.evaluate_script(
                                "if(window.__antiPetResetZoom)window.__antiPetResetZoom();",
                            );
                        } else if menu_event.id == item_switch_skin.id() {
                            let _ = webview.evaluate_script(
                                "if(window.__antiPetCycleSkin)window.__antiPetCycleSkin();",
                            );
                        } else if menu_event.id == item_quit.id() {
                            *control_flow = ControlFlow::Exit;
                        }
                    }

                    // Drain System Tray Icon Direct Clicks
                    while let Ok(tray_event) = TrayIconEvent::receiver().try_recv() {
                        match tray_event {
                            TrayIconEvent::Click {
                                button: MouseButton::Left,
                                button_state: MouseButtonState::Up,
                                ..
                            } => {
                                let vis = !is_window_visible.load(Ordering::Relaxed);
                                is_window_visible.store(vis, Ordering::Relaxed);
                                window.set_visible(vis);
                            }
                            _ => {}
                        }
                    }

                    #[cfg(target_os = "macos")]
                    let dock = macos_native::sync_fullscreen_overlay_and_query_dock(
                        window.ns_window(),
                        tick_counter % 20 == 0,
                    );
                    #[cfg(target_os = "macos")]
                    let is_typing = macos_native::is_user_typing_globally();

                    #[cfg(target_os = "windows")]
                    let dock = windows_native::query_taskbar_insets(window.hwnd(), window.scale_factor());
                    #[cfg(target_os = "windows")]
                    let is_typing = false;
                    #[cfg(target_os = "windows")]
                    let dock_tuple = (dock.bottom, dock.left, dock.right);

                    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
                    let dock = (0.0_f64, 0.0_f64, 0.0_f64);
                    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
                    let is_typing = false;
                    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
                    let dock_tuple = dock;

                    #[cfg(target_os = "macos")]
                    let dock_tuple = (dock.bottom, dock.left, dock.right);

                    #[cfg(target_os = "macos")]
                    let cursor_opt = macos_native::get_mouse_location_in_window(window.ns_window());

                    #[cfg(not(target_os = "macos"))]
                    let cursor_opt = if let Ok(cursor_phys) = window.cursor_position() {
                        let win_pos = window
                            .inner_position()
                            .or_else(|_| window.outer_position())
                            .unwrap_or(PhysicalPosition::new(0, 0));
                        let scale = window.scale_factor();
                        Some(((cursor_phys.x - win_pos.x as f64) / scale, (cursor_phys.y - win_pos.y as f64) / scale))
                    } else {
                        None
                    };

                    #[cfg(target_os = "macos")]
                    let is_ide_active = macos_native::is_ide_frontmost();
                    #[cfg(not(target_os = "macos"))]
                    let is_ide_active = false;

                    if let Some((local_x, local_y)) = cursor_opt {
                        let in_hit_box = if is_dragging.load(Ordering::Relaxed) {
                            true
                        } else if dnd_passthrough.load(Ordering::Relaxed) {
                            dnd_wake_rect
                                .lock()
                                .ok()
                                .and_then(|g| *g)
                                .map(|r| r.contains(local_x, local_y))
                                .unwrap_or(false)
                        } else {
                            hit_rects
                                .lock()
                                .map(|rects| rects.iter().any(|r| r.contains(local_x, local_y)))
                                .unwrap_or(false)
                        };

                        let should_ignore = if !in_hit_box {
                            frontend_pixel_transparent.store(false, Ordering::Relaxed);
                            true
                        } else {
                            frontend_pixel_transparent.load(Ordering::Relaxed)
                        };

                        if currently_ignoring.swap(should_ignore, Ordering::Relaxed)
                            != should_ignore
                        {
                            let _ = window.set_ignore_cursor_events(should_ignore);
                        }

                        let cursor_moved = (local_x - last_cursor_local.0).abs() > 1.0
                            || (local_y - last_cursor_local.1).abs() > 1.0;
                        let dock_changed = (dock_tuple.0 - last_dock_sig.0).abs() > 0.5
                            || (dock_tuple.1 - last_dock_sig.1).abs() > 0.5
                            || (dock_tuple.2 - last_dock_sig.2).abs() > 0.5;
                        let typing_changed = is_typing != last_typing_state;
                        let ide_changed = is_ide_active != last_ide_state;

                        if cursor_moved || dock_changed || typing_changed || ide_changed {
                            last_cursor_local = (local_x, local_y);
                            last_dock_sig = dock_tuple;
                            last_typing_state = is_typing;
                            last_ide_state = is_ide_active;
                            let js = format!(
                                "if(window.__antiPetOnGlobalCursor)window.__antiPetOnGlobalCursor({:.1},{:.1},{:.1},{:.1},{:.1},{},{});",
                                local_x,
                                local_y,
                                dock_tuple.0,
                                dock_tuple.1,
                                dock_tuple.2,
                                if is_typing { "true" } else { "false" },
                                if is_ide_active { "true" } else { "false" }
                            );
                            let _ = webview.evaluate_script(&js);
                        }
                    }
                }
            },
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                // Minimize/Hide to tray instead of terminating
                is_window_visible.store(false, Ordering::Relaxed);
                window.set_visible(false);
            }
            _ => (),
        }
    });
}
