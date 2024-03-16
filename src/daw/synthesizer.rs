use dyn_clone::{clone_trait_object, DynClone};
use fundsp::prelude::*;

use super::midi::{MidiMsg, MidiWrapper};

pub trait Synthesizer: DynClone + Send + Sync {
    fn set_midi(&mut self, midi: Vec<MidiMsg>);
    fn tick(&mut self, time: f64) -> Frame<f64, U2>;
}

#[derive(Clone)]
pub struct SinkSynth;
impl Synthesizer for SinkSynth {
    fn set_midi(&mut self, _midi: Vec<MidiMsg>) {}

    fn tick(&mut self, _time: f64) -> Frame<f64, U2> {
        [0.0, 0.0].into()
    }
}

clone_trait_object!(Synthesizer);

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
    fn update_notes(&mut self, dropped: Vec<u8>, new: Vec<(u8, f64)>) {
        // let change = dropped.len() + new.len() > 0;

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

        // if change {
        //     println!("{:?}", self.last_notes);
        // }
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
