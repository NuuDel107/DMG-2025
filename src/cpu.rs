use bitflags::bitflags;
use std::ops::RangeInclusive;

pub mod interrupts;
pub mod io;
pub mod memory;
mod opcodes;
pub mod ppu;
pub mod readwrite;
pub mod registers;
use interrupts::*;
use io::*;
use memory::*;
use ppu::*;
use readwrite::*;
use registers::*;

/// The main processing unit
#[allow(clippy::upper_case_acronyms)]
pub struct CPU {
    pub reg: Registers,
    pub mem: Memory,
    pub ppu: PPU,
    pub io: IO,
    pub istate: InterruptState,
    pub halt: bool,
    /// Remaining M-cycles until executing the next instruction
    pub cycles: u8,
    /// Remaining dots until executing the next M-cycle
    pub dots: u8,
}

impl CPU {
    pub fn new(rom_file: Vec<u8>) -> Self {
        Self {
            reg: Registers::new(),
            ppu: PPU::new(),
            mem: Memory::new(rom_file),
            io: IO::new(),
            istate: InterruptState::new(),
            halt: false,
            cycles: 0,
            dots: 0,
        }
    }

    /// Emulates the Game Boy for one dot / oscillator tick
    pub fn cycle(&mut self, run_whole_instruction: bool) {
        // When stepping over instructions (run_whole_instruction == true),
        // run as many cycles as is needed to set remaining cycles to 0
        // and to run the next instruction
        let n = if run_whole_instruction {
            // One M-cycle is 4 dots
            1 + self.dots + (self.cycles * 4)
        } else {
            // When cycling from normal clock, cycle only once
            1
        };

        for _ in 0..n {
            // PPU is cycled on every dot
            self.ppu.cycle();

            // If three dots have been executed since the last M-cycle,
            // reset dot counter and tick the rest of CPU alongside PPU
            if self.dots > 0 {
                self.dots -= 1;
                continue;
            }
            self.dots = 3;

            // Tick timer on every M-cycle
            self.io.timer.cycle();

            // Return if CPU is still "executing" last instruction
            if self.cycles > 0 {
                self.cycles -= 1;
                continue;
            }

            // Check possible interrupt requests
            if self.check_for_interrupt() {
                continue;
            }

            // Execute next CPU instruction if no interrupts should be executed
            if !self.halt {
                self.cycles = self.execute();
            }
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

    /// Triggered when program hits a breakpoint.
    /// Set a breakpoint here when debugging
    pub fn breakpoint(&mut self) {
        println!("Breakpoint at {:#06X}", self.reg.pc);
    }
}
