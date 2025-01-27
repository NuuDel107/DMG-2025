use super::*;

#[derive(Deserialize, Serialize)]
pub struct SquareChannel {
    // State variables
    pub on: bool,
    pub period_div: u16,
    pub duty_cycle_pointer: u8,
    pub length_timer: u8,
    pub volume: u8,
    pub period: u16,
    pub sweep_timer: u8,
    pub envelope_timer: u8,
    // Register variables
    pub sweep_pace: u8,
    pub sweep_increase: bool,
    pub sweep_step: u8,
    pub duty_cycle_index: u8,
    pub initial_length_timer: u8,
    pub length_timer_enabled: bool,
    pub initial_period: u16,
    pub initial_volume: u8,
    pub envelope_increase: bool,
    pub envelope_pace: u8,
}

impl SquareChannel {
    pub fn new() -> Self {
        Self {
            on: false,
            period_div: 0,
            duty_cycle_pointer: 0,
            length_timer: 64,
            volume: 0,
            period: 0,
            sweep_timer: 1,
            envelope_timer: 1,

            sweep_pace: 0,
            sweep_increase: true,
            sweep_step: 0,
            duty_cycle_index: 0,
            initial_length_timer: 0,
            length_timer_enabled: false,
            initial_period: 0,
            initial_volume: 0,
            envelope_increase: false,
            envelope_pace: 0,
        }
    }

    pub fn read_register(&self, reg_index: u16) -> u8 {
        match reg_index {
            0 => (self.sweep_pace << 4) | ((!self.sweep_increase as u8) << 3) | self.sweep_step,
            1 => (self.duty_cycle_index << 6) | self.initial_length_timer,
            2 => {
                (self.initial_volume << 4)
                    | ((self.envelope_increase as u8) << 3)
                    | self.envelope_pace
            }
            3 => (self.initial_period & 0xFF) as u8,
            4 => (self.initial_period >> 8) as u8 | ((self.length_timer_enabled as u8) << 6),
            _ => unreachable!(),
        }
    }

    pub fn write_register(&mut self, reg_index: u16, value: u8) {
        match reg_index {
            0 => {
                self.sweep_pace = value >> 4;
                // 0 == increase
                self.sweep_increase = value & 0b1000 == 0;
                self.sweep_step = value & 0b0111;
            }
            1 => {
                self.duty_cycle_index = value >> 6;
                self.initial_length_timer = value & 0b11_1111;
            }
            2 => {
                self.initial_volume = value >> 4;
                self.envelope_increase = value & 0b1000 > 0;
                self.envelope_pace = value & 0b0111;
            }
            3 => self.initial_period = (self.initial_period & 0xFF00) | value as u16,
            4 => {
                self.length_timer_enabled = value & 0b0100_0000 > 0;
                self.initial_period =
                    (self.initial_period & 0xFF) | (((value & 0b111) as u16) << 8);
                if value & 0b1000_0000 > 0 {
                    self.trigger();
                }
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

    pub fn update_sweep(&mut self) {
        // Pace of 0 disables period sweep
        if self.sweep_pace == 0 {
            return;
        }
        if self.sweep_timer < self.sweep_pace {
            self.sweep_timer += 1;
        } else {
            self.sweep_timer = 1;
            let period_change = self.period / 2u16.pow(self.sweep_step.into());
            if self.sweep_increase {
                self.period += period_change;
            } else if self.volume > 0 {
                self.period -= period_change;
            }
            if self.period > 0x7FF {
                self.period = 0x7FF;
                self.on = false;
            }
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
            self.period_div = self.period;
        } else {
            self.period_div += 1;
        }
    }

    pub fn trigger(&mut self) {
        self.on = true;
        self.period = self.initial_period;
        self.period_div = self.period;
        self.volume = self.initial_volume;
        self.envelope_timer = 1;
        self.sweep_timer = 1;
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

#[derive(Deserialize, Serialize)]
pub struct WaveChannel {
    // State variables
    pub on: bool,
    pub period_div: u16,
    pub wave_pointer: u8,
    pub length_timer: u8,
    pub output_level: u8,
    pub period: u16,
    // Register variables
    pub initial_length_timer: u8,
    pub length_timer_enabled: bool,
    pub initial_period: u16,
    pub wave_ram: [u8; 0x10],
}

impl WaveChannel {
    pub fn new() -> Self {
        Self {
            on: false,
            period_div: 0,
            wave_pointer: 0,
            length_timer: 64,
            output_level: 0,
            period: 0,

            initial_length_timer: 0,
            length_timer_enabled: false,
            initial_period: 0,
            wave_ram: [0; 0x10],
        }
    }

    pub fn read_register(&self, address: u16) -> u8 {
        match address {
            0xFF1A => (self.on as u8) << 7,
            0xFF1B => self.initial_length_timer,
            0xFF1C => self.output_level << 5,
            0xFF1D => (self.initial_period & 0xFF) as u8,
            0xFF1E => (self.initial_period >> 8) as u8 | ((self.length_timer_enabled as u8) << 6),
            0xFF30..=0xFF3F => self.wave_ram[(address - 0xFF30) as usize],
            _ => unreachable!(),
        }
    }

    pub fn write_register(&mut self, address: u16, value: u8) {
        match address {
            0xFF1A => self.on = value & 0b1000_0000 > 0,
            0xFF1B => self.initial_length_timer = value,
            0xFF1C => self.output_level = (value >> 5) & 0b11,
            0xFF1D => self.initial_period = (self.initial_period & 0xFF00) | value as u16,
            0xFF1E => {
                self.length_timer_enabled = value & 0b0100_0000 > 0;
                self.initial_period =
                    (self.initial_period & 0xFF) | (((value & 0b111) as u16) << 8);
                if value & 0b1000_0000 > 0 {
                    self.trigger();
                }
            }
            0xFF30..=0xFF3F => self.wave_ram[(address - 0xFF30) as usize] = value,
            _ => unreachable!(),
        }
    }

    pub fn update_length_timer(&mut self) {
        if self.length_timer == 255 {
            if self.length_timer_enabled {
                self.on = false;
            }
        } else {
            self.length_timer += 1;
        }
    }

    pub fn update_period(&mut self) {
        if self.period_div == 0x7FF {
            if self.wave_pointer == 31 {
                self.wave_pointer = 0;
            } else {
                self.wave_pointer += 1;
            }
            self.period_div = self.period;
        } else {
            self.period_div += 1;
        }
    }

    pub fn trigger(&mut self) {
        self.on = true;
        self.period = self.initial_period;
        self.period_div = self.period;
        if self.length_timer == 64 {
            self.length_timer = self.initial_length_timer;
        }
    }

    pub fn get_sample(&self) -> f32 {
        if self.on {
            let byte = self.wave_ram[(self.wave_pointer / 2) as usize];
            let nibble = if self.wave_pointer & 2 == 0 {
                byte >> 4
            } else {
                byte & 0xF
            };
            let val = match self.output_level {
                0 => 0,
                1 => nibble,
                2 => nibble >> 1,
                3 => nibble >> 2,
                _ => unreachable!(),
            };
            val as f32 / 15.0
        } else {
            0.0
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct NoiseChannel {
    // State variables
    pub on: bool,
    pub duty_cycle_pointer: u8,
    pub length_timer: u8,
    pub volume: u8,
    pub envelope_timer: u8,
    pub lfsr: u16,
    pub lfsr_bit: bool,
    pub lfsr_timer: u16,
    pub lfsr_pace: u16,
    // Register variables
    pub clock_shift: u8,
    pub short_lfsr: bool,
    pub clock_divider: u8,
    pub initial_length_timer: u8,
    pub length_timer_enabled: bool,
    pub initial_volume: u8,
    pub envelope_increase: bool,
    pub envelope_pace: u8,
}

impl NoiseChannel {
    pub fn new() -> Self {
        Self {
            on: false,
            duty_cycle_pointer: 0,
            length_timer: 64,
            volume: 0,
            envelope_timer: 1,
            lfsr: 0,
            lfsr_bit: false,
            lfsr_timer: 1,
            lfsr_pace: 1,

            clock_shift: 0,
            short_lfsr: false,
            clock_divider: 0,
            initial_length_timer: 0,
            length_timer_enabled: false,
            initial_volume: 0,
            envelope_increase: false,
            envelope_pace: 0,
        }
    }

    pub fn read_register(&self, address: u16) -> u8 {
        match address {
            0xFF20 => self.initial_length_timer,
            0xFF21 => {
                (self.initial_volume << 4)
                    | ((self.envelope_increase as u8) << 3)
                    | self.envelope_pace
            }
            0xFF22 => (self.clock_shift << 4) | ((self.short_lfsr as u8) << 3) | self.clock_divider,
            0xFF23 => (self.length_timer_enabled as u8) << 6,
            _ => unreachable!(),
        }
    }

    pub fn write_register(&mut self, address: u16, value: u8) {
        match address {
            0xFF20 => self.initial_length_timer = value & 0b0011_1111,
            0xFF21 => {
                self.initial_volume = value >> 4;
                self.envelope_increase = value & 0b1000 > 0;
                self.envelope_pace = value & 0b0111;
            }
            0xFF22 => {
                self.clock_shift = value >> 4;
                self.short_lfsr = value & 0b1000 > 0;
                self.clock_divider = value & 0b0111;
                // Divider value 0 is treated as 0.5
                self.lfsr_pace = if self.clock_divider == 0 {
                    2u16.pow(self.clock_shift as u32) / 2
                } else {
                    (self.clock_divider as u16) * 2u16.pow(self.clock_shift as u32)
                };
            }
            0xFF23 => {
                self.length_timer_enabled = value & 0b0100_0000 > 0;
                if value & 0b1000_0000 > 0 {
                    self.trigger();
                }
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

    pub fn update_lfsr(&mut self) {
        if self.lfsr_timer < self.lfsr_pace {
            self.lfsr_timer += 1;
        } else {
            self.lfsr_timer = 1;
            let xnor = self.lfsr & 0b1 == (self.lfsr & 0b10) >> 1;
            if xnor {
                self.lfsr |= (xnor as u16) << 15;
                if self.short_lfsr {
                    self.lfsr |= (xnor as u16) << 7;
                }
            }
            self.lfsr >>= 1;
            self.lfsr_bit = self.lfsr & 0b1 > 0;
        }
    }

    pub fn trigger(&mut self) {
        self.on = true;
        self.lfsr = 0;
        self.volume = self.initial_volume;
        self.envelope_timer = 1;
        if self.length_timer == 64 {
            self.length_timer = self.initial_length_timer;
        }
    }

    pub fn get_sample(&self) -> f32 {
        if self.on && self.lfsr_bit {
            (self.volume as f32) / 15.0
        } else {
            0.0
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Copy, PartialEq)]
pub struct PanRegister(u8);

bitflags! {
    impl PanRegister: u8 {
        const CH4_LEFT  = 0b1000_0000;
        const CH3_LEFT  = 0b0100_0000;
        const CH2_LEFT  = 0b0010_0000;
        const CH1_LEFT  = 0b0001_0000;
        const CH4_RIGHT = 0b0000_1000;
        const CH3_RIGHT = 0b0000_0100;
        const CH2_RIGHT = 0b0000_0010;
        const CH1_RIGHT = 0b0000_0001;
    }
}

/// Audio processing unit
#[allow(clippy::upper_case_acronyms)]
#[derive(Deserialize, Serialize)]
pub struct APU {
    pub on: bool,
    #[serde(skip)]
    pub buffer: Vec<f32>,
    pub sample_delay_counter: u32,
    pub sample_delay: u32,
    pub period_delay_counter: u8,
    pub div_apu: u8,
    pub last_div_bit: bool,
    pub pan_options: PanRegister,
    pub left_volume: u8,
    pub right_volume: u8,
    pub square_channel_1: SquareChannel,
    pub square_channel_2: SquareChannel,
    pub wave_channel: WaveChannel,
    pub noise_channel: NoiseChannel,
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
            pan_options: PanRegister::from_bits_truncate(0),
            left_volume: 0,
            right_volume: 0,

            square_channel_1: SquareChannel::new(),
            square_channel_2: SquareChannel::new(),
            wave_channel: WaveChannel::new(),
            noise_channel: NoiseChannel::new(),
        }
    }

    pub fn cycle(&mut self, timer_div: u16) {
        // Increment DIV-APU when DIV register bit 4 (actual divider bit 12)
        // goes from 1 to 0
        let div_bit = timer_div & 0b1_0000_0000_0000 > 0;
        if self.last_div_bit && !div_bit {
            self.div_apu = self.div_apu.wrapping_add(1);
            // Update length timers at 256hz (every 2 ticks)
            if self.div_apu % 2 == 0 {
                self.square_channel_1.update_length_timer();
                self.square_channel_2.update_length_timer();
                self.wave_channel.update_length_timer();
                self.noise_channel.update_length_timer();
                // Update CH1 period sweep at 128hz (every 4 ticks)
                if self.div_apu % 4 == 0 {
                    self.square_channel_1.update_sweep();
                    // Update envelopes at 64hz (every 8 ticks)
                    if self.div_apu % 8 == 0 {
                        self.square_channel_1.update_envelope();
                        self.square_channel_2.update_envelope();
                        self.noise_channel.update_envelope();
                    }
                }
            }
        }
        self.last_div_bit = div_bit;

        self.period_delay_counter = self.period_delay_counter.wrapping_add(1);
        // Update wave channel period every 2 T-cycles
        if self.period_delay_counter % 2 == 0 {
            self.wave_channel.update_period();
            // Update square channel period every 4 T-cycles
            if self.period_delay_counter % 4 == 0 {
                self.square_channel_1.update_period();
                self.square_channel_2.update_period();
                // Update noise channel frequency every 16 T-cycles
                if self.period_delay_counter % 16 == 0 {
                    self.noise_channel.update_lfsr();
                }
            }
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
            self.buffer.push(0.0);
            return;
        }

        let ch1 = self.square_channel_1.get_sample();
        let ch2 = self.square_channel_2.get_sample();
        let ch3 = self.wave_channel.get_sample();
        let ch4 = self.noise_channel.get_sample();

        let mut left_channel = 0f32;
        if self.pan_options.intersects(PanRegister::CH1_LEFT) {
            left_channel += ch1;
        }
        if self.pan_options.intersects(PanRegister::CH2_LEFT) {
            left_channel += ch2;
        }
        if self.pan_options.intersects(PanRegister::CH3_LEFT) {
            left_channel += ch3;
        }
        if self.pan_options.intersects(PanRegister::CH4_LEFT) {
            left_channel += ch4;
        }

        let mut right_channel = 0f32;
        if self.pan_options.intersects(PanRegister::CH1_RIGHT) {
            right_channel += ch1;
        }
        if self.pan_options.intersects(PanRegister::CH2_RIGHT) {
            right_channel += ch2;
        }
        if self.pan_options.intersects(PanRegister::CH3_RIGHT) {
            right_channel += ch3;
        }
        if self.pan_options.intersects(PanRegister::CH4_RIGHT) {
            right_channel += ch4;
        }

        left_channel *= (self.left_volume as f32) / 8.0;
        self.buffer.push(left_channel * 0.1);
        right_channel *= (self.right_volume as f32) / 8.0;
        self.buffer.push(right_channel * 0.1);
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
            0xFF1A..=0xFF1E | 0xFF30..=0xFF3F => self.wave_channel.read_register(address),
            0xFF20..=0xFF23 => self.noise_channel.read_register(address),
            0xFF24 => ((self.left_volume - 1) << 4) | (self.right_volume - 1),
            0xFF25 => self.pan_options.bits(),
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
            }
            0xFF16..=0xFF19 => {
                self.square_channel_2
                    .write_register(address - 0xFF15, value);
            }
            0xFF1A..=0xFF1E | 0xFF30..=0xFF3F => self.wave_channel.write_register(address, value),
            0xFF20..=0xFF23 => self.noise_channel.write_register(address, value),
            0xFF24 => {
                self.left_volume = ((value >> 4) & 0b111) + 1;
                self.right_volume = (value & 0b111) + 1;
            }
            0xFF25 => self.pan_options = PanRegister::from_bits_truncate(value),
            0xFF26 => {
                self.on = value & 0b1000_0000 > 0;
            }
            _ => {}
        }
    }
}
