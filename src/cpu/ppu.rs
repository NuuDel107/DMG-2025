use super::*;

#[derive(Clone, Copy, PartialEq)]
pub struct PPUControl(u8);

#[derive(Clone, Copy, PartialEq)]
pub struct STATEnable(u8);

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

    impl STATEnable: u8 {
        const LYC   = 0b0100_0000;
        const Mode2 = 0b0010_0000;
        const Mode1 = 0b0001_0000;
        const Mode0 = 0b0000_1000;
    }
}

pub type DisplayMatrix = [[u8; 144]; 160];
const EMPTY_DISPLAY: DisplayMatrix = [[0; 144]; 160];

/// The graphics processing unit
#[allow(clippy::upper_case_acronyms)]
pub struct PPU {
    pub front_display: DisplayMatrix,
    pub back_display: DisplayMatrix,
    pub vram: [u8; 0x2000],
    pub oam: [u8; 0xA0],
    pub control: PPUControl,
    pub lx: u16,
    pub ly: u8,
    pub bg_x: u8,
    pub bg_y: u8,
    pub win_x: u8,
    pub win_y: u8,
    pub palette: u8,
    pub interrupt_request: InterruptFlag,
    pub stat_enable: STATEnable,
    pub mode: u8,
    pub lyc: u8,
}

impl PPU {
    pub fn new() -> Self {
        Self {
            front_display: EMPTY_DISPLAY,
            back_display: EMPTY_DISPLAY,
            vram: [0; 0x2000],
            oam: [0; 0xA0],
            control: PPUControl::from_bits_truncate(0),
            lx: 0,
            ly: 0,
            bg_x: 0,
            bg_y: 0,
            win_x: 0,
            win_y: 0,
            palette: 0,
            interrupt_request: InterruptFlag::from_bits_truncate(0),
            stat_enable: STATEnable::from_bits_truncate(0),
            mode: 2,
            lyc: 0,
        }
    }

    pub fn cycle(&mut self) {
        self.interrupt_request = InterruptFlag::from_bits_truncate(0);

        if self.lx < 455 {
            self.lx += 1;
            if self.mode != 1 {
                if self.lx == 80 {
                    self.update_mode(3);
                } else if self.lx == 80 + 289 {
                    self.update_mode(0);
                }
            }
        } else {
            self.lx = 0;

            match self.ly {
                0..=143 => {
                    self.update_mode(2);
                    self.draw_horizontal(self.ly)
                }
                144 => {
                    self.interrupt_request.insert(InterruptFlag::VBLANK);
                    self.update_mode(1);
                    // VSync (very cool)
                    self.front_display = self.back_display;
                    self.back_display = EMPTY_DISPLAY;
                }
                153 => {
                    self.ly = 0;
                    self.update_mode(2);
                    return;
                }
                _ => {}
            }
            self.ly += 1;

            // Check for LYC=LY interrupt if its enabled
            if self.stat_enable.intersects(STATEnable::LYC) && self.lyc == self.ly {
                self.interrupt_request.insert(InterruptFlag::LCD);
            }
        }
    }

    fn update_mode(&mut self, mode: u8) {
        self.mode = mode;
        // Send interrupt if enabled
        if (mode == 2 && self.stat_enable.intersects(STATEnable::Mode2))
            || (mode == 1 && self.stat_enable.intersects(STATEnable::Mode1))
            || (mode == 0 && self.stat_enable.intersects(STATEnable::Mode0))
        {
            self.interrupt_request.insert(InterruptFlag::LCD);
        }
    }

    fn set_pixel(&mut self, x: u8, y: u8, col_id: u8) {
        // Get correct color value from the palette register
        let col = (self.palette >> (2 * col_id)) & 0b11;
        self.back_display[x as usize][y as usize] = col;
    }

    // Returns the color ID from current tile map at specified coordinates
    fn read_tile_map(&mut self, x: u8, y: u8, tile_map: bool) -> u8 {
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

        // Get color ID from the two bits
        (a as u8) | ((b as u8) << 1)
    }

    fn draw_horizontal(&mut self, y: u8) {
        for x in 0..=159u8 {
            // If background and window are disabled, return blank
            if !self.control.intersects(PPUControl::BG_WINDOW_ENABLE) {
                self.set_pixel(x, y, 0);
                return;
            }
            // Get window pixel instead of background if
            // window is enabled and pixel is inside window bounds
            let col_id = if self.control.intersects(PPUControl::WINDOW_ENABLE)
                && (x >= self.win_x || y >= self.win_y)
            {
                self.read_tile_map(x, y, self.control.intersects(PPUControl::WINDOW_TILE_MAP))
            } else {
                self.read_tile_map(
                    x.wrapping_add(self.bg_x),
                    y.wrapping_add(self.bg_y),
                    self.control.intersects(PPUControl::BG_TILE_MAP),
                )
            };
            self.set_pixel(x, y, col_id);
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
            0xFF41 => {
                let lyc = ((self.lyc == self.ly) as u8) << 2;
                self.stat_enable.bits() & lyc & self.mode
            }
            0xFF42 => self.bg_y,
            0xFF43 => self.bg_x,
            0xFF44 => self.ly,
            0xFF45 => self.lyc,
            0xFF47 => self.palette,
            0xFF4A => self.win_y,
            0xFF4B => self.win_x + 7,
            _ => 0,
        }
    }
    fn mem_write(&mut self, address: u16, value: u8) {
        match address {
            0x8000..=0x9FFF => self.vram[(address - 0x8000) as usize] = value,
            0xFE00..=0xFE9F => self.oam[(address - 0xFE00) as usize] = value,
            0xFF40 => self.control = PPUControl::from_bits_truncate(value),
            0xFF41 => self.stat_enable = STATEnable::from_bits_truncate(value),
            0xFF42 => self.bg_y = value,
            0xFF43 => self.bg_x = value,
            0xFF45 => self.lyc = value,
            0xFF47 => self.palette = value,
            0xFF4A => self.win_y = value,
            0xFF4B => self.win_x = value.saturating_sub(7),
            _ => {}
        }
    }
}
