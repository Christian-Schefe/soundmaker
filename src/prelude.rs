pub use crate::fm::*;
pub use crate::graph::*;
pub use crate::midi::*;
pub use crate::node::*;
pub use crate::oscilloscope::*;
pub use crate::output::*;
pub use crate::score::*;
pub use crate::wave::*;
pub use crate::wavetable::*;
pub use typenum::{U0, U1, U10, U2, U3, U4, U5, U6, U7, U8, U9};

pub fn sine() -> Box<SineOscillator> {
    Box::new(SineOscillator::new())
}

pub fn fm(speed: f64, amount: f64) -> Box<FM> {
    Box::new(FM::new(speed, amount))
}

pub fn wave_synth(wavetable: Wavetable) -> Box<WaveSynth> {
    Box::new(WaveSynth::new(wavetable))
}

pub fn graph_builder() -> GraphBuilder {
    GraphBuilder::new()
}

pub fn pipline(nodes: Vec<Box<dyn AudioNode>>) -> Box<Pipeline> {
    Box::new(Pipeline::new(nodes))
}

pub fn stack(nodes: Vec<Box<dyn AudioNode>>) -> Box<Stack> {
    Box::new(Stack::new(nodes))
}
