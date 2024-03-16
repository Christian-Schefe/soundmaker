pub use crate::daw::*;
pub use crate::oscilloscope::*;
pub use crate::playback::*;
pub use crate::score::*;
use fundsp::prelude::*;
pub use typenum::{U0, U1, U10, U2, U3, U4, U5, U6, U7, U8, U9};

#[derive(Clone)]
pub struct ADSR {
    attack: f64,
    decay: f64,
    sustain: f64,
    release: f64,
    time: f64,
    delta_time: f64,
    last_start_time: f64,
    last_end_time: f64,
    last_input: f64,
    attack_baseline: f64,
    sustain_baseline: f64,
    last_output: f64,
}

impl ADSR {
    pub fn new(attack: f64, decay: f64, sustain: f64, release: f64) -> Self {
        Self {
            attack,
            decay,
            sustain,
            release,
            time: 0.0,
            delta_time: 1.0 / DEFAULT_SR,
            last_start_time: 0.0,
            last_end_time: 0.0,
            last_input: -1.0,
            attack_baseline: 0.0,
            sustain_baseline: sustain,
            last_output: 0.0,
        }
    }
}

impl AudioNode for ADSR {
    const ID: u64 = 0xA23D385235;
    type Sample = f64;
    type Inputs = U1;
    type Outputs = U1;
    type Setting = ();
    fn tick(
        &mut self,
        input: &Frame<Self::Sample, Self::Inputs>,
    ) -> Frame<Self::Sample, Self::Outputs> {
        let control = input[0];
        if self.last_input <= 0.0 && control > 0.0 {
            self.last_start_time = self.time;
            self.attack_baseline = self.last_output;
        } else if self.last_input > 0.0 && control <= 0.0 {
            self.last_end_time = self.time;
            self.sustain_baseline = self.last_output;
        }

        let output = if control <= 0.0 {
            let time_diff = self.time - self.last_end_time;
            let alpha = if self.release > 0.0 {
                (time_diff / self.release).clamp(0.0, 1.0)
            } else {
                (time_diff / 0.01).clamp(0.0, 1.0)
            };
            lerp(self.sustain_baseline, 0.0, alpha)
        } else {
            let time_diff = self.time - self.last_start_time;
            if time_diff < self.attack {
                let alpha = (time_diff / self.attack).clamp(0.0, 1.0);
                lerp(self.attack_baseline, 1.0, alpha)
            } else {
                let alpha = if self.decay > 0.0 {
                    ((time_diff - self.attack) / self.decay).clamp(0.0, 1.0)
                } else {
                    1.0
                };
                lerp(1.0, self.sustain, alpha)
            }
        };

        self.time += self.delta_time;
        self.last_input = control;
        self.last_output = output;
        [output].into()
    }

    fn reset(&mut self) {
        self.time = 0.0;
        self.last_end_time = 0.0;
        self.last_start_time = 0.0;
        self.last_input = 0.0;
        self.attack_baseline = 0.0;
        self.sustain_baseline = self.sustain;
        self.last_output = 0.0;
    }

    fn set_sample_rate(&mut self, sample_rate: f64) {
        self.time = 0.0;
        self.delta_time = 1.0 / sample_rate;
    }
}

pub fn make_adsr(params: (f64, f64, f64, f64)) -> An<ADSR> {
    An(ADSR::new(params.0, params.1, params.2, params.3))
}