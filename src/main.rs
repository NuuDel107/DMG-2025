use std::sync::Arc;

mod cpu;
use cpu::CPU;
mod window;
use window::Window;
mod options;
use options::Options;

fn load_rom_file(path: &str) -> Vec<u8> {
    match std::fs::read(path) {
        Ok(rom_file) => rom_file,
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                eprintln!("ROM file not found!");
            }
            panic!("{}", e)
        }
    }
}

fn main() -> eframe::Result {
    // Display backtrace
    std::env::set_var("RUST_BACKTRACE", "1");
    // Log to stderr
    env_logger::init();

    let options = Options::load();
    let rom_file = load_rom_file(&options.rom_path);
    let cpu = CPU::new(rom_file);

    // Initialize main window
    let scale = options.window_scale;
    let window = Window::new(cpu, options);

    let icon = std::fs::read("icon.png").unwrap();
    let eframe_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_resizable(false)
            .with_inner_size([160.0 * scale as f32, 144.0 * scale as f32])
            .with_icon(Arc::new(eframe::icon_data::from_png_bytes(&icon).unwrap())),
        ..Default::default()
    };
    eframe::run_native(
        "DMG-2025",
        eframe_options,
        Box::new(|_| Ok(Box::new(window))),
    )
}
