use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};
use tao::{
    dpi::{LogicalPosition, LogicalSize, PhysicalPosition},
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::WindowBuilder,
};
#[cfg(target_os = "macos")]
use tao::platform::macos::WindowBuilderExtMacOS;
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

/// Custom events sent from WebView IPC or background cursor tracker to the main Window thread
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

/// Parse simple IPC message sent via `window.ipc.postMessage(...)`
/// Supported formats:
/// - `set_ignore_cursor_events:true` / `set_ignore_cursor_events:false`
/// - `set_dragging:true` / `set_dragging:false`
/// - `set_always_on_top:true` / `set_always_on_top:false`
/// - `set_visible:true` / `set_visible:false`
/// - `set_dnd:true:x,y,w,h` / `set_dnd:false`
/// - `hit_rects:x1,y1,w1,h1;x2,y2,w2,h2`
/// - `drag_window`
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    // Determine primary monitor dimensions so the transparent canvas covers the desktop
    // while allowing 100% click-through outside the pet & speech bubble.
    let (win_width, win_height) = if let Some(monitor) = event_loop.primary_monitor() {
        let scale = monitor.scale_factor();
        let size = monitor.size().to_logical::<f64>(scale);
        (size.width.max(360.0), size.height.max(400.0))
    } else {
        (1440.0, 900.0)
    };

    let mut win_builder = WindowBuilder::new()
        .with_title("Anti-Pet In Desktop")
        .with_transparent(true)
        .with_decorations(false)
        .with_always_on_top(true)
        .with_visible_on_all_workspaces(true)
        .with_resizable(false)
        .with_inner_size(LogicalSize::new(win_width, win_height))
        .with_position(LogicalPosition::new(0.0, 0.0));

    #[cfg(target_os = "macos")]
    {
        win_builder = win_builder
            .with_has_shadow(false)
            .with_titlebar_transparent(true);
    }

    let window = win_builder.build(&event_loop)?;

    // Start in click-through mode; hit-testing will dynamically enable interaction over the pet
    let _ = window.set_ignore_cursor_events(true);

    // Default initial hit rect in bottom-right corner before frontend reports exact bounds
    let initial_rect = HitRect {
        x: (win_width - 280.0).max(0.0),
        y: (win_height - 340.0).max(0.0),
        w: 270.0,
        h: 330.0,
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

    // Spawn 35Hz cursor hit-test & gaze-tracking ticker thread
    let ticker_proxy = proxy.clone();
    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(28));
        if ticker_proxy.send_event(UserEvent::TickCursor).is_err() {
            break;
        }
    });

    println!("🐾 Anti-Pet 桌面全透明悬浮穿透引擎已挂载 (Always-On-Top + Dynamic Hit-Testing)！");

    let mut last_cursor_phys = PhysicalPosition::new(-1.0, -1.0);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(StartCause::Init) => {
                let _ = window.set_always_on_top(true);
                let _ = window.set_visible_on_all_workspaces(true);
                println!("🐾 Anti-Pet 运行时已就绪 (macOS/Windows 顶置穿透已激活)");
            }
            Event::UserEvent(user_evt) => match user_evt {
                UserEvent::SetIgnoreCursorEvents(ignore) => {
                    // Frontend pixel-level alpha hit-test result
                    frontend_pixel_transparent.store(ignore, Ordering::Relaxed);
                    let effective_ignore = if is_dragging.load(Ordering::Relaxed) {
                        false
                    } else if dnd_passthrough.load(Ordering::Relaxed) {
                        true
                    } else {
                        ignore
                    };
                    if currently_ignoring.swap(effective_ignore, Ordering::Relaxed) != effective_ignore {
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
                    window.set_visible(visible);
                }
                UserEvent::DragWindow => {
                    let _ = window.drag_window();
                }
                UserEvent::TickCursor => {
                    if let Ok(cursor_phys) = window.cursor_position() {
                        let win_pos = window
                            .inner_position()
                            .or_else(|_| window.outer_position())
                            .unwrap_or(PhysicalPosition::new(0, 0));
                        let scale = window.scale_factor();

                        let local_x = (cursor_phys.x - win_pos.x as f64) / scale;
                        let local_y = (cursor_phys.y - win_pos.y as f64) / scale;

                        // Check if cursor is inside any interactive bounding box
                        let in_hit_box = if is_dragging.load(Ordering::Relaxed) {
                            true
                        } else if dnd_passthrough.load(Ordering::Relaxed) {
                            // In DND mode, only the tiny wake badge is interactive
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
                            // Inside bounding box: respect frontend's pixel-level alpha check if set
                            frontend_pixel_transparent.load(Ordering::Relaxed)
                        };

                        if currently_ignoring.swap(should_ignore, Ordering::Relaxed) != should_ignore {
                            let _ = window.set_ignore_cursor_events(should_ignore);
                        }

                        // Forward global cursor position to WebView for smooth eye/head gaze tracking
                        if (cursor_phys.x - last_cursor_phys.x).abs() > 1.5
                            || (cursor_phys.y - last_cursor_phys.y).abs() > 1.5
                        {
                            last_cursor_phys = cursor_phys;
                            let js = format!(
                                "if(window.__antiPetOnGlobalCursor)window.__antiPetOnGlobalCursor({:.1},{:.1});",
                                local_x, local_y
                            );
                            let _ = webview.evaluate_script(&js);
                        }
                    }
                }
            },
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            _ => (),
        }
    });
}
