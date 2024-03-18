use std::ops::Index;
use std::ops::IndexMut;
use std::time::Duration;

use fundsp::prelude::*;
use midly::Smf;

pub use self::midi::*;
pub use self::processor::*;
pub use self::render::*;
pub use self::synthesizer::*;

mod midi;
mod processor;
mod render;
mod synthesizer;

#[derive(Clone)]
pub struct DAW {
    channels: Vec<SynthChannel>,
    pub channel_count: usize,
    pub master: Channel,
    time: f64,
    delta_time: f64,
    pub duration: Duration,
}

impl DAW {
    pub fn new() -> Self {
        Self {
            channels: Vec::new(),
            channel_count: 0,
            master: Channel::new("Master".to_string(), 0, 1.0, 0.0, Vec::new()),
            time: 0.0,
            delta_time: 1.0 / DEFAULT_SR,
            duration: Duration::ZERO,
        }
    }
    pub fn set_midi(&mut self, midi: Smf) {
        println!(
            "MIDI has {} tracks, DAW has {} channels.",
            midi.tracks.len(),
            self.channel_count
        );

        let (fixed_midi, duration) = MidiMsg::convert_smf(midi);
        self.duration = duration + Duration::from_secs_f64(5.0);
        println!(
            "Determined duration of {:.2} seconds.",
            duration.as_secs_f64()
        );

        for (i, channel) in self.channels.iter_mut().enumerate() {
            let track = fixed_midi
                .get(i)
                .map(|x| x.iter().copied().collect())
                .unwrap_or(Vec::new());

            channel.synth.set_midi(track);
        }
    }
    pub fn add_instrument(
        &mut self,
        name: String,
        instrument: &dyn MidiInstrument,
        volume: f64,
        pan: f64,
    ) -> usize {
        self.add_channel_boxed(
            name,
            instrument.build_synth(),
            instrument.build_processors(),
            volume,
            pan,
        )
    }
    pub fn add_channel_boxed(
        &mut self,
        name: String,
        synth: Box<dyn Synthesizer>,
        processors: Vec<Box<dyn Processor>>,
        volume: f64,
        pan: f64,
    ) -> usize {
        let index = self.channel_count;
        self.channel_count += 1;
        self.channels.push(SynthChannel::new(
            Channel::new(name, index, volume, pan, processors),
            synth,
        ));
        index
    }
    pub fn add_channel<T>(
        &mut self,
        name: String,
        synth: T,
        processors: Vec<Box<dyn Processor>>,
        volume: f64,
        pan: f64,
    ) -> usize
    where
        T: Synthesizer + 'static,
    {
        self.add_channel_boxed(name, Box::new(synth), processors, volume, pan)
    }
    pub fn tick_channels(&mut self) -> Vec<Frame<f64, U2>> {
        let output = self
            .channels
            .iter_mut()
            .map(|x| x.tick(self.time))
            .collect();
        self.time += self.delta_time;
        output
    }
}

impl Index<usize> for DAW {
    type Output = Channel;

    fn index(&self, index: usize) -> &Self::Output {
        &self.channels[index].channel
    }
}

impl IndexMut<usize> for DAW {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.channels[index].channel
    }
}

impl AudioNode for DAW {
    const ID: u64 = 0x0C05;
    type Sample = f64;
    type Inputs = U0;
    type Outputs = U2;
    type Setting = ();

    fn tick(
        &mut self,
        _input: &Frame<Self::Sample, Self::Inputs>,
    ) -> Frame<Self::Sample, Self::Outputs> {
        self.tick_channels()
            .into_iter()
            .reduce(|a, b| a + b)
            .unwrap()
    }

    fn reset(&mut self) {
        self.time = 0.0;
    }

    fn set_sample_rate(&mut self, sample_rate: f64) {
        self.delta_time = 1.0 / sample_rate;
    }
}

#[derive(Clone)]
pub struct Channel {
    pub index: usize,
    pub volume: f64,
    pub pan: f64,
    pub processors: Vec<Box<dyn Processor>>,
    pub name: String,
}

impl Channel {
    fn new(
        name: String,
        index: usize,
        volume: f64,
        pan: f64,
        processors: Vec<Box<dyn Processor>>,
    ) -> Self {
        Self {
            index,
            volume,
            pan,
            processors,
            name,
        }
    }
    fn tick(&mut self, input: &Frame<f64, U2>) -> Frame<f64, U2> {
        let adjusted = self.volume_pan(input);
        self.processors
            .iter_mut()
            .fold(adjusted, |acc, x| x.tick(&acc))
    }
    fn volume_pan(&self, input: &Frame<f64, U2>) -> Frame<f64, U2> {
        let left_vol = self.volume * (1.0 - self.pan).clamp(0.0, 1.0);
        let right_vol = self.volume * (1.0 + self.pan).clamp(0.0, 1.0);
        [left_vol * input[0], right_vol * input[1]].into()
    }
    pub fn add<T>(&mut self, processor: T)
    where
        T: Processor + 'static,
    {
        self.processors.push(Box::new(processor))
    }
}

impl Index<usize> for Channel {
    type Output = Box<dyn Processor>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.processors[index]
    }
}

impl IndexMut<usize> for Channel {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.processors[index]
    }
}

#[derive(Clone)]
struct SynthChannel {
    channel: Channel,
    synth: Box<dyn Synthesizer>,
}

impl SynthChannel {
    fn new(channel: Channel, synth: Box<dyn Synthesizer>) -> Self {
        Self { channel, synth }
    }
    fn tick(&mut self, time: f64) -> Frame<f64, U2> {
        let input = self.synth.tick(time);
        self.channel.tick(&input)
    }
}
