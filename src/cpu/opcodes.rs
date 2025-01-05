use super::*;

impl CPU {
    /// Executes the next instruction at program counter.
    /// Returns the amount of M-cycles that instruction takes
    pub fn execute(&mut self) -> u8 {
        let opcode = self.mem.read_mem(self.mem.pc);
        let mut increment_pc = true;
        let cycles: u8 = match opcode {
            0x00..=0x3F => {
                // Mask out the first nibble for easier pattern matching
                let nibble = opcode & 0x0F;
                match nibble {
                    0x0 => match opcode {
                        // NOP
                        0x00 => 1,
                        // STOP
                        0x10 => {
                            eprintln!("Tried to STOP");
                            1
                        }
                        // JR
                        _ => {
                            let step = self.mem.read_operand() as i8;
                            if !((opcode == 0x20 && self.mem.f.intersects(FlagReg::ZERO))
                                || (opcode == 0x30 && self.mem.f.intersects(FlagReg::CARRY)))
                            {
                                self.mem.pc = self.mem.pc.wrapping_add_signed(step as i16);
                                3
                            } else {
                                2
                            }
                        }
                    },
                    // LD d16
                    0x1 => {
                        let reg = Self::get_opcode_reg16(opcode).unwrap_or(Reg16::SP);
                        let val = self.mem.read_operand_16();
                        self.mem.write_reg_16(&reg, val);
                        3
                    }
                    // LD r16
                    0x2 | 0xA => {
                        let reg = Self::get_opcode_reg16(opcode).unwrap_or(Reg16::HL);
                        let address = self.mem.read_reg_16(&reg);

                        if nibble == 0x02 {
                            self.mem.write_mem(address, self.mem.a);
                        } else {
                            self.mem.a = self.mem.read_mem(address);
                        }

                        if opcode == 0x22 || opcode == 0x2A {
                            self.mem
                                .write_reg_16(&reg, self.mem.read_reg_16(&reg).wrapping_add(1));
                        }
                        if opcode == 0x32 || opcode == 0x3A {
                            self.mem
                                .write_reg_16(&reg, self.mem.read_reg_16(&reg).wrapping_sub(1));
                        }
                        2
                    }

                    // INC/DEC r16
                    0x3 | 0xB => {
                        let reg = Self::get_opcode_reg16(opcode).unwrap_or(Reg16::SP);
                        let val = if nibble == 0x03 {
                            self.mem.read_reg_16(&reg).wrapping_add(1)
                        } else {
                            self.mem.read_reg_16(&reg).wrapping_sub(1)
                        };
                        self.mem.write_reg_16(&reg, val);
                        2
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
                            val = self.mem.read_mem(self.mem.read_reg_16(&Reg16::HL))
                        } else {
                            val = self.mem.read_reg(&reg.unwrap())
                        }

                        if nibble == 0x04 || nibble == 0x0C {
                            self.mem.f.remove(FlagReg::SUBTRACT);
                            self.mem.f.set(FlagReg::HALF_CARRY, (val & 0x0F) == 0x0F);
                            val = val.wrapping_add(1);
                        } else {
                            self.mem.f.insert(FlagReg::SUBTRACT);
                            self.mem.f.set(FlagReg::HALF_CARRY, (val & 0x0F) == 0x00);
                            val = val.wrapping_sub(1);
                        }
                        self.mem.f.set(FlagReg::ZERO, val == 0);

                        if let Some(reg_val) = reg {
                            self.mem.write_reg(&reg_val, val);
                        } else {
                            self.mem.write_mem(self.mem.read_reg_16(&Reg16::HL), val);
                        }

                        1
                    }
                    // LD d8
                    0x6 | 0xE => {
                        let offset = if nibble == 0x06 { 0 } else { 1 };
                        let reg = Self::get_opcode_reg(2 * (opcode >> 4) + offset);
                        let val = self.mem.read_operand();

                        if let Some(reg_val) = reg {
                            self.mem.write_reg(&reg_val, val);
                            2
                        } else {
                            self.mem.write_mem(self.mem.read_reg_16(&Reg16::HL), val);
                            3
                        }
                    }
                    0x7 => {
                        match opcode {
                            // RLC
                            0x07 => {
                                self.mem.a = self.rotate(self.mem.a, true, false);
                                self.mem.f.remove(FlagReg::ZERO);
                            }
                            // RL
                            0x17 => {
                                self.mem.a = self.rotate(self.mem.a, true, true);
                                self.mem.f.remove(FlagReg::ZERO);
                            }
                            // DAA (https://rgbds.gbdev.io/docs/v0.9.0/gbz80.7#DAA)
                            0x27 => {
                                let mut adj: u8 = 0;
                                let res = if self.mem.f.intersects(FlagReg::SUBTRACT) {
                                    if self.mem.f.intersects(FlagReg::HALF_CARRY) {
                                        adj += 0x6;
                                    }
                                    if self.mem.f.intersects(FlagReg::CARRY) {
                                        adj += 0x60;
                                        self.mem.f.insert(FlagReg::CARRY);
                                    }
                                    self.mem.a.wrapping_sub(adj)
                                } else {
                                    if self.mem.f.intersects(FlagReg::HALF_CARRY)
                                        || self.mem.a & 0x0F > 0x9
                                    {
                                        adj += 0x6;
                                    }
                                    if self.mem.f.intersects(FlagReg::CARRY) || self.mem.a > 0x99 {
                                        adj += 0x60;
                                        self.mem.f.insert(FlagReg::CARRY);
                                    }
                                    self.mem.a.wrapping_add(adj)
                                };

                                self.mem.f.set(FlagReg::ZERO, res == 0);
                                self.mem.f.remove(FlagReg::HALF_CARRY);
                                self.mem.a = res;
                            }
                            // SCF
                            0x37 => {
                                self.mem.f.remove(FlagReg::SUBTRACT);
                                self.mem.f.remove(FlagReg::HALF_CARRY);
                                self.mem.f.insert(FlagReg::CARRY);
                            }
                            _ => panic!("Invalid instruction: {:#04X}", opcode),
                        };
                        1
                    }
                    0x8 => {
                        // LD SP
                        if opcode == 0x08 {
                            let bytes = self.mem.sp.to_le_bytes();
                            let address = self.mem.read_operand_16();
                            self.mem.write_mem(address, bytes[0]);
                            self.mem.write_mem(address + 1, bytes[1]);
                            5
                        }
                        // JR
                        else {
                            let step = self.mem.read_operand() as i8;
                            if (opcode == 0x18)
                                || (opcode == 0x28 && self.mem.f.intersects(FlagReg::ZERO))
                                || (opcode == 0x38 && self.mem.f.intersects(FlagReg::CARRY))
                            {
                                self.mem.pc = self.mem.pc.wrapping_add_signed(step as i16);
                                3
                            } else {
                                2
                            }
                        }
                    }
                    // ADD r16
                    0x9 => {
                        let reg = Self::get_opcode_reg16(opcode).unwrap_or(Reg16::SP);
                        let reg_val = self.mem.read_reg_16(&reg);
                        let val = self.mem.read_reg_16(&Reg16::HL);
                        let (res, carry) = val.overflowing_add(reg_val);

                        self.mem.f.remove(FlagReg::SUBTRACT);
                        self.mem.f.set(
                            FlagReg::HALF_CARRY,
                            ((reg_val & 0x0FFF) + (val & 0x0FFF)) & 0x1000 > 0,
                        );
                        self.mem.f.set(FlagReg::CARRY, carry);

                        self.mem.write_reg_16(&Reg16::HL, res);
                        2
                    }
                    0xF => {
                        match opcode {
                            // RRC
                            0x0F => {
                                self.mem.a = self.rotate(self.mem.a, false, false);
                                self.mem.f.remove(FlagReg::ZERO);
                            }
                            // RR
                            0x1F => {
                                self.mem.a = self.rotate(self.mem.a, false, true);
                                self.mem.f.remove(FlagReg::ZERO);
                            }
                            // CPL
                            0x2F => {
                                self.mem.a = !self.mem.a;
                                self.mem.f.insert(FlagReg::SUBTRACT);
                                self.mem.f.insert(FlagReg::HALF_CARRY);
                            }
                            // CCF
                            0x3F => {
                                self.mem.f.remove(FlagReg::SUBTRACT);
                                self.mem.f.remove(FlagReg::HALF_CARRY);
                                self.mem.f.toggle(FlagReg::CARRY);
                            }
                            _ => panic!("Invalid instruction: {:#04X}", opcode),
                        };
                        1
                    }
                    _ => panic!("Invalid instruction: {:#04X}", opcode),
                }
            }
            // Similarly implemented 8-bit loading and arithmetic operations
            0x40..=0x75 | 0x77..=0xBF => {
                let reg = Self::get_opcode_reg(opcode);
                let val: u8;
                let mut long = false;
                if reg.is_none() {
                    val = self.mem.read_mem(self.mem.read_reg_16(&Reg16::HL));
                    long = true;
                } else {
                    val = self.mem.read_reg(&reg.unwrap());
                }

                match opcode {
                    // LD
                    0x40..=0x47 => self.mem.b = val,
                    0x48..=0x4F => self.mem.c = val,
                    0x50..=0x57 => self.mem.d = val,
                    0x58..=0x5F => self.mem.e = val,
                    0x60..=0x67 => self.mem.h = val,
                    0x68..=0x6F => self.mem.l = val,
                    0x70..=0x77 => {
                        self.mem.write_mem(self.mem.read_reg_16(&Reg16::HL), val);
                        long = true;
                    }
                    0x78..=0x7F => self.mem.a = val,

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
                if long {
                    2
                } else {
                    1
                }
            }
            // HALT
            0x76 => {
                self.halt = true;
                1
            }
            0xC0..=0xFF => {
                // Mask out the first nibble for easier pattern matching
                let nibble = opcode & 0x0F;
                match nibble {
                    0x0 | 0x2 | 0x3 | 0xA => {
                        // DI
                        if opcode == 0xF3 {
                            self.mem.ime = false;
                            1
                        }
                        // LD
                        else if opcode & 0xF0 >= 0xE0 {
                            let (address, cycles) = match nibble {
                                0x0 => (0xFF00u16 + self.mem.read_operand() as u16, 3),
                                0x2 => (0xFF00u16 + self.mem.c as u16, 2),
                                0xA => (self.mem.read_operand_16(), 4),
                                _ => panic!(),
                            };
                            if opcode & 0xF0 == 0xE0 {
                                self.mem.write_mem(address, self.mem.a);
                            } else {
                                self.mem.a = self.mem.read_mem(address);
                            }
                            cycles
                        } else {
                            let mut condition = self.get_opcode_condition(opcode);
                            match nibble {
                                // RET N
                                0x0 => {
                                    if !condition {
                                        increment_pc = false;
                                        self.mem.pc = self.mem.pop();
                                        5
                                    } else {
                                        2
                                    }
                                }
                                // JP
                                0x2 | 0x3 | 0xA => {
                                    if nibble == 0x2 {
                                        condition = !condition;
                                    }
                                    if nibble == 0x3 {
                                        condition = true;
                                    }

                                    let address = self.mem.read_operand_16();
                                    if condition {
                                        increment_pc = false;
                                        self.mem.pc = address;
                                        4
                                    } else {
                                        3
                                    }
                                }
                                _ => panic!(),
                            }
                        }
                    }
                    // POP r16
                    0x1 => {
                        let reg = Self::get_opcode_reg16(opcode - 0xC0).unwrap_or(Reg16::AF);
                        let val = self.mem.pop();
                        self.mem.write_reg_16(&reg, val);
                        3
                    }
                    // PUSH r16
                    0x5 => {
                        let reg = Self::get_opcode_reg16(opcode - 0xC0).unwrap_or(Reg16::AF);
                        self.mem.push(self.mem.read_reg_16(&reg));
                        4
                    }
                    // Arithmetics for d8
                    0x6 | 0xE => {
                        let val = self.mem.read_operand();
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
                        2
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

                        let address = self.mem.read_operand_16();
                        if condition {
                            increment_pc = false;
                            self.mem.push(self.mem.pc + 1);
                            self.mem.pc = address;
                            6
                        } else {
                            3
                        }
                    }
                    // RST
                    0x7 | 0xF => {
                        increment_pc = false;
                        self.halt = false;
                        self.mem.push(self.mem.pc + 1);
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
                        self.mem.pc = address;
                        4
                    }
                    0x8 => {
                        // ADD SP
                        if opcode >= 0xE0 {
                            let offset = (self.mem.read_operand() as i8) as i16;
                            let res = self.mem.sp.wrapping_add_signed(offset);

                            let (reg, cycles) = if opcode == 0xE8 {
                                (Reg16::SP, 4)
                            } else {
                                (Reg16::HL, 3)
                            };

                            self.mem.f.remove(FlagReg::ZERO);
                            self.mem.f.remove(FlagReg::SUBTRACT);
                            self.mem.f.set(
                                FlagReg::HALF_CARRY,
                                (self.mem.sp & 0x000F).wrapping_add_signed(offset & 0x000F)
                                    & 0x0010
                                    > 0,
                            );
                            self.mem.f.set(
                                FlagReg::CARRY,
                                (self.mem.sp & 0x00FF).wrapping_add_signed(offset & 0x00FF)
                                    & 0x0100
                                    > 0,
                            );

                            self.mem.write_reg_16(&reg, res);
                            cycles
                        }
                        // RET
                        else if self.get_opcode_condition(opcode) {
                            increment_pc = false;
                            self.mem.pc = self.mem.pop();
                            5
                        } else {
                            2
                        }
                    }
                    0x9 => match opcode {
                        // RET
                        0xC9 => {
                            increment_pc = false;
                            self.mem.pc = self.mem.pop();
                            4
                        }
                        // RETI
                        0xD9 => {
                            increment_pc = false;
                            self.mem.pc = self.mem.pop();
                            self.mem.ime = true;
                            4
                        }
                        // JP
                        0xE9 => {
                            increment_pc = false;
                            self.mem.pc = self.mem.read_reg_16(&Reg16::HL);
                            1
                        }
                        // LD
                        0xF9 => {
                            self.mem.sp = self.mem.read_reg_16(&Reg16::HL);
                            2
                        }
                        _ => panic!("Invalid instruction: {:#04X}", opcode),
                    },
                    0xB => {
                        // EI
                        if opcode == 0xFB {
                            self.mem.ime = true;
                            1
                        }
                        // 0xCB 16-bit opcodes
                        else {
                            self.arithmetic()
                        }
                    }
                    _ => panic!("Invalid instruction: {:#04X}", opcode),
                }
            }
        };
        if increment_pc {
            self.mem.pc += 1;
        }
        cycles
    }

    fn arithmetic(&mut self) -> u8 {
        let opcode = self.mem.read_operand();
        let reg = Self::get_opcode_reg(opcode);

        let val = if let Some(reg_val) = reg {
            self.mem.read_reg(&reg_val)
        } else {
            self.mem.read_mem(self.mem.read_reg_16(&Reg16::HL))
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
                self.mem.f = FlagReg::from_bits_truncate(0);
                self.mem.f.set(FlagReg::ZERO, res == 0);
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
                        self.mem.f.remove(FlagReg::SUBTRACT);
                        self.mem.f.insert(FlagReg::HALF_CARRY);
                        self.mem.f.set(FlagReg::ZERO, val & mask == 0);
                        return if reg.is_none() { 3 } else { 2 };
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
            self.mem.f.set(FlagReg::ZERO, res == 0);
        }

        if let Some(reg_val) = reg {
            self.mem.write_reg(&reg_val, res);
            2
        } else {
            self.mem.write_mem(self.mem.read_reg_16(&Reg16::HL), res);
            4
        }
    }

    fn rotate(&mut self, val: u8, left: bool, through_carry: bool) -> u8 {
        let carry_mask = if left { 0b10000000 } else { 0b00000001 };
        let carry = val & carry_mask > 0;

        let mut res = if left { val << 1 } else { val >> 1 };

        let overflow_bit = if left { 0b00000001 } else { 0b10000000 };
        if through_carry {
            if self.mem.f.intersects(FlagReg::CARRY) {
                res |= overflow_bit;
            }
        } else if carry {
            res |= overflow_bit;
        }

        self.mem.f = FlagReg::from_bits_truncate(0);
        self.mem.f.set(FlagReg::CARRY, carry);

        res
    }

    fn add_a(&mut self, val: u8, with_carry: bool) {
        let carry_val = if with_carry && self.mem.f.intersects(FlagReg::CARRY) {
            1
        } else {
            0
        };
        let (added_val, add_carry) = val.overflowing_add(carry_val);
        let (res, carry) = self.mem.a.overflowing_add(added_val);

        self.mem.f.set(FlagReg::ZERO, res == 0);
        self.mem.f.remove(FlagReg::SUBTRACT);
        self.mem.f.set(
            FlagReg::HALF_CARRY,
            (self.mem.a & 0x0F)
                .wrapping_add(val & 0x0F)
                .wrapping_add(carry_val)
                & 0x10
                > 0,
        );
        self.mem.f.set(FlagReg::CARRY, carry || add_carry);

        self.mem.a = res;
    }

    fn sub_a(&mut self, val: u8, with_carry: bool, set_a: bool) {
        let carry_val = if with_carry && self.mem.f.intersects(FlagReg::CARRY) {
            1
        } else {
            0
        };
        let (added_val, add_carry) = val.overflowing_add(carry_val);
        let (res, carry) = self.mem.a.overflowing_sub(added_val);

        self.mem.f.set(FlagReg::ZERO, res == 0);
        self.mem.f.insert(FlagReg::SUBTRACT);
        self.mem.f.set(
            FlagReg::HALF_CARRY,
            (self.mem.a & 0x0F)
                .wrapping_sub(val & 0x0F)
                .wrapping_sub(carry_val)
                & 0x10
                > 0,
        );
        self.mem.f.set(FlagReg::CARRY, carry || add_carry);

        if set_a {
            self.mem.a = res;
        }
    }

    fn and_a(&mut self, val: u8) {
        self.mem.a &= val;

        self.mem.f = FlagReg::from_bits_truncate(0);
        self.mem.f.set(FlagReg::ZERO, self.mem.a == 0);
        self.mem.f.insert(FlagReg::HALF_CARRY);
    }

    fn xor_a(&mut self, val: u8) {
        self.mem.a ^= val;

        self.mem.f = FlagReg::from_bits_truncate(0);
        self.mem.f.set(FlagReg::ZERO, self.mem.a == 0);
    }

    fn or_a(&mut self, val: u8) {
        self.mem.a |= val;

        self.mem.f = FlagReg::from_bits_truncate(0);
        self.mem.f.set(FlagReg::ZERO, self.mem.a == 0);
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
            self.mem.f.intersects(FlagReg::ZERO)
        } else {
            self.mem.f.intersects(FlagReg::CARRY)
        }
    }
}
