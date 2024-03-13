pub mod graph;
pub mod midi;
pub mod node;
pub mod oscilloscope;
pub mod output;
pub mod score;
pub mod wavetable;
pub mod fm;
pub mod wave;

pub use graph::*;
pub use midi::*;
pub use node::*;
pub use oscilloscope::*;
pub use output::*;
pub use score::*;
pub use wavetable::*;
pub use fm::*;
pub use wave::*;

#[cfg(test)]
mod tests {
    #[test]
    fn test() {

    }
}
