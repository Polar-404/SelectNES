#![windows_subsystem = "windows"]

use selectnes::frontend::app::App;
use winit::{event_loop::EventLoop};

fn main() {
    std::panic::set_hook(Box::new(|info| {
        let msg = format!("[Error] the program suffered a critical error:\n{}", info);
        if std::fs::create_dir_all(".log").is_ok() {
            let _ = std::fs::write(".log/crashlog.log", msg);
        }
    }));

    let event_loop = EventLoop::new().unwrap();
    let mut app = App::new();
    event_loop.run_app(&mut app).unwrap();
}