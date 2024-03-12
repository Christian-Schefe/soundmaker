use std::{rc::Rc, thread, time::Duration};

use fundsp::prelude::*;
use fundsp::wavetable::SAW_TABLE;
use fundsp::wavetable::TRIANGLE_TABLE;
use midly::Track;
use petgraph::Graph;
use soundmaker::{
    graph::NodeGraph,
    midi::MidiPlayer,
    node::{AudioNode, GraphExtensions, Mix, ADSR},
    oscilloscope::render,
    output::playback,
    score::*,
    wavetable::{MultiWavetableSynth, Wavetable, WavetableSynth},
};

fn main() {
    // let w1 = Wavetable::new(
    //     20.0,
    //     20000.0,
    //     4.0,
    //     &|i| if (i & 1) == 1 { 0.0 } else { 0.5 },
    //     &|_, i| 1.0 / i as f64,
    // );

    // let w2 = Wavetable::new(
    //     20.0,
    //     20_000.0,
    //     4.0,
    //     &|i| if (i & 3) == 3 { 0.5 } else { 0.0 },
    //     &|_, i| {
    //         if (i & 1) == 1 {
    //             1.0 / (i * i) as f64
    //         } else {
    //             0.0
    //         }
    //     },
    // );

    let key: Rc<Key> = Rc::new(Key::new(0, true));

    let mut bars = Vec::new();
    bars.push(Bar::new(4, 120.0, key.clone(), Dynamic::Forte));
    bars.push(Bar::new(4, 60.0, key.clone(), Dynamic::Forte));

    bars[0].add_note(0, 0, Note::new(480, 0, 5, None));
    bars[0].add_note(0, 480, Note::new(480, 1, 5, None));
    bars[0].add_note(0, 480 * 2, Note::new(480, 2, 5, None));
    bars[0].add_note(0, 480 * 3, Note::new(480, 3, 5, None));
    bars[1].add_note(0, 0, Note::new(480, 0, 5, None));
    bars[1].add_note(0, 480, Note::new(480, 1, 5, None));
    bars[1].add_note(0, 480 * 2, Note::new(480 * 2, 3, 5, None));

    bars.push(bars[0].clone());

    let section = Section::from_bars(bars);
    let score: Score<1> = Score::from_sections(vec![section.clone()]);
    let smf = score.to_midi();

    let net = build_instrument(smf.tracks[0].clone());
    let net2 = net.clone();

    thread::spawn(|| {
        playback(net2, Duration::from_secs_f32(30.0)).unwrap();
    });
    render(net, Duration::from_secs(30));

    smf.save("output/test.mid").unwrap();
}

fn build_instrument(track: Track) -> Box<dyn AudioNode> {
    let w1 = Wavetable::new(
        20.0,
        20_000.0,
        4.0,
        // To build the classic triangle shape, shift every other odd partial 180 degrees.
        &|i| if (i & 3) == 3 { 0.5 } else { 0.0 },
        &|_, i| {
            if (i & 1) == 1 {
                1.0 / (i * i) as f64
            } else {
                0.0
            }
        },
    );

    let w2 = Wavetable::new(20.0, 20_000.0, 4.0, &|_| 0.0, &|_, i| {
        if (i & 1) == 1 {
            1.0 / i as f64
        } else {
            0.0
        }
    });

    let mut graph: Graph<Box<dyn AudioNode>, _> = Graph::new();

    let player = graph.add(MidiPlayer::new(track));

    let synth = graph.add(MultiWavetableSynth::new(w2, 3));
    graph.add_edge(player, synth, (0, 0));

    let adsr = graph.add(ADSR::new(0.03, 0.2, 0.8, 0.04));
    graph.add_edge(player, adsr, (2, 0));

    let mix = graph.add(Mix::<U3>::new(false));
    graph.add_edge(synth, mix, (0, 0));
    graph.add_edge(player, mix, (1, 1));
    graph.add_edge(adsr, mix, (0, 2));

    let declick = graph.add(declick::<_, f64>());
    graph.add_edge(mix, declick, (0, 0));

    let net = NodeGraph::from_graph(graph, player, declick);
    Box::new(net)
}
