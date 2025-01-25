#![cfg_attr(
    all(target_os = "windows", not(debug_assertions),),
    windows_subsystem = "windows"
)]
use std::sync::Arc;

mod cpu;
use cpu::CPU;
mod window;
use window::Window;
mod options;
use options::*;

fn main() -> eframe::Result {
    // Display backtrace
    std::env::set_var("RUST_BACKTRACE", "1");
    // Log to stderr
    env_logger::init();

    // Initialize main window
    let options = Options::load();
    let scale = options.window_scale;
    let icon = include_bytes!("../assets/icon.png");
    let size = [160.0 * scale as f32, 144.0 * scale as f32];
    let eframe_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_resizable(false)
            .with_maximize_button(false)
            .with_inner_size(size)
            .with_icon(Arc::new(eframe::icon_data::from_png_bytes(icon).unwrap())),
        ..Default::default()
    };
    // Run app
    eframe::run_native(
        "DMG-2025",
        eframe_options,
        Box::new(|cc| Ok(Box::new(Window::new(options, cc)))),
    )
}
