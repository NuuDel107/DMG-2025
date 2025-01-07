use super::*;

#[derive(Debug, Clone, Copy)]
pub enum MBCType {
    NoMBC,
    MBC1,
    MBC2,
    MMM01,
    MBC3,
    MBC5,
    MBC6,
    MBC7,
}

#[derive(Debug, Clone, Copy)]
pub struct CartridgeInfo {
    /// Type of memory bank controller
    pub mbc: MBCType,
    /// If cartridge provides external RAM
    pub has_ram: bool,
    /// If cartridge has battery, meaning it can store external RAM in itself
    /// (a.k.a. saving is possible)
    pub has_battery: bool,
    /// Amount of 16 KiB ROM banks cartridge provides
    pub rom_banks: u16,
    /// Amount of 8 KiB RAM banks cartridge provides
    pub ram_banks: u16,
}

impl CartridgeInfo {
    /// Returns info about cartridge features from the ROM header
    pub fn from_header(header: &[u8]) -> Self {
        let mbc = match header[0x47] {
            0x01..=0x03 => MBCType::MBC1,
            0x05..=0x06 => MBCType::MBC2,
            0x0B..=0x0D => MBCType::MMM01,
            0x0F..=0x13 => MBCType::MBC3,
            0x19..=0x1E => MBCType::MBC5,
            0x20 => MBCType::MBC6,
            0x22 => MBCType::MBC7,
            _ => MBCType::NoMBC,
        };
        let has_ram = matches!(
            header[0x47],
            0x02 | 0x03 | 0x0C | 0x0D | 0x10 | 0x12 | 0x13 | 0x1A | 0x1B | 0x1D | 0x1E | 0x22
        );
        let has_battery = matches!(
            header[0x47],
            0x03 | 0x06 | 0x0D | 0x0F | 0x10 | 0x13 | 0x1B | 0x1E | 0x22
        );
        let rom_banks = 2u16.pow((1 + header[0x48]) as u32);
        let ram_banks = if !has_ram {
            0
        } else {
            match header[0x49] {
                0x02 => 1,
                0x03 => 4,
                0x04 => 16,
                0x05 => 8,
                _ => 0,
            }
        };
        Self {
            mbc,
            has_ram,
            has_battery,
            rom_banks,
            ram_banks,
        }
    }
}

pub struct Memory {
    pub wram: [u8; 0x2000],
    pub hram: [u8; 0x7F],
    pub info: CartridgeInfo,
    pub mbc: Box<dyn MBC + Send + Sync>,
}

impl Memory {
    pub fn new(rom_file: Vec<u8>) -> Self {
        let info = CartridgeInfo::from_header(&rom_file[0x0100..=0x014F]);
        println!("{:?}", info);
        let mbc: Box<dyn MBC + Send + Sync> = match info.mbc {
            MBCType::NoMBC => Box::new(NoMBC::init(rom_file, info)),
            MBCType::MBC1 => Box::new(MBC1::init(rom_file, info)),
            _ => todo!("MBC type not supported"),
        };

        Self {
            wram: [0; 0x2000],
            hram: [0; 0x7F],
            info,
            mbc,
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
        // ROM, external and work RAM, high RAM
        vec![0x0000..=0x7FFF, 0xA000..=0xDFFF, 0xFF80..=0xFFFE]
    }
    fn mem_read(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x7FFF | 0xA000..=0xBFFF => self.mbc.read(address),
            0xC000..=0xDFFF => self.wram[(address - 0xC000) as usize],
            0xFF80..=0xFFFE => self.hram[(address - 0xFF80) as usize],
            _ => {
                eprintln!("Memory reading not implemented for {:#06X}", address);
                0
            }
        }
    }
    fn mem_write(&mut self, address: u16, value: u8) {
        match address {
            0x0000..=0x7FFF | 0xA000..=0xBFFF => self.mbc.write(address, value),
            0xC000..=0xDFFF => self.wram[(address - 0xC000) as usize] = value,
            0xFF80..=0xFFFE => self.hram[(address - 0xFF80) as usize] = value,
            _ => eprintln!(
                "Memory writing not implemented for {:#06X}. Tried to write {:#04X}",
                address, value
            ),
        }
    }
}
