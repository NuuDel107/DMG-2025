use super::*;

/// Shared behavior and access points between MBC cartridges
#[allow(clippy::upper_case_acronyms)]
pub trait MBC {
    fn read(&self, address: u16) -> u8;
    fn write(&mut self, address: u16, value: u8);
}

pub struct NoMBC {
    pub rom: Vec<u8>,
    pub ram: Vec<u8>,
}

impl NoMBC {
    pub fn init(rom_file: Vec<u8>, info: CartridgeInfo) -> Self {
        Self {
            rom: rom_file,
            ram: vec![0; usize::from(0x2000 * info.ram_banks)],
        }
    }
}

impl MBC for NoMBC {
    fn read(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x7FFF => self.rom[address as usize],
            0xA000..=0xBFFF => self.ram[(address - 0xA000) as usize],
            _ => 0,
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        if !self.ram.is_empty() && address >= 0xA000 {
            self.ram[(address - 0xA000) as usize] = value;
        }
    }
}

pub struct MBC1 {
    rom: Vec<u8>,
    rom_bank: u8,
    ram: Vec<u8>,
    ram_bank: u8,
    ram_enabled: bool,
    advanced_banking: bool,
    info: CartridgeInfo,
}

impl MBC1 {
    pub fn init(rom_file: Vec<u8>, info: CartridgeInfo) -> Self {
        Self {
            rom: rom_file,
            rom_bank: 1,
            ram: vec![0; usize::from(0x2000 * info.ram_banks)],
            ram_bank: 0,
            ram_enabled: false,
            advanced_banking: false,
            info,
        }
    }
}

impl MBC for MBC1 {
    fn read(&self, address: u16) -> u8 {
        let mut address = address as usize;
        match address {
            0x0000..=0x7FFF => {
                // The ROM bank register is only applied
                // to the second ROM address range ($4000-$7FFF)
                if address >= 0x4000 {
                    address -= 0x4000;
                    address += (self.rom_bank as usize) * 0x4000;
                }
                // If cartridge has >512 KiB ROM, the 2-bit register that is also used to select RAM banks
                // can be used to select one of four large banks of 512 KiB memory
                // It is also applied to the first address range if using advanced banking mode
                if self.info.rom_banks > 32 && (address >= 0x4000 || self.advanced_banking) {
                    // Mask out upper bit of high address if not enough banks
                    let high_address = self.ram_bank
                        & if self.info.rom_banks <= 64 {
                            0b01
                        } else {
                            0b11
                        };
                    address += 0x20 * high_address as usize * 0x4000
                }

                if self.rom.len() <= address {
                    eprintln!(
                        "Tried to access ROM at {:#06X}, but length is only {:#06X}",
                        address,
                        self.rom.len()
                    );
                    return 0;
                }
                self.rom[address]
            }
            0xA000..=0xBFFF => {
                // Reads to disabled RAM usually return 0xFF
                if !self.ram_enabled {
                    return 0xFF;
                }
                address -= 0xA000;
                // RAM banks can only be changed when using advanced banking mode
                if self.advanced_banking && self.info.ram_banks > 1 {
                    // Mask out upper bit of bank number if not enough banks
                    let bank = self.ram_bank & if self.info.ram_banks <= 2 { 0b01 } else { 0b11 };
                    address += bank as usize * 0x2000;
                }

                if self.ram.len() <= address {
                    eprintln!(
                        "Tried to access external RAM at {:#06X}, but RAM size is only {:#06X}",
                        address,
                        self.ram.len()
                    );
                    return 0;
                }
                self.ram[address]
            }
            _ => 0,
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        match address {
            // Enable the RAM
            0x0000..=0x1FFF => self.ram_enabled = (value & 0x0F) == 0x0A,
            // Less significant ROM bank register
            0x2000..=0x3FFF => {
                // Only needed amount of bits to change between all ROM banks
                // are saved to the register, rest are masked out
                let mask = if self.info.rom_banks <= 4 {
                    0b11
                } else if self.info.rom_banks <= 8 {
                    0b111
                } else if self.info.rom_banks <= 16 {
                    0b1111
                } else {
                    0b1_1111
                };
                let mut masked = value & mask;

                // If register is tried to set to 0, it should be incremented to 1
                // The check is only done for the 5-bit version for the value though,
                // so for example if only 3 bits are used,
                // register value 0b1000 maps the second block to ROM bank 0
                if value & 0b1_1111 == 0 {
                    masked = 1
                };
                self.rom_bank = masked;
            }
            // 2 bit bank register that is used to select both ROM and RAM banks
            0x4000..=0x5FFF => self.ram_bank = value & 0b0000_0011,
            // Toggle between banking modes
            // The above register only has effect if this is set to true
            0x6000..=0x7FFF => self.advanced_banking = value & 0b1 > 0,
            // Write to RAM
            0xA000..=0xBFFF => {
                if !self.ram_enabled {
                    return;
                }
                let mut address = address as usize;
                address -= 0xA000;
                // RAM banks can only be changed when using advanced banking mode
                if self.advanced_banking && self.info.ram_banks > 1 {
                    address += self.ram_bank as usize * 0x2000;
                }

                if self.ram.len() <= address {
                    eprintln!(
                        "Tried to write {:#04X} into external RAM at {:#06X}, but RAM size is only {:#06X}",
                        value,
                        address,
                        self.ram.len()
                    );
                    return;
                }
                self.ram[address] = value;
            }
            _ => {}
        };
    }
}

pub struct MBC3 {
    rom: Vec<u8>,
    rom_bank: u8,
    ram: Vec<u8>,
    ram_bank: u8,
    ram_enabled: bool,
    info: CartridgeInfo,
}

impl MBC3 {
    pub fn init(rom_file: Vec<u8>, info: CartridgeInfo) -> Self {
        Self {
            rom: rom_file,
            rom_bank: 1,
            ram: vec![0; usize::from(0x2000 * info.ram_banks)],
            ram_bank: 0,
            ram_enabled: false,
            info,
        }
    }
}

impl MBC for MBC3 {
    fn read(&self, address: u16) -> u8 {
        let mut address = address as usize;
        match address {
            0x0000..=0x3FFF => {
                self.rom[address]
            }
            0x4000..=0x7FFF => {
                address += 0x4000 * ((self.rom_bank as usize) - 1);

                if self.rom.len() <= address {
                    eprintln!(
                        "Tried to access ROM at {:#06X}, but length is only {:#06X}",
                        address,
                        self.rom.len()
                    );
                    return 0;
                }
                self.rom[address]
            }
            0xA000..=0xBFFF => {
                if !self.ram_enabled {
                    return 0xFF;
                }
                address -= 0xA000;
                address += self.ram_bank as usize * 0x2000;

                if self.ram.len() <= address {
                    eprintln!(
                        "Tried to access external RAM at {:#06X}, but RAM size is only {:#06X}",
                        address,
                        self.ram.len()
                    );
                    return 0xFF;
                }
                self.ram[address]
            }
            _ => 0,
        }
    }

    fn write(&mut self, address: u16, value: u8) {
        match address {
            // Enable the RAM
            0x0000..=0x1FFF => self.ram_enabled = (value & 0x0F) == 0x0A,
            // ROM bank register
            0x2000..=0x3FFF => {
                // Only needed amount of bits to change between all ROM banks
                // are saved to the register, rest are masked out
                let mask = if self.info.rom_banks <= 4 {
                    0b11
                } else if self.info.rom_banks <= 8 {
                    0b111
                } else if self.info.rom_banks <= 16 {
                    0b1111
                } else if self.info.rom_banks <= 32 {
                    0b1_1111
                } else if self.info.rom_banks <= 64 {
                    0b11_1111
                } else {
                    0b111_1111
                };
                let mut masked = value & mask;

                if masked == 0 {
                    masked = 1
                };
                self.rom_bank = masked;
            }
            // 2 bit bank register that is used to select both ROM and RAM banks
            0x4000..=0x5FFF => {
                let mask = if self.info.ram_banks >= 1 {
                    0
                } else if self.info.ram_banks == 2 {
                    0b1
                } else {
                    0b11
                };
                self.ram_bank = value & mask
            },
            // Write to RAM
            0xA000..=0xBFFF => {
                if !self.ram_enabled {
                    return;
                }
                let mut address = address as usize;
                address -= 0xA000;
                address += self.ram_bank as usize * 0x2000;
                
                if self.ram.len() <= address {
                    eprintln!(
                        "Tried to write {:#04X} into external RAM at {:#06X}, but RAM size is only {:#06X}",
                        value,
                        address,
                        self.ram.len()
                    );
                    return;
                }
                self.ram[address] = value;
            }
            _ => {}
        };
    }
}
