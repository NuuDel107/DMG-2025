use super::*;

#[derive(Deserialize, Serialize, Clone, Copy, PartialEq)]
pub struct MasterOnOff(u8);

bitflags! {
    impl MasterOnOff: u8 {
        const ALL = 0b1000_0000;
        const CH4 = 0b0000_1000;
        const CH3 = 0b0000_0100;
        const CH2 = 0b0000_0010;
        const CH1 = 0b0000_0001;
    }
}

/// Audio processing unit
#[allow(clippy::upper_case_acronyms)]
#[derive(Deserialize, Serialize)]
pub struct APU {
    pub buffer: Vec<f32>,
    pub sample_delay_counter: u32,
    pub sample_delay: u32,
    pub div_apu: u8,
    pub last_div_bit: bool,
    pub period_div: u16,
    pub period_div_delay: u8,
    pub duty_cycle_pointer: u8,

    pub master_control: MasterOnOff,
    pub duty_cycle_index: u8,
    pub initial_length_timer: u8,
    pub period_value: u16,
}

impl APU {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            buffer: vec![],
            sample_delay: 4194304 / sample_rate,
            sample_delay_counter: 0,
            div_apu: 0,
            last_div_bit: false,
            period_div: 0,
            period_div_delay: 0,
            duty_cycle_pointer: 0,

            master_control: MasterOnOff::from_bits_truncate(0),
            duty_cycle_index: 0,
            initial_length_timer: 0,
            period_value: 0,
        }
    }

    pub fn cycle(&mut self, timer_div: u16) {
        // Increment DIV-APU when DIV bit 4 goes from 1 to 0
        let div_bit = timer_div & 0b1_0000 > 0;
        if self.last_div_bit && !div_bit {
            self.div_apu = self.div_apu.wrapping_add(1);
        }
        self.last_div_bit = div_bit;

        // Increment period divider every 4 T-cycles
        if self.period_div_delay != 3 {
            self.period_div_delay += 1;
        } else {
            self.period_div_delay = 0;

            // Increment duty cycle pointer when period divider overflows
            if self.period_div != 0x7FF {
                if self.duty_cycle_pointer == 7 {
                    self.duty_cycle_pointer = 0;
                } else {
                    self.duty_cycle_pointer += 1;
                }
                self.period_div = self.period_value;
            } else {
                self.period_div += 1;
            }
        }

        // Only calculate next sample when needed
        if self.sample_delay_counter != self.sample_delay {
            self.sample_delay_counter += 1;
            return;
        }
        self.sample_delay_counter = 0;

        if self
            .master_control
            .intersects(MasterOnOff::ALL | MasterOnOff::CH1)
        {
            self.buffer
                .push(self.get_duty_cycle_val(self.duty_cycle_pointer) as f32 / 100.0);
        }
    }

    /// Called from outside:
    /// returns audio buffer for playback, and empties it
    pub fn receive_buffer(&mut self) -> Vec<f32> {
        let buf = self.buffer.clone();
        self.buffer = vec![];
        buf
    }

    fn get_duty_cycle_val(&self, index: u8) -> u8 {
        let duty_cycle = match self.duty_cycle_index {
            // 12.5 %
            0 => [1, 1, 1, 1, 1, 1, 1, 0],
            // 25 %
            1 => [0, 1, 1, 1, 1, 1, 1, 0],
            // 50 %
            2 => [0, 1, 1, 1, 1, 0, 0, 0],
            // 75 %
            3 => [1, 0, 0, 0, 0, 0, 0, 1],
            _ => unreachable!(),
        };
        duty_cycle[index as usize]
    }
}

impl MemoryAccess for APU {
    fn get_range(&self) -> Vec<RangeInclusive<u16>> {
        // Audio I/O registers
        vec![0xFF10..=0xFF3F]
    }

    fn mem_read(&self, address: u16) -> u8 {
        match address {
            0xFF11 => (self.duty_cycle_index << 6) | self.initial_length_timer,
            0xFF13 => (self.period_value & 0xFF) as u8,
            0xFF14 => (self.period_value >> 8) as u8,
            0xFF26 => self.master_control.bits(),
            _ => 0xFF,
        }
    }

    fn mem_write(&mut self, address: u16, value: u8) {
        match address {
            0xFF11 => {
                self.duty_cycle_index = value >> 6;
                self.initial_length_timer = value & 0b1100_0000;
            }
            0xFF13 => self.period_value = (self.period_value & 0xFF00) | value as u16,
            0xFF14 => self.period_value = (self.period_value & 0xFF) | ((value as u16) << 8),
            0xFF26 => self.master_control = MasterOnOff::from_bits_truncate(value),
            _ => {}
        }
    }
}
