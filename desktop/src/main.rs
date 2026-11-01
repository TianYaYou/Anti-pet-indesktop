use tao::{
    dpi::LogicalSize,
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use wry::WebViewBuilder;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Anti-Pet In Desktop")
        .with_transparent(true)
        .with_decorations(false)
        .with_always_on_top(true)
        .with_inner_size(LogicalSize::new(320.0, 360.0))
        .build(&event_loop)?;

    let _webview = WebViewBuilder::new()
        .with_transparent(true)
        .with_url("http://127.0.0.1:4199")
        .build(&window)?;

    println!("🐾 Anti-Pet 桌面悬浮穿透窗口已成功挂载！");

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(StartCause::Init) => {
                println!("🐾 Anti-Pet 运行时已就绪 (macOS/Windows)");
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            _ => (),
        }
    });
}
