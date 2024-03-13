use std::thread;
use std::time::{Duration, Instant};

use piston_window::*;
use rustfft::num_complex::Complex32;
use rustfft::num_traits::Zero;
use rustfft::FftPlanner;
use typenum::{U0, U1};

use crate::node::AudioNode;
use crate::output::playback;
use crate::{Layer, Wave};

struct GlobalData {
    width: f64,
    height: f64,
    sample_rate: f64,
    start_time: Instant,
}

impl GlobalData {
    fn new(width: f64, height: f64, sample_rate: f64) -> Self {
        Self {
            width,
            height,
            sample_rate,
            start_time: Instant::now(),
        }
    }
    fn current_time(&self) -> f64 {
        self.start_time.elapsed().as_secs_f64()
    }
}

struct ChannelData {
    data: Vec<f64>,
    position: (f64, f64, f64, f64),
    buffer_size: usize,
    zoom: f64,
    prev: (usize, Vec<Complex32>),
}

impl ChannelData {
    fn new(data: Vec<f64>, position: (f64, f64, f64, f64), buffer_size: usize, zoom: f64) -> Self {
        Self {
            data: vec![0.0; buffer_size * 2].into_iter().chain(data).collect(),
            position,
            buffer_size,
            zoom,
            prev: (0, vec![Complex32::zero(); buffer_size]),
        }
    }
}

pub fn render(mut channels: Vec<Box<dyn AudioNode>>, sample_rate: f64, duration: Duration) {
    let mut global_data = GlobalData::new(800.0, 600.0, sample_rate);
    channels
        .iter_mut()
        .for_each(|x| x.set_sample_rate(sample_rate));

    let mut window: PistonWindow =
        WindowSettings::new("Oscilloscope View", [global_data.width, global_data.height])
            .exit_on_esc(true)
            .build()
            .unwrap();

    let y_fac = 1.0 / channels.len() as f64;

    let mut channel_data: Vec<ChannelData> = channels
        .clone()
        .into_iter()
        .map(|x| Wave::render(x, global_data.sample_rate, duration))
        .enumerate()
        .map(|(i, x)| {
            ChannelData::new(
                x.data,
                (0.0, 1.0, i as f64 * y_fac, (i + 1) as f64 * y_fac),
                4096,
                0.5,
            )
        })
        .collect();

    let playback_mix = Layer::<U0, U1>::new(channels.clone());
    // let playback_wave = Wave::render(Box::new(playback_mix), global_data.sample_rate, duration);

    thread::spawn(move || {
        playback(Box::new(playback_mix), duration).unwrap();
    });

    global_data.start_time = Instant::now();

    while let Some(event) = window.next() {
        window.draw_2d(&event, |c, g, _| {
            clear([0.0, 0.0, 0.0, 1.0], g);

            let current_time = global_data.current_time();
            channel_data
                .iter_mut()
                .for_each(|x| render_track(&global_data, x, current_time, c, g))
        });
    }
}

fn render_track<G>(
    global_data: &GlobalData,
    channel_data: &mut ChannelData,
    current_time: f64,
    c: Context,
    g: &mut G,
) where
    G: Graphics,
{
    let buffer_size = channel_data.buffer_size;
    let index = (channel_data.buffer_size * 2 + (current_time * global_data.sample_rate) as usize)
        .min(channel_data.data.len());

    find_best_i(
        &channel_data.data,
        &mut channel_data.prev,
        index,
        buffer_size,
    );

    let center = channel_data.prev.0 - buffer_size / 2;
    let offset = (buffer_size as f64 * channel_data.zoom * 0.5) as usize;

    let rect = (
        channel_data.position.0 * global_data.width,
        channel_data.position.2 * global_data.height,
        channel_data.position.1 * global_data.width - channel_data.position.0 * global_data.width,
        channel_data.position.3 * global_data.height - channel_data.position.2 * global_data.height,
    );

    render_samples(
        &channel_data.data[center - offset..center + offset],
        rect,
        c,
        g,
    );
}

fn find_best_i(data: &[f64], prev: &mut (usize, Vec<Complex32>), cur: usize, length: usize) {
    let mut best = None;

    let step_size = 30;
    let steps = length / (step_size * 2);
    for offset in 0..steps {
        let index = cur - (offset * step_size);

        let spectrum = perform_fft(&data[index - length..index]);
        let cross_corr = cross_correlation(&prev.1, &spectrum);

        best = match best {
            None => Some((index, cross_corr, spectrum)),
            Some((_, c, _)) if c < cross_corr => Some((index, cross_corr, spectrum)),
            Some(x) => Some(x),
        }
    }

    let cur = best.as_ref().unwrap().0;

    for offset in 0..step_size * 2 {
        let index = cur + step_size - offset;

        let spectrum = perform_fft(&data[index - length..index]);
        let cross_corr = cross_correlation(&prev.1, &spectrum);

        best = match best {
            None => Some((index, cross_corr, spectrum)),
            Some((_, c, _)) if c < cross_corr => Some((index, cross_corr, spectrum)),
            Some(x) => Some(x),
        }
    }

    *prev = best.map(|x| (x.0, x.2)).unwrap();
}

fn cross_correlation(prev_spectrum: &[Complex32], spectrum: &[Complex32]) -> f32 {
    let mut cross_correlation = 0.0;
    let window_size = prev_spectrum.len().min(spectrum.len());
    for i in 0..window_size {
        cross_correlation += prev_spectrum[i].re * spectrum[i].re;
    }
    cross_correlation /= window_size as f32;
    cross_correlation
}

fn perform_fft(samples: &[f64]) -> Vec<Complex32> {
    let length = samples.len();

    let mut spectrum: Vec<Complex32> = samples
        .into_iter()
        .map(|x| Complex32 {
            re: *x as f32,
            im: 0.0,
        })
        .collect();

    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(length);
    fft.process(&mut spectrum);
    spectrum
}

fn render_samples<G>(samples: &[f64], rect: (f64, f64, f64, f64), c: Context, g: &mut G)
where
    G: Graphics,
{
    let points = space_samples(samples, rect.2, rect.3)
        .into_iter()
        .map(|(x, y)| (x + rect.0, y + rect.1))
        .collect();
    let line = Line::new([1.0, 1.0, 1.0, 1.0], 1.0);

    draw_lines(points, line, c, g)
}

fn space_samples(samples: &[f64], width: f64, height: f64) -> Vec<(f64, f64)> {
    let spacing = width / (samples.len() - 1) as f64;
    samples
        .into_iter()
        .enumerate()
        .map(|(i, y)| (i as f64 * spacing, (y * 0.5 + 0.5) * height))
        .collect()
}

fn draw_lines<G>(points: Vec<(f64, f64)>, line: Line, c: Context, g: &mut G)
where
    G: Graphics,
{
    let segments: Vec<[f64; 4]> = points
        .windows(2)
        .map(|a| [a[0].0, a[0].1, a[1].0, a[1].1])
        .collect();

    for segment in segments {
        line.draw(segment, &c.draw_state, c.transform, g);
    }
}
