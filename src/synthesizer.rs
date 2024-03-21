use dyn_clone::{clone_trait_object, DynClone};
use fundsp::prelude::*;

use crate::prelude::{make_adsr, select, EQ};

use crate::prelude::*;

/// A synthesizer is a node that takes MIDI messages and produces audio.
/// A simple implementation is provided as `SimpleSynth`, but you can create your own.
/// See `MidiWrapper` for a simple way to handle MIDI messages in your implementation.
pub trait Synthesizer: DynClone + Send + Sync {
    fn set_midi(&mut self, midi: Vec<MidiMsg>);
    fn tick(&mut self, time: f64) -> Frame<f64, U2>;
    fn reset(&mut self) {}
}

/// A "sink" synthesizer that ignores all MIDI messages and produces silence.
#[derive(Clone)]
pub struct SinkSynth;
impl Synthesizer for SinkSynth {
    fn set_midi(&mut self, _midi: Vec<MidiMsg>) {}

    fn tick(&mut self, _time: f64) -> Frame<f64, U2> {
        [0.0, 0.0].into()
    }
}

clone_trait_object!(Synthesizer);

/// A basic synthesizer implementation that plays notes using a fixed number of voices.
/// The voices are cycled through, so if you have 8 voices and play 9 notes, the least recent note will be dropped.
/// Each voice receives three inputs: frequency, velocity (0..1), and adsr control (-1 or 1).
#[derive(Clone)]
pub struct SimpleSynth {
    midi_wrapper: MidiWrapper,
    voices: Vec<Box<dyn AudioUnit64>>,
    last_notes: Vec<(u8, f64, bool)>,
    voice_index: usize,
}

impl SimpleSynth {
    pub fn new(voices: usize, node: Box<dyn AudioUnit64>) -> Self {
        Self {
            midi_wrapper: MidiWrapper::new(Vec::new()),
            voices: vec![node; voices],
            last_notes: vec![(0, 0.0, false); voices],
            voice_index: 0,
        }
    }
    pub fn boxed(voices: usize, node: Box<dyn AudioUnit64>) -> Box<Self> {
        Box::new(Self::new(voices, node))
    }
    fn update_notes(&mut self, dropped: Vec<u8>, new: Vec<(u8, f64)>) {
        for note in dropped {
            for i in 0..self.last_notes.len() {
                let last_note = &mut self.last_notes[i];
                if last_note.0 == note && last_note.2 {
                    last_note.2 = false;
                }
            }
        }
        for note in new {
            self.voices[self.voice_index].reset();
            self.last_notes[self.voice_index] = (note.0, note.1, true);
            self.voice_index = (self.voice_index + 1) % self.voices.len();
        }
    }
}

impl Synthesizer for SimpleSynth {
    fn set_midi(&mut self, midi: Vec<MidiMsg>) {
        self.midi_wrapper = MidiWrapper::new(midi)
    }

    fn tick(&mut self, time: f64) -> Frame<f64, U2> {
        let (dropped, new) = self.midi_wrapper.tick(time);

        self.update_notes(dropped, new);

        let mut mix: Frame<f64, U2> = [0.0, 0.0].into();
        for i in 0..self.voices.len() {
            let voice = &mut self.voices[i];
            let data = self.last_notes[i];
            let input = [
                midi_hz(data.0 as f64),
                data.1,
                if data.2 { 1.0 } else { -1.0 },
            ];
            let mut output: Frame<f64, U2> = [0.0, 0.0].into();
            voice.tick(&input, &mut output);
            mix += output;
        }
        mix
    }
}

pub trait MidiInstrument: DynClone + Send + Sync {
    fn build_synth(&self) -> Box<dyn Synthesizer>;
    fn build_processors(&self) -> Vec<Box<dyn Processor>>;
}

clone_trait_object!(MidiInstrument);

#[derive(Clone)]
pub struct Vibrato {
    pub strength: f64,
    pub strength_envelope: (f64, f64, f64, f64),
    pub frequency: f64,
    pub freq_envelope: (f64, f64, f64, f64),
}

impl Vibrato {
    pub fn new(
        strength: f64,
        strength_envelope: (f64, f64, f64, f64),
        frequency: f64,
        freq_envelope: (f64, f64, f64, f64),
    ) -> Self {
        Self {
            strength,
            strength_envelope,
            frequency,
            freq_envelope,
        }
    }
    pub fn build(&self) -> An<impl AudioNode<Sample = f64, Inputs = U2, Outputs = U1>> {
        let freq_envelope = pass() | make_adsr(self.freq_envelope);
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
        vec![EQ::boxed((8000.0, 0.1), (200.0, 0.5))]
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
        vec![EQ::boxed((5000.0, 0.1), (200.0, 0.5))]
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
        let signal = triangle() * 0.7 & sine() * 0.3;

        let freq_graph = select([0, 2]) >> self.vibrato.build() >> signal;
        let graph = freq_graph ^ (sink() | pass() * make_adsr(self.envelope));
        let unit = graph >> pass() * pass() >> pan(0.0);

        SimpleSynth::boxed(8, Box::new(unit))
    }

    fn build_processors(&self) -> Vec<Box<dyn Processor>> {
        vec![EQ::boxed((8000.0, 0.1), (200.0, 0.5))]
    }
}
