use crate::prelude::*;
use fundsp::prelude::*;
use fundsp::sound::*;

#[derive(Clone)]
pub struct PercussionSynth {
    midi_wrapper: MidiWrapper,
    samples: Vec<(u8, Box<dyn AudioUnit64>, f64)>,
}

impl PercussionSynth {
    pub fn new(samples: Vec<(u8, Box<dyn AudioUnit64>)>) -> Self {
        let samples = samples
            .into_iter()
            .map(|x| (x.0, x.1.clone(), 0.0))
            .collect();
        Self {
            midi_wrapper: MidiWrapper::new(Vec::new()),
            samples,
        }
    }
    pub fn boxed(samples: Vec<(u8, Box<dyn AudioUnit64>)>) -> Box<Self> {
        Box::new(Self::new(samples))
    }
    fn update_notes(&mut self, new: Vec<(u8, f64)>) {
        for note in new {
            if let Some(i) = self.samples.iter().position(|x| x.0 == note.0) {
                self.samples[i].1.reset();
                self.samples[i].2 = note.1
            }
            println!("{:?}", note);
        }
    }
}

impl Synthesizer for PercussionSynth {
    fn set_midi(&mut self, midi: Vec<MidiMsg>) {
        self.midi_wrapper = MidiWrapper::new(midi)
    }

    fn tick(&mut self, time: f64) -> Frame<f64, U2> {
        let (_, new) = self.midi_wrapper.tick(time);
        self.update_notes(new);

        let mut mix: Frame<f64, U2> = [0.0, 0.0].into();
        for i in 0..self.samples.len() {
            let vel = self.samples[i].2;
            let voice = &mut self.samples[i].1;
            let input = [vel];
            let mut output: Frame<f64, U2> = [0.0, 0.0].into();
            voice.tick(&input, &mut output);
            mix += output;
        }
        mix
    }
    fn set_sample_rate(&mut self, sample_rate: f64) {
        for voice in self.samples.iter_mut() {
            voice.1.set_sample_rate(sample_rate);
        }
    }
    fn reset(&mut self) {
        for voice in self.samples.iter_mut() {
            voice.1.reset();
            voice.2 = 0.0;
        }
        self.midi_wrapper.reset();
    }
}

pub fn bass_drum(volume: f64) -> Box<dyn AudioUnit64> {
    let sound = bassdrum(0.2, 180.0, 60.0) * fundsp::prelude::pass() >> fundsp::prelude::pan(0.0);
    Box::new(sound * volume)
}

pub fn snare_drum(volume: f64) -> Box<dyn AudioUnit64> {
    let sound = snaredrum(0, 0.3) * fundsp::prelude::pass() >> fundsp::prelude::pan(0.0);
    Box::new(sound * volume)
}

pub fn shaker(volume: f64) -> Box<dyn AudioUnit64> {
    let sound = noise()
        * (constant(1.0) >> make_adsr((0.01, 0.1, 0.0, 0.0)))
        * constant(0.25)
        * fundsp::prelude::pass()
        >> highpass_hz::<f64, f64>(6000.0, 0.5)
        >> fundsp::prelude::pan(0.0)
        >> ((pass() | pass()) & 0.2 * reverb_stereo::<f64>(10.0, 0.3));
    Box::new(sound * volume)
}

pub fn hihat(volume: f64) -> Box<dyn AudioUnit64> {
    let sound = noise()
        * (constant(1.0) >> make_adsr((0.01, 0.1, 0.0, 0.0)))
        * constant(0.15)
        * fundsp::prelude::pass()
        >> highpass_hz::<f64, f64>(800.0, 0.5)
        >> fundsp::prelude::pan(0.0);
    Box::new(sound * volume)
}

pub fn percussion(mapping: Vec<Percussion>) -> Box<dyn MidiInstrument> {
    let samples = mapping
        .into_iter()
        .map(|x| match x {
            Percussion::BassDrum(note, vol) => (note, bass_drum(vol)),
            Percussion::SnareDrum(note, vol) => (note, snare_drum(vol)),
            Percussion::Shaker(note, vol) => (note, shaker(vol)),
            Percussion::HiHat(note, vol) => (note, hihat(vol)),
        })
        .collect();

    Box::new(PercussionSynth::new(samples))
}

pub enum Percussion {
    BassDrum(u8, f64),
    SnareDrum(u8, f64),
    Shaker(u8, f64),
    HiHat(u8, f64),
}
