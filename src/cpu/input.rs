use super::*;

#[derive(Clone, Copy, PartialEq)]
pub struct InputFlag(u8);

bitflags! {
    impl InputFlag: u8 {
        const START  = 0b1000_0000;
        const SELECT = 0b0100_0000;
        const B      = 0b0010_0000;
        const A      = 0b0001_0000;
        const DOWN   = 0b0000_1000;
        const UP     = 0b0000_0100;
        const LEFT   = 0b0000_0010;
        const RIGHT  = 0b0000_0001;
    }
}

pub struct InputReg {
    pub select_button: bool,
    pub select_dpad: bool,
    pub flags: InputFlag,
}

impl InputReg {
    pub fn new() -> Self {
        Self {
            select_button: false,
            select_dpad: false,
            flags: InputFlag::from_bits_truncate(0xFF),
        }
    }
}

impl MemoryAccess for InputReg {
    fn get_range(&self) -> Vec<RangeInclusive<u16>> {
        vec![0xFF00..=0xFF00]
    }

    fn mem_read(&self, _: u16) -> u8 {
        if self.select_button {
            (!self.flags.bits() & 0xF0u8) >> 4
        } else if self.select_dpad {
            !self.flags.bits() & 0x0F
        } else {
            0x0F
        }
    }

    fn mem_write(&mut self, _: u16, value: u8) {
        self.select_button = value & 0b0010_0000 > 0;
        self.select_dpad = value & 0b0001_0000 > 0;
    }
}
