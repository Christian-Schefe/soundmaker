use std::time::Duration;

use midly::MetaMessage;
use midly::MidiMessage;
use midly::Smf;
use midly::Track;
use midly::TrackEventKind;

#[derive(Clone)]
pub struct MidiWrapper {
    midi: Vec<MidiMsg>,
    msg_index: usize,
    current_tempo: (u32, f64, f64),
}

impl MidiWrapper {
    pub fn new(midi: Vec<MidiMsg>) -> Self {
        Self {
            midi,
            msg_index: 0,
            current_tempo: (0, 0.0, 480.0),
        }
    }
    fn consume_message(
        &mut self,
        msg: MidiMsg,
        ticks: u32,
        time: f64,
        dropped_notes: &mut Vec<u8>,
        new_notes: &mut Vec<(u8, f64)>,
    ) {
        // println!("consume: {msg:?}");
        match msg.kind {
            MsgType::NoteOn(pitch, vel) => {
                new_notes.push((pitch, vel as f64 / 127.0));
            }
            MsgType::NoteOff(pitch) => {
                dropped_notes.push(pitch);
            }
            MsgType::Tempo(tempo) => {
                self.current_tempo = (ticks, time, tempo);
            }
        }
    }
    pub fn tick(&mut self, time: f64) -> (Vec<u8>, Vec<(u8, f64)>) {
        let time_in_tempo = time - self.current_tempo.1;
        let ticks_in_tempo = (time_in_tempo * self.current_tempo.2) as u32;
        let ticks = self.current_tempo.0 + ticks_in_tempo;

        let mut dropped_notes = Vec::new();
        let mut new_notes = Vec::new();

        while let Some(msg) = self.midi.get(self.msg_index) {
            if ticks >= msg.abs_ticks {
                self.consume_message(*msg, ticks, time, &mut dropped_notes, &mut new_notes);
                self.msg_index += 1;
            } else {
                break;
            }
        }

        (dropped_notes, new_notes)
    }
    pub fn reset(&mut self) {
        self.msg_index = 0;
        self.current_tempo = (0, 0.0, 480.0);
    }
}

#[derive(Clone, Debug, Copy)]
pub struct MidiMsg {
    kind: MsgType,
    abs_ticks: u32,
}

impl MidiMsg {
    pub fn new(kind: MsgType, abs_ticks: u32) -> Self {
        Self { kind, abs_ticks }
    }

    pub fn convert_track(track: &Track) -> Vec<Self> {
        let mut vec = Vec::new();
        let mut abs_ticks = 0;
        for msg in track {
            let delta_ticks = msg.delta.as_int();
            abs_ticks += delta_ticks;
            match msg.kind {
                TrackEventKind::Midi {
                    channel: _,
                    message,
                } => match message {
                    MidiMessage::NoteOn { key, vel } if vel == 0 => {
                        vec.push(Self::new(MsgType::NoteOff(key.as_int()), abs_ticks))
                    }
                    MidiMessage::NoteOn { key, vel } => vec.push(Self::new(
                        MsgType::NoteOn(key.as_int(), vel.as_int()),
                        abs_ticks,
                    )),
                    MidiMessage::NoteOff { key, vel: _ } => {
                        vec.push(Self::new(MsgType::NoteOff(key.as_int()), abs_ticks))
                    }
                    _ => (),
                },
                TrackEventKind::Meta(MetaMessage::Tempo(tempo)) => vec.push(Self::new(
                    MsgType::Tempo(480_000_000.0 / tempo.as_int() as f64),
                    abs_ticks,
                )),
                _ => (),
            }
        }
        vec
    }

    pub fn convert_smf(midi: Smf) -> (Vec<Vec<Self>>, Duration) {
        let mut messages: Vec<Vec<Self>> = midi.tracks.iter().map(Self::convert_track).collect();
        let mut tempo_messages: Vec<Self> = messages
            .iter()
            .flat_map(|x| {
                x.iter().filter(|&x| match x.kind {
                    MsgType::Tempo(_) => true,
                    _ => false,
                })
            })
            .copied()
            .collect();
        tempo_messages.sort_by(|a, b| a.abs_ticks.cmp(&b.abs_ticks));

        for channel in messages.iter_mut() {
            channel.extend(tempo_messages.iter());
            channel.sort_by(|a, b| a.abs_ticks.cmp(&b.abs_ticks));
        }

        let last_message = messages
            .iter()
            .flatten()
            .max_by(|a, b| a.abs_ticks.cmp(&b.abs_ticks))
            .unwrap();

        let duration = Self::calc_duration(&tempo_messages, last_message.abs_ticks);

        (messages, duration)
    }

    fn calc_duration(tempo_messages: &[Self], total_ticks: u32) -> Duration {
        let mut tempo_i = 0;
        let mut current_tempo = 960.0; // 120 BPM

        let mut ticks = 0;
        let mut time = 0.0;

        while let Some(next_tempo_msg) = tempo_messages.get(tempo_i) {
            let ticks_to_tempo_change = next_tempo_msg.abs_ticks - ticks;
            let time_to_change = ticks_to_tempo_change as f64 / current_tempo;

            tempo_i += 1;
            ticks += ticks_to_tempo_change;
            time += time_to_change;
            current_tempo = if let MsgType::Tempo(t) = next_tempo_msg.kind {
                t
            } else {
                panic!("Not a tempo msg!")
            }
        }

        let ticks_to_end = total_ticks - ticks;
        let time_to_end = ticks_to_end as f64 / current_tempo;

        time += time_to_end;

        Duration::from_secs_f64(time)
    }
}

#[derive(Clone, Debug, Copy)]
pub enum MsgType {
    NoteOn(u8, u8),
    NoteOff(u8),
    Tempo(f64), //Ticks Per Second
}
