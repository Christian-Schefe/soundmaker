use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SizedSample, Stream};
use fundsp::prelude::*;
use fundsp::wave::Wave64;

pub use fundsp::prelude::Shared;

pub fn find_sample_rate() -> f64 {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("No default output device");
    let config = device.default_output_config().unwrap();
    config.sample_rate().0 as f64
}

pub fn play_and_save(
    data: Vec<(f64, f64)>,
    sample_rate: f64,
    file_path: PathBuf,
    tx: Sender<(Instant, (Shared<f32>, Shared<f64>, Shared<f64>))>,
) -> Result<(), anyhow::Error> {
    let mut wave = Wave64::new(0, sample_rate);
    let (left_channel, right_channel): (Vec<f64>, Vec<f64>) = data.into_iter().unzip();

    wave.push_channel(&left_channel);
    wave.push_channel(&right_channel);

    wave.save_wav32(file_path)?;
    let player = WavePlayback::new(wave);
    let controls = (
        player.is_paused.clone(),
        player.set_time.clone(),
        player.volume.clone(),
    );

    let stream = get_stream(player.clone())?;
    stream.play()?;

    tx.send((Instant::now(), controls)).unwrap();

    while !player.is_finished() {}

    Ok(())
}

pub fn play_wave(wave: Wave64) -> Result<(), anyhow::Error> {
    let duration = wave.duration();

    let player = WavePlayback::new(wave);
    let stream = get_stream(player.clone())?;
    stream.play()?;

    std::thread::sleep(Duration::from_secs_f64(duration));

    Ok(())
}

pub fn get_stream<T>(mut sound: T) -> Result<Stream, anyhow::Error>
where
    T: AudioNode<Sample = f64> + 'static,
{
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("No default output device");
    let config = device.default_output_config().unwrap();
    let sample_rate = config.sample_rate();
    println!("Sample Rate: {:?}", sample_rate);
    sound.set_sample_rate(sample_rate.0 as f64);

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => run::<f32, _>(&device, &config.into(), sound),
        cpal::SampleFormat::I16 => run::<i16, _>(&device, &config.into(), sound),
        cpal::SampleFormat::U16 => run::<u16, _>(&device, &config.into(), sound),
        _ => panic!("Unsupported format"),
    }?;

    Ok(stream)
}

fn run<T, Q>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mut sound: Q,
) -> Result<Stream, anyhow::Error>
where
    T: SizedSample + FromSample<f64>,
    Q: AudioNode<Sample = f64> + 'static,
{
    let sample_rate = config.sample_rate.0 as f64;
    let channels = config.channels as usize;

    sound.set_sample_rate(sample_rate);

    let mut next_value = move || sound.get_stereo();

    let err_fn = |err| eprintln!("An error has occured on stream: {}", err);

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _| write_data(data, channels, &mut next_value),
        err_fn,
        None,
    )?;

    Ok(stream)
}

fn write_data<T>(output: &mut [T], channels: usize, next_sample: &mut dyn FnMut() -> (f64, f64))
where
    T: SizedSample + FromSample<f64>,
{
    for frame in output.chunks_mut(channels) {
        let sample = next_sample();
        let left = T::from_sample(sample.0);
        let right = T::from_sample(sample.1);

        for (channel, sample) in frame.iter_mut().enumerate() {
            if channel & 1 == 0 {
                *sample = left;
            } else {
                *sample = right;
            }
        }
    }
}

#[derive(Clone)]
pub struct WavePlayback {
    wave: Wave64,
    is_paused: Shared<f32>, // positive = playing, negative = paused
    set_time: Shared<f64>,  // if positive, set time to this value, then set to -1.0
    volume: Shared<f64>,
    current_index: usize,
    current_time: f64,
    delta_time: f64,
}

impl WavePlayback {
    pub fn new(wave: Wave64) -> Self {
        let sample_rate = wave.sample_rate();
        Self {
            wave,
            is_paused: Shared::new(1.0),
            set_time: Shared::new(-1.0),
            volume: Shared::new(1.0),
            current_index: 0,
            current_time: 0.0,
            delta_time: 1.0 / sample_rate,
        }
    }
    fn is_finished(&self) -> bool {
        false
    }
}

impl AudioNode for WavePlayback {
    const ID: u64 = 0x54C1D;
    type Sample = f64;
    type Inputs = U0;
    type Outputs = U2;
    type Setting = ();

    fn tick(
        &mut self,
        _input: &Frame<Self::Sample, Self::Inputs>,
    ) -> Frame<Self::Sample, Self::Outputs> {
        if self.is_paused.value() < 0.0 {
            return [0.0, 0.0].into();
        }
        if self.set_time.value() >= 0.0 {
            self.current_time = self.set_time.value();
            self.current_index = (self.current_time * self.wave.sample_rate()) as usize;
            self.set_time.set_value(-1.0);
        }

        let vol = self.volume.value();

        let left = self.wave.at(0, self.current_index) * vol;
        let right = self.wave.at(1, self.current_index) * vol;

        if self.current_index + 1 < self.wave.len() {
            self.current_index += 1;
            self.current_time += self.delta_time;
        }

        [left, right].into()
    }

    fn reset(&mut self) {
        self.current_index = 0;
        self.is_paused.set_value(1.0);
        self.set_time.set_value(-1.0);
    }
}
