use std::thread;
use std::time::Instant;

use fundsp::prelude::lerp;
use piston_window::*;
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
            position,
            buffer_size,
            zoom,
            prev: (0, vec![Complex32::zero(); buffer_size]),
            name,
        }
    }
}

pub fn render(mut daw: DAW, sample_rate: f64) {
    let mut global_data = GlobalData::new(1600.0, 1200.0, sample_rate);

    let (duration, mix_wave, channel_waves) = daw.render_waves(sample_rate);

    println!("duration: {}", duration.as_secs_f32());

    let y_fac = 1.0 / (daw.channel_count + 1) as f64;

    let mut channel_data: Vec<ChannelData> = channel_waves
        .into_iter()
        .enumerate()
        .map(|(i, x)| {
            ChannelData::new(
                x,
                daw[i].name.clone(),
                (0.0, 1.0, i as f64 * y_fac, (i + 1) as f64 * y_fac),
                4096,
                0.5,
            )
        })
        .collect();

    channel_data.push(ChannelData::new(
        mix_wave.clone(),
        daw.master.name,
        (0.0, 1.0, 1.0 - y_fac, 1.0),
        4096,
        0.5,
    ));

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

    while let Some(event) = window.next() {
        window.draw_2d(&event, |c, g, device| {
            clear([0.0, 0.0, 0.0, 1.0], g);

            let current_time = global_data.current_time().min(duration.as_secs_f64());
            channel_data.iter_mut().for_each(|x| {
                render_track(&global_data, x, current_time, c, g);
                render_text(
                    &x.name,
                    global_data.width,
                    global_data.height,
                    x.position,
                    c,
                    g,
                    &mut glyphs,
                );
            });

            draw_grid(
                global_data.width,
                global_data.height,
                (1, channel_data.len()),
                c,
                g,
            );

            glyphs.factory.encoder.flush(device);
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

    find_best_window(
        &channel_data.data,
        &mut channel_data.prev,
        index,
        buffer_size,
    );

    let center = channel_data.prev.0 - buffer_size / 2;
    let offset = (buffer_size as f64 * channel_data.zoom * 0.5) as usize;

    render_samples(
        &channel_data.data[center - offset..center + offset],
        global_data.width,
        global_data.height,
        channel_data.position,
        c,
        g,
    );
}

fn find_best_window(data: &[f64], prev: &mut (usize, Vec<Complex32>), cur: usize, length: usize) {
    let mut best = None;

    let update_best = |index, best: &mut Option<(usize, f32, Vec<Complex32>)>| {
        let spectrum = perform_fft(&data[index - length..index]);
        let score = cross_correlation(&prev.1, &spectrum);

        if best.is_none() || best.as_ref().is_some_and(|x| x.1 < score) {
            *best = Some((index, score, spectrum))
        }
    };

    let step_size = 30;
    let steps = length / (step_size * 2);
    for offset in 0..steps {
        let index = cur - (offset * step_size);
        update_best(index, &mut best);
    }

    let cur = best.as_ref().unwrap().0;

    for offset in 0..step_size * 2 {
        let index = cur + step_size - offset;
        update_best(index, &mut best);
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
    let points = space_samples(samples)
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

fn render_text<G, T>(
    text: &str,
    width: f64,
    height: f64,
    position: (f64, f64, f64, f64),
    c: Context,
    g: &mut G,
    glyphs: &mut T,
) where
    G: Graphics<Texture = T::Texture>,
    T: CharacterCache,
{
    let font_size = 24;

    let x = position.0 * width + 10.0;
    let y = position.2 * height + 30.0;

    text::Text::new_color([1.0, 1.0, 1.0, 1.0], font_size)
        .draw(text, glyphs, &c.draw_state, c.transform.trans(x, y), g)
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
