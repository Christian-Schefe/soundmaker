use fundsp::prelude::*;
use midly::Smf;
use soundmaker::{daw::*, oscilloscope::render, prelude::make_adsr};

fn main() {
    let bytes = include_bytes!("../assets/spring_rain.mid");
    let smf = Smf::parse(bytes).unwrap();

    let sample_rate = 48000.0;

    let mut daw = DAW::new();

    let violin1 = violin();
    daw.add_channel_boxed("Violin".to_string(), violin1.0, violin1.1, 0.75, 0.0);

    let cello = violoncello();
    daw.add_channel_boxed("Violoncello".to_string(), cello.0, cello.1, 3.5, 0.0);

    let piano_r = piano();
    daw.add_channel_boxed("Piano LH".to_string(), piano_r.0, piano_r.1, 2.0, 0.0);

    let piano_l = piano();
    daw.add_channel_boxed("Piano RH".to_string(), piano_l.0, piano_l.1, 2.5, 0.0);

    daw.set_midi(smf);
    render(daw, sample_rate);
}

fn piano() -> (Box<dyn Synthesizer>, Vec<Box<dyn Processor>>) {
    (simple_synth(soft_saw(), (0.02, 1.0, 0.1, 0.5)), vec![Box::new(EQ::new((5000.0, 0.1), (200.0, 0.5)))])
}

fn violin() -> (Box<dyn Synthesizer>, Vec<Box<dyn Processor>>) {
    (
        vibrato_synth(
            square() * 0.7 & saw() * 0.3,
            (0.1, 1.5, 0.9, 0.5),
            (0.004, 5.0, 0.0),
        ),
        vec![Box::new(EQ::new((8000.0, 0.1), (200.0, 0.5)))],
    )
}

fn violoncello() -> (Box<dyn Synthesizer>, Vec<Box<dyn Processor>>) {
    (
        vibrato_synth(
            square() * 0.7 & saw() * 0.3,
            (0.1, 1.5, 0.9, 0.5),
            (0.004, 4.0, 0.0),
        ),
        vec![Box::new(EQ::new((8000.0, 0.1), (200.0, 0.5)))],
    )
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
