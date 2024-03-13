use std::f64::consts::TAU;

use fundsp::DEFAULT_SR;
use typenum::U1;

use crate::node::FixedAudioNode;

#[derive(Clone)]
pub struct SineOscillator {
    phase: f64,
    initial_phase: f64,
    delta_time: f64,
}

impl SineOscillator {
    pub fn new() -> Self {
        Self::with_phase(0.0)
    }
    pub fn with_phase(phase: f64) -> Self {
        Self {
            phase: 0.0,
            delta_time: 1.0 / DEFAULT_SR,
            initial_phase: phase,
        }
    }
}

impl FixedAudioNode for SineOscillator {
    type Inputs = U1;
    type Outputs = U1;
    fn tick(&mut self, input: &[f64], output: &mut [f64]) {
        self.phase += self.delta_time * input[0];
        while self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        output[0] = (self.phase * TAU).sin();
    }

    fn reset(&mut self) {
        self.phase = self.initial_phase;
    }

    fn set_sample_rate(&mut self, sample_rate: f64) {
        self.delta_time = 1.0 / sample_rate;
    }
}

#[derive(Clone)]
pub struct FM {
    phase: f64,
    initial_phase: f64,
    delta_time: f64,
    modulation_speed: f64,
    modulation_amount: f64,
}

impl FM {
    pub fn new(speed: f64, amount: f64) -> Self {
        Self::with_phase(speed, amount, 0.0)
    }
    pub fn with_phase(speed: f64, amount: f64, phase: f64) -> Self {
        Self {
            phase: 0.0,
            delta_time: 1.0 / DEFAULT_SR,
            initial_phase: phase,
            modulation_speed: speed,
            modulation_amount: amount,
        }
    }
}

impl FixedAudioNode for FM {
    type Inputs = U1;
    type Outputs = U1;
    fn tick(&mut self, input: &[f64], output: &mut [f64]) {
        self.phase += self.delta_time * input[0];
        output[0] = (self.phase * TAU
            + self.modulation_amount * (self.phase * TAU * self.modulation_speed).sin())
        .sin();
    }

    fn reset(&mut self) {
        self.phase = self.initial_phase;
    }

    fn set_sample_rate(&mut self, sample_rate: f64) {
        self.delta_time = 1.0 / sample_rate;
    }
}
