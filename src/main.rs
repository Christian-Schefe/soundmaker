use fundsp::prelude::*;
use midly::Smf;
use soundmaker::{daw::*, oscilloscope::render};

fn main() {
    let bytes = include_bytes!("../output/file.mid");
    let smf = Smf::parse(bytes).unwrap();

    let sample_rate = 48000.0;

    let mut daw = DAW::new();
    daw.add_channel(
        SimpleSynth::new(1, Box::new((sine() * pass() | sink()) >> pan(0.0))),
        1.0,
    );
    daw.add_channel(
        SimpleSynth::new(1, Box::new((sine() * pass() | sink()) >> pan(0.0))),
        1.0,
    );
    daw.add_channel(
        SimpleSynth::new(1, Box::new((sine() * pass() | sink()) >> pan(0.0))),
        1.0,
    );
    daw.add_channel(
        SimpleSynth::new(1, Box::new((sine() * pass() | sink()) >> pan(0.0))),
        1.0,
    );
    daw.add_channel(
        SimpleSynth::new(1, Box::new((sine() * pass() | sink()) >> pan(0.0))),
        1.0,
    );

    daw.set_midi(smf);
    render(daw, sample_rate);
}
