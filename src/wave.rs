use std::time::Duration;

use typenum::{U0, U1};

use crate::prelude::{AudioNode, FixedAudioNode};

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
    pub fn render_to_silence(mut node: Box<dyn AudioNode>, sample_rate: f64) -> Self {
        let mut data = Vec::new();
        let mut last_max = (0, 1.0);
        let threshold = 0.01;
        while last_max.0 + (sample_rate * 3.0) as usize > data.len() {
            let sample = node.get_stereo().0;
            data.push(sample);
            let abs_sample = sample.abs();
            if abs_sample > threshold
                && (last_max.0 + 20000 < data.len() || abs_sample > last_max.1)
            {
                last_max = (data.len(), abs_sample);
            }
        }
        let length = Duration::from_secs_f64(data.len() as f64 / sample_rate);
        Self {
            data,
            length,
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
