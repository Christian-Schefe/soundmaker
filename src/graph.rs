use std::collections::{HashMap, VecDeque};

use crate::node::AudioNode;
use petgraph::{stable_graph::NodeIndex, *};

#[derive(Clone)]
pub struct NodeGraph {
    inputs: usize,
    outputs: usize,
    source: NodeIndex,
    sink: NodeIndex,
    graph: Graph<Box<dyn AudioNode>, (usize, usize)>,
    execution_order: Vec<NodeIndex>,
}

impl NodeGraph {
    pub fn from_graph(
        graph: Graph<Box<dyn AudioNode>, (usize, usize)>,
        source: NodeIndex,
        sink: NodeIndex,
    ) -> Self {
        let inputs = graph[source].inputs();
        let outputs = graph[sink].outputs();
        let node_count = graph.node_count();
        let mut x = Self {
            graph,
            inputs,
            outputs,
            source,
            sink,
            execution_order: Vec::with_capacity(node_count),
        };
        x.update_execution_order();
        x
    }

    pub fn update_execution_order(&mut self) {
        let mut weights = HashMap::new();

        let mut q = VecDeque::with_capacity(self.graph.node_count());
        q.push_back(self.sink);

        while let Some(cur) = q.pop_front() {
            let incoming = self.graph.neighbors_directed(cur, Direction::Incoming);
            let mut weight = Some(0);
            for n in incoming {
                if weights.contains_key(&n) {
                    weight = weight.map(|x| x.max(weights[&n] + 1));
                } else {
                    weight = None;
                    q.push_back(n)
                }
            }

            if weight.is_none() {
                q.push_back(cur);
                continue;
            }

            weights.insert(cur, weight.unwrap());

            let outgoing = self.graph.neighbors_directed(cur, Direction::Outgoing);
            outgoing.for_each(|x| q.push_back(x));
        }

        println!("{:?}", weights);
        let mut order: Vec<(NodeIndex, usize)> = weights.into_iter().collect();
        order.sort_by(|a, b| a.1.cmp(&b.1));
        self.execution_order = order.into_iter().map(|x| x.0).collect();

        println!("{:?}", self.execution_order);
    }
}

impl AudioNode for NodeGraph {
    fn tick(&mut self, input: &[f64], output: &mut [f64]) {
        let mut output_cache: HashMap<NodeIndex, Vec<f64>> =
            HashMap::with_capacity(self.graph.node_count());

        for &node in self.execution_order.iter() {
            let node_box = &self.graph[node];
            let mut out_buffer = vec![0.0; node_box.outputs()];
            
            if node == self.source {
                self.graph[node].tick(input, &mut out_buffer);
            } else {
                let mut in_buffer = vec![0.0; node_box.inputs()];

                let dependencies = self.graph.neighbors_directed(node, Direction::Incoming);

                for neighbour in dependencies {
                    let data: &[f64] = &output_cache[&neighbour];
                    let edges = self.graph.edges_connecting(neighbour, node);
                    for edge in edges {
                        let (source_i, target_i) = edge.weight();
                        in_buffer[*target_i] = data[*source_i];
                    }
                }
                self.graph[node].tick(&in_buffer, &mut out_buffer);
            }

            output_cache.insert(node, out_buffer);
        }

        output.copy_from_slice(&output_cache[&self.sink]);
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
