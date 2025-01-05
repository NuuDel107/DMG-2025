use super::*;

#[derive(Clone, Copy, PartialEq)]
pub struct PPUControl(u8);

bitflags! {
    impl PPUControl: u8 {
        const ENABLE              = 0b1000_0000;
        const WINDOW_TILE_MAP     = 0b0100_0000;
        const WINDOW_ENABLE       = 0b0010_0000;
        const TILE_DATA_AREA      = 0b0001_0000;
        const BG_TILE_MAP         = 0b0000_1000;
        const OBJ_SIZE            = 0b0000_0100;
        const OBJ_ENABLE          = 0b0000_0010;
        const BG_WINDOW_ENABLE    = 0b0000_0001;
    }
}

pub type DisplayMatrix = [[u8; 144]; 160];
const EMPTY_DISPLAY: DisplayMatrix = [[0; 144]; 160];

/// The graphics processing unit
#[allow(clippy::upper_case_acronyms)]
pub struct PPU {
    pub front_display: DisplayMatrix,
    pub back_display: DisplayMatrix,
    pub dot_x: u16,
    pub dot_y: u8,
    pub control: PPUControl,
    pub viewport_x: u8,
    pub viewport_y: u8,
    pub vram: [u8; 0x2000],
    pub oam: [u8; 0xA0],
    pub interrupt_request: InterruptFlag,
}

impl PPU {
    pub fn new() -> Self {
        Self {
            front_display: EMPTY_DISPLAY,
            back_display: EMPTY_DISPLAY,
            dot_x: 0,
            dot_y: 0,
            control: PPUControl::from_bits_truncate(0),
            viewport_x: 0,
            viewport_y: 0,
            vram: [0; 0x2000],
            oam: [0; 0xA0],
            interrupt_request: InterruptFlag::from_bits_truncate(0),
        }
    }

    pub fn cycle(&mut self) {
        self.interrupt_request = InterruptFlag::from_bits_truncate(0);

        if self.dot_x < 455 {
            self.dot_x += 1;
        } else {
            self.dot_x = 0;

            match self.dot_y {
                0..=143 => self.draw_horizontal(self.dot_y),
                144 => {
                    self.interrupt_request = InterruptFlag::VBLANK;
                    // VSync (very cool)
                    self.front_display = self.back_display;
                    self.back_display = EMPTY_DISPLAY;
                }
                153 => {
                    self.dot_y = 0;
                    return;
                }
                _ => {}
            }
            self.dot_y += 1;
        }
    }

    fn set_pixel(&mut self, x: u8, y: u8, color: u8) {
        self.back_display[x as usize][y as usize] = color;
    }

    // Returns the color value from current tile map at specified coordinates
    fn get_color(&mut self, x: u8, y: u8, tile_map: bool) -> u8 {
        // Get tile index in tile map
        let tile_map_index = u16::from(y / 8)
            .wrapping_mul(32)
            .wrapping_add((x / 8) as u16);
        // Get start position in memory of selected tile map
        let tile_map_root: u16 = if tile_map { 0x1C00 } else { 0x1800 };
        // Get index of tile data from tile map
        let tile_index = self.vram[usize::from(tile_map_root + tile_map_index)];

        // Get memory position of tile inside VRAM (one tile is 16 bytes)
        // and add target row to it to get address of the two bytes
        // that make up a tile row of color data
        let byte_index = (16 * (tile_index as u16)) + (2 * ((y as u16) % 8));

        // Get start position in memory of selected tile data block
        let tile_data_root = if self.control.intersects(PPUControl::TILE_DATA_AREA) {
            0x0000
        } else {
            0x0800
        };
        // Get the color bytes from VRAM
        let a_byte = self.vram[usize::from(tile_data_root + byte_index)];
        let b_byte = self.vram[usize::from(tile_data_root + byte_index + 1)];
        // Get the bit values of correct tile column
        let a = a_byte & (0b1000_0000 >> (x % 8)) != 0;
        let b = b_byte & (0b1000_0000 >> (x % 8)) != 0;

        // Get color value from the two boolean values
        // (0 = white, 3 = black)
        if a && b {
            3
        } else if a {
            1
        } else if b {
            2
        } else {
            0
        }
    }

    fn draw_horizontal(&mut self, y: u8) {
        for x in 0..=159u8 {
            let col = self.get_color(
                x.wrapping_add(self.viewport_x),
                y.wrapping_add(self.viewport_y),
                self.control.intersects(PPUControl::BG_TILE_MAP),
            );
            self.set_pixel(x, y, col);
        }
    }
}

impl MemoryAccess for PPU {
    fn get_range(&self) -> Vec<RangeInclusive<u16>> {
        // VRAM, OAM, LCD I/O
        vec![0x8000..=0x9FFF, 0xFE00..=0xFE9F, 0xFF40..=0xFF4B]
    }

    fn mem_read(&self, address: u16) -> u8 {
        match address {
            0x8000..=0x9FFF => self.vram[(address - 0x8000) as usize],
            0xFE00..=0xFE9F => self.oam[(address - 0xFE00) as usize],
            0xFF40 => self.control.bits(),
            0xFF42 => self.viewport_y,
            0xFF43 => self.viewport_x,
            0xFF44 => self.dot_y,
            0xFF40..=0xFF4B => 0,
            _ => panic!(),
        }
    }
    fn mem_write(&mut self, address: u16, value: u8) {
        match address {
            0x8000..=0x9FFF => self.vram[(address - 0x8000) as usize] = value,
            0xFE00..=0xFE9F => self.oam[(address - 0xFE00) as usize] = value,
            0xFF40 => self.control = PPUControl::from_bits_truncate(value),
            0xFF42 => self.viewport_y = value,
            0xFF43 => self.viewport_x = value,
            0xFF40..=0xFF4B => {}
            _ => panic!(),
        }
    }
}
