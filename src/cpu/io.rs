use super::*;

#[derive(Clone, Copy, PartialEq)]
pub struct TimerControl(u8);

#[derive(Clone, Copy, PartialEq)]
pub struct InputFlag(u8);

bitflags! {
    impl TimerControl: u8 {
        const ENABLE   = 0b0000_0100;
        const CONTROL1 = 0b0000_0010;
        const CONTROL2 = 0b0000_0001;
    }

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

pub struct Timer {
    pub cycles: u8,
    pub modulo: u8,
    pub counter: u8,
    pub control: TimerControl,
    pub target: u8,
    pub enabled: bool,
    pub overflowed: bool,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            cycles: 0,
            modulo: 0,
            counter: 0,
            control: TimerControl::from_bits_truncate(0),
            target: 255,
            enabled: false,
            overflowed: false,
        }
    }

    pub fn cycle(&mut self) {
        if !self.enabled {
            return;
        }

        if self.cycles == self.target {
            self.cycles = 0;
            if self.counter == 255 {
                self.overflowed = true;
                self.counter = self.modulo;
            } else {
                self.overflowed = false;
                self.counter += 1;
            }
        } else {
            self.cycles += 1;
        }
    }

    pub fn control(&mut self, value: u8) {
        self.control = TimerControl::from_bits_truncate(value);
        self.enabled = self.control.intersects(TimerControl::ENABLE);
        self.target = match self.control.bits() & 0b11 {
            0b00 => 255,
            0b01 => 4,
            0b10 => 16,
            0b11 => 64,
            _ => panic!(),
        }
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

    pub fn read(&self) -> u8 {
        if self.select_button {
            (!self.flags.bits() & 0xF0u8) >> 4
        } else if self.select_dpad {
            !self.flags.bits() & 0x0F
        } else {
            0x0F
        }
    }

    pub fn write(&mut self, value: u8) {
        self.select_button = value & 0b0010_0000 > 0;
        self.select_dpad = value & 0b0001_0000 > 0;
    }
}

#[allow(clippy::upper_case_acronyms)]
pub struct IO {
    pub input: InputReg,
    pub serial: u8,
    pub timer: Timer,
}

impl IO {
    pub fn new() -> Self {
        Self {
            input: InputReg::new(),
            serial: 0,
            timer: Timer::new(),
        }
    }
}

impl MemoryAccess for IO {
    fn get_range(&self) -> Vec<RangeInclusive<u16>> {
        vec![0xFF00..=0xFF07]
    }

    fn mem_read(&self, address: u16) -> u8 {
        match address {
            0xFF00 => self.input.read(),
            0xFF01 => self.serial,
            0xFF05 => self.timer.counter,
            0xFF06 => self.timer.modulo,
            0xFF07 => self.timer.control.bits(),
            _ => {
                eprintln!("IO read not implemented for address {:#06X}", address);
                0
            }
        }
    }

    fn mem_write(&mut self, address: u16, value: u8) {
        match address {
            0xFF00 => self.input.write(value),
            0xFF01 => self.serial = value,
            0xFF02 => {
                println!("SERIAL: {:#04X} ({})", self.serial, self.serial as char);
            }
            0xFF05 => self.timer.counter = value,
            0xFF06 => self.timer.modulo = value,
            0xFF07 => self.timer.control(value),
            _ => eprintln!("IO write not implemented for address {:#06X}", address),
        }
    }
}
