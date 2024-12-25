use super::cpu::Renderer;
use macroquad::prelude::*;

pub struct Window {
    window_scale: u8,
}

impl Window {
    pub fn new(window_scale: u8) -> Window {
        Window { window_scale }
    }

    fn draw_pixel(&self, x: u8, y: u8, color: Color) {
        draw_rectangle(
            (u16::from(x) * u16::from(self.window_scale)).into(),
            (u16::from(y) * u16::from(self.window_scale)).into(),
            self.window_scale.into(),
            self.window_scale.into(),
            color,
        );
    }
}

impl Renderer for Window {
    fn draw_tile(&self, x: u8, y: u8, tile_data: [u8; 16]) {
        for tile_y in 0..8 {
            for tile_x in 0..8 {
                let a = tile_data[usize::from(tile_y * 2)].reverse_bits() & (1 << tile_x) != 0;
                let b = tile_data[usize::from(tile_y * 2 + 1)].reverse_bits() & (1 << tile_x) != 0;

                let mut color = WHITE;
                if a && b {
                    color = BLACK;
                } else if a {
                    color = GRAY;
                } else if b {
                    color = DARKGRAY;
                }

                self.draw_pixel(x + tile_x, y + tile_y, color);
            }
        }
    }
}
