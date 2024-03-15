use std::collections::VecDeque;

use dyn_clone::{clone_trait_object, DynClone};
use fundsp::prelude::*;

use super::midi::{MidiMsg, MidiWrapper};

pub trait Synthesizer: DynClone + Send + Sync {
    fn set_midi(&mut self, midi: Vec<MidiMsg>);
    fn tick(&mut self, time: f64) -> Frame<f64, U2>;
}

#[derive(Clone)]
pub struct TestSynth {}
impl Synthesizer for TestSynth {
    fn set_midi(&mut self, midi: Vec<MidiMsg>) {}

    fn tick(&mut self, time: f64) -> Frame<f64, U2> {
        let val = (time * 440.0).sin();
        [val, val].into()
    }
}

clone_trait_object!(Synthesizer);

#[derive(Clone)]
pub struct SimpleSynth {
    midi_wrapper: MidiWrapper,
    voices: Vec<Box<dyn AudioUnit64>>,
    last_notes: Vec<(u8, f64, bool)>,
    free_voices: VecDeque<usize>,
}

impl SimpleSynth {
    pub fn new(voices: usize, node: Box<dyn AudioUnit64>) -> Self {
        Self {
            midi_wrapper: MidiWrapper::new(Vec::new()),
            voices: vec![node; voices],
            last_notes: vec![(0, 0.0, false); voices],
            free_voices: (0..voices).collect(),
        }
    }
    fn update_notes(&mut self, dropped: Vec<u8>, new: Vec<(u8, f64)>) {
        for note in dropped {
            let i = self.last_notes.iter().position(|x| x.0 == note);
            if let Some(pos) = i {
                self.last_notes[pos].2 = false;
                self.free_voices.push_back(pos);
            }
        }
        for note in new {
            let i = self.free_voices.pop_front();
            if let Some(pos) = i {
                self.last_notes[pos] = (note.0, note.1, true)
            }
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
