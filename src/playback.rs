use std::sync::Arc;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SizedSample, Stream};
use fundsp::prelude::AudioNode;
use fundsp::wave::{Wave64, Wave64Player};

pub fn play_data(data: Vec<(f64, f64)>, sample_rate: f64, duration: Duration) -> Result<(), anyhow::Error> {
    let mut wave = Wave64::new(0, sample_rate);
    let (left_channel, right_channel): (Vec<f64>, Vec<f64>) = data.into_iter().unzip();

    wave.push_channel(&left_channel);
    wave.push_channel(&right_channel);

    let len = wave.len();

    let arc = Arc::new(wave);

    let player = Wave64Player::new(&arc, 0, 0, len, None);
    arc.save_wav32("output/master.wav")?;
    play_sound(player, duration)?;
    Ok(())
}

pub fn play_sound<T>(mut sound: T, duration: Duration) -> Result<(), anyhow::Error>
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

    stream.play()?;
    std::thread::sleep(duration);
    Ok(())
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
