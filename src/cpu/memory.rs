use super::IO;
use bitflags::bitflags;

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

#[derive(Clone, Copy, PartialEq)]
pub struct InterruptFlag(u8);
#[derive(Clone, Copy, PartialEq)]
pub struct FlagReg(u8);

bitflags! {
    impl InterruptFlag: u8 {
        const JOYPAD = 0b0001_0000;
        const SERIAL = 0b0000_1000;
        const TIMER  = 0b0000_0100;
        const LCD    = 0b0000_0010;
        const VBLANK = 0b0000_0001;
    }

    impl FlagReg: u8 {
        const ZERO       = 0b1000_0000;
        const SUBTRACT   = 0b0100_0000;
        const HALF_CARRY = 0b0010_0000;
        const CARRY      = 0b0001_0000;
    }
}

pub struct Memory {
    pub io: IO,
    // Registers
    pub a: u8,
    pub f: FlagReg,
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

    // Interrupts
    pub ime: bool,
    pub iflag: InterruptFlag,
    pub ie: InterruptFlag,
}

impl Memory {
    pub fn new(rom_file: Vec<u8>) -> Self {
        Self {
            io: IO::new(),
            a: 0x01,
            f: FlagReg::from_bits_truncate(0xB0),
            b: 0,
            c: 0x13,
            d: 0x00,
            e: 0xD8,
            h: 0x01,
            l: 0x4D,
            sp: 0xFFFE,
            pc: 0x0100,

            rom: Self::load_range::<0x4000>(&rom_file, 0x0000),
            bank: Self::load_range::<0x4000>(&rom_file, 0x4000),
            vram: [0; 0x2000],
            wram: [0; 0x2000],
            oam: [0; 0xA0],
            hram: [0; 0x7F],

            ime: false,
            iflag: InterruptFlag::from_bits_truncate(0),
            ie: InterruptFlag::from_bits_truncate(0),
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

            0xFF00..=0xFF0E | 0xFF10..=0xFF7F => self.io.read(address),
            0xFF0F => self.iflag.bits(),
            0xFFFF => self.ie.bits(),
            // _ => todo!("{:#06X}", address),
            _ => {
                eprintln!("Memory reading not implemented for {:#06X}", address);
                0
            }
        }
    }

    pub fn read_mem_16(&self, address: u16) -> u16 {
        u16::from_le_bytes([self.read_mem(address), self.read_mem(address + 1)])
    }

    pub fn read_reg(&self, register: &Reg8) -> u8 {
        match register {
            Reg8::A => self.a,
            Reg8::F => self.f.bits(),
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
            Reg16::AF => u16::from_be_bytes([self.a, self.f.bits()]),
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
            0x8000..=0x9FFF => self.vram[(address - 0x8000) as usize] = value,
            0xA000..=0xBFFF => println!("External RAM write at {:#06X}", address),
            0xC000..=0xDFFF => self.wram[(address - 0xC000) as usize] = value,
            0xFE00..=0xFE9F => self.oam[(address - 0xFE00) as usize] = value,
            0xFF80..=0xFFFE => self.hram[(address - 0xFF80) as usize] = value,

            0xFF00..=0xFF0E | 0xFF10..=0xFF7F => self.io.write(address, value),
            0xFF0F => self.iflag = InterruptFlag::from_bits_truncate(value),
            0xFFFF => self.ie = InterruptFlag::from_bits_truncate(value),
            // _ => todo!("{:#06X}", address),
            _ => eprintln!("Memory writing not implemented for {:#06X}", address),
        }
    }

    pub fn write_reg(&mut self, register: &Reg8, value: u8) {
        match register {
            Reg8::A => self.a = value,
            Reg8::F => self.f = FlagReg::from_bits_truncate(value),
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
                self.f = FlagReg::from_bits_truncate(bytes[1]);
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
        self.write_mem(self.sp - 1, bytes[1]);
        self.write_mem(self.sp - 2, bytes[0]);
        self.sp -= 2;
    }
}
