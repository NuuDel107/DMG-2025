use super::*;

#[derive(Clone, Copy)]
pub enum Reg8 {
    A,
    #[allow(dead_code)]
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
pub struct FlagReg(u8);

bitflags! {
    impl FlagReg: u8 {
        const ZERO       = 0b1000_0000;
        const SUBTRACT   = 0b0100_0000;
        const HALF_CARRY = 0b0010_0000;
        const CARRY      = 0b0001_0000;
    }
}

pub struct Registers {
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
}

impl Registers {
    pub fn new() -> Self {
        Self {
            // State of registers after executing boot ROM
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
        }
    }

    pub fn read(&self, register: &Reg8) -> u8 {
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

    pub fn read_16(&self, register: &Reg16) -> u16 {
        match register {
            Reg16::AF => u16::from_be_bytes([self.a, self.f.bits()]),
            Reg16::BC => u16::from_be_bytes([self.b, self.c]),
            Reg16::DE => u16::from_be_bytes([self.d, self.e]),
            Reg16::HL => u16::from_be_bytes([self.h, self.l]),
            Reg16::SP => self.sp,
            Reg16::PC => self.pc,
        }
    }

    pub fn write(&mut self, register: &Reg8, value: u8) {
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

    pub fn write_16(&mut self, register: &Reg16, value: u16) {
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
}
