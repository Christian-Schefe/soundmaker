use std::rc::Rc;

use midly::{Header, MetaMessage, MidiMessage, Smf, Track, TrackEvent, TrackEventKind};

#[derive(Debug, Clone)]
pub struct Score<const N: usize> {
    sections: Vec<Section<N>>,
}

impl<const N: usize> Score<N> {
    pub fn from_sections(sections: Vec<Section<N>>) -> Self {
        Self { sections }
    }
    pub fn to_midi(&self) -> Smf {
        let mut smf = Smf::new(Header::new(
            midly::Format::SingleTrack,
            midly::Timing::Metrical(480.into()),
        ));

        for i in 0..N {
            let mut ticks = 0;
            smf.tracks.push(Vec::new());

            self.sections
                .iter()
                .for_each(|x| x.to_midi(&mut smf, i, &mut ticks));

            smf.tracks[i].sort_by(|a, b| a.delta.cmp(&b.delta));
            let mut prev = 0;
            for j in 0..smf.tracks[i].len() {
                let absolute = smf.tracks[i][j].delta.as_int();
                let delta = absolute - prev;
                smf.tracks[i][j].delta = delta.into();
                prev = absolute;
            }

            smf.tracks[i].push(TrackEvent {
                delta: 0.into(),
                kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
            });
        }

        smf
    }
}

#[derive(Debug, Clone)]
pub struct Section<const N: usize> {
    bars: Vec<Bar<N>>,
}

impl<const N: usize> Section<N> {
    pub fn from_bars(bars: Vec<Bar<N>>) -> Self {
        Self { bars }
    }
    pub fn to_midi(&self, smf: &mut Smf, track: usize, ticks: &mut u32) {
        self.bars.iter().for_each(|x| x.to_midi(smf, track, ticks))
    }
}

#[derive(Debug, Clone)]
pub struct Bar<const N: usize> {
    pub beats: u8,
    pub notes: [Vec<(u32, Note)>; N],
    pub bpm: f64,
    pub key: Rc<Key>,
    pub dynamic: Dynamic,
}

impl<const N: usize> Bar<N> {
    pub fn new(beats: u8, bpm: f64, key: Rc<Key>, dynamic: Dynamic) -> Self {
        Self {
            beats,
            notes: vec![Vec::new(); N].try_into().unwrap(),
            bpm,
            key,
            dynamic,
        }
    }
    pub fn add_note(&mut self, voice: usize, beat: u32, note: Note) {
        self.notes[voice].push((beat, note))
    }
    fn to_midi(&self, smf: &mut Smf, track: usize, ticks: &mut u32) {
        smf.tracks[track].push(TrackEvent {
            delta: (*ticks).into(),
            kind: TrackEventKind::Meta(MetaMessage::Tempo(
                ((60_000_000.0 / self.bpm) as u32).into(),
            )),
        });

        self.notes[track]
            .iter()
            .for_each(|(offset, x)| x.to_midi(&mut smf.tracks[track], *ticks + *offset, self));
        *ticks += self.beats as u32 * 480;
    }
}

#[derive(Debug, Clone)]
pub struct Note {
    pub length: u32,
    pub pitch: u8,
    pub octave: u8,
    pub accidental: Option<bool>,
}

impl Note {
    pub fn new(length: u32, pitch: u8, octave: u8, accidental: Option<bool>) -> Self {
        Self {
            length,
            pitch: pitch % 7,
            octave,
            accidental,
        }
    }
    fn to_midi<const N: usize>(&self, track: &mut Track, ticks: u32, bar: &Bar<N>) {
        let vel = bar.dynamic.velocity().into();
        let key = bar.key.midi(self).into();

        track.push(TrackEvent {
            delta: ticks.into(),
            kind: TrackEventKind::Midi {
                channel: 0.into(),
                message: MidiMessage::NoteOn { key, vel },
            },
        });
        track.push(TrackEvent {
            delta: (ticks + self.length).into(),
            kind: TrackEventKind::Midi {
                channel: 0.into(),
                message: MidiMessage::NoteOff { key, vel },
            },
        });
    }
}

#[derive(Debug, Clone)]
pub struct Key {
    tonic: i8,
    scale: [i8; 7],
}

impl Key {
    pub fn new(tonic: i8, mode: bool) -> Self {
        Self {
            tonic,
            scale: Self::gen_scale(mode),
        }
    }
    fn gen_scale(mode: bool) -> [i8; 7] {
        if mode {
            [0, 2, 4, 5, 7, 9, 11]
        } else {
            [0, 2, 3, 5, 7, 8, 11]
        }
    }
    fn midi(&self, note: &Note) -> u8 {
        let octave = (note.octave * 12) as i8;
        let offset = octave + note.accidental.map_or(0, |b| if b { 1 } else { -1 });
        (self.tonic + self.scale[note.pitch as usize] + offset) as u8
    }
}

#[derive(Debug, Clone)]
pub enum Dynamic {
    Piano,
    MezzoPiano,
    MezzoForte,
    Forte,
}

impl Dynamic {
    fn velocity(&self) -> u8 {
        match self {
            Self::Piano => 52,
            Self::MezzoPiano => 77,
            Self::MezzoForte => 102,
            Self::Forte => 127,
        }
    }
}
