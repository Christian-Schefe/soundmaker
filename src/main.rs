use std::{thread, time::Duration};

use fundsp::prelude::*;
use petgraph::Graph;
use soundmaker::{
    node::{AudioNode, Const, Envelope, Envelope0, GraphExtensions, NodeNetwork},
    oscilloscope::render,
    output::playback,
    wavetable::{Wavetable, WavetableSynth},
};

fn main() {
    let w1 = Wavetable::new(
        20.0,
        20000.0,
        4.0,
        &|i| if (i & 1) == 1 { 0.0 } else { 0.5 },
        &|_, i| 1.0 / i as f64,
    );

    let w2 = Wavetable::new(
        20.0,
        20_000.0,
        4.0,
        &|i| if (i & 3) == 3 { 0.5 } else { 0.0 },
        &|_, i| {
            if (i & 1) == 1 {
                1.0 / (i * i) as f64
            } else {
                0.0
            }
        },
    );

    let synth = WavetableSynth::new(w1);

    let mut graph: Graph<Box<dyn AudioNode>, _> = Graph::new();

    let input = graph.add(Envelope0::new(|t| 440.0 * (-t * 0.0).exp()));
    let envelope = graph.add(Envelope::new(|t, x: &Frame<f64, U1>| {
        let y = x[0].signum() * x[0].abs().sqrt();
        (3.0 * y).clamp(-1.0, 1.0) * 0.5
        // (x[0] * 10.0).round() / 10.0 * (-t * 0.1).exp()
    }));
    let synth = graph.add(triangle());

    graph.add_edge(input, synth, (0, 0));
    graph.add_edge(synth, envelope, (0, 0));

    let net = NodeNetwork::from_graph(graph, input, envelope);

    let net2 = net.clone();
    thread::spawn(|| {
        playback(Box::new(net2), Duration::from_secs_f32(30.0)).unwrap();
    });
    render(Box::new(net), Duration::from_secs(30));
}
