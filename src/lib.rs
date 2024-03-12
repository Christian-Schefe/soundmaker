pub mod graph;
pub mod midi;
pub mod node;
pub mod oscilloscope;
pub mod output;
pub mod score;
pub mod wavetable;

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use crate::{
        midi::{save_midi_file, play_midi_file},
        score::{Bar, Key, Note, Score, Section},
    };

    #[test]
    fn test() {
        let key: Rc<Key> = Rc::new(Key::new(0, true));

        let mut bars = Vec::new();
        bars.push(Bar::new(4, 121.0, key.clone(), crate::score::Dynamic::Forte));
        bars.push(Bar::new(4, 60.0, key.clone(), crate::score::Dynamic::Forte));

        bars[0].add_note(0, 0, Note::new(480, 0, 5, None));
        bars[0].add_note(0, 480, Note::new(480, 1, 5, None));
        bars[0].add_note(0, 480 * 2, Note::new(480, 2, 5, None));
        bars[0].add_note(0, 480 * 3, Note::new(480, 3, 5, None));
        bars[1].add_note(0, 0, Note::new(480, 0, 5, None));
        bars[1].add_note(0, 480, Note::new(480, 1, 5, None));
        bars[1].add_note(0, 480 * 2, Note::new(480 * 2, 3, 5, None));

        bars.push(bars[0].clone());

        let section = Section::from_bars(bars);
        let score: Score<1> = Score::from_sections(vec![section.clone(), section]);
        let smf = score.to_midi();

        save_midi_file(smf, "output/test.mid").unwrap();
        play_midi_file("output/test.mid").unwrap();
    }
}
