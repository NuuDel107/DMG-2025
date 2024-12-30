#[derive(Clone, Copy)]
pub enum Reg8 {
    A,
    F,
    B,
    C,
    D,
    E,
    H,
    L,
}

#[derive(Clone, Copy)]
pub enum Reg16 {
    AF,
    BC,
    DE,
    HL,
    SP,
    PC,
}

#[derive(Clone, Copy)]
pub struct FlagRegister {
    pub zero: bool,
    pub subtract: bool,
    pub half_carry: bool,
    pub carry: bool,
}

impl From<u8> for FlagRegister {
    fn from(value: u8) -> Self {
        Self {
            zero: (value & 0b10000000) > 0,
            subtract: (value & 0b01000000) > 0,
            half_carry: (value & 0b00100000) > 0,
            carry: (value & 0b00010000) > 0,
        }
    }
}

impl From<FlagRegister> for u8 {
    fn from(reg: FlagRegister) -> Self {
        let mut value = 0;
        if reg.zero {
            value |= 0b10000000;
        }
        if reg.subtract {
            value |= 0b01000000;
        }
        if reg.half_carry {
            value |= 0b00100000;
        }
        if reg.carry {
            value |= 0b00010000;
        }
        value
    }
}

pub struct Memory {
    // Registers
    pub a: u8,
    pub f: FlagRegister,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub sp: u16,
    pub pc: u16,

    // Memory
    pub rom: [u8; 0x4000],
    pub bank: [u8; 0x4000],
    pub vram: [u8; 0x2000],
    pub wram: [u8; 0x2000],
    pub oam: [u8; 0xA0],
    pub hram: [u8; 0x7F],
}

impl Memory {
    pub fn new(rom_file: Vec<u8>) -> Self {
        Self {
            a: 0,
            f: 0.into(),
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            h: 0,
            l: 0,
            sp: 0xFFFE,
            pc: 0x0100,
            rom: Self::load_range::<0x4000>(&rom_file, 0x0000),
            bank: Self::load_range::<0x4000>(&rom_file, 0x4000),
            vram: [0; 0x2000],
            wram: [0; 0x2000],
            oam: [0; 0xA0],
            hram: [0; 0x7F],
        }
    }

    pub fn load_range<const SIZE: usize>(rom: &[u8], start: usize) -> [u8; SIZE] {
        let mut range: [u8; SIZE] = [0; SIZE];
        range[..SIZE].copy_from_slice(&rom[start..(SIZE + start)]);
        range
    }

    pub fn read_mem(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x3FFF => self.rom[address as usize],
            0x4000..=0x7FFF => self.bank[(address - 0x4000) as usize],
            0x8000..=0x9FFF => self.vram[(address - 0x8000) as usize],
            0xC000..=0xDFFF => self.wram[(address - 0xC000) as usize],
            0xFE00..=0xFE9F => self.oam[(address - 0xFE00) as usize],
            0xFF80..=0xFFFE => self.hram[(address - 0xFF80) as usize],
            _ => todo!(),
        }
    }

    pub fn read_mem_16(&self, address: u16) -> u16 {
        u16::from_le_bytes([self.read_mem(address), self.read_mem(address + 1)])
    }

    pub fn read_reg(&self, register: &Reg8) -> u8 {
        match register {
            Reg8::A => self.a,
            Reg8::F => self.f.into(),
            Reg8::B => self.b,
            Reg8::C => self.c,
            Reg8::D => self.d,
            Reg8::E => self.e,
            Reg8::H => self.h,
            Reg8::L => self.l,
        }
    }

    pub fn read_reg_16(&self, register: &Reg16) -> u16 {
        match register {
            Reg16::AF => u16::from_be_bytes([self.a, self.f.into()]),
            Reg16::BC => u16::from_be_bytes([self.b, self.c]),
            Reg16::DE => u16::from_be_bytes([self.d, self.e]),
            Reg16::HL => u16::from_be_bytes([self.h, self.l]),
            Reg16::SP => self.sp,
            Reg16::PC => self.pc,
        }
    }

    /// Returns the immediate 8-bit operand from memory and increments program counter
    pub fn read_operand(&mut self) -> u8 {
        self.pc += 1;
        self.read_mem(self.pc)
    }

    /// Returns the 16-bit immediate operand from memory and increments program counter
    pub fn read_operand_16(&mut self) -> u16 {
        self.pc += 2;
        self.read_mem_16(self.pc - 1)
    }

    pub fn write_mem(&mut self, address: u16, value: u8) {
        match address {
            0x0000..0x4000 => self.rom[address as usize] = value,
            0x4000..0x8000 => self.bank[(address - 0x4000) as usize] = value,
            0x8000..0xA000 => self.vram[(address - 0x8000) as usize] = value,
            0xC000..=0xDFFF => self.wram[(address - 0xC000) as usize] = value,
            0xFE00..=0xFE9F => self.oam[(address - 0xFE00) as usize] = value,
            0xFF80..=0xFFFE => self.hram[(address - 0xFF80) as usize] = value,
            _ => todo!(),
        }
    }

    pub fn write_reg(&mut self, register: &Reg8, value: u8) {
        match register {
            Reg8::A => self.a = value,
            Reg8::F => self.f = value.into(),
            Reg8::B => self.b = value,
            Reg8::C => self.c = value,
            Reg8::D => self.d = value,
            Reg8::E => self.e = value,
            Reg8::H => self.h = value,
            Reg8::L => self.l = value,
        }
    }

    pub fn write_reg_16(&mut self, register: &Reg16, value: u16) {
        match register {
            Reg16::AF => {
                let bytes = value.to_be_bytes();
                self.a = bytes[0];
                self.f = bytes[1].into();
            }
            Reg16::BC => {
                let bytes = value.to_be_bytes();
                self.b = bytes[0];
                self.c = bytes[1];
            }
            Reg16::DE => {
                let bytes = value.to_be_bytes();
                self.d = bytes[0];
                self.e = bytes[1];
            }
            Reg16::HL => {
                let bytes = value.to_be_bytes();
                self.h = bytes[0];
                self.l = bytes[1];
            }
            Reg16::SP => self.sp = value,
            Reg16::PC => self.pc = value,
        }
    }

    // Pops word from memory stack and increments stack pointer
    pub fn pop(&mut self) -> u16 {
        let val = self.read_mem_16(self.sp);
        self.sp += 2;
        val
    }

    // Pushes word into memory stack and decrements stack pointer
    pub fn push(&mut self, value: u16) {
        let bytes = value.to_le_bytes();
        self.write_mem(self.sp - 1, bytes[0]);
        self.write_mem(self.sp - 2, bytes[1]);
        self.sp -= 2;
    }
}
