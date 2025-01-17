use super::*;

#[derive(Deserialize, Serialize, Clone, Copy, PartialEq)]
pub struct TimerControl(u8);

bitflags! {
    impl TimerControl: u8 {
        const ENABLE   = 0b0000_0100;
        const CONTROL1 = 0b0000_0010;
        const CONTROL2 = 0b0000_0001;
    }
}

#[derive(Deserialize, Serialize)]
pub struct Timer {
    pub div: u16,
    pub tima: u8,
    pub tma: u8,
    pub control: TimerControl,
    pub enabled: bool,
    pub request_interrupt: bool,
    /// Index of bit in divider used for checking if TIMA should be incremented
    pub div_bit: u8,
    /// The result of previous AND expression between div & div_bit and enabled flag
    pub previous_and: bool,
    /// Cycles left in the simulated delay after overflowing.
    pub overflow_delay: i8,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            div: 0,
            tima: 0,
            tma: 0,
            control: TimerControl::from_bits_truncate(0),
            enabled: false,
            div_bit: 9,
            previous_and: false,
            request_interrupt: false,
            overflow_delay: 0,
        }
    }

    // Cycles the timer forward by one T-cycle
    pub fn cycle(&mut self) {
        self.request_interrupt = false;

        // Simulate the 4 T-cycle delay after overflowing
        // before TMA is written to TIMA and interrupt is requested
        if self.overflow_delay >= 0 {
            if self.overflow_delay == 0 {
                self.tima = self.tma;
                self.request_interrupt = true;
            }
            self.overflow_delay -= 1;
        }

        // The 16-bit divider is incremented every T-cycle
        // DIV register only maps to the upper 8 bits,
        // so to the software its incremented only every 256 dots
        self.div = self.div.wrapping_add(1);

        // Get the AND value from selected DIV bit and the enabled flag
        let div_val = (self.div >> (self.div_bit)) & 0b1;
        let and = (self.enabled as u16) & div_val > 0;

        // TIMA is incremented on falling edge (previous AND = true, current AND = false)
        // This means that setting the enabled flag can also trigger a TIMA increment
        if !and && self.previous_and {
            let overflow: bool;
            (self.tima, overflow) = self.tima.overflowing_add(1);

            if overflow {
                self.overflow_delay = 3;
            }
        }

        self.previous_and = and;
    }
}

impl MemoryAccess for Timer {
    fn get_range(&self) -> Vec<RangeInclusive<u16>> {
        vec![0xFF04..=0xFF07]
    }

    fn mem_read(&self, address: u16) -> u8 {
        match address {
            0xFF04 => (self.div >> 8) as u8,
            0xFF05 => self.tima,
            0xFF06 => self.tma,
            0xFF07 => self.control.bits(),
            _ => unreachable!(),
        }
    }

    fn mem_write(&mut self, address: u16, value: u8) {
        match address {
            0xFF04 => self.div = 0,
            0xFF05 => {
                self.tima = value;
                // Writing to TIMA during the delay period after overflowing
                // prevents TMA being written to TIMA and sending interrupt
                self.overflow_delay = -1;
            }
            0xFF06 => self.tma = value,
            0xFF07 => {
                self.control = TimerControl::from_bits_truncate(value);
                self.enabled = self.control.intersects(TimerControl::ENABLE);
                self.div_bit = match self.control.bits() & 0b11 {
                    0b00 => 9,
                    0b01 => 3,
                    0b10 => 5,
                    0b11 => 7,
                    _ => unreachable!(),
                }
            }
            _ => unreachable!(),
        }
    }
}
