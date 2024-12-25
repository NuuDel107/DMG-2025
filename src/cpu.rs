/// The main processing unit
pub struct CPU<'r> {
    renderer: &'r dyn Renderer,

    // Registers
    af: u16,
    bc: u16,
    de: u16,
    hl: u16,
    sp: u16,
    pc: u16,

    vram: [u8; 8096],
    wram: [u8; 4096],
}

impl CPU<'_> {
    pub fn init(renderer: &dyn Renderer) -> CPU {
        CPU {
            renderer,
            af: 0,
            bc: 0,
            de: 0,
            hl: 0,
            sp: 0,
            pc: 0,
            vram: [0; 8096],
            wram: [0; 4096],
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
