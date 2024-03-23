use dyn_clone::{clone_trait_object, DynClone};
use fundsp::prelude::*;

pub trait Processor: DynClone + Send + Sync {
    fn tick(&mut self, time: f64, input: &Frame<f64, U2>) -> Frame<f64, U2>;
    fn set_sample_rate(&mut self, _sample_rate: f64) {}
    fn reset(&mut self) {}
}

clone_trait_object!(Processor);

#[derive(Clone)]
pub struct EQ {
    lowpass: (f64, f64),
    highpass: (f64, f64),
    lowpass_node: Box<dyn AudioUnit64>,
    highpass_node: Box<dyn AudioUnit64>,
}

impl EQ {
    pub fn new(lowpass: (f64, f64), highpass: (f64, f64)) -> Self {
        Self {
            lowpass,
            highpass,
            lowpass_node: Box::new(
                fundsp::prelude::lowpass::<f64, f64>() | fundsp::prelude::lowpass::<f64, f64>(),
            ),
            highpass_node: Box::new(
                fundsp::prelude::highpass::<f64, f64>() | fundsp::prelude::highpass::<f64, f64>(),
            ),
        }
    }
    pub fn boxed(lowpass: (f64, f64), highpass: (f64, f64)) -> Box<Self> {
        Box::new(Self::new(lowpass, highpass))
    }
}

impl Processor for EQ {
    fn tick(&mut self, _time: f64, input: &Frame<f64, U2>) -> Frame<f64, U2> {
        let low_in = [
            input[0],
            self.lowpass.0,
            self.lowpass.1,
            input[1],
            self.lowpass.0,
            self.lowpass.1,
        ];
        let mut output = [0.0, 0.0];
        self.lowpass_node.tick(&low_in, &mut output);

        let high_in = [
            output[0],
            self.highpass.0,
            self.highpass.1,
            output[1],
            self.highpass.0,
            self.highpass.1,
        ];
        self.highpass_node.tick(&high_in, &mut output);

        output.into()
    }
}

impl<T> Processor for T
where
    T: AudioUnit64,
{
    fn tick(&mut self, _time: f64, input: &Frame<f64, U2>) -> Frame<f64, U2> {
        let mut output = [0.0, 0.0];
        self.tick(input, &mut output);
        output.into()
    }
}
