use super::*;

/// Trait implemented by objects whose registers can be accessed from the address bus
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
        let targets: Vec<&dyn MemoryAccess> = vec![
            &self.mem,
            &self.ppu,
            &self.apu,
            &self.input,
            &self.timer,
            &self.istate,
        ];
        for target in targets {
            for range in target.get_range() {
                if range.contains(&address) {
                    return target.mem_read(address);
                }
            }
        }
        // eprintln!("No target found for reading from address {:#06X}", address);
        0xFF
    }

    /// Reads 16-bit value from given memory address
    pub fn read_16(&self, address: u16) -> u16 {
        u16::from_le_bytes([self.read(address), self.read(address + 1)])
    }

    /// Writes to given memory address
    pub fn write(&mut self, address: u16, value: u8) {
        let targets: Vec<&mut dyn MemoryAccess> = vec![
            &mut self.mem,
            &mut self.ppu,
            &mut self.apu,
            &mut self.input,
            &mut self.timer,
            &mut self.istate,
        ];
        for target in targets {
            for range in target.get_range() {
                if range.contains(&address) {
                    return target.mem_write(address, value);
                }
            }
        }
        // eprintln!("No target found for writing to address {:#06X}", address);
    }

    /// Returns the immediate 8-bit operand from memory.
    /// Increments program counter and cycles the system for one M-cycle
    pub fn read_operand(&mut self) -> u8 {
        self.cycle(1);
        self.reg.pc = self.reg.pc.wrapping_add(1);
        self.read(self.reg.pc)
    }

    /// Returns the immediate 16-bit operand from memory.
    /// Increments program counter and cycles the system for two M-cycles
    pub fn read_operand_16(&mut self) -> u16 {
        self.cycle(2);
        self.reg.pc = self.reg.pc.wrapping_add(2);
        self.read_16(self.reg.pc - 1)
    }

    /// Pops word from memory stack and increments stack pointer.
    /// Also cycles system for two M-cycles
    pub fn pop(&mut self) -> u16 {
        self.cycle(2);
        let val = self.read_16(self.reg.sp);
        self.reg.sp = self.reg.sp.wrapping_add(2);
        val
    }

    /// Pushes word into memory stack and decrements stack pointer
    /// Also cycles system for two M-cycles
    pub fn push(&mut self, value: u16) {
        self.cycle(2);
        let bytes = value.to_le_bytes();
        self.reg.sp = self.reg.sp.wrapping_sub(2);
        self.write(self.reg.sp.wrapping_add(1), bytes[1]);
        self.write(self.reg.sp, bytes[0]);
    }

    /// Starts OAM DMA transfer, which copies memory from given source address to OAM
    pub fn oam_dma(&mut self, address: u8) {
        let source_address = (address as u16) * 0x100;
        for sprite_index in 0..40 {
            let sprite_address = source_address + (sprite_index * 4);
            let mut data = [0u8; 4];
            for i in 0..4u16 {
                data[i as usize] = self.read(sprite_address + i);
            }
            self.ppu.oam.sprites[sprite_index as usize] = OAMSprite::from(data);
        }
    }
}
