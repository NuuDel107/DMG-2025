use macroquad::prelude::*;
use miniquad::conf::Icon;

mod cpu;
mod window;

const WINDOW_SCALE: u8 = 4;

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

#[macroquad::main(window_conf)]
async fn main() {
    let window = window::Window::new(WINDOW_SCALE);
    let cpu = cpu::CPU::init(&window);

    loop {
        clear_background(BLACK);
        cpu.frame();
        next_frame().await
    }
}
