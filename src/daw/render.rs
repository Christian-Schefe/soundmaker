use std::time::Instant;

use rayon::prelude::{IntoParallelIterator, IntoParallelRefMutIterator, ParallelIterator};

use super::{SynthChannel, DAW};

pub struct Render {
    pub master: Vec<(f64, f64)>,
    pub channels: Vec<Vec<(f64, f64)>>,
}

pub fn render_daw(daw: &mut DAW, sample_rate: f64) -> Render {
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

    let master = render_master(&channels, sample_count);
    println!(
        "Finished rendering in {:.2} seconds.",
        start_time.elapsed().as_secs_f64()
    );
    Render { master, channels }
}

fn render_master(channels: &[Vec<(f64, f64)>], sample_count: usize) -> Vec<(f64, f64)> {
    (0..sample_count)
        .into_par_iter()
        .map(|x| {
            channels
                .iter()
                .map(|vec| vec[x])
                .fold((0.0, 0.0), |acc, x| (acc.0 + x.0, acc.1 + x.1))
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
