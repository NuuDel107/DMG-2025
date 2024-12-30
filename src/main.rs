use std::sync::Arc;

mod cpu;
use cpu::CPU;
mod window;
use window::Window;

const WINDOW_SCALE: u8 = 4;
const ROM_PATH: &str = "roms/test.gb";

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
    let rom_file = load_rom_file(ROM_PATH);
    let cpu = CPU::new(rom_file);
    let window = Window::new(cpu, WINDOW_SCALE);

    // Display backtrace
    std::env::set_var("RUST_BACKTRACE", "1");
    // Log to stderr
    env_logger::init();

    // Initialize main window
    let icon = std::fs::read("icon.png").unwrap();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_resizable(false)
            .with_inner_size([160.0 * WINDOW_SCALE as f32, 144.0 * WINDOW_SCALE as f32])
            .with_icon(Arc::new(eframe::icon_data::from_png_bytes(&icon).unwrap())),
        ..Default::default()
    };
    eframe::run_native("DMG-2025", options, Box::new(|_| Ok(Box::new(window))))
}
