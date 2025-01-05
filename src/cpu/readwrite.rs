use super::*;

/// Trait implemented by objects that have values that need to be accessed from the address bus
pub trait MemoryAccess {
    /// Returns vec of memory address ranges supported by this object
    fn get_range(&self) -> Vec<RangeInclusive<u16>>;
    /// Returns value from given memory address
    fn mem_read(&self, address: u16) -> u8;
    /// Writes given value to given memory address
    fn mem_write(&mut self, address: u16, value: u8);
}

impl CPU {
    /// Reads from given memory address
    pub fn read(&self, address: u16) -> u8 {
        let targets: Vec<&dyn MemoryAccess> = vec![&self.mem, &self.ppu, &self.io];
        for target in targets {
            for range in target.get_range() {
                if range.contains(&address) {
                    return target.mem_read(address);
                }
            }
        }
        eprintln!("No target found for reading from address {:#06X}", address);
        0
    }

    /// Reads 16-bit value from given memory address
    pub fn read_16(&self, address: u16) -> u16 {
        u16::from_le_bytes([self.read(address), self.read(address + 1)])
    }

    /// Writes to given memory address
    pub fn write(&mut self, address: u16, value: u8) {
        let targets: Vec<&mut dyn MemoryAccess> = vec![&mut self.mem, &mut self.ppu, &mut self.io];
        for target in targets {
            for range in target.get_range() {
                if range.contains(&address) {
                    return target.mem_write(address, value);
                }
            }
        }
        eprintln!("No target found for writing to address {:#06X}", address);
    }

    /// Returns the immediate 8-bit operand from memory and increments program counter
    pub fn read_operand(&mut self) -> u8 {
        self.reg.pc += 1;
        self.read(self.reg.pc)
    }

    /// Returns the 16-bit immediate operand from memory and increments program counter
    pub fn read_operand_16(&mut self) -> u16 {
        self.reg.pc += 2;
        self.read_16(self.reg.pc - 1)
    }

    // Pops word from memory stack and increments stack pointer
    pub fn pop(&mut self) -> u16 {
        let val = self.read_16(self.reg.sp);
        self.reg.sp += 2;
        val
    }

    // Pushes word into memory stack and decrements stack pointer
    pub fn push(&mut self, value: u16) {
        let bytes = value.to_le_bytes();
        self.write(self.reg.sp - 1, bytes[1]);
        self.write(self.reg.sp - 2, bytes[0]);
        self.reg.sp -= 2;
    }
}
