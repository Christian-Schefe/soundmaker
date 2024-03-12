use std::path::Path;

use midly::Smf;
use midly::TrackEventKind::*;

pub fn save_midi_file<P>(smf: Smf, path: P) -> Result<(), anyhow::Error>
where
    P: AsRef<Path>,
{
    smf.save(path)?;
    Ok(())
}

pub fn play_midi_file<P>(path: P) -> Result<(), anyhow::Error>
where
    P: AsRef<Path>,
{
    let data = std::fs::read(path)?;
    let smf = Smf::parse(&data).expect("Failed to parse MIDI file");

    for track in &smf.tracks {
        println!("Track:");

        for event in track.iter() {
            match event.kind {
                Midi { channel, message } => {
                    println!("Channel {}: {:?}", channel, message);
                }
                Meta(meta) => {
                    println!("Meta: {:?}", meta);
                }
                _ => {}
            }
        }
    }

    Ok(())
}
