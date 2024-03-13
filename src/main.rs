use std::{rc::Rc, time::Duration};

use midly::Track;
use petgraph::Graph;
use soundmaker::*;
use typenum::*;

fn main() {
    let score = build_piece();
    let smf = score.to_midi();

    let sample_rate = 48000.0;

    let i1 = midi_adsr_envelope(
        Box::new(FM::new(4.0, 0.5)),
        smf.tracks[0].clone(),
        (0.1, 0.6, 0.0, 0.3),
        0.3,
    );
    let i2 = midi_adsr_envelope(
        Box::new(WaveSynth::new(square_table())),
        smf.tracks[1].clone(),
        (0.1, 2.0, 0.8, 0.6),
        0.3,
    );
    let i3 = midi_adsr_envelope(
        // Box::new(WaveSynth::new(organ_table())),
        Box::new(FM::new(3.0, 1.0)),
        smf.tracks[2].clone(),
        (0.01, 0.3, 0.0, 0.0),
        0.7,
    );

    render(vec![i1, i2, i3], sample_rate, Duration::from_secs(30));

    smf.save("output/test.mid").unwrap();
}

fn build_piece() -> Score<3> {
    let key: Rc<Key> = Rc::new(Key::new(0, true));

    let mut bars = Vec::new();
    bars.push(Bar::new(4, 140.0, key.clone(), Dynamic::Forte));
    bars.push(Bar::new(4, 140.0, key.clone(), Dynamic::Forte));
    bars.push(Bar::new(4, 140.0, key.clone(), Dynamic::Forte));
    bars.push(Bar::new(4, 140.0, key.clone(), Dynamic::Forte));

    bars[0].add_note(0, 0, Note::new(240, 0, 4, None));
    bars[0].add_note(0, 240, Note::new(240, 2, 4, None));
    bars[0].add_note(0, 480, Note::new(240, 4, 4, None));
    bars[0].add_note(0, 720, Note::new(240, 6, 4, None));
    bars[0].add_note(0, 960, Note::new(240, 0, 5, None));
    bars[0].add_note(0, 960 + 240, Note::new(240, 6, 4, None));
    bars[0].add_note(0, 960 + 480, Note::new(240, 4, 4, None));
    bars[0].add_note(0, 960 + 720, Note::new(240, 2, 4, None));

    bars[1].add_note(0, 0, Note::new(240, 1, 4, None));
    bars[1].add_note(0, 240, Note::new(240, 3, 4, None));
    bars[1].add_note(0, 480, Note::new(240, 5, 4, None));
    bars[1].add_note(0, 720, Note::new(240, 0, 5, None));
    bars[1].add_note(0, 960, Note::new(240, 2, 5, None));
    bars[1].add_note(0, 960 + 240, Note::new(240, 0, 5, None));
    bars[1].add_note(0, 960 + 480, Note::new(240, 5, 4, None));
    bars[1].add_note(0, 960 + 720, Note::new(240, 3, 4, None));

    bars[2] = bars[0].clone();
    bars[3] = bars[1].clone();

    bars[0].add_note(1, 0, Note::new(480 * 3, 0, 4, None));
    bars[1].add_note(1, 0, Note::new(480 * 3, 1, 4, None));
    bars[2].add_note(1, 0, Note::new(480 * 3, 2, 4, None));
    bars[3].add_note(1, 0, Note::new(480 * 3, 1, 4, None));

    for i in 0..4 {
        bars[0].add_note(2, 480 * i, Note::new(480, 0, 3, None));
        bars[1].add_note(2, 480 * i, Note::new(480, 1, 3, None));
        bars[2].add_note(2, 480 * i, Note::new(480, 0, 3, None));
        bars[3].add_note(2, 480 * i, Note::new(480, 1, 3, None));
    }

    let section = Section::from_bars(bars);
    Score::from_sections(vec![
        section.clone(),
        section.clone(),
        section.clone(),
        section,
    ])
}

fn midi_adsr_envelope(
    synth: Box<dyn AudioNode>,
    track: Track,
    adsr: (f64, f64, f64, f64),
    volume: f64,
) -> Box<dyn AudioNode> {
    let mut graph: Graph<Box<dyn AudioNode>, _> = Graph::new();

    let player = graph.add(MidiPlayer::new(track));

    let node = graph.add_node(synth);
    graph.add_edge(player, node, (0, 0));

    let adsr = graph.add(ADSR::new(adsr.0, adsr.1, adsr.2, adsr.3));
    graph.add_edge(player, adsr, (2, 0));

    let vol = graph.add(Const(volume));
    let mix = graph.add(Mix::<U4>::mul());
    graph.add_edge(node, mix, (0, 0));
    graph.add_edge(player, mix, (1, 1));
    graph.add_edge(adsr, mix, (0, 2));
    graph.add_edge(vol, mix, (0, 3));

    let net = NodeGraph::from_graph(graph, player, mix);
    Box::new(net)
}
