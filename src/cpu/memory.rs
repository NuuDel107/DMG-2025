use super::*;

pub struct Memory {
    pub rom: [u8; 0x4000],
    pub bank: [u8; 0x4000],
    pub wram: [u8; 0x2000],
    pub hram: [u8; 0x7F],
}

impl Memory {
    pub fn new(rom_file: Vec<u8>) -> Self {
        Self {
            rom: Self::load_range::<0x4000>(&rom_file, 0x0000),
            bank: Self::load_range::<0x4000>(&rom_file, 0x4000),
            wram: [0; 0x2000],
            hram: [0; 0x7F],
        }
    }

    pub fn load_range<const SIZE: usize>(rom: &[u8], start: usize) -> [u8; SIZE] {
        let mut range: [u8; SIZE] = [0; SIZE];
        range[..SIZE].copy_from_slice(&rom[start..(SIZE + start)]);
        range
    }
}

impl MemoryAccess for Memory {
    fn get_range(&self) -> Vec<RangeInclusive<u16>> {
        // ROM, external RAM, work RAM, high RAM
        vec![
            0x0000..=0x7FFF,
            0xA000..=0xBFFF,
            0xC000..=0xDFFF,
            0xFF80..=0xFFFE,
        ]
    }
    fn mem_write(&mut self, address: u16, value: u8) {
        match address {
            0xA000..=0xBFFF => println!("External RAM write at {:#06X}", address),
            0xC000..=0xDFFF => self.wram[(address - 0xC000) as usize] = value,
            0xFF80..=0xFFFE => self.hram[(address - 0xFF80) as usize] = value,

            // _ => todo!("{:#06X}", address),
            _ => eprintln!("Memory writing not implemented for {:#06X}", address),
        }
    }

    fn mem_read(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x3FFF => self.rom[address as usize],
            0x4000..=0x7FFF => self.bank[(address - 0x4000) as usize],
            0xC000..=0xDFFF => self.wram[(address - 0xC000) as usize],
            0xFF80..=0xFFFE => self.hram[(address - 0xFF80) as usize],
            // _ => todo!("{:#06X}", address),
            _ => {
                eprintln!("Memory reading not implemented for {:#06X}", address);
                0
            }
        }
    }
}
