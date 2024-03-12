use std::{thread, time::Duration};

use fundsp::prelude::*;
use petgraph::Graph;
use soundmaker::{
    graph::NodeGraph,
    node::{AudioNode, Const, Envelope, Envelope0, GraphExtensions, Mix},
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

    let freq = graph.add(Envelope0::new(|t| 110.0 * (-t * 0.0).exp()));
    let envelope = graph.add(Envelope::new(|t, x: &Frame<f64, U1>| {
        x[0] * (-t * 0.02).exp() * (t * 20.0).min(1.0)
    }));

    let vibrato = graph.add(Envelope::new(|t, x: &Frame<f64, U1>| {
        let a = (t * 0.3).clamp(0.0, 1.0);
        let ease = 1.0 - (1.0 - a) * (1.0 - a);
        let factor = 1.0 + (t * PI * 2.0 * 5.0).sin() * 0.003 * ease;
        x[0] * factor
    }));

    let synth = graph.add(sine() * 0.5);
    let synth2 = graph.add(sine() * 0.5);

    let mix = graph.add(Mix::<U2>::new());

    graph.add_edge(freq, vibrato, (0, 0));

    graph.add_edge(vibrato, synth, (0, 0));
    graph.add_edge(vibrato, synth2, (0, 0));

    graph.add_edge(synth, mix, (0, 0));
    graph.add_edge(synth2, mix, (0, 1));

    graph.add_edge(mix, envelope, (0, 0));

    let net = NodeGraph::from_graph(graph, freq, envelope);

    let net2 = net.clone();
    thread::spawn(|| {
        playback(Box::new(net2), Duration::from_secs_f32(30.0)).unwrap();
    });
    render(Box::new(net), Duration::from_secs(30));
}
