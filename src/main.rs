use std::{fs::File, io::Read};

use midly::Smf;
use soundmaker::{
    daw::*,
    oscilloscope::launch_app,
    prelude::{piano, violin},
};

use clap::*;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(short = 'i', long = "input")]
    midi_path: std::path::PathBuf,

    #[arg(short = 'o', long = "output")]
    save_path: std::path::PathBuf,
}

fn main() {
    let args = Args::parse();

    let mut file = File::open(args.midi_path).unwrap();

    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();

    let smf = Smf::parse(&buffer).unwrap();

    let sample_rate = 48000.0;

    let mut daw = DAW::new();

    let violin = violin();
    daw.add_instrument("Violin".to_string(), &violin, 2.5, 0.0);
    daw.add_instrument("Violoncello".to_string(), &violin, 2.5, 0.0);

    let piano = piano();
    daw.add_instrument("Piano LH".to_string(), &piano, 2.0, 0.0);
    daw.add_instrument("Piano RH".to_string(), &piano, 2.5, 0.0);

    daw.set_midi(smf);
    launch_app(daw, sample_rate, args.save_path);
}
