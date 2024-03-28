use soundmaker::prelude::*;

fn main() {
    let midi = std::fs::read("temp/Chill Beats.mid").unwrap();
    let sample_rate = find_sample_rate();
    let mut daw = DAW::new();

    let violin = violin();
    let flute = flute();

    let percussion = percussion(vec![
        Percussion::BassDrum(36, 0.4),
        Percussion::SnareDrum(38, 0.7),
        Percussion::HiHat(44, 1.0),
        Percussion::Shaker(70, 1.0),
    ]);

    daw.add_instrument("Flute".to_string(), &flute, 2.0, 0.0);
    daw.add_instrument("Percussion 1".to_string(), percussion.as_ref(), 1.0, 0.0);
    daw.add_instrument("Percussion 2".to_string(), percussion.as_ref(), 1.0, 0.0);
    daw.add_instrument("Viola".to_string(), &violin, 2.0, 0.0);
    daw.add_instrument("Cello".to_string(), &violin, 2.0, 0.0);

    daw.set_midi_bytes(&midi);

    let render = render_daw(&mut daw, sample_rate);

    render.save("output/chill_beats.bin").unwrap();
    let wave = render.master_wave(sample_rate, true);
    wave.save_wav32("output/chill_beats.wav").unwrap();

    play_wave(wave).unwrap();
}
