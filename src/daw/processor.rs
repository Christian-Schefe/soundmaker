use dyn_clone::{clone_trait_object, DynClone};
use fundsp::prelude::*;

pub trait Processor: DynClone + Send + Sync {
    fn tick(&mut self, input: &Frame<f64, U2>) -> Frame<f64, U2>;
}

clone_trait_object!(Processor);
