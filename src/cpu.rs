use bitflags::bitflags;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_big_array::BigArray;
use std::ops::RangeInclusive;

pub mod apu;
pub mod input;
pub mod interrupts;
pub mod memory;
pub mod ppu;
pub mod readwrite;
pub mod registers;
pub mod timer;
use super::Options;
use apu::*;
use input::*;
use interrupts::*;
use memory::*;
use ppu::*;
use readwrite::*;
use registers::*;
use timer::*;

/// The main processing unit
#[allow(clippy::upper_case_acronyms)]
#[derive(Deserialize, Serialize)]
pub struct CPU {
    pub reg: Registers,
    pub mem: Memory,
    pub ppu: PPU,
    pub apu: APU,
    pub timer: Timer,
    pub input: InputReg,
    pub istate: InterruptState,
    pub halt: bool,
}

impl CPU {
    pub fn new(rom_file: Vec<u8>, options: &Options) -> Self {
        Self {
            reg: Registers::new(),
            ppu: PPU::new(),
            apu: APU::new(options.audio_sample_rate),
            mem: Memory::new(rom_file),
            timer: Timer::new(),
            input: InputReg::new(),
            istate: InterruptState::new(),
            halt: false,
        }
    }

    /// Triggered when program hits a breakpoint.
    /// Set a breakpoint here when debugging
    pub fn breakpoint(&mut self) {
        println!("Breakpoint at {:#06X}", self.reg.pc);
    }

    /// Emulates the rest of the Game Boy (apart from instructions) for given amount of M-cycles
    pub fn cycle(&mut self, cycles: u8) {
        // Rest of the system runs on T-cycles, which is 1/4 of an M-cycle
        for _ in 0..(4 * cycles) {
            // Check if OAM DMA should be started
            if self.ppu.oam_dma_timer == 640 {
                self.oam_dma(self.ppu.oam_dma_source);
            }
            // Cycle PPU
            self.ppu.cycle();
            self.request_interrupt(self.ppu.interrupt_request);

            // Cycle timer
            self.timer.cycle();
            if self.timer.request_interrupt {
                self.request_interrupt(InterruptFlag::TIMER);
            }

            // Cycle APU based on timer state
            self.apu.cycle(self.timer.div);
        }
    }

    /// Executes the next instruction at program counter.
    /// Ticks the rest of the system at correct points between the execution of instructions.
    /// Returns whether or not the system hit VBlank during execution
    pub fn execute(&mut self) -> bool {
        let start_vblank = self.ppu.mode == 1;
        // Check for possible interrupt requests
        self.check_for_interrupt();

        if self.halt {
            // CPU doesn't execute anything when HALTed,
            // so just cycle the system forward until HALT is lifted
            self.cycle(1);

            // Check current PPU mode and return true
            // if it was changed to VBlank during execution
            let end_vblank = self.ppu.mode == 1;
            return !start_vblank && end_vblank;
        }

        let opcode = self.read(self.reg.pc);
        let mut increment_pc = true;

        match opcode {
            0x00..=0x3F => {
                // Mask out the first nibble for easier pattern matching
                let nibble = opcode & 0x0F;
                match nibble {
                    0x0 => match opcode {
                        // NOP
                        0x00 => {}
                        // STOP
                        0x10 => {
                            eprintln!("Tried to STOP");
                        }
                        // JR
                        _ => {
                            let step = self.read_operand() as i8;
                            if !((opcode == 0x20 && self.reg.f.intersects(FlagReg::ZERO))
                                || (opcode == 0x30 && self.reg.f.intersects(FlagReg::CARRY)))
                            {
                                self.reg.pc = self.reg.pc.wrapping_add_signed(step as i16);
                                self.cycle(1);
                            }
                        }
                    },
                    // LD d16
                    0x1 => {
                        let reg = Self::get_opcode_reg16(opcode).unwrap_or(Reg16::SP);
                        let val = self.read_operand_16();
                        self.reg.write_16(&reg, val);
                    }
                    // LD r16
                    0x2 | 0xA => {
                        let reg = Self::get_opcode_reg16(opcode).unwrap_or(Reg16::HL);
                        let address = self.reg.read_16(&reg);

                        if nibble == 0x02 {
                            self.write(address, self.reg.a);
                        } else {
                            self.reg.a = self.read(address);
                        }

                        if opcode == 0x22 || opcode == 0x2A {
                            self.reg
                                .write_16(&reg, self.reg.read_16(&reg).wrapping_add(1));
                        }
                        if opcode == 0x32 || opcode == 0x3A {
                            self.reg
                                .write_16(&reg, self.reg.read_16(&reg).wrapping_sub(1));
                        }
                        self.cycle(1);
                    }

                    // INC/DEC r16
                    0x3 | 0xB => {
                        let reg = Self::get_opcode_reg16(opcode).unwrap_or(Reg16::SP);
                        let val = if nibble == 0x03 {
                            self.reg.read_16(&reg).wrapping_add(1)
                        } else {
                            self.reg.read_16(&reg).wrapping_sub(1)
                        };
                        self.reg.write_16(&reg, val);
                        self.cycle(1);
                    }
                    // INC/DEC r8
                    0x4 | 0x5 | 0xC | 0xD => {
                        let offset = if nibble == 0x04 || nibble == 0x05 {
                            0
                        } else {
                            1
                        };
                        let reg = Self::get_opcode_reg(2 * (opcode >> 4) + offset);

                        let mut val: u8;
                        if reg.is_none() {
                            val = self.read(self.reg.read_16(&Reg16::HL))
                        } else {
                            val = self.reg.read(&reg.unwrap())
                        }

                        if nibble == 0x04 || nibble == 0x0C {
                            self.reg.f.remove(FlagReg::SUBTRACT);
                            self.reg.f.set(FlagReg::HALF_CARRY, (val & 0x0F) == 0x0F);
                            val = val.wrapping_add(1);
                        } else {
                            self.reg.f.insert(FlagReg::SUBTRACT);
                            self.reg.f.set(FlagReg::HALF_CARRY, (val & 0x0F) == 0x00);
                            val = val.wrapping_sub(1);
                        }
                        self.reg.f.set(FlagReg::ZERO, val == 0);

                        if let Some(reg_val) = reg {
                            self.reg.write(&reg_val, val);
                        } else {
                            self.write(self.reg.read_16(&Reg16::HL), val);
                            self.cycle(2);
                        }
                    }
                    // LD d8
                    0x6 | 0xE => {
                        let offset = if nibble == 0x06 { 0 } else { 1 };
                        let reg = Self::get_opcode_reg(2 * (opcode >> 4) + offset);
                        let val = self.read_operand();

                        if let Some(reg_val) = reg {
                            self.reg.write(&reg_val, val);
                        } else {
                            self.write(self.reg.read_16(&Reg16::HL), val);
                            self.cycle(1);
                        }
                    }
                    0x7 => {
                        match opcode {
                            // RLC
                            0x07 => {
                                self.reg.a = self.rotate(self.reg.a, true, false);
                                self.reg.f.remove(FlagReg::ZERO);
                            }
                            // RL
                            0x17 => {
                                self.reg.a = self.rotate(self.reg.a, true, true);
                                self.reg.f.remove(FlagReg::ZERO);
                            }
                            // DAA (https://rgbds.gbdev.io/docs/v0.9.0/gbz80.7#DAA)
                            0x27 => {
                                let mut adj: u8 = 0;
                                let res = if self.reg.f.intersects(FlagReg::SUBTRACT) {
                                    if self.reg.f.intersects(FlagReg::HALF_CARRY) {
                                        adj += 0x6;
                                    }
                                    if self.reg.f.intersects(FlagReg::CARRY) {
                                        adj += 0x60;
                                        self.reg.f.insert(FlagReg::CARRY);
                                    }
                                    self.reg.a.wrapping_sub(adj)
                                } else {
                                    if self.reg.f.intersects(FlagReg::HALF_CARRY)
                                        || self.reg.a & 0x0F > 0x9
                                    {
                                        adj += 0x6;
                                    }
                                    if self.reg.f.intersects(FlagReg::CARRY) || self.reg.a > 0x99 {
                                        adj += 0x60;
                                        self.reg.f.insert(FlagReg::CARRY);
                                    }
                                    self.reg.a.wrapping_add(adj)
                                };

                                self.reg.f.set(FlagReg::ZERO, res == 0);
                                self.reg.f.remove(FlagReg::HALF_CARRY);
                                self.reg.a = res;
                            }
                            // SCF
                            0x37 => {
                                self.reg.f.remove(FlagReg::SUBTRACT);
                                self.reg.f.remove(FlagReg::HALF_CARRY);
                                self.reg.f.insert(FlagReg::CARRY);
                            }
                            _ => panic!("Invalid instruction: {:#04X}", opcode),
                        };
                    }
                    0x8 => {
                        // LD SP
                        if opcode == 0x08 {
                            let bytes = self.reg.sp.to_le_bytes();
                            let address = self.read_operand_16();
                            self.write(address, bytes[0]);
                            self.write(address + 1, bytes[1]);
                            self.cycle(2);
                        }
                        // JR
                        else {
                            let step = self.read_operand() as i8;
                            if (opcode == 0x18)
                                || (opcode == 0x28 && self.reg.f.intersects(FlagReg::ZERO))
                                || (opcode == 0x38 && self.reg.f.intersects(FlagReg::CARRY))
                            {
                                self.reg.pc = self.reg.pc.wrapping_add_signed(step as i16);
                                self.cycle(1);
                            }
                        }
                    }
                    // ADD r16
                    0x9 => {
                        let reg = Self::get_opcode_reg16(opcode).unwrap_or(Reg16::SP);
                        let reg_val = self.reg.read_16(&reg);
                        let val = self.reg.read_16(&Reg16::HL);
                        let (res, carry) = val.overflowing_add(reg_val);

                        self.reg.f.remove(FlagReg::SUBTRACT);
                        self.reg.f.set(
                            FlagReg::HALF_CARRY,
                            ((reg_val & 0x0FFF) + (val & 0x0FFF)) & 0x1000 > 0,
                        );
                        self.reg.f.set(FlagReg::CARRY, carry);

                        self.reg.write_16(&Reg16::HL, res);
                        self.cycle(1);
                    }
                    0xF => {
                        match opcode {
                            // RRC
                            0x0F => {
                                self.reg.a = self.rotate(self.reg.a, false, false);
                                self.reg.f.remove(FlagReg::ZERO);
                            }
                            // RR
                            0x1F => {
                                self.reg.a = self.rotate(self.reg.a, false, true);
                                self.reg.f.remove(FlagReg::ZERO);
                            }
                            // CPL
                            0x2F => {
                                self.reg.a = !self.reg.a;
                                self.reg.f.insert(FlagReg::SUBTRACT);
                                self.reg.f.insert(FlagReg::HALF_CARRY);
                            }
                            // CCF
                            0x3F => {
                                self.reg.f.remove(FlagReg::SUBTRACT);
                                self.reg.f.remove(FlagReg::HALF_CARRY);
                                self.reg.f.toggle(FlagReg::CARRY);
                            }
                            _ => panic!("Invalid instruction: {:#04X}", opcode),
                        };
                    }
                    _ => panic!("Invalid instruction: {:#04X}", opcode),
                }
            }
            // Similarly implemented 8-bit loading and arithmetic operations
            0x40..=0x75 | 0x77..=0xBF => {
                let reg = Self::get_opcode_reg(opcode);
                let val: u8;
                if reg.is_none() {
                    val = self.read(self.reg.read_16(&Reg16::HL));
                    self.cycle(1);
                } else {
                    val = self.reg.read(&reg.unwrap());
                }

                match opcode {
                    // LD
                    0x40..=0x47 => self.reg.b = val,
                    0x48..=0x4F => self.reg.c = val,
                    0x50..=0x57 => self.reg.d = val,
                    0x58..=0x5F => self.reg.e = val,
                    0x60..=0x67 => self.reg.h = val,
                    0x68..=0x6F => self.reg.l = val,
                    0x70..=0x77 => {
                        self.write(self.reg.read_16(&Reg16::HL), val);
                        self.cycle(1);
                    }
                    0x78..=0x7F => self.reg.a = val,

                    // ADD / ADC
                    0x80..=0x8F => self.add_a(val, opcode >= 0x88),
                    // SUB / SBC
                    0x90..=0x9F => self.sub_a(val, opcode >= 0x98, true),
                    // AND
                    0xA0..=0xA7 => self.and_a(val),
                    // XOR
                    0xA8..=0xAF => self.xor_a(val),
                    // OR
                    0xB0..=0xB7 => self.or_a(val),
                    // CP
                    0xB8..=0xBF => self.sub_a(val, false, false),
                    _ => panic!("Invalid instruction: {:#04X}", opcode),
                }
            }
            // HALT
            0x76 => {
                self.halt = true;
            }
            0xC0..=0xFF => {
                // Mask out the first nibble for easier pattern matching
                let nibble = opcode & 0x0F;
                match nibble {
                    0x0 | 0x2 | 0x3 | 0xA => {
                        // DI
                        if opcode == 0xF3 {
                            self.istate.ime = false;
                        }
                        // LD
                        else if opcode & 0xF0 >= 0xE0 {
                            let address = match nibble {
                                0x0 => 0xFF00u16 + self.read_operand() as u16,
                                0x2 => 0xFF00u16 + self.reg.c as u16,
                                0xA => self.read_operand_16(),
                                _ => panic!(),
                            };
                            if opcode & 0xF0 == 0xE0 {
                                self.write(address, self.reg.a);
                            } else {
                                self.reg.a = self.read(address);
                            }
                            self.cycle(1);
                        } else {
                            let mut condition = self.get_opcode_condition(opcode);
                            match nibble {
                                // RET N
                                0x0 => {
                                    if !condition {
                                        increment_pc = false;
                                        self.reg.pc = self.pop();
                                        self.cycle(1);
                                    }
                                    self.cycle(1);
                                }
                                // JP
                                0x2 | 0x3 | 0xA => {
                                    if nibble == 0x2 {
                                        condition = !condition;
                                    }
                                    if nibble == 0x3 {
                                        condition = true;
                                    }

                                    let address = self.read_operand_16();
                                    if condition {
                                        increment_pc = false;
                                        self.reg.pc = address;
                                        self.cycle(1);
                                    }
                                }
                                _ => panic!(),
                            }
                        }
                    }
                    // POP r16
                    0x1 => {
                        let reg = Self::get_opcode_reg16(opcode - 0xC0).unwrap_or(Reg16::AF);
                        let val = self.pop();
                        self.reg.write_16(&reg, val);
                    }
                    // PUSH r16
                    0x5 => {
                        let reg = Self::get_opcode_reg16(opcode - 0xC0).unwrap_or(Reg16::AF);
                        self.push(self.reg.read_16(&reg));
                        self.cycle(1);
                    }
                    // Arithmetics for d8
                    0x6 | 0xE => {
                        let val = self.read_operand();
                        match opcode {
                            0xC6 => self.add_a(val, false),
                            0xCE => self.add_a(val, true),
                            0xD6 => self.sub_a(val, false, true),
                            0xDE => self.sub_a(val, true, true),
                            0xE6 => self.and_a(val),
                            0xEE => self.xor_a(val),
                            0xF6 => self.or_a(val),
                            0xFE => self.sub_a(val, false, false),
                            _ => {}
                        }
                    }
                    // CALL
                    0x4 | 0xC | 0xD => {
                        let mut condition = self.get_opcode_condition(opcode);
                        if nibble == 0x4 {
                            condition = !condition;
                        }
                        if nibble == 0xD {
                            condition = true;
                        }

                        let address = self.read_operand_16();
                        if condition {
                            increment_pc = false;
                            self.push(self.reg.pc + 1);
                            self.reg.pc = address;
                            self.cycle(1);
                        }
                    }
                    // RST
                    0x7 | 0xF => {
                        increment_pc = false;
                        self.halt = false;
                        self.push(self.reg.pc + 1);
                        let address: u16 = match opcode {
                            0xC7 => 0x00,
                            0xCF => 0x08,
                            0xD7 => 0x10,
                            0xDF => 0x18,
                            0xE7 => 0x20,
                            0xEF => 0x28,
                            0xF7 => 0x30,
                            0xFF => 0x38,
                            _ => panic!(),
                        };
                        self.reg.pc = address;
                        self.cycle(1);
                    }
                    0x8 => {
                        // ADD SP
                        if opcode >= 0xE0 {
                            let offset = (self.read_operand() as i8) as i16;
                            let res = self.reg.sp.wrapping_add_signed(offset);

                            let reg = if opcode == 0xE8 {
                                self.cycle(2);
                                Reg16::SP
                            } else {
                                self.cycle(1);
                                Reg16::HL
                            };

                            self.reg.f.remove(FlagReg::ZERO);
                            self.reg.f.remove(FlagReg::SUBTRACT);
                            self.reg.f.set(
                                FlagReg::HALF_CARRY,
                                (self.reg.sp & 0x000F).wrapping_add_signed(offset & 0x000F)
                                    & 0x0010
                                    > 0,
                            );
                            self.reg.f.set(
                                FlagReg::CARRY,
                                (self.reg.sp & 0x00FF).wrapping_add_signed(offset & 0x00FF)
                                    & 0x0100
                                    > 0,
                            );

                            self.reg.write_16(&reg, res);
                        }
                        // RET
                        else if self.get_opcode_condition(opcode) {
                            increment_pc = false;
                            self.reg.pc = self.pop();
                            self.cycle(2);
                        } else {
                            self.cycle(1);
                        }
                    }
                    0x9 => match opcode {
                        // RET
                        0xC9 => {
                            increment_pc = false;
                            self.reg.pc = self.pop();
                            self.cycle(1);
                        }
                        // RETI
                        0xD9 => {
                            increment_pc = false;
                            self.reg.pc = self.pop();
                            self.istate.ime = true;
                            self.cycle(1);
                        }
                        // JP
                        0xE9 => {
                            increment_pc = false;
                            self.reg.pc = self.reg.read_16(&Reg16::HL);
                        }
                        // LD
                        0xF9 => {
                            self.reg.sp = self.reg.read_16(&Reg16::HL);
                            self.cycle(1);
                        }
                        _ => panic!("Invalid instruction: {:#04X}", opcode),
                    },
                    0xB => {
                        // EI
                        if opcode == 0xFB {
                            self.istate.ime = true;
                        }
                        // 0xCB 16-bit opcodes
                        else {
                            self.arithmetic();
                        }
                    }
                    _ => panic!("Invalid instruction: {:#04X}", opcode),
                }
            }
        };

        if increment_pc {
            self.reg.pc += 1;
        }
        // Every instruction takes at least one M-cycle to execute
        self.cycle(1);

        // Check current PPU mode and return true
        // if it was changed to VBlank during execution
        let end_vblank = self.ppu.mode == 1;
        !start_vblank && end_vblank
    }

    /// Executes the 16-bit long arithmetic opcodes that start with 0xCB
    fn arithmetic(&mut self) {
        let opcode = self.read_operand();
        let reg = Self::get_opcode_reg(opcode);

        let val = if let Some(reg_val) = reg {
            self.reg.read(&reg_val)
        } else {
            self.read(self.reg.read_16(&Reg16::HL))
        };

        let mut set_zero_flag = true;
        let res = match opcode {
            // RLC
            0x00..=0x07 => self.rotate(val, true, false),
            // RRC
            0x08..=0x0F => self.rotate(val, false, false),
            // RL
            0x10..=0x17 => self.rotate(val, true, true),
            // RR
            0x18..=0x1F => self.rotate(val, false, true),
            // SLA
            0x20..=0x27 => self.rotate(val, true, false) & 0b1111_1110,
            // SRA
            0x28..=0x2F => (self.rotate(val, false, false) & 0b0111_1111) | (val & 0b1000_0000),
            // SWAP
            0x30..=0x37 => {
                let res = val.rotate_right(4);
                self.reg.f = FlagReg::from_bits_truncate(0);
                self.reg.f.set(FlagReg::ZERO, res == 0);
                res
            }
            // SRL
            0x38..=0x3F => self.rotate(val, false, false) & 0b0111_1111,
            _ => {
                set_zero_flag = false;
                let bit = (opcode - 0x40) % 0x40 / 0x8;
                let mask = 1 << bit;
                match opcode {
                    // BIT
                    0x40..=0x7F => {
                        self.reg.f.remove(FlagReg::SUBTRACT);
                        self.reg.f.insert(FlagReg::HALF_CARRY);
                        self.reg.f.set(FlagReg::ZERO, val & mask == 0);

                        // BIT doesn't write into memory, only sets flags
                        if reg.is_none() {
                            self.cycle(2);
                        } else {
                            self.cycle(1);
                        }
                        return;
                    }
                    // RES
                    0x80..=0xBF => val & !mask,
                    // SET
                    0xC0..=0xFF => val | mask,
                    _ => panic!(),
                }
            }
        };

        if set_zero_flag {
            self.reg.f.set(FlagReg::ZERO, res == 0);
        }

        if let Some(reg_val) = reg {
            self.reg.write(&reg_val, res);
            self.cycle(1);
        } else {
            // Instructions that write to HL take 2 cycles longer
            self.write(self.reg.read_16(&Reg16::HL), res);
            self.cycle(3);
        }
    }

    fn rotate(&mut self, val: u8, left: bool, through_carry: bool) -> u8 {
        let carry_mask = if left { 0b10000000 } else { 0b00000001 };
        let carry = val & carry_mask > 0;

        let mut res = if left { val << 1 } else { val >> 1 };

        let overflow_bit = if left { 0b00000001 } else { 0b10000000 };
        if through_carry {
            if self.reg.f.intersects(FlagReg::CARRY) {
                res |= overflow_bit;
            }
        } else if carry {
            res |= overflow_bit;
        }

        self.reg.f = FlagReg::from_bits_truncate(0);
        self.reg.f.set(FlagReg::CARRY, carry);

        res
    }

    fn add_a(&mut self, val: u8, with_carry: bool) {
        let carry_val = if with_carry && self.reg.f.intersects(FlagReg::CARRY) {
            1
        } else {
            0
        };
        let (added_val, add_carry) = val.overflowing_add(carry_val);
        let (res, carry) = self.reg.a.overflowing_add(added_val);

        self.reg.f.set(FlagReg::ZERO, res == 0);
        self.reg.f.remove(FlagReg::SUBTRACT);
        self.reg.f.set(
            FlagReg::HALF_CARRY,
            (self.reg.a & 0x0F)
                .wrapping_add(val & 0x0F)
                .wrapping_add(carry_val)
                & 0x10
                > 0,
        );
        self.reg.f.set(FlagReg::CARRY, carry || add_carry);

        self.reg.a = res;
    }

    fn sub_a(&mut self, val: u8, with_carry: bool, set_a: bool) {
        let carry_val = if with_carry && self.reg.f.intersects(FlagReg::CARRY) {
            1
        } else {
            0
        };
        let (added_val, add_carry) = val.overflowing_add(carry_val);
        let (res, carry) = self.reg.a.overflowing_sub(added_val);

        self.reg.f.set(FlagReg::ZERO, res == 0);
        self.reg.f.insert(FlagReg::SUBTRACT);
        self.reg.f.set(
            FlagReg::HALF_CARRY,
            (self.reg.a & 0x0F)
                .wrapping_sub(val & 0x0F)
                .wrapping_sub(carry_val)
                & 0x10
                > 0,
        );
        self.reg.f.set(FlagReg::CARRY, carry || add_carry);

        if set_a {
            self.reg.a = res;
        }
    }

    fn and_a(&mut self, val: u8) {
        self.reg.a &= val;

        self.reg.f = FlagReg::from_bits_truncate(0);
        self.reg.f.set(FlagReg::ZERO, self.reg.a == 0);
        self.reg.f.insert(FlagReg::HALF_CARRY);
    }

    fn xor_a(&mut self, val: u8) {
        self.reg.a ^= val;

        self.reg.f = FlagReg::from_bits_truncate(0);
        self.reg.f.set(FlagReg::ZERO, self.reg.a == 0);
    }

    fn or_a(&mut self, val: u8) {
        self.reg.a |= val;

        self.reg.f = FlagReg::from_bits_truncate(0);
        self.reg.f.set(FlagReg::ZERO, self.reg.a == 0);
    }

    fn get_opcode_reg(opcode: u8) -> Option<Reg8> {
        let mut nibble = opcode & 0x0F;
        if nibble >= 0x08 {
            nibble -= 0x08;
        }
        match nibble {
            0x00 => Some(Reg8::B),
            0x01 => Some(Reg8::C),
            0x02 => Some(Reg8::D),
            0x03 => Some(Reg8::E),
            0x04 => Some(Reg8::H),
            0x05 => Some(Reg8::L),
            0x07 => Some(Reg8::A),
            _ => None,
        }
    }

    fn get_opcode_reg16(opcode: u8) -> Option<Reg16> {
        let nibble = (opcode & 0xF0) >> 4;
        match nibble {
            0x00 => Some(Reg16::BC),
            0x01 => Some(Reg16::DE),
            0x02 => Some(Reg16::HL),
            _ => None,
        }
    }

    fn get_opcode_condition(&self, opcode: u8) -> bool {
        if opcode & 0xF0 == 0xC0 {
            self.reg.f.intersects(FlagReg::ZERO)
        } else {
            self.reg.f.intersects(FlagReg::CARRY)
        }
    }
}
