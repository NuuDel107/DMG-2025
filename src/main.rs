use macroquad::prelude::*;
use miniquad::conf::Icon;

mod cpu;
mod window;

const WINDOW_SCALE: u8 = 4;
const ROM_PATH: &str = "roms/Pokemon - Red Version (USA, Europe) (SGB Enhanced).gb";

fn window_conf() -> Conf {
    Conf {
        window_title: "DMG-2025".to_owned(),
        icon: Some(Icon {
            small: image::open("img/icon_small.ico")
                .unwrap()
                .into_bytes()
                .try_into()
                .unwrap(),
            medium: image::open("img/icon_medium.ico")
                .unwrap()
                .into_bytes()
                .try_into()
                .unwrap(),
            big: image::open("img/icon_large.ico")
                .unwrap()
                .into_bytes()
                .try_into()
                .unwrap(),
        }),
        window_resizable: false,
        window_width: 160 * i32::from(WINDOW_SCALE),
        window_height: 144 * i32::from(WINDOW_SCALE),
        ..Default::default()
    }
}

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

#[macroquad::main(window_conf)]
async fn main() {
    let rom_file = load_rom_file(ROM_PATH);
    let window = window::Window::new(WINDOW_SCALE);
    let cpu = cpu::CPU::new(rom_file, &window);

    loop {
        clear_background(BLACK);
        cpu.frame();
        next_frame().await
    }
}
