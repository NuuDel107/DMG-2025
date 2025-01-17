use serde::{de::Visitor, ser::SerializeSeq};

use super::*;

#[derive(Deserialize, Serialize, Clone, Copy, PartialEq)]
pub struct PPUControl(u8);

#[derive(Deserialize, Serialize, Clone, Copy, PartialEq)]
pub struct STATEnable(u8);

#[derive(Deserialize, Serialize, Clone, Copy, PartialEq)]
pub struct SpriteFlags(u8);

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

    impl SpriteFlags: u8 {
        const PRIORITY = 0b1000_0000;
        const Y_FLIP   = 0b0100_0000;
        const X_FLIP   = 0b0010_0000;
        const PALETTE  = 0b0001_0000;
    }
}

#[derive(Deserialize, Serialize, Clone, Copy)]
pub struct OAMSprite {
    pub y: u8,
    pub x: u8,
    pub tile_index: u8,
    pub flags: SpriteFlags,
}

impl OAMSprite {
    pub fn from(data: [u8; 4]) -> Self {
        Self {
            y: data[0],
            x: data[1],
            tile_index: data[2],
            flags: SpriteFlags::from_bits_truncate(data[3]),
        }
    }
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Deserialize, Serialize)]
pub struct OAM {
    #[serde(with = "BigArray")]
    pub sprites: [OAMSprite; 40],
}

impl OAM {
    pub fn new() -> Self {
        Self {
            sprites: [OAMSprite::from([0; 4]); 40],
        }
    }

    pub fn read(&self, address: u16) -> u8 {
        let sprite = self.sprites[usize::from(address / 4)];
        match address % 4 {
            0 => sprite.y,
            1 => sprite.x,
            2 => sprite.tile_index,
            3 => sprite.flags.bits(),
            _ => unreachable!(),
        }
    }

    pub fn write(&mut self, address: u16, value: u8) {
        let mut sprite = self.sprites[usize::from(address / 4)];
        match address % 4 {
            0 => sprite.y = value,
            1 => sprite.x = value,
            2 => sprite.tile_index = value,
            3 => sprite.flags = SpriteFlags::from_bits_truncate(value),
            _ => unreachable!(),
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct DMGPalettes {
    bg: u8,
    obj0: u8,
    obj1: u8,
}

pub type DisplayMatrix = [[u8; 144]; 160];
const EMPTY_DISPLAY: DisplayMatrix = [[0; 144]; 160];

/// Serializes 2D display matrix as a 1D array
fn serialize_display<S>(display: &DisplayMatrix, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer
    {
        let mut seq = serializer.serialize_seq(Some(160*144))?;
        for x in display {
            for y in x {
                seq.serialize_element(y)?;
            }
        }
        seq.end()
    }

struct DisplayVisitor;
impl <'de> Visitor<'de> for DisplayVisitor {
    type Value = DisplayMatrix;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "an array with length = pixels in display (160*144)")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where A: serde::de::SeqAccess<'de>, {
        let mut display = EMPTY_DISPLAY;
        for x in 0..160 {
            for y in 0..144 {
                if let Ok(Some(pixel)) = seq.next_element() {
                    display[x][y] = pixel;
                }
                else {
                    return Err(serde::de::Error::custom("Display array too short!"));
                }
            }
        }
        Ok(display)
    }
}

/// Deserializes 2D display matrix from a 1D array
fn deserialize_display<'de, D>(deserializer: D) -> Result<DisplayMatrix, D::Error>
    where D: Deserializer<'de>
    {
        deserializer.deserialize_seq(DisplayVisitor)
    }

/// The graphics processing unit
#[allow(clippy::upper_case_acronyms)]
#[derive(Deserialize, Serialize)]
pub struct PPU {
    #[serde(serialize_with = "serialize_display")]
    #[serde(deserialize_with = "deserialize_display")]
    pub front_display: DisplayMatrix,
    #[serde(serialize_with = "serialize_display")]
    #[serde(deserialize_with = "deserialize_display")]
    pub back_display: DisplayMatrix,
    #[serde(with = "BigArray")]
    pub vram: [u8; 0x2000],
    pub oam: OAM,
    pub oam_dma_source: u8,
    pub oam_dma_timer: u16,
    pub control: PPUControl,
    pub lx: u16,
    pub ly: u8,
    pub bg_x: u8,
    pub bg_y: u8,
    pub win_x: u8,
    pub win_y: u8,
    pub palettes: DMGPalettes,
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
            oam: OAM::new(),
            oam_dma_source: 0,
            oam_dma_timer: 0,
            control: PPUControl::from_bits_truncate(0),
            lx: 0,
            ly: 0,
            bg_x: 0,
            bg_y: 0,
            win_x: 0,
            win_y: 0,
            palettes: DMGPalettes {
                bg: 0,
                obj0: 0,
                obj1: 0,
            },
            interrupt_request: InterruptFlag::from_bits_truncate(0),
            stat_enable: STATEnable::from_bits_truncate(0),
            mode: 2,
            lyc: 0,
        }
    }

    pub fn cycle(&mut self) {
        self.interrupt_request = InterruptFlag::from_bits_truncate(0);
        // Decrement simulated OAM DMA timer
        // When it is non-zero, CPU memory access should be limited
        if self.oam_dma_timer > 0 {
            self.oam_dma_timer -= 1;
        }

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
                    self.draw_scanline(self.ly)
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

    /// Get color value from given palette
    fn get_palette_color(&mut self, col_id: u8, palette: u8) -> u8 {
        (palette >> (2 * col_id)) & 0b11
    }

    /// Saves given color into the display buffer
    fn set_pixel(&mut self, x: u8, y: u8, col: u8) {
        self.back_display[x as usize][y as usize] = col;
    }

    /// Returns the tile index from specified tile map at specified coordinates
    fn get_tile_index(&self, x: u8, y: u8, tile_map: bool) -> u8 {
        // Get tile index in tile map
        let tile_map_index = u16::from(y / 8)
            .wrapping_mul(32)
            .wrapping_add((x / 8) as u16);
        // Get start position in memory of selected tile map
        let tile_map_root: u16 = if tile_map { 0x1C00 } else { 0x1800 };
        // Get index of tile data from tile map
        self.vram[usize::from(tile_map_root + tile_map_index)]
    }

    /// Returns the color ID of given tile at specified coordinates
    fn get_tile_color(&self, x: u8, y: u8, tile_index: u8, addressing_mode: bool) -> u8 {
        // Get memory position of tile inside VRAM (one tile is 16 bytes)
        // and add target row to it to get address of the two bytes
        // that make up a tile row of color data
        let byte_index = (16 * (tile_index as u16)) + (2 * ((y as u16) % 8));

        // When using alternative addressing mode, tile data at index < 128
        // are found at address 0x9000-0x97FF
        let tile_data_root = if addressing_mode && tile_index < 128 {
            0x1000
        } else {
            0x0000
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

    /// Returns list of sprites that occupy given scanline
    fn get_sprites(&self, y: u8) -> Vec<OAMSprite> {
        // Convert screen Y to object space,
        // where y = 0 completely hides the object
        let obj_y = y + 16;
        // Get object height based on current LCD control
        let height = if self.control.intersects(PPUControl::OBJ_SIZE) {
            16
        } else {
            8
        };

        let mut sprites: Vec<OAMSprite> = vec![];
        for sprite in self.oam.sprites {
            if obj_y < sprite.y.saturating_add(height) && obj_y >= sprite.y {
                sprites.push(sprite);
            }
            if sprites.len() == 10 {
                break;
            }
        }
        // Sort sprites by their x coordinate
        sprites.sort_by_key(|sprite| sprite.x);
        sprites
    }

    fn draw_scanline(&mut self, y: u8) {
        let sprites = self.get_sprites(y);
        for x in 0..=159u8 {
            let mut drawn_sprite: Option<&OAMSprite> = None;
            let mut sprite_col = 0u8;
            if self.control.intersects(PPUControl::OBJ_ENABLE) {
                // Convert screen X to object space
                let obj_x = x + 8;
                for sprite in &sprites {
                    if obj_x < sprite.x.saturating_add(8) && obj_x >= sprite.x {
                        let mut tile_x = (x as i16) - ((sprite.x as i16) - 8);
                        tile_x %= 8;
                        if sprite.flags.intersects(SpriteFlags::X_FLIP) {
                            tile_x = 7 - tile_x;
                        }

                        let mut tile_y = (y as i16) - ((sprite.y as i16) - 16);
                        tile_y %= 8;
                        if sprite.flags.intersects(SpriteFlags::Y_FLIP) {
                            tile_y = 7 - tile_y;
                        }

                        let col_id = self.get_tile_color(
                            tile_x as u8,
                            tile_y as u8,
                            sprite.tile_index,
                            false,
                        );
                        // With objects, color ID of 0 means transparent,
                        // so render background or window instead
                        if col_id != 0 {
                            drawn_sprite = Some(sprite);
                            let palette = if sprite.flags.intersects(SpriteFlags::PALETTE) {
                                self.palettes.obj1
                            } else {
                                self.palettes.obj0
                            };
                            sprite_col = self.get_palette_color(col_id, palette);
                            break;
                        }
                    }
                }
            }

            let mut use_bg_sprite = false;
            if let Some(sprite) = drawn_sprite {
                // If object priority flag is true,
                // background / window can be rendered on top of it
                if !sprite.flags.intersects(SpriteFlags::PRIORITY) {
                    self.set_pixel(x, y, sprite_col);
                    continue;
                }
                use_bg_sprite = true;
            }

            // If background and window are disabled, render object or just blank
            if !self.control.intersects(PPUControl::BG_WINDOW_ENABLE) {
                if use_bg_sprite {
                    self.set_pixel(x, y, sprite_col);
                } else {
                    let col = self.get_palette_color(0, self.palettes.bg);
                    self.set_pixel(x, y, col);
                }
                continue;
            }

            // Get window pixel instead of background if
            // window is enabled and pixel is inside window bounds
            let col_id = if self.control.intersects(PPUControl::WINDOW_ENABLE)
                && (x >= self.win_x || y >= self.win_y)
            {
                let tile =
                    self.get_tile_index(x, y, self.control.intersects(PPUControl::WINDOW_TILE_MAP));
                self.get_tile_color(
                    x,
                    y,
                    tile,
                    !self.control.intersects(PPUControl::TILE_DATA_AREA),
                )
            } else {
                let tile =
                    self.get_tile_index(x, y, self.control.intersects(PPUControl::BG_TILE_MAP));
                // Coordinates of background tiles may wrap around
                self.get_tile_color(
                    x.wrapping_add(self.bg_x),
                    y.wrapping_add(self.bg_y),
                    tile,
                    !self.control.intersects(PPUControl::TILE_DATA_AREA),
                )
            };
            // If pixel color ID is 0, render sprite instead
            if col_id == 0 && use_bg_sprite {
                self.set_pixel(x, y, sprite_col);
            }
            // Otherwise render background / window pixel
            else {
                let col = self.get_palette_color(col_id, self.palettes.bg);
                self.set_pixel(x, y, col);
            }
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
            0xFE00..=0xFE9F => self.oam.read(address - 0xFE00),
            0xFF40 => self.control.bits(),
            0xFF41 => {
                let lyc = ((self.lyc == self.ly) as u8) << 2;
                self.stat_enable.bits() | lyc | self.mode
            }
            0xFF42 => self.bg_y,
            0xFF43 => self.bg_x,
            0xFF44 => self.ly,
            0xFF45 => self.lyc,
            0xFF46 => self.oam_dma_source,
            0xFF47 => self.palettes.bg,
            0xFF48 => self.palettes.obj0,
            0xFF49 => self.palettes.obj1,
            0xFF4A => self.win_y,
            0xFF4B => self.win_x + 7,
            _ => 0xFF,
        }
    }
    fn mem_write(&mut self, address: u16, value: u8) {
        match address {
            0x8000..=0x9FFF => self.vram[(address - 0x8000) as usize] = value,
            0xFE00..=0xFE9F => self.oam.write(address - 0xFE00, value),
            0xFF40 => self.control = PPUControl::from_bits_truncate(value),
            0xFF41 => self.stat_enable = STATEnable::from_bits_truncate(value),
            0xFF42 => self.bg_y = value,
            0xFF43 => self.bg_x = value,
            0xFF45 => self.lyc = value,
            0xFF46 => {
                self.oam_dma_source = value;
                // Setting the timer activates OAM DMA transfer on CPU
                self.oam_dma_timer = 640;
            }
            0xFF47 => self.palettes.bg = value,
            0xFF48 => self.palettes.obj0 = value,
            0xFF49 => self.palettes.obj1 = value,
            0xFF4A => self.win_y = value,
            0xFF4B => self.win_x = value.saturating_sub(7),
            _ => {}
        }
    }
}
