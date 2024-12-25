use macroquad::prelude::*;
use miniquad::conf::Icon;

fn window_conf() -> Conf {
    const SCALE: i32 = 4;
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
        window_width: 160 * SCALE,
        window_height: 144 * SCALE,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    loop {
        clear_background(BLACK);
        next_frame().await
    }
}
