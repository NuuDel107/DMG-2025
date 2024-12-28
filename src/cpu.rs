mod memory;
mod opcodes;

use memory::*;

/// The main processing unit
pub struct CPU<'r> {
    mem: Memory,
    renderer: &'r dyn Renderer,
}

impl CPU<'_> {
    pub fn new(rom_file: Vec<u8>, renderer: &dyn Renderer) -> CPU {
        CPU {
            mem: Memory::new(rom_file),
            renderer,
        }
    }

    pub fn frame(&self) {
        self.renderer.draw_tile(
            0,
            0,
            [
                0xFF, 0x00, 0x7E, 0xFF, 0x85, 0x81, 0x89, 0x83, 0x93, 0x85, 0xA5, 0x8B, 0xC9, 0x97,
                0x7E, 0xFF,
            ],
        );
    }
}

/// Trait for a general renderer
pub trait Renderer {
    fn draw_tile(&self, x: u8, y: u8, tile_data: [u8; 16]);
}
