pub use crate::daw::*;
pub use crate::oscilloscope::*;
pub use crate::playback::*;
pub use crate::score::*;
use crate::ADSR;
use fundsp::prelude::*;
pub use typenum::{U0, U1, U10, U2, U3, U4, U5, U6, U7, U8, U9};

pub fn make_adsr(params: (f64, f64, f64, f64)) -> An<ADSR> {
    An(ADSR::new(params.0, params.1, params.2, params.3))
}
