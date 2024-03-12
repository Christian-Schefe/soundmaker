use std::marker::PhantomData;

use dyn_clone::DynClone;
use fundsp::{
    prelude::{Frame, Size},
    DEFAULT_SR,
};
use petgraph::{prelude::*, stable_graph::IndexType, EdgeType};
use typenum::{U0, U1};

pub trait AudioNode: Send + Sync + DynClone {
    fn tick(&mut self, input: &[f64], output: &mut [f64]);

    fn reset(&mut self);

    fn set_sample_rate(&mut self, _sample_rate: f64);

    fn inputs(&self) -> usize;

    fn outputs(&self) -> usize;

    fn get_stereo(&mut self) -> (f64, f64) {
        match self.outputs() {
            1 => {
                let input = [];
                let mut output = [0.0];
                self.tick(&input, &mut output);
                (output[0], output[0])
            }
            2 => {
                let input = [];
                let mut output = [0.0, 0.0];
                self.tick(&input, &mut output);
                (output[0], output[1])
            }
            _ => panic!("Invalid Output Amount"),
        }
    }
}

dyn_clone::clone_trait_object!(AudioNode);

pub trait FixedAudioNode: Send + Sync + DynClone {
    type Inputs: Size<f64>;
    type Outputs: Size<f64>;

    fn tick(&mut self, input: &[f64], output: &mut [f64]);

    fn reset(&mut self);

    fn set_sample_rate(&mut self, _sample_rate: f64);
}

impl<I: Size<f64>, O: Size<f64>, T: FixedAudioNode<Inputs = I, Outputs = O>> AudioNode for T {
    fn tick(&mut self, input: &[f64], output: &mut [f64]) {
        self.tick(input, output)
    }

    fn inputs(&self) -> usize {
        I::to_usize()
    }

    fn outputs(&self) -> usize {
        O::to_usize()
    }

    fn reset(&mut self) {
        self.reset()
    }

    fn set_sample_rate(&mut self, sample_rate: f64) {
        self.set_sample_rate(sample_rate)
    }
}

// impl<I: Size<f64>, O: Size<f64>, T> FixedAudioNode for T
// where
//     T: fundsp::prelude::AudioNode<Sample = f64, Inputs = I, Outputs = O>,
// {
//     type Inputs = I;
//     type Outputs = O;
//     fn tick(&mut self, input: &[f64], output: &mut [f64]) {
//         output.copy_from_slice(self.tick(&Frame::from_slice(input)).as_slice());
//     }

//     fn set_sample_rate(&mut self, sample_rate: f64) {
//         self.set_sample_rate(sample_rate)
//     }

//     fn reset(&mut self) {
//         self.reset()
//     }
// }

impl<I: Size<f64>, O: Size<f64>, T> FixedAudioNode for fundsp::prelude::An<T>
where
    T: fundsp::prelude::AudioNode<Sample = f64, Inputs = I, Outputs = O>,
{
    type Inputs = I;
    type Outputs = O;
    fn tick(&mut self, input: &[f64], output: &mut [f64]) {
        output.copy_from_slice(self.tick(&Frame::from_slice(input)).as_slice());
    }

    fn set_sample_rate(&mut self, sample_rate: f64) {
        self.set_sample_rate(sample_rate)
    }

    fn reset(&mut self) {
        self.reset()
    }
}

#[derive(Clone)]
pub struct Const(pub f64);

impl FixedAudioNode for Const {
    type Inputs = U0;
    type Outputs = U1;
    fn tick(&mut self, _input: &[f64], output: &mut [f64]) {
        output[0] = self.0;
    }

    fn reset(&mut self) {}

    fn set_sample_rate(&mut self, _sample_rate: f64) {}
}

#[derive(Clone)]
pub struct Envelope<F, I>
where
    F: Fn(f64, &Frame<f64, I>) -> f64 + Sync + Send + Clone,
    I: Size<f64>,
{
    envelope: F,
    time: f64,
    delta_time: f64,
    _marker: PhantomData<I>,
}

impl<F, I> Envelope<F, I>
where
    F: Fn(f64, &Frame<f64, I>) -> f64 + Sync + Send + Clone,
    I: Size<f64>,
{
    pub fn new(envelope: F) -> Self {
        Self {
            envelope,
            time: 0.0,
            delta_time: 1.0 / DEFAULT_SR,
            _marker: PhantomData,
        }
    }
}

impl<F, I> FixedAudioNode for Envelope<F, I>
where
    F: Fn(f64, &Frame<f64, I>) -> f64 + Sync + Send + Clone,
    I: Size<f64>,
{
    type Inputs = I;
    type Outputs = U1;
    fn tick(&mut self, input: &[f64], output: &mut [f64]) {
        output[0] = (self.envelope)(self.time, Frame::from_slice(input));
        self.time += self.delta_time;
    }

    fn reset(&mut self) {
        self.time = 0.0;
    }

    fn set_sample_rate(&mut self, sample_rate: f64) {
        self.time = 0.0;
        self.delta_time = 1.0 / sample_rate;
    }
}

#[derive(Clone)]
pub struct Envelope0<F>
where
    F: Fn(f64) -> f64 + Sync + Send + Clone,
{
    envelope: F,
    time: f64,
    delta_time: f64,
}

impl<F> Envelope0<F>
where
    F: Fn(f64) -> f64 + Sync + Send + Clone,
{
    pub fn new(envelope: F) -> Self {
        Self {
            envelope,
            time: 0.0,
            delta_time: 1.0 / DEFAULT_SR,
        }
    }
}

impl<F> FixedAudioNode for Envelope0<F>
where
    F: Fn(f64) -> f64 + Sync + Send + Clone,
{
    type Inputs = U0;
    type Outputs = U1;
    fn tick(&mut self, _input: &[f64], output: &mut [f64]) {
        output[0] = (self.envelope)(self.time);
        self.time += self.delta_time;
    }

    fn reset(&mut self) {
        self.time = 0.0;
    }

    fn set_sample_rate(&mut self, sample_rate: f64) {
        self.time = 0.0;
        self.delta_time = 1.0 / sample_rate;
    }
}

#[derive(Clone, Default)]
pub struct Mix<I>(PhantomData<I>)
where
    I: Size<f64>;

impl<I> Mix<I>
where
    I: Size<f64>,
{
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<I> FixedAudioNode for Mix<I>
where
    I: Size<f64>,
{
    type Inputs = I;
    type Outputs = U1;
    fn tick(&mut self, input: &[f64], output: &mut [f64]) {
        output[0] = input.iter().sum();
    }

    fn reset(&mut self) {}

    fn set_sample_rate(&mut self, _sample_rate: f64) {}
}

pub trait GraphExtensions<N, Ix> {
    fn add(&mut self, node: N) -> NodeIndex<Ix>;
}

impl<N: AudioNode + 'static, E, Ty: EdgeType, Ix: IndexType> GraphExtensions<N, Ix>
    for Graph<Box<dyn AudioNode>, E, Ty, Ix>
{
    fn add(&mut self, node: N) -> NodeIndex<Ix> {
        self.add_node(Box::new(node))
    }
}
