use std::rc::Rc;

use fundsp::prelude::*;
use midly::Smf;
use soundmaker::{
    daw::*,
    oscilloscope::render,
    prelude::make_adsr,
    score::{Bar, Dynamic, Key, Note, Score, Section},
};

fn main() {
    let bytes = include_bytes!("../output/castle.mid");
    let smf = Smf::parse(bytes).unwrap();
    // let smf = test_midi();
    // let smf = smf.to_midi();

    let sample_rate = 48000.0;

    let mut daw = DAW::new();
    daw.add_channel_boxed("Piano Right Hand".to_string(), simple_synth(soft_saw(), (0.02, 1.0, 0.1, 0.5)), 1.0, 0.0);
    daw.add_channel_boxed("Piano Left Hand".to_string(), simple_synth(soft_saw(), (0.02, 1.0, 0.1, 0.5)), 1.0, 0.0);
    daw.add_channel_boxed("Violin 1".to_string(), 
        vibrato_synth(
            square() * 0.7 & saw() * 0.3,
            (0.1, 1.5, 0.9, 0.5),
            (0.004, 5.0, 0.0),
        ),
        2.0,
        0.0,
    );
    daw[2].add(EQ::new((8000.0, 0.1), (200.0, 0.5)));
    daw.add_channel_boxed("Violin 2".to_string(), 
        vibrato_synth(
            square() * 0.7 & saw() * 0.3,
            (0.1, 1.5, 0.9, 0.5),
            (0.004, 5.0, 0.5),
        ),
        1.0,
        0.0,
    );
    daw[3].add(EQ::new((8000.0, 0.1), (200.0, 0.5)));
    daw.add_channel_boxed("Violoncello".to_string(), 
        vibrato_synth(
            square() * 0.7 & saw() * 0.3,
            (0.1, 1.5, 0.9, 0.5),
            (0.004, 4.0, 0.2),
        ),
        3.0,
        0.0,
    );
    daw[4].add(EQ::new((8000.0, 0.1), (200.0, 0.5)));

    daw.set_midi(smf);
    render(daw, sample_rate);
}

fn test_midi() -> Score<1> {
    let key = Rc::new(Key::new(0, true));

    let mut bars = Vec::new();
    bars.push(Bar::new(4, 140.0, key.clone(), Dynamic::Forte));
    bars.push(Bar::new(4, 140.0, key.clone(), Dynamic::Forte));

    bars[0].add_note(0, 0, Note::new(480, 0, 5, None));
    bars[1].add_note(0, 0, Note::new(480, 0, 5, None));
    bars[1].add_note(0, 0, Note::new(480, 2, 5, None));
    bars[1].add_note(0, 0, Note::new(480, 4, 5, None));

    let section = Section::from_bars(bars);
    let score = Score::from_sections(vec![section]);
    score
}

fn simple_synth<T>(signal: An<T>, params: (f64, f64, f64, f64)) -> Box<dyn Synthesizer>
where
    T: AudioNode<Sample = f64, Inputs = U1, Outputs = U1> + 'static,
{
    let graph = signal * pass() * make_adsr(params);

    Box::new(SimpleSynth::new(8, Box::new(graph >> pan(0.0))))
}

fn vibrato_synth<T>(
    signal: An<T>,
    params: (f64, f64, f64, f64),
    vibrato: (f64, f64, f64),
) -> Box<dyn Synthesizer>
where
    T: AudioNode<Sample = f64, Inputs = U1, Outputs = U1> + 'static,
{
    let freq_graph = pass()
        * (1.0 + vibrato.0 * (dc(vibrato.1) >> An(Sine::with_phase(DEFAULT_SR, Some(vibrato.2)))))
        >> signal;
    let graph = freq_graph * pass() * make_adsr(params);

    Box::new(SimpleSynth::new(8, Box::new(graph >> pan(0.0))))
}
