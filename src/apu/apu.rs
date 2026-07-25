use super::{square::SquareWave, triangle::TriangleWave, noise::Noise};

const CPU_FREQ: f64 = 1_789_773.0;
/// NES Audio Processing Unit
/// 
/// for more info:
/// https://www.nesdev.org/wiki/APU
#[derive(Debug)]
pub struct APU {
    clock: u64,
    frame_counter: usize,
    frame_sequence: u8,
    cycles_since_sample: f64,
    pub volume: f32,
    pub pulse1: SquareWave,
    pub pulse2: SquareWave,
    pub triangle: TriangleWave,
    pub noise: Noise
}
impl Default for APU {
    fn default() -> Self {
        Self {
            cycles_since_sample: 0.0,
            clock: 0,
            frame_counter: 0,
            frame_sequence: 0,
            volume: 1.0,
            pulse1: SquareWave::new(true),
            pulse2: SquareWave::new(false),
            triangle: TriangleWave::default(),
            noise: Noise::new(),
        }
    }
}
impl APU {
    pub fn write_register(&mut self, addr: u16, data: u8) {
        match addr {
            //pulse 1
            0x4000 => self.pulse1.write_control(data),
            0x4001 => self.pulse1.write_sweep(data),
            0x4002 => self.pulse1.write_timer_lo(data),
            0x4003 => self.pulse1.write_timer_hi(data),
            
            0x4004 => self.pulse2.write_control(data),
            0x4005 => self.pulse2.write_sweep(data),
            0x4006 => self.pulse2.write_timer_lo(data),
            0x4007 => self.pulse2.write_timer_hi(data),

            0x4008 => self.triangle.write_linear_counter(data), // linear counter control (C), linear counter load (CRRR RRRR)
            0x4009 => {/* unused */}
            0x400A => self.triangle.write_timer_lo(data), //timer low 	TTTT TTTT
            0x400B => self.triangle.write_timer_hi(data), // LLLL LTTT	Length counter load (L), timer high (T), set linear counter reload flag

            0x400C => self.noise.write_halt_and_volume(data),
            0x400D => {/* unused */}
            0x400E => self.noise.write_noise(data),
            0x400F => self.noise.write_length_counter(data),

            0x4015 => {
                self.pulse1.enabled     = data & 0x01 != 0;
                self.pulse2.enabled     = data & 0x02 != 0;
                self.triangle.enabled   = data & 0x04 != 0;
            }

            0x4017 => {} //unmute immediately
            _ => {}
        }
    }

    pub fn step(&mut self) {
        self.clock += 1;
        self.frame_counter += 1;

        if self.frame_counter >= 7457 { // cpu cycles per sec divided by 240Hz
            self.frame_counter -= 7457;
            self.frame_sequence = (self.frame_sequence + 1) % 4;

            self.pulse1.clock_envelope();
            self.pulse2.clock_envelope();
            self.triangle.clock_linear_counter();
            self.noise.clock_envelope();

            if self.frame_sequence % 2 == 0 {
                self.pulse1.clock_length();
                self.pulse1.clock_sweep();

                self.pulse2.clock_length();
                self.pulse2.clock_sweep();

                self.triangle.clock_length();

                self.noise.clock_length();
            }
        }

        if self.clock % 2 == 0 {
            self.pulse1.step();
            self.pulse2.step();
            self.noise.step();
        }

        self.triangle.step();
    }
    
    pub fn tick<F>(&mut self, cycles: u8, sample_rate: u32, fullness: f64, mut push_sample: F)
    where F: FnMut(f32) {
        self.cycles_since_sample += cycles as f64;

        let rate_adjustment = if fullness < 0.4 {
            0.98
        } else if fullness > 0.6 {
            1.02
        } else {
            1.0
        };

        let cycles_per_sample = (CPU_FREQ / sample_rate as f64) * rate_adjustment;

        while self.cycles_since_sample >= cycles_per_sample {
            self.cycles_since_sample -= cycles_per_sample;
            push_sample(self.get_sample());
        }
    }


    pub fn get_sample(&mut self) -> f32 {
        let p1 = self.pulse1.get_amplitude();
        let p2 = self.pulse2.get_amplitude();
        let tg = self.triangle.get_amplitude();
        let ns = self.noise.get_amplitude();

        let pulse_out = if p1 + p2 > 0.0 {
            95.88 / ((8128.0 / (p1 + p2)) + 100.0)
        } else {
            0.0
        };

        let tnd_out = if tg + ns > 0.0 {
            159.79 / ((1.0 / ((tg / 8227.0) + (ns / 12241.0))) + 100.0)
        } else {
            0.0
        };

        let mixed = (pulse_out + tnd_out) * self.volume;
        mixed
    }

    pub fn read_status(&self) -> u8 {
        0
    }
}
