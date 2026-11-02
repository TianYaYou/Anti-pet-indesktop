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
use muda::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
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

const COMPACT_W: f64 = 340.0;
const COMPACT_H: f64 = 400.0;

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

struct CompactFallPhysics {
    active: bool,
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    target_floor_y: f64,
}

#[derive(Debug, Clone)]
enum UserEvent {
    SetIgnoreCursorEvents(bool),
    UpdateHitRects(Vec<HitRect>),
    SetDragging(bool),
    MoveWindowBy(f64, f64),
    ReleaseWindow {
        vx: f64,
        gravity: bool,
        floor_mode: String,
    },
    SetDndPassthrough(bool, Option<HitRect>),
    SetAlwaysOnTop(bool),
    SetVisible(bool),
    DragWindow,
    TickCursor,
    OpenSettings,
    CloseSettings,
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
        fn CGEventSourceSecondsSinceLastEventType(state_id: i32, event_type: u32) -> f64;
    }

    pub fn is_user_typing_globally() -> bool {
        unsafe {
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

            let top_menu_inset = (full_frame.origin.y + full_frame.size.height)
                - (vis_frame.origin.y + vis_frame.size.height);
            if top_menu_inset < 1.0 {
                DockInsets::default()
            } else {
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
}

fn parse_ipc_command(msg: &str) -> Option<UserEvent> {
    let trimmed = msg.trim();
    if trimmed == "drag_window" {
        return Some(UserEvent::DragWindow);
    }
    if trimmed == "open_settings" {
        return Some(UserEvent::OpenSettings);
    }
    if trimmed == "close_settings" {
        return Some(UserEvent::CloseSettings);
    }
    if let Some(val) = trimmed.strip_prefix("set_ignore_cursor_events:") {
        return Some(UserEvent::SetIgnoreCursorEvents(val.trim() == "true"));
    }
    if let Some(val) = trimmed.strip_prefix("set_dragging:") {
        return Some(UserEvent::SetDragging(val.trim() == "true"));
    }
    if let Some(val) = trimmed.strip_prefix("move_window_by:") {
        let nums: Vec<f64> = val
            .split(',')
            .filter_map(|n| n.trim().parse::<f64>().ok())
            .collect();
        if nums.len() == 2 {
            return Some(UserEvent::MoveWindowBy(nums[0], nums[1]));
        }
    }
    if let Some(val) = trimmed.strip_prefix("release_window:") {
        let parts: Vec<&str> = val.split(',').collect();
        let vx = parts
            .first()
            .and_then(|s| s.trim().parse::<f64>().ok())
            .unwrap_or(0.0);
        let gravity = parts.get(1).map(|s| s.trim() == "true").unwrap_or(true);
        let floor_mode = parts
            .get(2)
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "auto".to_string());
        return Some(UserEvent::ReleaseWindow {
            vx,
            gravity,
            floor_mode,
        });
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

pub fn run_legacy_webkit() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let start_compact = args.iter().any(|a| a == "--compact" || a == "-c");

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    let (mon_width, mon_height, _mon_scale) = if let Some(monitor) = event_loop.primary_monitor() {
        let scale = monitor.scale_factor();
        let size = monitor.size().to_logical::<f64>(scale);
        (size.width.max(360.0), size.height.max(400.0), scale)
    } else {
        (1440.0, 900.0, 1.0)
    };

    let tray_rgba = include_bytes!("../../assets/tray.rgba");
    let win_icon = tao::window::Icon::from_rgba(tray_rgba.to_vec(), 32, 32).ok();

    let init_compact_x = (mon_width - COMPACT_W - 24.0).max(0.0);
    let init_compact_y = (mon_height - 72.0 - COMPACT_H).max(0.0);

    let (win_width, win_height, win_x, win_y) = if start_compact {
        (COMPACT_W, COMPACT_H, init_compact_x, init_compact_y)
    } else {
        (mon_width, mon_height, 0.0, 0.0)
    };

    #[allow(unused_mut)]
    let mut win_builder = WindowBuilder::new()
        .with_title("Anti-Pet In Desktop (Legacy WebKit)")
        .with_transparent(true)
        .with_decorations(false)
        .with_always_on_top(true)
        .with_visible_on_all_workspaces(true)
        .with_resizable(false)
        .with_inner_size(LogicalSize::new(win_width, win_height))
        .with_position(LogicalPosition::new(win_x, win_y));

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

    #[cfg(target_os = "macos")]
    {
        macos_native::set_accessory_activation_policy();
        let _ = macos_native::sync_fullscreen_overlay_and_query_dock(
            window.ns_window(),
            !start_compact,
        );
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

    let init_script = if start_compact {
        "window.__ANTI_PET_DESKTOP__ = true; window.__ANTI_PET_INITIAL_MODE__ = 'compact';"
    } else {
        "window.__ANTI_PET_DESKTOP__ = true; window.__ANTI_PET_INITIAL_MODE__ = 'fullscreen';"
    };

    let ipc_proxy = proxy.clone();
    let webview = WebViewBuilder::new()
        .with_transparent(true)
        .with_accept_first_mouse(true)
        .with_initialization_script(init_script)
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

    let tray_menu = Menu::new();
    let item_toggle = MenuItem::new("显示 / 隐藏 大肥猫", true, None);
    let sep1 = PredefinedMenuItem::separator();
    let item_quit = MenuItem::new("退出", true, None);
    let _ = tray_menu.append_items(&[&item_toggle, &sep1, &item_quit]);

    let tray_icon = TrayIcon::from_rgba(tray_rgba.to_vec(), 32, 32).ok();
    let mut tray_builder = TrayIconBuilder::new()
        .with_tooltip("大肥猫桌宠挂件 (Legacy WebKit)")
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
    let is_compact_mode = start_compact;
    let mut compact_x = init_compact_x;
    let mut compact_y = init_compact_y;
    let mut compact_physics = CompactFallPhysics {
        active: false,
        x: init_compact_x,
        y: init_compact_y,
        vx: 0.0,
        vy: 0.0,
        target_floor_y: init_compact_y,
    };

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(StartCause::Init) => {
                window.set_always_on_top(true);
                window.set_visible_on_all_workspaces(true);
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
                    if dragging {
                        compact_physics.active = false;
                        if currently_ignoring.swap(false, Ordering::Relaxed) {
                            let _ = window.set_ignore_cursor_events(false);
                        }
                    }
                }
                UserEvent::MoveWindowBy(dx, dy) => {
                    if is_compact_mode {
                        compact_physics.active = false;
                        compact_x = (compact_x + dx).clamp(-100.0, mon_width - COMPACT_W + 100.0);
                        compact_y = (compact_y + dy).clamp(0.0, mon_height - COMPACT_H);
                        window.set_outer_position(LogicalPosition::new(compact_x, compact_y));
                    }
                }
                UserEvent::ReleaseWindow {
                    vx,
                    gravity,
                    floor_mode,
                } => {
                    if !is_compact_mode {
                        return;
                    }
                    let dock_bottom = last_dock_sig.0.max(0.0);
                    let true_bottom_y = (mon_height - COMPACT_H).max(0.0);
                    let dock_ledge_y = (mon_height - COMPACT_H - dock_bottom).max(0.0);
                    let target_floor_y = if floor_mode == "bottom" || dock_bottom <= 2.0 {
                        true_bottom_y
                    } else {
                        dock_ledge_y
                    };
                    if !gravity || floor_mode == "free" || compact_y >= target_floor_y - 2.0 {
                        compact_y = target_floor_y;
                        window.set_outer_position(LogicalPosition::new(compact_x, compact_y));
                    } else {
                        compact_physics = CompactFallPhysics {
                            active: true,
                            x: compact_x,
                            y: compact_y,
                            vx: vx * 0.35,
                            vy: 0.0,
                            target_floor_y,
                        };
                    }
                }
                UserEvent::SetDndPassthrough(enabled, wake_rect) => {
                    dnd_passthrough.store(enabled, Ordering::Relaxed);
                    if let Ok(mut guard) = dnd_wake_rect.lock() {
                        *guard = wake_rect;
                    }
                }
                UserEvent::SetAlwaysOnTop(on_top) => window.set_always_on_top(on_top),
                UserEvent::SetVisible(visible) => window.set_visible(visible),
                UserEvent::DragWindow => {
                    let _ = window.drag_window();
                }
                UserEvent::OpenSettings | UserEvent::CloseSettings => {}
                UserEvent::TickCursor => {
                    tick_counter = tick_counter.wrapping_add(1);
                    if is_compact_mode && compact_physics.active {
                        compact_physics.vy += 1.15;
                        compact_physics.y += compact_physics.vy;
                        compact_physics.x += compact_physics.vx;
                        compact_physics.vx *= 0.94;
                        if compact_physics.y >= compact_physics.target_floor_y {
                            compact_physics.y = compact_physics.target_floor_y;
                            compact_physics.active = false;
                            let _ = webview.evaluate_script(
                                "if(window.__antiPetOnCompactBounce)window.__antiPetOnCompactBounce();",
                            );
                        }
                        compact_x = compact_physics.x;
                        compact_y = compact_physics.y;
                        window.set_outer_position(LogicalPosition::new(compact_x, compact_y));
                    }

                    while let Ok(menu_event) = MenuEvent::receiver().try_recv() {
                        if menu_event.id == item_toggle.id() {
                            let vis = !is_window_visible.load(Ordering::Relaxed);
                            is_window_visible.store(vis, Ordering::Relaxed);
                            window.set_visible(vis);
                        } else if menu_event.id == item_quit.id() {
                            *control_flow = ControlFlow::Exit;
                        }
                    }

                    while let Ok(tray_event) = TrayIconEvent::receiver().try_recv() {
                        if let TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        } = tray_event
                        {
                            let vis = !is_window_visible.load(Ordering::Relaxed);
                            is_window_visible.store(vis, Ordering::Relaxed);
                            window.set_visible(vis);
                        }
                    }

                    #[cfg(target_os = "macos")]
                    let dock = macos_native::sync_fullscreen_overlay_and_query_dock(
                        window.ns_window(),
                        !is_compact_mode && (tick_counter % 20 == 0),
                    );
                    #[cfg(target_os = "macos")]
                    let is_typing = macos_native::is_user_typing_globally();
                    #[cfg(target_os = "macos")]
                    let dock_tuple = (dock.bottom, dock.left, dock.right);
                    #[cfg(target_os = "macos")]
                    let cursor_opt =
                        macos_native::get_mouse_location_in_window(window.ns_window());
                    #[cfg(target_os = "macos")]
                    let is_ide_active = macos_native::is_ide_frontmost();

                    #[cfg(not(target_os = "macos"))]
                    let (dock_tuple, is_typing, cursor_opt, is_ide_active) = (
                        (0.0, 0.0, 0.0),
                        false,
                        window.cursor_position().ok().map(|p| {
                            let w = window.outer_position().unwrap_or(PhysicalPosition::new(0, 0));
                            let s = window.scale_factor();
                            ((p.x - w.x as f64) / s, (p.y - w.y as f64) / s)
                        }),
                        false,
                    );

                    if let Some((local_x, local_y)) = cursor_opt {
                        let in_hit_box = if is_dragging.load(Ordering::Relaxed) {
                            true
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

                        if (local_x - last_cursor_local.0).abs() > 1.0
                            || (local_y - last_cursor_local.1).abs() > 1.0
                            || is_typing != last_typing_state
                            || is_ide_active != last_ide_state
                        {
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
                window.set_visible(false);
            }
            _ => (),
        }
    });
}
