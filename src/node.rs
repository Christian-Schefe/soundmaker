use fundsp::prelude::{Frame, Size};
use petgraph::{prelude::*, stable_graph::IndexType, EdgeType};
use typenum::{U0, U1};

pub trait AudioNode: Send + Sync {
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

pub trait FixedAudioNode: Send + Sync {
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

impl<I: Size<f64>, O: Size<f64>, T> FixedAudioNode for T
where
    T: fundsp::prelude::AudioNode<Sample = f64, Inputs = I, Outputs = O>,
{
    type Inputs = I;
    type Outputs = I;
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

pub struct NodeNetwork {
    inputs: usize,
    outputs: usize,
    input: NodeIndex,
    output: NodeIndex,
    graph: Graph<Box<dyn AudioNode>, (usize, usize)>,
}

impl NodeNetwork {
    pub fn wrap(node: Box<dyn AudioNode>) -> Self {
        let mut graph = Graph::with_capacity(1, 0);
        let inputs = node.inputs();
        let outputs = node.outputs();
        let input = graph.add_node(node);
        let output = input;

        Self {
            graph,
            inputs,
            outputs,
            input,
            output,
        }
    }
    pub fn process(&mut self, node: NodeIndex, global: &[f64]) -> Vec<f64> {
        let cur_node = &mut self.graph[node];
        let inputs = cur_node.inputs();
        let outputs = cur_node.outputs();
        let mut input = vec![0.0; inputs];
        let mut output = vec![0.0; outputs];

        if node == self.input {
            self.graph[node].tick(global, output.as_mut_slice());

            return output;
        }

        let neighbours: Vec<NodeIndex> = self
            .graph
            .neighbors_directed(node, Direction::Incoming)
            .collect();
        for n in neighbours {
            let vals = self.process(n, global);
            let edges = self.graph.edges_connecting(n, node);

            for edge in edges {
                let weight = edge.weight();
                input[weight.1] = vals[weight.0];
            }
        }

        self.graph[node].tick(input.as_slice(), output.as_mut_slice());

        output
    }
    pub fn add_boxed(&mut self, node: Box<dyn AudioNode>) {
        self.graph.add_node(node);
    }
    pub fn add_node<T>(&mut self, node: T)
    where
        T: AudioNode + 'static,
    {
        self.graph.add_node(Box::new(node));
    }
    pub fn from_graph(
        graph: Graph<Box<dyn AudioNode>, (usize, usize)>,
        input: NodeIndex,
        output: NodeIndex,
    ) -> Self {
        let inputs = graph[input].inputs();
        let outputs = graph[output].outputs();
        Self {
            graph,
            inputs,
            outputs,
            input,
            output,
        }
    }
}

impl AudioNode for NodeNetwork {
    fn tick(&mut self, input: &[f64], output: &mut [f64]) {
        let vec = self.process(self.output, input);
        output.copy_from_slice(vec.as_slice())
    }

    fn inputs(&self) -> usize {
        self.inputs
    }

    fn outputs(&self) -> usize {
        self.outputs
    }

    fn reset(&mut self) {
        self.graph
            .node_indices()
            .for_each(|x| self.graph[x].reset())
    }

    fn set_sample_rate(&mut self, sample_rate: f64) {
        self.graph
            .node_indices()
            .for_each(|x| self.graph[x].set_sample_rate(sample_rate))
    }
}

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
