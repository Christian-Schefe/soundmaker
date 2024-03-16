use std::thread;
use std::time::Instant;

use fundsp::prelude::lerp;
use piston_window::*;
use rayon::prelude::{IntoParallelRefMutIterator, ParallelIterator};
use rustfft::num_complex::Complex32;
use rustfft::num_traits::Zero;
use rustfft::FftPlanner;

use crate::daw::DAW;
use crate::playback::play_data;

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
    frame_indices: Vec<usize>,
    position: (f64, f64, f64, f64),
    buffer_size: usize,
    zoom: f64,
    prev: (usize, Vec<Complex32>),
    name: String,
}

impl ChannelData {
    fn new(
        data: Vec<f64>,
        name: String,
        position: (f64, f64, f64, f64),
        buffer_size: usize,
        zoom: f64,
    ) -> Self {
        Self {
            data: vec![0.0; buffer_size * 2]
                .into_iter()
                .chain(data)
                .chain(vec![0.0; buffer_size * 2])
                .collect(),
            frame_indices: Vec::new(),
            position,
            buffer_size,
            zoom,
            prev: (0, vec![Complex32::zero(); buffer_size]),
            name,
        }
    }
    fn precompute_indices(&mut self, global_data: &GlobalData, fps: f64) {
        let mut indices = Vec::new();
        let secs_per_frame = 1.0 / fps;
        let mut i = 0;
        println!("Precomputing...");
        let start_time = Instant::now();
        loop {
            let passed_time = i as f64 * secs_per_frame;
            let index = 2 * self.buffer_size + (global_data.sample_rate * passed_time) as usize;
            if index >= self.data.len() {
                break;
            }

            let best_i = self.find_by_zero(index);
            indices.push(best_i);
            i += 1;
        }
        println!(
            "Finished precomputing {} in {:.2}s",
            self.name,
            start_time.elapsed().as_secs_f32()
        );
        self.frame_indices = indices;
    }
    fn find_by_search(&mut self, index: usize) -> usize {
        let mut best_spectrum = perform_fft(&self.data[index - self.buffer_size..index]);
        let mut best_score = cross_correlation(&self.prev.1, &best_spectrum);
        let mut best_index = index;

        let mut update_best = |i, best_index: usize| -> usize {
            let spectrum = perform_fft(&self.data[i - self.buffer_size..i]);
            let score = cross_correlation(&self.prev.1, &spectrum);

            if score > best_score {
                best_score = score;
                best_spectrum = spectrum;
                i
            } else {
                best_index
            }
        };

        let steps = 40;
        let step_size = 800 / steps;

        for offset in 0..steps {
            let i = index - (offset * step_size);
            best_index = update_best(i, best_index);
        }

        let cur = best_index;

        let steps = step_size * 2;
        for offset in 0..steps {
            let i = cur + step_size - offset;
            if i > index {
                break;
            }
            best_index = update_best(i, best_index);
        }

        self.prev = (best_index, best_spectrum);
        best_index
    }
    fn find_by_zero(&mut self, index: usize) -> usize {
        let zeros = (0..800).filter_map(|x| {
            let i = index - x;
            let val = self.data[i];
            if val >= 0.0 && self.data[i - 1] < 0.0 {
                Some(i)
            } else {
                None
            }
        });

        let best = zeros
            .map(|x| {
                let spectrum = perform_fft(&self.data[x - self.buffer_size..x]);
                let score = cross_correlation(&self.prev.1, &spectrum);
                (score, x, spectrum)
            })
            .max_by(|a, b| a.0.total_cmp(&b.0));

        if let Some((_, best_index, best_spectrum)) = best {
            self.prev = (best_index, best_spectrum);
        } else {
            self.prev = (
                index,
                perform_fft(&self.data[index - self.buffer_size..index]),
            );
        }

        self.prev.0 + self.buffer_size / 2
    }
}

pub fn render(mut daw: DAW, sample_rate: f64) {
    let mut global_data = GlobalData::new(1000.0, 1000.0, sample_rate);

    let (duration, mix_wave, channel_waves) = daw.render_waves(sample_rate);

    println!("duration: {}", duration.as_secs_f32());

    let y_fac = 1.0 / (daw.channel_count + 1) as f64;
    let buffer_size = 4096;
    let zoom = 1.0;
    let resolution_fps = 60.0;

    let mut channel_data: Vec<ChannelData> = channel_waves
        .into_iter()
        .enumerate()
        .map(|(i, x)| {
            ChannelData::new(
                x,
                daw[i].name.clone(),
                (0.0, 1.0, i as f64 * y_fac, (i + 1) as f64 * y_fac),
                buffer_size,
                zoom,
            )
        })
        .collect();

    channel_data.push(ChannelData::new(
        mix_wave.clone(),
        daw.master.name,
        (0.0, 1.0, 1.0 - y_fac, 1.0),
        buffer_size,
        zoom,
    ));

    channel_data
        .par_iter_mut()
        .for_each(|x| x.precompute_indices(&global_data, resolution_fps));

    let mut window: PistonWindow =
        WindowSettings::new("Oscilloscope", [global_data.width, global_data.height])
            .exit_on_esc(true)
            .build()
            .unwrap();

    let mut glyphs = Glyphs::new(
        "assets/Sarala-Regular.ttf",
        TextureContext {
            factory: window.factory.clone(),
            encoder: window.factory.create_command_buffer().into(),
        },
        TextureSettings::new()
            .min(Filter::Nearest)
            .mag(Filter::Nearest),
    )
    .unwrap();

    thread::spawn(move || {
        play_data(mix_wave, sample_rate).unwrap();
    });

    global_data.start_time = Instant::now();

    let mut delta_time_buffer = [0.0, 0.0, 0.0];
    let mut delta_index = 0;
    let mut prev_time = 0.0;

    // let mut frame = 0;

    while let Some(event) = window.next() {
        window.draw_2d(&event, |c, g, device| {
            clear([0.0, 0.0, 0.0, 1.0], g);

            let current_time = global_data.current_time();
            let delta_time = current_time - prev_time;
            delta_time_buffer[delta_index] = delta_time;
            delta_index = (delta_index + 1) % delta_time_buffer.len();
            prev_time = current_time;

            let fps = delta_time_buffer.len() as f64 / delta_time_buffer.iter().sum::<f64>();
            println!("fps: {fps:.2}");

            let actual_frame = (current_time * resolution_fps) as usize;

            channel_data.iter_mut().for_each(|x| {
                render_track(&global_data, x, actual_frame, c, g);
                render_name(&global_data, x, c, g, &mut glyphs);
            });

            draw_grid(
                global_data.width,
                global_data.height,
                (1, channel_data.len()),
                c,
                g,
            );

            glyphs.factory.encoder.flush(device);
            // frame += 1;
        });
    }
}

fn render_track<G>(
    global_data: &GlobalData,
    channel_data: &mut ChannelData,
    frame: usize,
    c: Context,
    g: &mut G,
) where
    G: Graphics,
{
    let index = channel_data.frame_indices[frame.min(channel_data.frame_indices.len() - 1)];

    let center = index - channel_data.buffer_size / 2;
    let offset = (channel_data.buffer_size as f64 * channel_data.zoom * 0.5) as usize;

    render_samples(
        &channel_data.data[center - offset..center + offset],
        global_data.width,
        global_data.height,
        channel_data.position,
        c,
        g,
    );
}

fn cross_correlation(prev_spectrum: &[Complex32], spectrum: &[Complex32]) -> f32 {
    let mut cross_correlation = 0.0;
    let window_size = prev_spectrum.len().min(spectrum.len());
    for i in 0..window_size {
        cross_correlation +=
            prev_spectrum[i].re * spectrum[i].re + prev_spectrum[i].im * spectrum[i].im;
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

fn render_samples<G>(
    samples: &[f64],
    width: f64,
    height: f64,
    position: (f64, f64, f64, f64),
    c: Context,
    g: &mut G,
) where
    G: Graphics,
{
    let new_samples: Vec<f64> = (0..=width as usize)
        .map(|x| {
            let alpha = x as f64 / width;
            let i = (alpha * (samples.len() - 1) as f64) as usize;
            samples[i]
        })
        .collect();

    let points = space_samples(&new_samples)
        .into_iter()
        .map(|(x, y)| {
            (
                lerp(position.0, position.1, x.clamp(0.0, 1.0)) * width,
                lerp(position.2, position.3, y.clamp(0.0, 1.0)) * height,
            )
        })
        .collect();
    let line = Line::new([1.0, 1.0, 1.0, 1.0], 1.0);

    draw_lines(points, line, c, g)
}

fn render_name<G, T>(
    global_data: &GlobalData,
    channel_data: &mut ChannelData,
    c: Context,
    g: &mut G,
    glyphs: &mut T,
) where
    G: Graphics<Texture = T::Texture>,
    T: CharacterCache,
{
    let font_size = 24;

    let x = channel_data.position.0 * global_data.width + 10.0;
    let y = channel_data.position.2 * global_data.height + 30.0;

    text::Text::new_color([1.0, 1.0, 1.0, 1.0], font_size)
        .draw(
            &channel_data.name,
            glyphs,
            &c.draw_state,
            c.transform.trans(x, y),
            g,
        )
        .unwrap();
}

fn space_samples(samples: &[f64]) -> Vec<(f64, f64)> {
    let spacing = 1.0 / (samples.len() - 1) as f64;
    samples
        .into_iter()
        .enumerate()
        .map(|(i, y)| (i as f64 * spacing, (y * 0.5 + 0.5)))
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

fn draw_grid<G>(width: f64, height: f64, divisions: (usize, usize), c: Context, g: &mut G)
where
    G: Graphics,
{
    let line = Line::new([0.0, 0.0, 0.6, 1.0], 1.0);
    let x_w = width / (divisions.0) as f64;
    let y_w = height / (divisions.1) as f64;

    for x in 1..divisions.0 {
        let x_pos = x as f64 * x_w;
        line.draw([x_pos, 0.0, x_pos, height], &c.draw_state, c.transform, g)
    }
    for y in 1..divisions.1 {
        let y_pos = y as f64 * y_w;
        line.draw([0.0, y_pos, width, y_pos], &c.draw_state, c.transform, g)
    }
}
