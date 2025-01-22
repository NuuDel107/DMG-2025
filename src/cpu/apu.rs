use super::*;

#[derive(Deserialize, Serialize)]
pub struct SquareChannel {
    // State variables
    pub on: bool,
    pub sweep: bool,
    pub period_div: u16,
    pub duty_cycle_pointer: u8,
    pub length_timer: u8,
    pub volume: u8,
    pub envelope_timer: u8,
    // Register variables
    pub duty_cycle_index: u8,
    pub initial_length_timer: u8,
    pub length_timer_enabled: bool,
    pub period_value: u16,
    pub initial_volume: u8,
    pub envelope_increase: bool,
    pub envelope_pace: u8,
}

impl SquareChannel {
    pub fn new(sweep: bool) -> Self {
        Self {
            on: false,
            sweep,
            period_div: 0,
            duty_cycle_pointer: 0,
            length_timer: 64,
            volume: 0,
            envelope_timer: 1,

            duty_cycle_index: 0,
            initial_length_timer: 0,
            length_timer_enabled: false,
            period_value: 0,
            initial_volume: 0,
            envelope_increase: false,
            envelope_pace: 0,
        }
    }

    pub fn read_register(&self, reg_index: u16) -> u8 {
        match reg_index {
            0 => 0x00,
            1 => (self.duty_cycle_index << 6) | self.initial_length_timer,
            2 => {
                (self.initial_volume << 4)
                    | ((self.envelope_increase as u8) << 3)
                    | self.envelope_pace
            }
            3 => (self.period_value & 0xFF) as u8,
            4 => (self.period_value >> 8) as u8 | ((self.length_timer_enabled as u8) << 6),
            _ => unreachable!(),
        }
    }

    pub fn write_register(&mut self, reg_index: u16, value: u8) {
        match reg_index {
            0 => {}
            1 => {
                self.duty_cycle_index = value >> 6;
                self.initial_length_timer = value & 0b11_1111;
            }
            2 => {
                self.initial_volume = value >> 4;
                self.envelope_increase = value & 0b1000 > 0;
                self.envelope_pace = value & 0b0111;
            }
            3 => self.period_value = (self.period_value & 0xFF00) | value as u16,
            4 => {
                self.length_timer_enabled = value & 0b0100_0000 > 0;
                self.period_value = (self.period_value & 0xFF) | (((value & 0b111) as u16) << 8);
            }
            _ => unreachable!(),
        }
    }

    pub fn update_length_timer(&mut self) {
        if self.length_timer == 64 {
            if self.length_timer_enabled {
                self.on = false;
            }
        } else {
            self.length_timer += 1;
        }
    }

    pub fn update_envelope(&mut self) {
        // Pace of 0 disables the envelope
        if self.envelope_pace == 0 {
            return;
        }
        if self.envelope_timer < self.envelope_pace {
            self.envelope_timer += 1;
        } else {
            self.envelope_timer = 1;
            if self.envelope_increase {
                if self.volume < 15 {
                    self.volume += 1;
                }
            } else if self.volume > 0 {
                self.volume -= 1;
            }
        }
    }

    pub fn update_period(&mut self) {
        if self.period_div == 0x7FF {
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

    pub fn trigger(&mut self) {
        self.on = true;
        self.period_div = self.period_value;
        self.volume = self.initial_volume;
        self.envelope_timer = 1;
        if self.length_timer == 64 {
            self.length_timer = self.initial_length_timer;
        }
    }

    pub fn get_sample(&self) -> f32 {
        if self.on {
            let val = self.get_duty_cycle_val(self.duty_cycle_pointer) as f32;
            val * (self.volume as f32) / 15.0
        } else {
            0.0
        }
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

/// Audio processing unit
#[allow(clippy::upper_case_acronyms)]
#[derive(Deserialize, Serialize)]
pub struct APU {
    pub on: bool,
    pub buffer: Vec<f32>,
    pub sample_delay_counter: u32,
    pub sample_delay: u32,
    pub period_delay_counter: u8,
    pub div_apu: u8,
    pub last_div_bit: bool,
    pub square_channel_1: SquareChannel,
    pub square_channel_2: SquareChannel,
}

impl APU {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            on: true,
            buffer: vec![],
            sample_delay: 4194304 / sample_rate,
            // sample_delay: 0,
            sample_delay_counter: 0,
            period_delay_counter: 0,
            div_apu: 0,
            last_div_bit: false,

            square_channel_1: SquareChannel::new(true),
            square_channel_2: SquareChannel::new(false),
        }
    }

    pub fn cycle(&mut self, timer_div: u16) {
        // Increment DIV-APU when DIV bit 4 (actual divider bit 12) goes from 1 to 0
        let div_bit = timer_div & 0b1_0000_0000_0000 > 0;
        if self.last_div_bit && !div_bit {
            self.div_apu = self.div_apu.wrapping_add(1);
            // Update length timers at 256hz (every 2 ticks)
            if self.div_apu % 2 == 0 {
                self.square_channel_1.update_length_timer();
                self.square_channel_2.update_length_timer();
            }
            // Update envelopes at 64hz (every 8 ticks)
            if self.div_apu % 8 == 0 {
                self.square_channel_1.update_envelope();
                self.square_channel_2.update_envelope();
            }
        }
        self.last_div_bit = div_bit;

        // Increment period divider every 4 T-cycles
        if self.period_delay_counter == 3 {
            self.period_delay_counter = 0;
            self.square_channel_1.update_period();
            self.square_channel_2.update_period();
        } else {
            self.period_delay_counter += 1;
        }

        // Only calculate next sample when needed
        if self.sample_delay_counter != self.sample_delay {
            self.sample_delay_counter += 1;
            return;
        }
        self.sample_delay_counter = 0;

        // If APU is turned off, just push silence to the buffer
        if !self.on {
            self.buffer.push(0.0);
            return;
        }

        let ch1 = self.square_channel_1.get_sample();
        let ch2 = self.square_channel_2.get_sample();
        let val = (ch1 + ch2).clamp(0.0, 1.0) * 0.1;
        self.buffer.push(val);
    }

    /// Called from outside:
    /// returns audio buffer for playback, and empties it
    pub fn receive_buffer(&mut self) -> Vec<f32> {
        let buf = self.buffer.clone();
        self.buffer = vec![];
        buf
    }
}

impl MemoryAccess for APU {
    fn get_range(&self) -> Vec<RangeInclusive<u16>> {
        // Audio I/O registers
        vec![0xFF10..=0xFF3F]
    }

    fn mem_read(&self, address: u16) -> u8 {
        match address {
            0xFF10..=0xFF14 => self.square_channel_1.read_register(address - 0xFF10),
            0xFF16..=0xFF19 => self.square_channel_2.read_register(address - 0xFF15),
            0xFF26 => {
                ((self.on as u8) << 7)
                    | ((self.square_channel_2.on as u8) << 1)
                    | (self.square_channel_1.on as u8)
            }
            _ => 0xFF,
        }
    }

    fn mem_write(&mut self, address: u16, value: u8) {
        match address {
            0xFF10..=0xFF14 => {
                self.square_channel_1
                    .write_register(address - 0xFF10, value);
                if address == 0xFF14 && (value & 0b1000_0000 > 0) {
                    self.square_channel_1.trigger();
                }
            }
            0xFF16..=0xFF19 => {
                self.square_channel_2
                    .write_register(address - 0xFF15, value);
                if address == 0xFF19 && (value & 0b1000_0000 > 0) {
                    self.square_channel_2.trigger();
                }
            }
            0xFF26 => {
                self.on = value & 0b1000_0000 > 0;
            }
            _ => {}
        }
    }
}
