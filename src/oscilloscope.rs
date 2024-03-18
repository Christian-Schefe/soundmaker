use std::collections::VecDeque;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

use fundsp::prelude::lerp;
use geo::{Coord, LineString};
use piston_window::types::{Color, FontSize};
use piston_window::*;
use rayon::prelude::{IntoParallelRefMutIterator, ParallelIterator};
use rustfft::num_complex::Complex32;
use rustfft::num_traits::Zero;
use rustfft::FftPlanner;

use geo::algorithm::simplify::*;

use crate::daw::DAW;
use crate::playback::play_and_save;
use crate::prelude::render_daw;

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
        data: Vec<(f64, f64)>,
        name: String,
        position: (f64, f64, f64, f64),
        buffer_size: usize,
        zoom: f64,
    ) -> Self {
        Self {
            data: vec![0.0; buffer_size * 2]
                .into_iter()
                .chain(data.into_iter().map(|x| (x.0 + x.1) / 2.0))
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
    fn precompute_indices(&mut self, sample_rate: f64, fps: f64) {
        let mut indices = Vec::new();
        let secs_per_frame = 1.0 / fps;
        let mut i = 0;
        println!("Precomputing {}...", self.name);
        let start_time = Instant::now();
        loop {
            let passed_time = i as f64 * secs_per_frame;
            let index = 2 * self.buffer_size + (sample_rate * passed_time) as usize;
            if index > self.data.len() {
                indices.push(self.data.len()); // Last Frame is just zeros
                break;
            }

            let best_i = self.find_by_zero(index);
            let clamped_i = best_i.clamp(self.buffer_size * 2, self.data.len());
            indices.push(clamped_i);
            i += 1;
        }
        println!(
            "Finished precomputing {} in {:.2}s",
            self.name,
            start_time.elapsed().as_secs_f32()
        );
        self.frame_indices = indices;
    }
    // fn find_by_search(&mut self, index: usize) -> usize {
    //     let mut best_spectrum = perform_fft(&self.data[index - self.buffer_size..index]);
    //     let mut best_score = cross_correlation(&self.prev.1, &best_spectrum);
    //     let mut best_index = index;

    //     let mut update_best = |i, best_index: usize| -> usize {
    //         let spectrum = perform_fft(&self.data[i - self.buffer_size..i]);
    //         let score = cross_correlation(&self.prev.1, &spectrum);

    //         if score > best_score {
    //             best_score = score;
    //             best_spectrum = spectrum;
    //             i
    //         } else {
    //             best_index
    //         }
    //     };

    //     let steps = 40;
    //     let step_size = 800 / steps;

    //     for offset in 0..steps {
    //         let i = index - (offset * step_size);
    //         best_index = update_best(i, best_index);
    //     }

    //     let cur = best_index;

    //     let steps = step_size * 2;
    //     for offset in 0..steps {
    //         let i = cur + step_size - offset;
    //         if i > index {
    //             break;
    //         }
    //         best_index = update_best(i, best_index);
    //     }

    //     self.prev = (best_index, best_spectrum);
    //     best_index
    // }
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

pub fn launch_app(mut daw: DAW, sample_rate: f64, file_path: PathBuf) {
    let render = render_daw(&mut daw, sample_rate);

    let y_fac = 1.0 / (daw.channel_count + 1) as f64;
    let buffer_size = 4096;
    let zoom = 1.0;
    let resolution_fps = 60.0;

    let mut channel_data: Vec<ChannelData> = render
        .channels
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
        render.master.clone(),
        daw.master.name,
        (0.0, 1.0, 1.0 - y_fac, 1.0),
        buffer_size,
        zoom,
    ));

    channel_data
        .par_iter_mut()
        .for_each(|x| x.precompute_indices(sample_rate, resolution_fps));

    let mut window: PistonWindow = WindowSettings::new("Oscilloscope", [1000.0, 1000.0])
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
        play_and_save(render.master, sample_rate, daw.duration, file_path).unwrap();
    });

    let mut fps_counter = FPSCounter::new();
    let start_time = Instant::now();

    while let Some(event) = window.next() {
        let size = window.size();

        window.draw_2d(&event, |c, g, device| {
            clear([0.0, 0.0, 0.0, 1.0], g);

            let size = (size.width, size.height);

            let current_time = start_time.elapsed().as_secs_f64();
            let actual_frame = (current_time * resolution_fps) as usize;

            channel_data.iter_mut().for_each(|x| {
                render_track(size, x, actual_frame, c, g);
                render_name(size, x, c, g, &mut glyphs);
            });

            draw_grid(size, (1, channel_data.len()), c, g);

            let fps = fps_counter.tick();
            let fps_text = format!("FPS: {:.2}", fps);
            render_text(
                &fps_text,
                [1.0; 4],
                24,
                (size.0 - 80.0, 30.0),
                c,
                g,
                &mut glyphs,
            );

            glyphs.factory.encoder.flush(device);
        });
    }
}

fn render_track<G>(
    size: (f64, f64),
    channel_data: &mut ChannelData,
    frame: usize,
    c: Context,
    g: &mut G,
) where
    G: Graphics,
{
    let index = channel_data.frame_indices[frame.min(channel_data.frame_indices.len() - 1)];

    let half_size = channel_data.buffer_size / 2;
    let center = index - half_size;
    let offset = ((half_size as f64 * channel_data.zoom) as usize).min(half_size);

    render_samples(
        &channel_data.data[center - offset..center + offset],
        size.0,
        size.1,
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
    let sample_count = samples.len() as f64;

    let points = space_samples(&samples)
        .into_iter()
        .map(|(x, y)| Coord {
            x: x * sample_count,
            y: y * sample_count,
        })
        .collect();

    let line = LineString::new(points);
    let line = line.simplify(&0.5);
    let vertices: Vec<[f64; 2]> = line
        .points()
        .map(|x| {
            [
                lerp(
                    position.0,
                    position.1,
                    (x.0.x / sample_count).clamp(0.0, 1.0),
                ) * width,
                lerp(
                    position.2,
                    position.3,
                    (x.0.y / sample_count).clamp(0.0, 1.0),
                ) * height,
            ]
        })
        .collect();

    let p = Line::new([1.0, 1.0, 1.0, 1.0], 1.0);

    draw_points(vertices, p, c, g)
}

fn render_name<G, T>(
    size: (f64, f64),
    channel_data: &mut ChannelData,
    c: Context,
    g: &mut G,
    glyphs: &mut T,
) where
    G: Graphics<Texture = T::Texture>,
    T: CharacterCache,
{
    let font_size = 24;

    let x = channel_data.position.0 * size.0 + 10.0;
    let y = channel_data.position.2 * size.1 + 30.0;

    render_text(
        &channel_data.name,
        [1.0, 1.0, 1.0, 1.0],
        font_size,
        (x, y),
        c,
        g,
        glyphs,
    );
}

fn render_text<G, T>(
    text: &str,
    color: Color,
    font_size: FontSize,
    pos: (f64, f64),
    c: Context,
    g: &mut G,
    glyphs: &mut T,
) where
    G: Graphics<Texture = T::Texture>,
    T: CharacterCache,
{
    text::Text::new_color(color, font_size)
        .draw(
            text,
            glyphs,
            &c.draw_state,
            c.transform.trans(pos.0, pos.1),
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

fn draw_points<G>(points: Vec<[f64; 2]>, p: Line, c: Context, g: &mut G)
where
    G: Graphics,
{
    points.windows(2).for_each(|x| {
        let segment = [x[0][0], x[0][1], x[1][0], x[1][1]];
        p.draw(segment, &c.draw_state, c.transform, g);
    });
}

fn draw_grid<G>(size: (f64, f64), divisions: (usize, usize), c: Context, g: &mut G)
where
    G: Graphics,
{
    let line = Line::new([0.0, 0.0, 0.6, 1.0], 1.0);
    let x_w = size.0 / (divisions.0) as f64;
    let y_w = size.1 / (divisions.1) as f64;

    for x in 1..divisions.0 {
        let x_pos = x as f64 * x_w;
        line.draw([x_pos, 0.0, x_pos, size.1], &c.draw_state, c.transform, g)
    }
    for y in 1..divisions.1 {
        let y_pos = y as f64 * y_w;
        line.draw([0.0, y_pos, size.0, y_pos], &c.draw_state, c.transform, g)
    }
}

#[derive(Debug)]
pub struct FPSCounter {
    last_second_frames: VecDeque<Instant>,
}

impl Default for FPSCounter {
    fn default() -> Self {
        FPSCounter::new()
    }
}

impl FPSCounter {
    pub fn new() -> FPSCounter {
        FPSCounter {
            last_second_frames: VecDeque::with_capacity(128),
        }
    }

    pub fn tick(&mut self) -> usize {
        let now = Instant::now();
        let a_second_ago = now - Duration::from_secs(1);

        while self
            .last_second_frames
            .front()
            .map_or(false, |t| *t < a_second_ago)
        {
            self.last_second_frames.pop_front();
        }

        self.last_second_frames.push_back(now);
        self.last_second_frames.len()
    }
}
