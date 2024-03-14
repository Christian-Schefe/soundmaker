use std::{f64::consts::TAU, rc::Rc, time::Duration};

use fundsp::prelude::{xerp, Frame};
use midly::{MetaMessage, Smf, Track, TrackEventKind};
use soundmaker::prelude::*;

fn main() {
    let score = build_piece();
    let smf = score.to_midi();

    let bytes = include_bytes!("../output/file.mid");
    let mut smf = Smf::parse(bytes).unwrap();

    let sample_rate = 48000.0;

    let tempo_msg = smf.tracks[0]
        .iter()
        .find(|&x| {
            if let TrackEventKind::Meta(MetaMessage::Tempo(_)) = x.kind {
                true
            } else {
                false
            }
        })
        .unwrap()
        .clone();

    for track in smf.tracks.iter_mut() {
        track.insert(0, tempo_msg.clone());
    }

    let i1 = midi_adsr_envelope(
        vibrato_synth(Box::new(FM::new(4.0, 0.5)), 0.005),
        smf.tracks[0].clone(),
        (0.1, 0.6, 0.7, 0.3),
        0.7,
    );
    let strings = Layer::new(vec![
        Box::new(WaveSynth::new(square_table())),
        Box::new(WaveSynth::new(saw_table())),
    ]);
    let i2 = midi_adsr_envelope(
        vibrato_synth(Box::new(strings.clone()), 0.002),
        smf.tracks[3].clone(),
        (0.1, 2.0, 0.8, 0.6),
        0.8,
    );
    let i3 = midi_adsr_envelope(
        vibrato_synth(Box::new(strings.clone()), 0.002),
        smf.tracks[4].clone(),
        (0.1, 2.0, 0.8, 0.6),
        0.8,
    );
    // let i4 = midi_adsr_envelope(
    //     Box::new(WaveSynth::new(square_table())),
    //     smf.tracks[3].clone(),
    //     (0.1, 2.0, 0.8, 0.6),
    //     0.8,
    // );
    // let i5 = midi_adsr_envelope(
    //     Box::new(WaveSynth::new(square_table())),
    //     smf.tracks[4].clone(),
    //     (0.1, 2.0, 0.8, 0.6),
    //     0.8,
    // );
    // let i6 = midi_adsr_envelope(
    //     Box::new(WaveSynth::new(square_table())),
    //     smf.tracks[5].clone(),
    //     (0.1, 2.0, 0.8, 0.6),
    //     0.8,
    // );

    for (track_idx, track) in smf.tracks.iter().enumerate() {
        println!("Track {}", track_idx);

        // Iterate through each event in the track
        for event in track.iter() {
            // Print the event data
            println!("{:?}", event);
        }
    }

    render(vec![i1, i2, i3], sample_rate);

    smf.save("output/test.mid").unwrap();
}

fn build_piece() -> Score<3> {
    let key: Rc<Key> = Rc::new(Key::new(10, true));

    let mut bars = Vec::new();
    bars.push(Bar::new(4, 140.0, key.clone(), Dynamic::Forte));
    bars.push(Bar::new(4, 140.0, key.clone(), Dynamic::Forte));
    bars.push(Bar::new(4, 140.0, key.clone(), Dynamic::Forte));
    bars.push(Bar::new(4, 140.0, key.clone(), Dynamic::Forte));

    bars[0].add_note(0, 0, Note::new(240, 0, 5, None));
    bars[0].add_note(0, 240, Note::new(240, 2, 5, None));
    bars[0].add_note(0, 480, Note::new(240, 4, 5, None));
    bars[0].add_note(0, 720, Note::new(240, 6, 5, None));
    bars[0].add_note(0, 960, Note::new(240, 0, 6, None));
    bars[0].add_note(0, 960 + 240, Note::new(240, 6, 5, None));
    bars[0].add_note(0, 960 + 480, Note::new(240, 4, 5, None));
    bars[0].add_note(0, 960 + 720, Note::new(240, 2, 5, None));

    bars[1].add_note(0, 0, Note::new(240, 1, 5, None));
    bars[1].add_note(0, 240, Note::new(240, 3, 5, None));
    bars[1].add_note(0, 480, Note::new(240, 5, 5, None));
    bars[1].add_note(0, 720, Note::new(240, 0, 6, None));
    bars[1].add_note(0, 960, Note::new(240, 2, 6, None));
    bars[1].add_note(0, 960 + 240, Note::new(240, 0, 6, None));
    bars[1].add_note(0, 960 + 480, Note::new(240, 5, 5, None));
    bars[1].add_note(0, 960 + 720, Note::new(240, 3, 5, None));

    bars[2] = bars[0].clone();
    bars[3] = bars[1].clone();

    bars[0].add_note(1, 0, Note::new(480 * 2, 4, 4, None));
    bars[0].add_note(1, 960 + 480, Note::new(480, 0, 4, None));
    bars[1].add_note(1, 0, Note::new(480 * 2, 5, 4, None));
    bars[1].add_note(1, 960 + 480, Note::new(480, 1, 4, None));
    bars[2].add_note(1, 0, Note::new(480 * 2, 4, 4, None));
    bars[2].add_note(1, 960 + 480, Note::new(480, 0, 4, None));
    bars[3].add_note(1, 0, Note::new(480 * 2, 3, 4, None));
    bars[3].add_note(1, 960 + 480, Note::new(480, 1, 4, None));

    for i in 0..4 {
        bars[0].add_note(2, 480 * i, Note::new(480, 0, 3, None));
        bars[1].add_note(2, 480 * i, Note::new(480, 1, 3, None));
        bars[2].add_note(2, 480 * i, Note::new(480, 2, 3, None));
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

fn vibrato_synth(synth: Box<dyn AudioNode>, strength: f64) -> Box<dyn AudioNode> {
    let vibrato =
        // Envelope::new(|t, x: &Frame<f64, U1>| x[0] * (1.0 + 0.005 * (5.0 * TAU * t).sin()));
        Envelope::new(move |t, x: &Frame<f64, U1>| x[0] * xerp(1.0 + strength, 1.0 / (1.0 + strength), (5.0 * TAU * t).sin()));
    pipline(vec![Box::new(vibrato), synth])
}

fn midi_adsr_envelope(
    synth: Box<dyn AudioNode>,
    track: Track,
    adsr: (f64, f64, f64, f64),
    volume: f64,
) -> Box<dyn AudioNode> {
    let mut graph = GraphBuilder::new();

    let player = graph.add(MidiPlayer::new(track));

    let node = graph.add_node(synth);
    graph.add_edge((player, 0), (node, 0));

    let adsr = graph.add(ADSR::new(adsr.0, adsr.1, adsr.2, adsr.3));
    graph.add_edge((player, 2), (adsr, 0));

    let vol = graph.add(Const(volume));
    let mix = graph.add(Fold::new(Box::new(Pass::<U4>::new()), |a, b| a * b));
    graph.from_0(node, mix, 0);
    graph.add_edge((player, 1), (mix, 1));
    graph.from_0(adsr, mix, 2);
    graph.from_0(vol, mix, 3);

    Box::new(graph.set_in(player).set_out(mix).build())
}
