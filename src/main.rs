use std::time::Duration;

use petgraph::Graph;
use soundmaker::{
    node::{AudioNode, Const, GraphExtensions, NodeNetwork},
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

    let input = graph.add(Const(440.0));
    let output = graph.add(synth);

    graph.add_edge(input, output, (0, 0));

    let net = NodeNetwork::from_graph(graph, input, output);

    // playback(Box::new(net), Duration::from_secs_f32(5.0)).unwrap();

    render(Box::new(net))
}
