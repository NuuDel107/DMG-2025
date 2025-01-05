use super::*;

/// The graphics processing unit
#[allow(clippy::upper_case_acronyms)]
pub struct PPU {
    pub dots: u32,
    pub vram: [u8; 0x2000],
    pub oam: [u8; 0xA0],
}

impl PPU {
    pub fn new() -> Self {
        Self {
            dots: 0,
            vram: [0; 0x2000],
            oam: [0; 0xA0],
        }
    }

    pub fn cycle(&mut self) {}
}

impl MemoryAccess for PPU {
    fn get_range(&self) -> Vec<RangeInclusive<u16>> {
        // VRAM, OAM
        vec![0x8000..=0x9FFF, 0xFE00..=0xFE9F]
    }

    fn mem_read(&self, address: u16) -> u8 {
        match address {
            0x8000..=0x9FFF => self.vram[(address - 0x8000) as usize],
            0xFE00..=0xFE9F => self.oam[(address - 0xFE00) as usize],
            _ => panic!(),
        }
    }
    fn mem_write(&mut self, address: u16, value: u8) {
        match address {
            0x8000..=0x9FFF => self.vram[(address - 0x8000) as usize] = value,
            0xFE00..=0xFE9F => self.oam[(address - 0xFE00) as usize] = value,
            _ => panic!(),
        }
    }
}
