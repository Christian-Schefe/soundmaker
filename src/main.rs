use fundsp::{
    prelude::{constant, highpass_hz, noise, pass, reverb_stereo},
    sound::{bassdrum, snaredrum},
};
use soundmaker::prelude::*;

fn main() {
    let midi = std::fs::read("temp/Chill Beats.mid").unwrap();
    let sample_rate = 48000.0;
    let mut daw = DAW::new();

    let violin = violin();
    let flute = flute();

    let percussion = percussion();

    daw.add_instrument("Flute".to_string(), &flute, 2.0, 0.0);
    daw.add_instrument("Percussion 1".to_string(), percussion.as_ref(), 1.0, 0.0);
    daw.add_instrument("Percussion 2".to_string(), percussion.as_ref(), 1.0, 0.0);
    daw.add_instrument("Viola".to_string(), &violin, 2.0, 0.0);
    daw.add_instrument("Cello".to_string(), &violin, 2.0, 0.0);

    daw.set_midi_bytes(&midi);

    let render = render_daw(&mut daw, sample_rate);

    play_to_end(
        render.master,
        sample_rate,
        Some("output/chill_beats.wav".into()),
    )
    .unwrap();
}

fn percussion() -> Box<dyn MidiInstrument> {
    let bassdrum =
        Box::new(bassdrum(0.2, 180.0, 60.0) * fundsp::prelude::pass() >> fundsp::prelude::pan(0.0));

    let snare = Box::new(snaredrum(0, 0.3) * fundsp::prelude::pass() >> fundsp::prelude::pan(0.0));
    let shaker = Box::new(
        noise()
            * (constant(1.0) >> make_adsr((0.01, 0.1, 0.0, 0.0)))
            * constant(0.25)
            * fundsp::prelude::pass()
            >> highpass_hz::<f64, f64>(6000.0, 0.5)
            >> fundsp::prelude::pan(0.0)
            >> ((pass() | pass()) & 0.2 * reverb_stereo::<f64>(10.0, 0.3)),
    );

    let hihat = Box::new(
        noise()
            * (constant(1.0) >> make_adsr((0.01, 0.1, 0.0, 0.0)))
            * constant(0.15)
            * fundsp::prelude::pass()
            >> highpass_hz::<f64, f64>(800.0, 0.5)
            >> fundsp::prelude::pan(0.0),
    );

    Box::new(PercussionSynth::new(vec![
        (36, bassdrum),
        (38, snare),
        (44, hihat),
        (70, shaker),
    ]))
}
