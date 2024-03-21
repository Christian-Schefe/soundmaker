pub use crate::daw::*;
pub use crate::playback::*;
pub use crate::score::*;
pub use crate::midi::*;
pub use crate::processor::*;
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
    let vibrato = Vibrato::new(0.004, (0.1, 1.5, 0.9, 0.5), 5.0, (0.1, 1.5, 0.9, 0.5));
    Violin::new(vibrato, (0.1, 1.5, 0.9, 0.5))
}

pub fn piano() -> Piano {
    Piano::new((0.02, 1.0, 0.1, 0.5))
}

pub fn flute() -> Flute {
    let envelope = (0.1, 1.0, 0.8, 0.5);
    let vibrato = Vibrato::new(0.004, envelope, 5.0, envelope);
    Flute::new(vibrato, envelope)
}
