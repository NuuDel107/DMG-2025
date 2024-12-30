use super::*;

impl CPU {
    pub fn execute(&mut self) {
        let opcode = self.mem.read_mem(self.mem.pc);
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
                        0x10 => todo!(),
                        // JR
                        _ => {
                            if !((opcode == 0x20 && self.mem.f.zero)
                                || (opcode == 0x30 && self.mem.f.carry))
                            {
                                let step = self.mem.read_operand() as i8;
                                self.mem.pc = self.mem.pc.overflowing_add_signed(step as i16).0;
                            }
                        }
                    },
                    // LD d16
                    0x1 => {
                        let reg = Self::get_opcode_reg16(opcode).unwrap_or(Reg16::SP);
                        let val = self.mem.read_operand_16();
                        self.mem.write_reg_16(&reg, val);
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

                        if opcode == 0x22 {
                            self.mem.write_reg_16(&reg, self.mem.read_reg_16(&reg) + 1);
                        }
                        if opcode == 0x32 {
                            self.mem.write_reg_16(&reg, self.mem.read_reg_16(&reg) - 1);
                        }
                    }

                    // INC/DEC r16
                    0x3 | 0xB => {
                        let reg = Self::get_opcode_reg16(opcode).unwrap_or(Reg16::SP);
                        let val = if nibble == 0x03 {
                            self.mem.read_reg_16(&reg) + 1
                        } else {
                            self.mem.read_reg_16(&reg) - 1
                        };
                        self.mem.write_reg_16(&reg, val);
                    }
                    // INC/DEC r8
                    0x4 | 0x5 | 0xC | 0xD => {
                        let offset = if nibble == 0x04 | 0x05 { 0 } else { 1 };
                        let reg = Self::get_opcode_reg(2 * (opcode >> 4) + offset);

                        let mut val: u8;
                        if reg.is_none() {
                            val = self.mem.read_mem(self.mem.read_reg_16(&Reg16::HL))
                        } else {
                            val = self.mem.read_reg(&reg.unwrap())
                        }

                        if nibble == 0x04 | 0x0C {
                            self.mem.f.subtract = false;
                            self.mem.f.half_carry = (val & 0x0F) == 0x0F;
                            val = val.overflowing_add(1).0;
                        } else {
                            self.mem.f.subtract = true;
                            self.mem.f.half_carry = (val & 0x0F) == 0x00;
                            val = val.overflowing_sub(1).0;
                        }
                        self.mem.f.zero = val == 0;

                        if reg.is_none() {
                            self.mem.write_mem(self.mem.read_reg_16(&Reg16::HL), val);
                        } else {
                            self.mem.write_reg(&reg.unwrap(), val);
                        }
                    }
                    // LD d8
                    0x6 | 0xE => {
                        let offset = if nibble == 0x06 { 0 } else { 1 };
                        let reg = Self::get_opcode_reg(2 * (opcode >> 4) + offset);
                        let val = self.mem.read_operand();

                        if reg.is_none() {
                            self.mem.write_mem(self.mem.read_reg_16(&Reg16::HL), val);
                        } else {
                            self.mem.write_reg(&reg.unwrap(), val);
                        };
                    }
                    0x7 => {
                        match opcode {
                            // RLC
                            0x07 => {
                                self.mem.a = self.rotate(self.mem.a, true, false);
                                self.mem.f.zero = false;
                            }
                            // RL
                            0x17 => {
                                self.mem.a = self.rotate(self.mem.a, true, true);
                                self.mem.f.zero = false;
                            }
                            // DAA (https://rgbds.gbdev.io/docs/v0.9.0/gbz80.7#DAA)
                            0x27 => {
                                let mut adj = 0;
                                let res;
                                let carry;

                                if self.mem.f.subtract {
                                    if self.mem.f.half_carry {
                                        adj += 0x6;
                                    }
                                    if self.mem.f.carry {
                                        adj += 0x60;
                                    }
                                    (res, carry) = self.mem.a.overflowing_sub(adj);
                                } else {
                                    if self.mem.f.half_carry || self.mem.a & 0xF > 0x9 {
                                        adj += 0x6;
                                    }
                                    if self.mem.f.carry || self.mem.a > 0x9F {
                                        adj += 0x60;
                                    }
                                    (res, carry) = self.mem.a.overflowing_add(adj);
                                }

                                self.mem.a = res;
                                self.mem.f.zero = res == 0;
                                self.mem.f.half_carry = false;
                                self.mem.f.carry = carry;
                            }
                            // SCF
                            0x37 => {
                                self.mem.f.subtract = false;
                                self.mem.f.half_carry = false;
                                self.mem.f.carry = true;
                            }
                            _ => eprintln!("Invalid instruction: {}", opcode),
                        }
                    }
                    0x8 => {
                        // LD SP
                        if opcode == 0x08 {
                            let bytes = self.mem.sp.to_le_bytes();
                            let address = self.mem.read_operand_16();
                            self.mem.write_mem(address, bytes[0]);
                            self.mem.write_mem(address + 1, bytes[1]);
                        }
                        // JR
                        else {
                            self.mem.pc += 1;
                            if !((opcode == 0x28 && !self.mem.f.zero)
                                || (opcode == 0x38 && !self.mem.f.carry))
                            {
                                let step = self.mem.read_mem(self.mem.pc) as i8;
                                self.mem.pc = self.mem.pc.overflowing_add_signed(step as i16).0;
                            }
                        }
                    }
                    // ADD r16
                    0x9 => {
                        let reg = Self::get_opcode_reg16(opcode).unwrap_or(Reg16::SP);
                        let reg_val = self.mem.read_reg_16(&reg);
                        let val = self.mem.read_reg_16(&Reg16::HL);
                        let (res, carry) = val.overflowing_add(reg_val);

                        self.mem.f.zero = res == 0;
                        self.mem.f.subtract = false;
                        self.mem.f.half_carry = (reg_val & 0x00FF) + val > 0x00FF;
                        self.mem.f.carry = carry;
                    }
                    0xF => {
                        match opcode {
                            // RRC
                            0x0F => {
                                self.mem.a = self.rotate(self.mem.a, false, false);
                                self.mem.f.zero = false;
                            }
                            // RR
                            0x1F => {
                                self.mem.a = self.rotate(self.mem.a, false, true);
                                self.mem.f.zero = false;
                            }
                            // CPL
                            0x2F => {
                                self.mem.a = !self.mem.a;
                                self.mem.f.subtract = true;
                                self.mem.f.half_carry = true;
                            }
                            // CCF
                            0x3F => {
                                self.mem.f.subtract = false;
                                self.mem.f.half_carry = false;
                                self.mem.f.carry = !self.mem.f.carry;
                            }
                            _ => eprintln!("Invalid instruction: {}", opcode),
                        }
                    }
                    _ => eprintln!("Invalid instruction: {}", opcode),
                }
            }
            // Similarly implemented 8-bit loading and arithmetic operations
            0x40..=0x75 | 0x77..=0x7F => {
                let reg = Self::get_opcode_reg(opcode);
                let val: u8;
                if reg.is_none() {
                    val = self.mem.read_mem(self.mem.read_reg_16(&Reg16::HL));
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
                    0x70..=0x77 => self.mem.write_mem(self.mem.read_reg_16(&Reg16::HL), val),
                    0x78..=0x7F => self.mem.a = val,

                    // ADD / ADC
                    0x80..=0x8F => self.add_a(val, opcode <= 0x88),
                    // SUB / SBC
                    0x90..=0x9F => self.sub_a(val, opcode <= 0x98),
                    // AND
                    0xA0..=0xA7 => self.and_a(val),
                    // XOR
                    0xA8..=0xAF => self.xor_a(val),
                    // OR
                    0xB0..=0xB7 => self.or_a(val),
                    // CP
                    0xB8..=0xBF => self.cp_a(val),
                    _ => eprintln!("Invalid instruction: {}", opcode),
                }
            }
            // HALT
            0x76 => todo!(),
            0xC0..=0xFF => {
                // Mask out the first nibble for easier pattern matching
                let nibble = opcode & 0x0F;
                match nibble {
                    0x0 | 0x2 | 0x3 | 0xA => {
                        // DI
                        if opcode == 0xF3 {
                            todo!()
                        }
                        // LD
                        else if opcode & 0xF0 >= 0xE0 {
                            let address: u16 = match nibble {
                                0x0 => 0xFF00u16 + self.mem.read_operand() as u16,
                                0x2 => 0xFF00u16 + self.mem.c as u16,
                                0xA => self.mem.read_operand_16(),
                                _ => panic!(),
                            };
                            if opcode & 0xF0 == 0xE0 {
                                self.mem.write_mem(address, self.mem.a);
                            } else {
                                self.mem.a = self.mem.read_mem(address);
                            }
                        } else {
                            let mut condition = self.get_opcode_condition(opcode);
                            match nibble {
                                // RET N
                                0x0 => {
                                    if !condition {
                                        increment_pc = false;
                                        self.mem.pc = self.mem.read_mem_16(self.mem.sp);
                                        self.mem.sp += 2;
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
                                    }
                                }
                                _ => panic!(),
                            }
                        }
                    }
                    // POP r16
                    0x1 => {
                        let reg = Self::get_opcode_reg16(opcode).unwrap_or(Reg16::AF);
                        let val = self.mem.pop();
                        self.mem.write_reg_16(&reg, val);
                    }
                    // PUSH r16
                    0x5 => {
                        let reg = Self::get_opcode_reg16(opcode).unwrap_or(Reg16::AF);
                        self.mem.push(self.mem.read_reg_16(&reg))
                    }
                    // Arithmetics for d8
                    0x6 | 0xE => {
                        let val = self.mem.read_operand();
                        match opcode {
                            0xC6 => self.add_a(val, false),
                            0xCF => self.add_a(val, true),
                            0xD6 => self.sub_a(val, false),
                            0xDF => self.sub_a(val, true),
                            0xE6 => self.and_a(val),
                            0xEF => self.xor_a(val),
                            0xF6 => self.or_a(val),
                            0xFF => self.cp_a(val),
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

                        let address = self.mem.read_operand_16();
                        if condition {
                            increment_pc = false;
                            self.mem.push(self.mem.pc);
                            self.mem.pc = address;
                        }
                    }
                    // RST
                    0x7 | 0xF => {
                        increment_pc = false;
                        self.mem.push(self.mem.pc);
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
                    }
                    0x8 => {
                        // ADD SP
                        if opcode <= 0xE8 {
                            let offset = self.mem.read_operand() as i8;
                            let (res, carry) = self.mem.sp.overflowing_add_signed(offset as i16);

                            let reg = if opcode == 0xE8 { Reg16::SP } else { Reg16::HL };
                            self.mem.write_reg_16(&reg, res);

                            self.mem.f.zero = false;
                            self.mem.f.subtract = false;
                            self.mem.f.half_carry = ((self.mem.sp & 0x00FF) as u8)
                                .overflowing_add_signed(offset)
                                .1;
                            self.mem.f.carry = carry;
                        }
                        // RET
                        else if self.get_opcode_condition(opcode) {
                            increment_pc = false;
                            self.mem.pc = self.mem.read_mem_16(self.mem.sp);
                            self.mem.sp += 2;
                        }
                    }
                    0x9 => match opcode {
                        // RET
                        0xC9 => self.mem.pc = self.mem.pop(),
                        // RETI
                        0xD9 => todo!(),
                        // JP
                        0xE9 => {
                            increment_pc = false;
                            self.mem.pc = self.mem.read_reg_16(&Reg16::HL);
                        }
                        // LD
                        0xF9 => {
                            increment_pc = false;
                            self.mem.sp = self.mem.read_reg_16(&Reg16::HL);
                        }
                        _ => {}
                    },
                    0xB => {
                        // EI
                        if opcode == 0xFB {
                            todo!()
                        }
                        // 0xCB 16-bit opcodes
                        else {
                            self.arithmetic();
                        }
                    }
                    _ => eprintln!("Invalid instruction: {}", opcode),
                }
            }
            _ => eprintln!("Invalid instruction: {}", opcode),
        }
        if increment_pc {
            self.mem.pc += 1;
        }
    }

    fn arithmetic(&mut self) {
        let opcode = self.mem.read_operand();
        let reg = Self::get_opcode_reg(opcode);
        let val: u8;
        if reg.is_none() {
            val = self.mem.read_mem(self.mem.read_reg_16(&Reg16::HL));
        } else {
            val = self.mem.read_reg(&reg.unwrap());
        }

        let res: u8 = match opcode {
            // RLC
            0x00..=0x07 => self.rotate(val, true, false),
            // RRC
            0x08..=0x0F => self.rotate(val, true, true),
            // RL
            0x10..=0x17 => self.rotate(val, false, false),
            // RR
            0x18..=0x1F => self.rotate(val, false, true),
            // SLA
            0x20..=0x27 => self.rotate(val, true, false) & 0b1111_1110,
            // SRL
            0x28..=0x2F => self.rotate(val, false, false) | (val & 0b1000_0000),
            // SWAP
            0x30..=0x37 => {
                let res = val.rotate_right(4);
                self.mem.f = 0.into();
                self.mem.f.zero = res == 0;
                res
            }
            // SRL
            0x38..=0x3F => self.rotate(val, false, false) & 0b0111_1111,
            _ => {
                let bit = (opcode - 0x40) / 0x8;
                let mask = 0b1u8 << bit;
                match opcode {
                    // BIT
                    0x40..=0x7F => {
                        self.mem.f.zero = val & mask > 0;
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

        if reg.is_none() {
            self.mem.write_mem(self.mem.read_reg_16(&Reg16::HL), res);
        } else {
            self.mem.write_reg(&reg.unwrap(), res);
        }
    }

    fn rotate(&mut self, val: u8, left: bool, through_carry: bool) -> u8 {
        let carry_mask = if left { 0b10000000 } else { 0b00000001 };
        let carry = val & carry_mask > 0;

        let mut res = if left { val << 1 } else { val >> 1 };

        let overflow_bit = if left { 0b00000001 } else { 0b10000000 };
        if through_carry {
            if self.mem.f.carry {
                res |= overflow_bit;
            }
        } else if carry {
            res |= overflow_bit;
        }

        self.mem.f = 0.into();
        self.mem.f.zero = res == 0;
        self.mem.f.carry = carry;

        res
    }

    fn add_a(&mut self, mut val: u8, with_carry: bool) {
        if with_carry && self.mem.f.carry {
            val += 1;
        }
        let (res, carry) = self.mem.a.overflowing_add(val);
        self.mem.a = res;

        self.mem.f.zero = res == 0;
        self.mem.f.subtract = false;
        self.mem.f.half_carry = (self.mem.a & 0x0F) + val > 0x0F;
        self.mem.f.carry = carry;
    }

    fn sub_a(&mut self, mut val: u8, with_carry: bool) {
        if with_carry && self.mem.f.carry {
            val += 1;
        }
        let (res, carry) = self.mem.a.overflowing_sub(val);
        self.mem.a = res;

        self.mem.f.zero = res == 0;
        self.mem.f.subtract = true;
        self.mem.f.half_carry = (self.mem.a & 0x0F) < val;
        self.mem.f.carry = carry;
    }

    fn and_a(&mut self, val: u8) {
        self.mem.a &= val;

        self.mem.f = 0.into();
        self.mem.f.zero = self.mem.a == 0;
        self.mem.f.half_carry = true;
    }

    fn xor_a(&mut self, val: u8) {
        self.mem.a ^= val;

        self.mem.f = 0.into();
        self.mem.f.zero = self.mem.a == 0;
    }

    fn or_a(&mut self, val: u8) {
        self.mem.a |= val;

        self.mem.f = 0.into();
        self.mem.f.zero = self.mem.a == 0;
    }

    fn cp_a(&mut self, val: u8) {
        let (res, carry) = self.mem.a.overflowing_sub(val);

        self.mem.f.zero = res == 0;
        self.mem.f.subtract = true;
        self.mem.f.half_carry = (self.mem.a & 0x0F) < val;
        self.mem.f.carry = carry;
    }

    fn get_opcode_reg(opcode: u8) -> Option<Reg8> {
        let mut nibble = opcode & 0x0F;
        if nibble > 0x08 {
            nibble <<= 2;
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
        let nibble = (opcode & 0xF0) >> 2;
        match nibble {
            0x00 => Some(Reg16::BC),
            0x01 => Some(Reg16::DE),
            0x02 => Some(Reg16::HL),
            _ => None,
        }
    }

    fn get_opcode_condition(&self, opcode: u8) -> bool {
        if opcode & 0xF0 == 0xC0 {
            self.mem.f.zero
        } else {
            self.mem.f.carry
        }
    }
}
