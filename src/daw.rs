use std::ops::Index;
use std::ops::IndexMut;
use std::time::Duration;

use fundsp::prelude::*;
use midly::Smf;

use crate::daw::midi::MidiMsg;

pub use self::processor::*;
pub use self::synthesizer::*;

mod midi;
mod processor;
mod synthesizer;

fn pan_weights<T: Real>(value: T) -> (T, T) {
    let angle = (clamp11(value) + T::one()) * T::from_f64(PI * 0.25);
    (cos(angle), sin(angle))
}

#[derive(Clone)]
pub struct DAW {
    channels: Vec<SynthChannel>,
    pub channel_count: usize,
    master: Channel,
    time: f64,
    delta_time: f64,
}

impl DAW {
    pub fn new() -> Self {
        Self {
            channels: Vec::new(),
            channel_count: 0,
            master: Channel::new(0, 1.0, 0.0, 0.0),
            time: 0.0,
            delta_time: 1.0 / DEFAULT_SR,
        }
    }
    pub fn set_midi(&mut self, midi: Smf) {
        // assert_eq!(midi.tracks.len(), self.channel_count);

        for (i, channel) in self.channels.iter_mut().enumerate() {
            let track = MidiMsg::from_track(&midi.tracks[i]);
            
            for event in track.iter() {
                println!("{:?}", event);
            }
            channel.synth.set_midi(track);
        }
    }
    pub fn render_waves(&mut self, sample_rate: f64) -> (Duration, Vec<f64>, Vec<Vec<f64>>) {
        self.reset();
        self.set_sample_rate(sample_rate);

        let mut channel_data = vec![Vec::new(); self.channel_count];
        let mut master_data: Vec<f64> = Vec::new();

        let mut last_meaningful = 0.0;

        println!("rendering");

        while self.time <= 50.0 {
            let outputs: Vec<f64> = self.tick_channels().into_iter().map(|x| x[0]).collect();
            for i in 0..self.channel_count {
                channel_data[i].push(outputs[i])
            }
            let mix = outputs.into_iter().sum();
            master_data.push(mix);
            if mix > 0.1 {
                last_meaningful = self.time;
            }
        }
        println!("rendering finished with duration: {}", self.time);

        (
            Duration::from_secs_f64(self.time),
            master_data,
            channel_data,
        )
    }
    pub fn add_channel_boxed(&mut self, synth: Box<dyn Synthesizer>, volume: f64) -> usize {
        let index = self.channel_count;
        self.channel_count += 1;
        self.channels.push(SynthChannel::new(
            Channel::new(index, volume, 0.0, 0.0),
            synth,
        ));
        index
    }
    pub fn add_channel<T>(&mut self, synth: T, volume: f64) -> usize
    where
        T: Synthesizer + 'static,
    {
        self.add_channel_boxed(Box::new(synth), volume)
    }
    pub fn set_master_volume(&mut self, volume: f64) {
        self.master.volume_pan.set_volume(volume);
    }
    pub fn set_master_pan(&mut self, left_pan: f64, right_pan: f64) {
        self.master.volume_pan.set_pan(left_pan, right_pan);
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
    volume_pan: VolumePanStereo,
    pub processors: Vec<Box<dyn Processor>>,
}

impl Channel {
    fn new(index: usize, volume: f64, pan_left: f64, pan_right: f64) -> Self {
        Self {
            index,
            volume_pan: VolumePanStereo::new(volume, pan_left, pan_right),
            processors: Vec::new(),
        }
    }
    pub fn set_volume(&mut self, volume: f64) {
        self.volume_pan.set_volume(volume);
    }
    pub fn set_pan(&mut self, left_pan: f64, right_pan: f64) {
        self.volume_pan.set_pan(left_pan, right_pan);
    }
    fn tick(&mut self, input: &Frame<f64, U2>) -> Frame<f64, U2> {
        self.processors
            .iter_mut()
            .fold(self.volume_pan.tick(input), |acc, x| x.tick(&acc))
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

#[derive(Clone)]
struct VolumePanStereo {
    volume: f64,
    left_pan: (f64, f64),
    right_pan: (f64, f64),
}

impl VolumePanStereo {
    fn new(volume: f64, left_pan: f64, right_pan: f64) -> Self {
        Self {
            volume,
            left_pan: pan_weights(left_pan),
            right_pan: pan_weights(right_pan),
        }
    }
    fn set_volume(&mut self, volume: f64) {
        self.volume = volume;
    }
    fn set_pan(&mut self, left_pan: f64, right_pan: f64) {
        self.left_pan = pan_weights(left_pan);
        self.right_pan = pan_weights(right_pan);
    }
    fn tick(&mut self, input: &Frame<f64, U2>) -> Frame<f64, U2> {
        [
            self.volume * (input[0] * self.left_pan.0 + input[1] * self.right_pan.0),
            self.volume * (input[0] * self.left_pan.1 + input[1] * self.right_pan.1),
        ]
        .into()
    }
}
