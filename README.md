# soundmaker

## Description

This project utilizes a DAW struct based on the [FunDSP](https://github.com/SamiPerttu/fundsp) crate. It enables MIDI file parsing via the [midly](https://crates.io/crates/midly) crate and offers playback via [cpal](https://github.com/RustAudio/cpal) and an oscilloscope display using [piston-window](https://github.com/PistonDevelopers/piston_window).

## Disclaimer - Assets (found in git history)

The MIDI files in the `./assets` folder (removed, but present in git history) contain both released and unreleased music that I composed. All rights are reserved, and distribution, modification, commercial use, or any other form of unauthorized usage of these MIDI files is expressly prohibited without explicit written permission from the author. If you want to use them for anything unrelated to this project, just ask.

## Installation

1. Clone the repository:

```bash
git clone https://github.com/Christian-Schefe/soundmaker.git
```

2. Navigate to the project directory:

```bash
cd soundmaker
```

3. Build and run:

```bash
cargo run -r -- -i "<path to midi file (.mid)>" -o "<path to output file (.wav)>"
```
