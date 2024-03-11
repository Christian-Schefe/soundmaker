use std::time::{Duration, Instant};

use piston_window::*;
use rustfft::num_complex::Complex32;
use rustfft::num_traits::Zero;
use rustfft::FftPlanner;

use crate::node::AudioNode;

pub fn render(node: Box<dyn AudioNode>, duration: Duration) {
    let width = 800.0;
    let height = 600.0;

    let mut window: PistonWindow = WindowSettings::new("Oscilloscope View", [width, height])
        .exit_on_esc(true)
        .build()
        .unwrap();

    let sample_rate = 44100.0;
    let data = compute_data(node, sample_rate, duration);
    let start_time = Instant::now();
    let buffer_size = 4096;
    let data: Vec<f64> = vec![0.0; buffer_size * 2].into_iter().chain(data).collect();

    let zoom = 0.5;

    let mut prev = (buffer_size, vec![Complex32::zero(); buffer_size]);

    while let Some(event) = window.next() {
        window.draw_2d(&event, |c, g, _| {
            clear([1.0; 4], g);

            let current_time = start_time.elapsed().as_secs_f64();
            let index = (buffer_size * 2 + (current_time * sample_rate) as usize).min(data.len());

            find_best_i(&data, &mut prev, index, buffer_size);

            let center = prev.0 - buffer_size / 2;
            let offset = (buffer_size as f64 * zoom * 0.5) as usize;

            render_samples(&data[center - offset..center + offset], width, height, c, g);
        });
    }
}

fn find_best_i(data: &[f64], prev: &mut (usize, Vec<Complex32>), cur: usize, length: usize) {
    let mut best = None;

    let step_size = 30;
    for offset in 0..length / (step_size * 2) {
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

fn compute_data(mut node: Box<dyn AudioNode>, sample_rate: f64, duration: Duration) -> Vec<f64> {
    let samples = (sample_rate * duration.as_secs_f64()).round() as usize;

    let mut data = Vec::with_capacity(samples);
    for _i in 0..samples {
        data.push(node.get_stereo().0);
    }
    data
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

fn render_samples<G>(samples: &[f64], width: f64, height: f64, c: Context, g: &mut G)
where
    G: Graphics,
{
    let points = space_samples(samples, width, height);
    let line = Line::new([0.0, 0.0, 0.0, 1.0], 1.0);

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
