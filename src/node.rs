use std::marker::PhantomData;

use crate::prelude::{U0, U1};
use dyn_clone::DynClone;
use fundsp::{
    prelude::{lerp, Frame, Size},
    DEFAULT_SR,
};

pub trait AudioNode: Send + Sync + DynClone {
    fn tick(&mut self, input: &[f64], output: &mut [f64]);

    fn reset(&mut self) {}

    fn set_sample_rate(&mut self, _sample_rate: f64) {}

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

    fn reset(&mut self) {}

    fn set_sample_rate(&mut self, _sample_rate: f64) {}
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

#[derive(Clone)]
pub struct Fold<OP>(Box<dyn AudioNode>, OP)
where
    OP: Fn(f64, f64) -> f64;

impl<OP> Fold<OP>
where
    OP: Fn(f64, f64) -> f64,
{
    pub fn new(node: Box<dyn AudioNode>, op: OP) -> Self {
        Self(node, op)
    }
}

impl<OP> AudioNode for Fold<OP>
where
    OP: Fn(f64, f64) -> f64 + Send + Sync + Clone,
{
    fn tick(&mut self, input: &[f64], output: &mut [f64]) {
        let num = self.0.outputs();
        let mut buffer = vec![0.0; num];
        self.0.tick(input, &mut buffer);
        output[0] = buffer.into_iter().reduce(|x, a| (self.1)(x, a)).unwrap();
    }

    fn inputs(&self) -> usize {
        self.0.inputs()
    }

    fn outputs(&self) -> usize {
        1
    }
}

#[derive(Clone, Default)]
pub struct Layer(Vec<Box<dyn AudioNode>>, usize, usize);

impl Layer {
    pub fn new(layers: Vec<Box<dyn AudioNode>>) -> Self {
        let inp = layers[0].inputs();
        let outp = layers[0].inputs();
        assert!(layers
            .iter()
            .find(|x| x.inputs() != inp || x.outputs() != outp)
            .is_none());
        Self(layers, inp, outp)
    }
}

impl AudioNode for Layer {
    fn tick(&mut self, input: &[f64], output: &mut [f64]) {
        let outputs = self.outputs();
        let mut acc = vec![0.0; outputs];
        for node in self.0.iter_mut() {
            node.tick(input, output);
            for i in 0..outputs {
                acc[i] += output[i];
            }
        }
        output.copy_from_slice(&acc);
    }

    fn inputs(&self) -> usize {
        self.1
    }

    fn outputs(&self) -> usize {
        self.2
    }
}

#[derive(Clone)]
pub struct ADSR {
    attack: f64,
    decay: f64,
    sustain: f64,
    release: f64,
    time: f64,
    delta_time: f64,
    last_start_time: f64,
    last_end_time: f64,
    last_input: f64,
    attack_baseline: f64,
    sustain_baseline: f64,
    last_output: f64,
}

impl ADSR {
    pub fn new(attack: f64, decay: f64, sustain: f64, release: f64) -> Self {
        Self {
            attack,
            decay,
            sustain,
            release,
            time: 0.0,
            delta_time: 1.0 / DEFAULT_SR,
            last_start_time: 0.0,
            last_end_time: 0.0,
            last_input: -1.0,
            attack_baseline: 0.0,
            sustain_baseline: sustain,
            last_output: 0.0,
        }
    }
}

impl FixedAudioNode for ADSR {
    type Inputs = U1;
    type Outputs = U1;
    fn tick(&mut self, input: &[f64], output: &mut [f64]) {
        let control = input[0];
        if self.last_input <= 0.0 && control > 0.0 {
            self.last_start_time = self.time;
            self.attack_baseline = self.last_output;
        } else if self.last_input > 0.0 && control <= 0.0 {
            self.last_end_time = self.time;
            self.sustain_baseline = self.last_output;
        }

        if control <= 0.0 {
            let time_diff = self.time - self.last_end_time;
            let alpha = if self.release > 0.0 {
                (time_diff / self.release).clamp(0.0, 1.0)
            } else {
                (time_diff / 0.01).clamp(0.0, 1.0)
            };
            output[0] = lerp(self.sustain_baseline, 0.0, alpha);
        } else {
            let time_diff = self.time - self.last_start_time;
            if time_diff < self.attack {
                let alpha = (time_diff / self.attack).clamp(0.0, 1.0);
                output[0] = lerp(self.attack_baseline, 1.0, alpha);
            } else {
                let alpha = if self.decay > 0.0 {
                    ((time_diff - self.attack) / self.decay).clamp(0.0, 1.0)
                } else {
                    1.0
                };
                output[0] = lerp(1.0, self.sustain, alpha);
            }
        }

        self.time += self.delta_time;
        self.last_input = control;
        self.last_output = output[0];
    }

    fn reset(&mut self) {
        self.time = 0.0;
        self.last_end_time = 0.0;
        self.last_start_time = 0.0;
        self.last_input = 0.0;
        self.attack_baseline = 0.0;
        self.sustain_baseline = self.sustain;
        self.last_output = 0.0;
    }

    fn set_sample_rate(&mut self, sample_rate: f64) {
        self.time = 0.0;
        self.delta_time = 1.0 / sample_rate;
    }
}

#[derive(Clone, Default)]
pub struct Pipeline(Vec<Box<dyn AudioNode>>, usize, usize);

impl Pipeline {
    pub fn new(steps: Vec<Box<dyn AudioNode>>) -> Self {
        let inputs = steps[0].inputs();
        let outputs = steps[steps.len() - 1].outputs();
        Self(steps, inputs, outputs)
    }
    pub fn wrap<T>(node: T) -> Self
    where
        T: AudioNode + 'static,
    {
        Self::new(vec![Box::new(node)])
    }
    pub fn add<T>(mut self, node: T) -> Self
    where
        T: AudioNode + 'static,
    {
        self.2 = node.outputs();
        self.0.push(Box::new(node));
        self
    }
}

impl AudioNode for Pipeline {
    fn tick(&mut self, input: &[f64], output: &mut [f64]) {
        let mut prev_output = input.to_vec();
        for i in 0..self.0.len() {
            let node = self.0.get_mut(i).unwrap();
            let mut outputs = vec![0.0; node.outputs()];
            node.tick(&prev_output, &mut outputs);
            prev_output = outputs;
        }
        output.copy_from_slice(&prev_output);
    }

    fn inputs(&self) -> usize {
        self.1
    }

    fn outputs(&self) -> usize {
        self.2
    }
}

#[derive(Clone, Default)]
pub struct Stack(Vec<Box<dyn AudioNode>>, usize, usize);

impl Stack {
    pub fn new(steps: Vec<Box<dyn AudioNode>>) -> Self {
        let inputs = steps.iter().map(|x| x.inputs()).sum();
        let outputs = steps.iter().map(|x| x.outputs()).sum();
        Self(steps, inputs, outputs)
    }
    pub fn wrap<T>(node: T) -> Self
    where
        T: AudioNode + 'static,
    {
        Self::new(vec![Box::new(node)])
    }
    pub fn add<T>(mut self, node: T) -> Self
    where
        T: AudioNode + 'static,
    {
        self.2 = node.outputs();
        self.0.push(Box::new(node));
        self
    }
}

impl AudioNode for Stack {
    fn tick(&mut self, input: &[f64], output: &mut [f64]) {
        let mut in_off = 0;
        let mut out_off = 0;
        for i in 0..self.0.len() {
            let node = self.0.get_mut(i).unwrap();
            let inp = node.inputs();
            let outp = node.outputs();

            node.tick(
                &input[in_off..in_off + inp],
                &mut output[out_off..out_off + outp],
            );

            in_off += inp;
            out_off += inp;
        }
    }

    fn inputs(&self) -> usize {
        self.1
    }

    fn outputs(&self) -> usize {
        self.2
    }
}

#[derive(Clone, Default)]
pub struct Pass<T: Size<f64>>(PhantomData<T>);

impl<T: Size<f64>> Pass<T> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<T: Size<f64>> FixedAudioNode for Pass<T> {
    type Inputs = T;
    type Outputs = T;
    fn tick(&mut self, input: &[f64], output: &mut [f64]) {
        output.copy_from_slice(&input);
    }
}

impl FixedAudioNode for f64 {
    type Inputs = U0;
    type Outputs = U1;

    fn tick(&mut self, _input: &[f64], output: &mut [f64]) {
        output[0] = *self;
    }
}

impl FixedAudioNode for f32 {
    type Inputs = U0;
    type Outputs = U1;

    fn tick(&mut self, _input: &[f64], output: &mut [f64]) {
        output[0] = *self as f64;
    }
}
