use super::*;

#[derive(Clone, Copy, PartialEq)]
pub struct InterruptFlag(u8);

bitflags! {
    impl InterruptFlag: u8 {
        const JOYPAD = 0b0001_0000;
        const SERIAL = 0b0000_1000;
        const TIMER  = 0b0000_0100;
        const LCD    = 0b0000_0010;
        const VBLANK = 0b0000_0001;
    }
}

pub struct InterruptState {
    /// Master interrupt enable
    pub ime: bool,
    /// Interrupt enable flag
    pub ie: InterruptFlag,
    /// Interrupt request flag
    pub iflag: InterruptFlag,
    // If CPU is moving to an interrupt
    pub executing: bool,
}

impl InterruptState {
    pub fn new() -> Self {
        Self {
            ime: false,
            iflag: InterruptFlag::from_bits_truncate(0),
            ie: InterruptFlag::from_bits_truncate(0),
            executing: false,
        }
    }
}

impl MemoryAccess for InterruptState {
    fn get_range(&self) -> Vec<RangeInclusive<u16>> {
        vec![0xFF0F..=0xFF0F, 0xFFFF..=0xFFFF]
    }

    fn mem_read(&self, address: u16) -> u8 {
        match address {
            0xFF0F => self.iflag.bits(),
            0xFFFF => self.ie.bits(),
            _ => panic!(),
        }
    }

    fn mem_write(&mut self, address: u16, value: u8) {
        match address {
            0xFF0F => self.iflag = InterruptFlag::from_bits_truncate(value),
            0xFFFF => self.ie = InterruptFlag::from_bits_truncate(value),
            _ => panic!(),
        }
    }
}

impl CPU {
    /// Executed from outside the CPU when input state should be updated
    pub fn update_input(&mut self, input: InputFlag, pressed: bool) {
        self.io.input.flags.set(input, !pressed);
        self.request_interrupt(InterruptFlag::JOYPAD);
    }

    /// Sets corresponding interrupt flag to true
    pub fn request_interrupt(&mut self, interrupt: InterruptFlag) {
        self.istate.iflag.insert(interrupt);
    }

    /// Executes interrupt handler
    fn run_interrupt(&mut self, interrupt: InterruptFlag) {
        self.istate.iflag.remove(interrupt);
        let address: u16 = match interrupt {
            InterruptFlag::VBLANK => 0x40,
            InterruptFlag::LCD => 0x48,
            InterruptFlag::TIMER => 0x50,
            InterruptFlag::SERIAL => 0x58,
            InterruptFlag::JOYPAD => 0x60,
            _ => panic!(),
        };
        self.push(self.reg.pc);
        self.reg.pc = address;
        // Interrupt handling takes 5 M-cycles before executing actual instruction
        self.istate.executing = true;
        self.cycles = 5;
    }

    /// Checks for interrupts,
    /// returns true if interrupt should be ran instead of next instruction
    pub fn check_for_interrupt(&mut self) -> bool {
        let interrupt_requests = self.istate.ie.intersection(self.istate.iflag);
        if interrupt_requests.bits() > 0 {
            // Exit halt mode even if IME is disabled
            self.halt = false;
            // Handle interrupt
            if self.istate.ime {
                if interrupt_requests.intersects(InterruptFlag::VBLANK) {
                    self.run_interrupt(InterruptFlag::VBLANK);
                } else if interrupt_requests.intersects(InterruptFlag::LCD) {
                    self.run_interrupt(InterruptFlag::LCD);
                } else if interrupt_requests.intersects(InterruptFlag::TIMER) {
                    self.run_interrupt(InterruptFlag::TIMER);
                } else if interrupt_requests.intersects(InterruptFlag::SERIAL) {
                    self.run_interrupt(InterruptFlag::SERIAL);
                } else if interrupt_requests.intersects(InterruptFlag::JOYPAD) {
                    self.run_interrupt(InterruptFlag::JOYPAD);
                }
                self.istate.ime = false;
                return true;
            }
        }
        false
    }
}
