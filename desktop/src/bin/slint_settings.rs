use anti_pet_desktop::core::store::SettingsStore;
use anti_pet_desktop::state_watcher::{resolve_project_root, NativeStateWatcher};
use anti_pet_desktop::ui::windows::SlintSettingsApp;
use slint::ComponentHandle;
use std::sync::{Arc, Mutex};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("正在启动 Anti-Pet 下一代 Slint 现代偏好设置 (GPU 硬件加速与流畅滚动)...");
    let watcher = NativeStateWatcher::new();
    let settings = watcher.load_settings();
    let root_dir = resolve_project_root();

    let store = Arc::new(Mutex::new(SettingsStore::new(root_dir, settings)));
    let app = SlintSettingsApp::new(store)?;

    println!("Slint 偏好设置窗口已就绪。正在以 120Hz 硬件加速呈现...");
    app.window.run()?;
    Ok(())
}
