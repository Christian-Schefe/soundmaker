use fundsp::prelude::midi_hz;
use fundsp::DEFAULT_SR;
use midly::MetaMessage;
use midly::MidiMessage;
use midly::Track;
use midly::TrackEventKind::*;
use typenum::U0;
use typenum::U3;

use crate::node::FixedAudioNode;

#[derive(Clone, Debug)]
pub struct MidiPlayer {
    notes: Vec<(u32, MidiMessage)>,
    time: f64,
    delta_time: f64,
    index: usize,
    currently_playing: (u8, f64, bool),
    tempos: Vec<(u32, u32)>,
    current_tempo: (u32, u32),
    tempo_index: usize,
    last_tempo_change: f64,
}

impl MidiPlayer {
    pub fn new(mut track: Track) -> Self {
        Self::to_absolute_values(&mut track);
        Self {
            notes: track
                .iter()
                .filter_map(|event| match event.kind {
                    Midi {
                        channel: _,
                        message,
                    } => Some((event.delta.as_int(), message)),
                    _ => None,
                })
                .collect(),
            time: 0.0,
            delta_time: 1.0 / DEFAULT_SR,
            index: 0,
            currently_playing: (0, 0.0, false),
            tempos: track
                .iter()
                .filter_map(|event| match event.kind {
                    Meta(MetaMessage::Tempo(tempo)) => Some((event.delta.as_int(), tempo.into())),
                    _ => None,
                })
                .collect(),
            current_tempo: (0, 1_000_000),
            tempo_index: 0,
            last_tempo_change: 0.0,
        }
    }

    fn to_absolute_values(track: &mut Track) {
        let mut acc = 0;
        track.iter_mut().for_each(|x| {
            let delta = x.delta.as_int();
            x.delta += acc.into();
            acc += delta;
        });
    }

    fn consume_message(&mut self, msg: MidiMessage) {
        match msg {
            MidiMessage::NoteOn { key, vel } => {
                if vel == 0 {
                    self.currently_playing.2 = false;
                } else {
                    self.currently_playing = (key.as_int(), 127.0 / vel.as_int() as f64, true)
                }
            }
            MidiMessage::NoteOff { key, vel: _ } => {
                if key == self.currently_playing.0 {
                    self.currently_playing.2 = false;
                }
            }
            _ => (),
        }
    }
}

impl FixedAudioNode for MidiPlayer {
    type Inputs = U0;
    type Outputs = U3;

    fn tick(&mut self, _input: &[f64], output: &mut [f64]) {
        let passed_ticks = ((self.time - self.last_tempo_change) * 480_000_000.0
            / self.current_tempo.1 as f64) as u32
            + self.current_tempo.0;

        // println!("{}", passed_ticks);
        let mut is_new_note = false;
        let mut cur = self.notes.get(self.index).copied();
        while cur.is_some_and(|x| passed_ticks >= x.0) {
            self.consume_message(cur.unwrap().1);
            self.index += 1;
            cur = self.notes.get(self.index).copied();
            is_new_note = true;
        }

        let mut cur = self.tempos.get(self.tempo_index).copied();
        while cur.is_some_and(|x| passed_ticks >= x.0) {
            self.current_tempo = self.tempos[self.tempo_index];
            self.last_tempo_change = self.time;
            self.tempo_index += 1;
            cur = self.tempos.get(self.tempo_index).copied();
        }

        output[0] = midi_hz(self.currently_playing.0 as f64);
        output[1] = self.currently_playing.1;
        output[2] = if self.currently_playing.2 && !is_new_note {
            1.0
        } else {
            -1.0
        };

        self.time += self.delta_time;
    }

    fn reset(&mut self) {
        self.time = 0.0;
        self.last_tempo_change = 0.0;
        self.currently_playing = (0, 0.0, false);
        self.current_tempo = (0, 1_000_000);
        self.index = 0;
        self.tempo_index = 0;
    }

    fn set_sample_rate(&mut self, sample_rate: f64) {
        self.delta_time = 1.0 / sample_rate;
        self.reset();
    }
}
