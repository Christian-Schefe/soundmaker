use std::ops::Index;
use std::ops::IndexMut;
use std::time::Duration;
use std::time::Instant;

use fundsp::prelude::*;
use midly::Smf;
use rayon::iter::IntoParallelIterator;
use rayon::iter::IntoParallelRefMutIterator;
use rayon::iter::ParallelIterator;

use crate::prelude::*;

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

        let track_names = midi
            .tracks
            .iter()
            .map(|x| MidiMsg::extract_track_name(x))
            .collect::<Vec<String>>();

        let (fixed_midi, duration) = MidiMsg::convert_smf(midi);
        self.duration = duration + Duration::from_secs_f64(5.0);
        println!(
            "Determined duration of {:.2} seconds.",
            duration.as_secs_f64()
        );

        for (i, channel) in self.channels.iter_mut().enumerate() {
            if let Some(name) = track_names.get(i).cloned() {
                println!("Channel {} has name: {}", i, name);
                channel.channel.name = name;
            }

            let track = fixed_midi
                .get(i)
                .map(|x| x.iter().copied().collect())
                .unwrap_or(Vec::new());

            channel.synth.set_midi(track);
        }
    }
    pub fn set_midi_bytes(&mut self, bytes: &[u8]) {
        let smf = Smf::parse(bytes).unwrap();
        self.set_midi(smf);
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
        for channel in self.channels.iter_mut() {
            channel.reset();
        }
        self.master.reset();
    }

    fn set_sample_rate(&mut self, sample_rate: f64) {
        self.delta_time = 1.0 / sample_rate;
        for channel in self.channels.iter_mut() {
            channel.set_sample_rate(sample_rate);
        }
        self.master.set_sample_rate(sample_rate);
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
    fn set_sample_rate(&mut self, sample_rate: f64) {
        for processor in self.processors.iter_mut() {
            processor.set_sample_rate(sample_rate);
        }
    }
    fn reset(&mut self) {
        for processor in self.processors.iter_mut() {
            processor.reset();
        }
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
    fn set_sample_rate(&mut self, sample_rate: f64) {
        self.channel.set_sample_rate(sample_rate);
        self.synth.set_sample_rate(sample_rate);
    }
    fn reset(&mut self) {
        self.channel.reset();
        self.synth.reset();
    }
}

pub struct RenderedAudio {
    pub master: Vec<(f64, f64)>,
    pub channels: Vec<Vec<(f64, f64)>>,
}

pub fn render_daw(daw: &mut DAW, sample_rate: f64) -> RenderedAudio {
    daw.set_sample_rate(sample_rate);
    daw.reset();
    let start_time = Instant::now();
    println!("Started rendering...");
    let sample_count = (daw.duration.as_secs_f64() * sample_rate).round() as usize;
    let channels: Vec<Vec<(f64, f64)>> = daw
        .channels
        .par_iter_mut()
        .map(|x| {
            let render = render_channel(x, sample_count, sample_rate);
            render
        })
        .collect();

    let master = render_master(&mut daw.master, &channels, sample_count);
    println!(
        "Finished rendering in {:.2} seconds.",
        start_time.elapsed().as_secs_f64()
    );
    RenderedAudio { master, channels }
}

fn render_master(
    master: &mut Channel,
    channels: &[Vec<(f64, f64)>],
    sample_count: usize,
) -> Vec<(f64, f64)> {
    (0..sample_count)
        .into_par_iter()
        .map(|x| {
            channels
                .iter()
                .map(|vec| vec[x])
                .fold((0.0, 0.0), |acc, x| (acc.0 + x.0, acc.1 + x.1))
        })
        .collect::<Vec<(f64, f64)>>()
        .into_iter()
        .map(|x| {
            let input = [x.0, x.1].into();
            let output = master.tick(&input);
            (output[0], output[1])
        })
        .collect()
}

fn render_channel(
    channel: &mut SynthChannel,
    sample_count: usize,
    sample_rate: f64,
) -> Vec<(f64, f64)> {
    let mut samples = Vec::with_capacity(sample_count);

    for i in 0..sample_count {
        let time = i as f64 / sample_rate;
        let sample = channel.tick(time);
        samples.push((sample[0], sample[1]))
    }

    samples
}
