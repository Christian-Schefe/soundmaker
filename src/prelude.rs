pub use crate::daw::*;
pub use crate::instrument::*;
pub use crate::midi::*;
pub use crate::percussion::*;
pub use crate::playback::*;
pub use crate::processor::*;
pub use crate::score::*;
pub use crate::synthesizer::*;

use crate::Selector;
use crate::ADSR;
use fundsp::prelude::*;

pub fn make_adsr(params: (f64, f64, f64, f64)) -> An<ADSR> {
    An(ADSR::from_tuple(params))
}

pub fn select<I, O>(selection: impl Into<Frame<usize, O>>) -> An<Selector<I, O>>
where
    I: Size<f64> + Size<usize>,
    O: Size<f64> + Size<usize>,
{
    An(Selector::new(selection.into()))
}

pub fn violin() -> Violin {
    let envelope = (0.1, 1.5, 0.9, 0.5);
    let vibrato_env = (0.4, 0.0, 1.0, 0.5);
    let vibrato = Vibrato::new(0.005, 5.0, vibrato_env);
    Violin::new(vibrato, envelope)
}

pub fn piano() -> Piano {
    Piano::new((0.02, 1.0, 0.1, 0.5))
}

pub fn flute() -> Flute {
    let envelope = (0.03, 1.0, 0.8, 0.3);
    let vibrato_env = (0.4, 0.0, 1.0, 0.3);
    let vibrato = Vibrato::new(0.005, 5.0, vibrato_env);
    Flute::new(vibrato, envelope)
}

pub fn reverb(room_size: f64, time: f64) -> Box<dyn Processor> {
    Box::new(reverb_stereo(room_size, time))
}

pub fn distortion(smoothing: f64, hardness: f64) -> Box<dyn Processor> {
    Box::new(shape(Shape::AdaptiveTanh(smoothing, hardness)))
}

pub fn crush(levels: f64) -> Box<dyn Processor> {
    Box::new(shape(Shape::Crush(levels)))
}

pub fn gain(factor: f64) -> Box<dyn Processor> {
    Box::new((pass() * constant(factor)) | (pass() * constant(factor)))
}
