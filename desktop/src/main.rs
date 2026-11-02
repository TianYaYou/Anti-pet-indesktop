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
        renderer::{bubble::ThoughtBubble, sprite::CatSprite, Pixmap, SoftwareSurface},
        state_watcher::{HookStateUpdate, NativeStateWatcher},
    };

    #[derive(Debug, Clone)]
    enum AppEvent {
        Tick25ms,
        QuotaUpdated(QuotaSnapshot),
        HookStateChanged(HookStateUpdate),
        CloseSettings,
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

        pub fn enforce_topmost_and_query_dock(ns_window: *mut c_void) -> DockInsets {
            if ns_window.is_null() {
                return DockInsets::default();
            }
            unsafe {
                // NSStatusWindowLevel (25) floats above normal apps and full-screen windows
                let set_level: unsafe extern "C" fn(*mut c_void, *mut c_void, isize) =
                    std::mem::transmute(objc_msgSend as *const ());
                set_level(ns_window, sel(b"setLevel:\0"), 25);

                // CanJoinAllSpaces (1<<0) | Stationary (1<<4) | IgnoresCycle (1<<6) | FullScreenAuxiliary (1<<8)
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
    }

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

        let foot_pad_bottom = (COMPACT_WIN_H - 386.0).max(0.0);
        let default_x = settings
            .pos_x
            .unwrap_or_else(|| (mon_width - COMPACT_WIN_W - 28.0).max(0.0));
        let default_y = settings
            .pos_y
            .unwrap_or_else(|| (mon_height - COMPACT_WIN_H + foot_pad_bottom).max(0.0));

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
        let _ = window.set_ignore_cursor_events(true);

        let mut settings_win_builder = WindowBuilder::new()
            .with_title("偏好设置")
            .with_inner_size(LogicalSize::new(680.0, 520.0))
            .with_resizable(false)
            .with_visible(false);

        if let Some(ref icon) = win_icon {
            settings_win_builder = settings_win_builder.with_window_icon(Some(icon.clone()));
        }

        let settings_window = settings_win_builder.build(&event_loop)?;

        let settings_ipc_proxy = proxy.clone();
        let settings_webview = wry::WebViewBuilder::new()
            .with_accept_first_mouse(true)
            .with_ipc_handler(move |req| {
                let body = req.body();
                if body == "close_settings" {
                    let _ = settings_ipc_proxy.send_event(AppEvent::CloseSettings);
                }
            })
            .with_url("http://127.0.0.1:4199/settings.html")
            .build(&settings_window)?;

        let mut pending_settings_save: Option<Instant> = None;


        #[cfg(target_os = "macos")]
        let surface = {
            macos_window::set_accessory_activation_policy();
            let _ = macos_window::enforce_topmost_and_query_dock(window.ns_window());
            SoftwareSurface::new_macos(window.ns_window())
        };

        #[cfg(target_os = "windows")]
        let surface = SoftwareSurface::new_windows(window.hwnd());

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

        let mut physics = PetPhysics::new(default_x, default_y);
        physics.zoom_scale = settings.scale;
        physics.gravity_enabled = settings.gravity_enabled;
        physics.floor_mode = settings.floor_mode.clone();

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
        };
        let mut bubble_hide_deadline: Option<Instant> =
            Some(Instant::now() + Duration::from_millis(5000));

        let mut hit_tester = PixelHitTester::default();
        let mut audio = NativeAudioPlayer::new(settings.sound_enabled);
        let menu_mgr = NativeMenuManager::new(
            settings.show_quota,
            settings.dnd_passthrough,
            settings.gravity_enabled,
            &settings.floor_mode,
            settings.sound_enabled,
        );

        let tray_icon = TrayIcon::from_rgba(tray_rgba.to_vec(), 32, 32).ok();
        let mut tray_builder = TrayIconBuilder::new()
            .with_tooltip("大肥猫桌宠 v0.3.0 (Pure CPU 2D)")
            .with_menu(Box::new(menu_mgr.menu.clone()));
        if let Some(icon) = tray_icon {
            tray_builder = tray_builder.with_icon(icon);
        }
        let _tray = tray_builder.build()?;

        // Background ticker (40 Hz cursor/physics tick + 250ms hook file watch + 12s live quota probe)
        let ticker_proxy = proxy.clone();
        let force_quota_flag = Arc::new(AtomicBool::new(true));
        let force_quota_bg = force_quota_flag.clone();
        thread::spawn(move || {
            let mut prober = NativeQuotaProber::default();
            let mut hook_watcher = NativeStateWatcher::new();
            let mut tick_count: u64 = 0;

            loop {
                thread::sleep(Duration::from_millis(25));
                tick_count = tick_count.wrapping_add(1);

                if ticker_proxy.send_event(AppEvent::Tick25ms).is_err() {
                    break;
                }

                // Every 250ms: check if Antigravity hooks updated `.antigravity-pet-state.json`
                if tick_count % 10 == 0 {
                    if let Some(update) = hook_watcher.poll_state_change() {
                        let _ = ticker_proxy.send_event(AppEvent::HookStateChanged(update));
                        force_quota_bg.store(true, Ordering::Relaxed);
                    }
                }

                // Every 12s (or on force flag): poll live Antigravity `language_server` quota
                if force_quota_bg.swap(false, Ordering::Relaxed) || tick_count % 480 == 0 {
                    let snap = prober.fetch_live_quota();
                    let _ = ticker_proxy.send_event(AppEvent::QuotaUpdated(snap));
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

        println!("Anti-Pet v0.3.0 Pure CPU 2D 软渲染引擎已启动 (Zero WebKit / Zero Node.js)");

        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;

            match event {
                Event::NewEvents(StartCause::Init) => {
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
                    }

                    AppEvent::Tick25ms => {


                        // 1. Drain native context/tray menu events
                        while let Ok(menu_evt) = MenuEvent::receiver().try_recv() {
                            if let Some(action) = menu_mgr.resolve_event(&menu_evt.id) {
                                match action {
                                    PetMenuAction::OpenSettings => {
                                        audio.play(SoundEffect::ClickRelease);
                                        bubble.text = "已为你打开偏好设置~".to_string();
                                        bubble.visible = !settings.dnd_passthrough;
                                        bubble.opacity = 1.0;
                                        bubble_hide_deadline =
                                            Some(Instant::now() + Duration::from_millis(3500));
                                        ensure_backend_server();
                                        settings_window.set_visible(true);
                                        settings_window.set_focus();
                                        let _ = settings_webview.load_url("http://127.0.0.1:4199/settings.html");
                                        dirty_frame = true;
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
                                    PetMenuAction::SetFloorMode(mode) => {
                                        settings.floor_mode = mode.to_string();
                                        physics.floor_mode = settings.floor_mode.clone();
                                        physics.on_drag_release(
                                            mon_width,
                                            mon_height,
                                            last_dock.0,
                                            last_dock.1,
                                            last_dock.2,
                                        );
                                        state_watcher.save_settings(&settings);
                                        dirty_frame = true;
                                    }
                                    PetMenuAction::SetZoom(z) => {
                                        settings.scale = z;
                                        physics.zoom_scale = z;
                                        physics.trigger_duang_rebound(380);
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
                            physics.floor_mode = new_settings.floor_mode.clone();
                            audio.enabled = new_settings.sound_enabled;
                            bubble.show_quota = new_settings.show_quota;
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
                            let d = macos_window::enforce_topmost_and_query_dock(window.ns_window());
                            let typing = macos_window::is_user_typing_globally();
                            let cur = macos_window::get_cursor_state(window.ns_window(), mon_height);
                            ((d.bottom, d.left, d.right), typing, cur)
                        };

                        #[cfg(not(target_os = "macos"))]
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
                                        physics.on_drag_move(dx, dy, mon_width, mon_height);
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
                        let (win_moved, anim_active) =
                            physics.step_frame(mon_width, dock.1, dock.2);
                        if win_moved {
                            window.set_outer_position(LogicalPosition::new(
                                physics.win_x,
                                physics.win_y,
                            ));
                            settings.pos_x = Some(physics.win_x);
                            settings.pos_y = Some(physics.win_y);
                        }

                        // Redraw at 60 FPS during active animation, or ~8 FPS during calm idle breathing
                        if dirty_frame || win_moved || anim_active {
                            dirty_frame = false;
                            let scale = window.scale_factor() as f32;
                            let pw = ((COMPACT_WIN_W as f32) * scale).round() as u32;
                            let ph = ((COMPACT_WIN_H as f32) * scale).round() as u32;
                            pixmap.resize(pw, ph);
                            pixmap.clear();

                            let tf = physics.current_sprite_transform(settings.dnd_passthrough);
                            let (_, min_py, _, _) = cat_sprite.draw(&mut pixmap, &tf, scale);
                            let cat_top_logical_y = (min_py as f32) / scale.max(1.0);

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
                    }
                },

                Event::WindowEvent { window_id, event, .. } => match event {
                    WindowEvent::CloseRequested => {
                        if window_id == settings_window.id() {
                            settings_window.set_visible(false);
                        } else if window_id == window.id() {
                            is_window_visible = false;
                            window.set_visible(false);
                        }
                    }

                    WindowEvent::Focused(false) => {
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
                                    physics.trigger_duang_rebound(280);
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

                                    #[cfg(target_os = "macos")]
                                    menu_mgr.show_context_menu_macos(window.ns_view(), None);
                                    #[cfg(target_os = "windows")]
                                    menu_mgr.show_context_menu_windows(window.hwnd(), None);
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
                },

                _ => (),
            }
        });
    }
}
