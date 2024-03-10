use piston_window::*;

use crate::node::AudioNode;

pub fn render(mut node: Box<dyn AudioNode>) {
    let mut window: PistonWindow = WindowSettings::new("Oscilloscope View", [800, 600])
        .exit_on_esc(true)
        .build()
        .unwrap();

    let sample_time = 0.1;
    let sample_rate = 44100.0;

    let buffer_size = (sample_rate * sample_time) as usize;

    let display_buffer = vec![0.0; buffer_size];
    let backbuffer = vec![0.0; buffer_size];

    let index = 0;

    while let Some(event) = window.next() {
        window.draw_2d(&event, |c, g, _| {
            let width = 800.0;
            let height = 600.0;
            clear([1.0; 4], g);

            let mut points: Vec<[f64; 2]> = Vec::new();
            let num_samples = width as usize;

            for i in 0..num_samples {
                let x = i as f64 / num_samples as f64;
                let sample = node.get_stereo().0;
                let y = (sample * 0.5 + 0.5) * height; // Normalize the sample to fit within the window
                points.push([x * width, y]);
            }

            let segments: Vec<[f64; 4]> = points
                .windows(2)
                .map(|a| [a[0][0], a[0][1], a[1][0], a[1][1]])
                .collect();

            let line = Line::new([0.0, 0.0, 0.0, 1.0], 2.0);
            for segment in segments {
                line.draw(segment, &c.draw_state, c.transform, g);
            }
        });
    }
}
