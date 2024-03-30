use crate::prelude::*;
use dyn_clone::{clone_trait_object, DynClone};
use fundsp::prelude::*;

pub trait MidiInstrument: DynClone + Send + Sync {
    fn build_synth(&self) -> Box<dyn Synthesizer>;
    fn build_processors(&self) -> Vec<Box<dyn Processor>>;
}

clone_trait_object!(MidiInstrument);

impl<T> MidiInstrument for T
where
    T: Synthesizer + Clone + 'static,
{
    fn build_synth(&self) -> Box<dyn Synthesizer> {
        let synth = self.clone();
        Box::new(synth)
    }

    fn build_processors(&self) -> Vec<Box<dyn Processor>> {
        Vec::new()
    }
}

#[derive(Clone)]
pub struct Vibrato {
    pub strength: f64,
    pub frequency: f64,
    pub envelope: (f64, f64, f64, f64),
}

impl Vibrato {
    pub fn new(
        strength: f64,
        frequency: f64,
        freq_envelope: (f64, f64, f64, f64),
    ) -> Self {
        Self {
            strength,
            frequency,
            envelope: freq_envelope,
        }
    }
    pub fn build(&self) -> An<impl AudioNode<Sample = f64, Inputs = U2, Outputs = U1>> {
        let freq_envelope = pass() | make_adsr(self.envelope);
        let freq_graph = freq_envelope
            >> pass()
                * (1.0
                    + pass()
                        * self.strength
                        * (dc(self.frequency) >> An(Sine::with_phase(DEFAULT_SR, Some(0.0)))));
        freq_graph
    }
}

#[derive(Clone)]
pub struct Violin {
    vibrato: Vibrato,
    envelope: (f64, f64, f64, f64),
}

impl Violin {
    pub fn new(vibrato: Vibrato, envelope: (f64, f64, f64, f64)) -> Self {
        Self { vibrato, envelope }
    }
}

impl MidiInstrument for Violin {
    fn build_synth(&self) -> Box<dyn Synthesizer> {
        let signal = square() * 0.7 & saw() * 0.3;

        let freq_graph = select([0, 2]) >> self.vibrato.build() >> signal;
        let graph = freq_graph ^ (sink() | pass() * make_adsr(self.envelope));
        let unit = graph >> pass() * pass() >> pan(0.0);

        SimpleSynth::boxed(8, Box::new(unit))
    }

    fn build_processors(&self) -> Vec<Box<dyn Processor>> {
        vec![EQ::boxed((8000.0, 0.1), (200.0, 0.5)), gain(2.0)]
    }
}

#[derive(Clone)]
pub struct Piano {
    envelope: (f64, f64, f64, f64),
}

impl Piano {
    pub fn new(envelope: (f64, f64, f64, f64)) -> Self {
        Self { envelope }
    }
}

impl MidiInstrument for Piano {
    fn build_synth(&self) -> Box<dyn Synthesizer> {
        let signal = soft_saw();

        let graph = signal * pass() * make_adsr(self.envelope);
        let unit = graph >> pan(0.0);

        SimpleSynth::boxed(8, Box::new(unit))
    }

    fn build_processors(&self) -> Vec<Box<dyn Processor>> {
        vec![EQ::boxed((5000.0, 0.1), (200.0, 0.5)), gain(2.0)]
    }
}

#[derive(Clone)]
pub struct Flute {
    vibrato: Vibrato,
    envelope: (f64, f64, f64, f64),
}

impl Flute {
    pub fn new(vibrato: Vibrato, envelope: (f64, f64, f64, f64)) -> Self {
        Self { vibrato, envelope }
    }
}

impl MidiInstrument for Flute {
    fn build_synth(&self) -> Box<dyn Synthesizer> {
        let signal = triangle() * 0.8 & sine() * 0.2;

        let freq_graph = select([0, 2]) >> self.vibrato.build() >> signal;
        let graph = freq_graph ^ (sink() | pass() * make_adsr(self.envelope));
        let unit = graph >> pass() * pass() >> pan(0.0);

        SimpleSynth::boxed(8, Box::new(unit))
    }

    fn build_processors(&self) -> Vec<Box<dyn Processor>> {
        vec![EQ::boxed((12000.0, 0.1), (400.0, 0.5)), gain(2.0)]
    }
}

#[derive(Clone)]
pub struct FM {
    vibrato: Vibrato,
    envelope: (f64, f64, f64, f64),
    fm: (f64, f64),
}

impl FM {
    pub fn new(vibrato: Vibrato, envelope: (f64, f64, f64, f64), fm: (f64, f64)) -> Self {
        Self {
            vibrato,
            envelope,
            fm,
        }
    }
}

impl MidiInstrument for FM {
    fn build_synth(&self) -> Box<dyn Synthesizer> {
        let signal = An(FreqMod::new(self.fm.0, self.fm.1));

        let freq_graph = select([0, 2]) >> self.vibrato.build() >> signal;
        let graph = freq_graph ^ (sink() | pass() * make_adsr(self.envelope));
        let unit = graph >> pass() * pass() >> pan(0.0);

        SimpleSynth::boxed(8, Box::new(unit))
    }

    fn build_processors(&self) -> Vec<Box<dyn Processor>> {
        vec![EQ::boxed((8000.0, 0.1), (200.0, 0.5))]
    }
}

#[derive(Clone)]
pub struct FreqMod {
    phase: f64,
    initial_phase: f64,
    delta_time: f64,
    modulation_speed: f64,
    modulation_amount: f64,
}

impl FreqMod {
    pub fn new(speed: f64, amount: f64) -> Self {
        Self::with_phase(speed, amount, 0.0)
    }
    pub fn with_phase(speed: f64, amount: f64, phase: f64) -> Self {
        Self {
            phase: 0.0,
            delta_time: 1.0 / DEFAULT_SR,
            initial_phase: phase,
            modulation_speed: speed,
            modulation_amount: amount,
        }
    }
}

impl AudioNode for FreqMod {
    const ID: u64 = 0x3A2D385235;
    type Sample = f64;
    type Inputs = U1;
    type Outputs = U1;
    type Setting = ();

    fn tick(
        &mut self,
        input: &Frame<Self::Sample, Self::Inputs>,
    ) -> Frame<Self::Sample, Self::Outputs> {
        self.phase += self.delta_time * input[0];
        [(self.phase * TAU
            + self.modulation_amount * (self.phase * TAU * self.modulation_speed).sin())
        .sin()]
        .into()
    }

    fn reset(&mut self) {
        self.phase = self.initial_phase;
    }

    fn set_sample_rate(&mut self, sample_rate: f64) {
        self.delta_time = 1.0 / sample_rate;
    }
}
