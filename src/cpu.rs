pub mod io;
pub mod memory;
mod opcodes;
use io::*;
use memory::*;

/// The main processing unit
#[allow(clippy::upper_case_acronyms)]
pub struct CPU {
    pub mem: Memory,
    pub halt: bool,
    pub cycles: u8,
}

impl CPU {
    pub fn new(rom_file: Vec<u8>) -> Self {
        Self {
            mem: Memory::new(rom_file),
            halt: false,
            cycles: 0,
        }
    }

    pub fn cycle(&mut self, ignore_cycles: bool) {
        if self.cycles > 0 {
            if ignore_cycles {
                self.cycles = 0;
            } else {
                self.cycles -= 1;
                return;
            }
        }

        let interrupt_requests = self.mem.ie.intersection(self.mem.iflag);
        if interrupt_requests.bits() > 0 {
            self.halt = false;
            if self.mem.ime {
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
                self.mem.ime = false;
                return;
            }
        }

        if !self.halt {
            self.cycles = self.execute();
        }
    }

    pub fn frame(&mut self) {
        // self.renderer.draw_tile(
        //     0,
        //     0,
        //     [
        //         0xFF, 0x00, 0x7E, 0xFF, 0x85, 0x81, 0x89, 0x83, 0x93, 0x85, 0xA5, 0x8B, 0xC9, 0x97,
        //         0x7E, 0xFF,
        //     ],
        // );
    }

    pub fn update_input(&mut self, input: InputFlag, pressed: bool) {
        self.mem.io.input.flags.set(input, !pressed);
        self.request_interrupt(InterruptFlag::JOYPAD);
    }

    pub fn request_interrupt(&mut self, interrupt: InterruptFlag) {
        self.mem.iflag.insert(interrupt);
    }

    fn run_interrupt(&mut self, interrupt: InterruptFlag) {
        self.mem.iflag.remove(interrupt);
        let address: u16 = match interrupt {
            InterruptFlag::VBLANK => 0x40,
            InterruptFlag::LCD => 0x48,
            InterruptFlag::TIMER => 0x50,
            InterruptFlag::SERIAL => 0x58,
            InterruptFlag::JOYPAD => 0x60,
            _ => panic!(),
        };
        self.mem.push(self.mem.pc);
        self.mem.pc = address;
    }

    pub fn breakpoint(&mut self) {
        println!("Breakpoint at {:#06X}", self.mem.pc);
    }
}
