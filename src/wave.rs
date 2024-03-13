use std::time::Duration;

use typenum::{U0, U1};

use crate::{AudioNode, FixedAudioNode};

#[derive(Clone)]
pub struct Wave {
    pub data: Vec<f64>,
    pub length: Duration,
    index: usize,
}

impl Wave {
    pub fn render(mut node: Box<dyn AudioNode>, sample_rate: f64, duration: Duration) -> Self {
        let samples = (duration.as_secs_f64() * sample_rate).round() as usize;
        Self {
            data: (0..samples).map(|_| node.get_stereo().0).collect(),
            length: duration,
            index: 0,
        }
    }
}

impl FixedAudioNode for Wave {
    type Inputs = U0;
    type Outputs = U1;

    fn tick(&mut self, _input: &[f64], output: &mut [f64]) {
        output[0] = self.data[self.index];
        self.index += 1;
    }

    fn reset(&mut self) {
        self.index = 0;
    }

    fn set_sample_rate(&mut self, _sample_rate: f64) {}
}
