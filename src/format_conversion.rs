use crate::types::{Graph, Node, Edge};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

pub fn dfg_to_json(dfg: &HashMap<(String, String), usize>) -> Value {
    let mut seen_nodes: HashSet<String> = HashSet::new();
    let mut nodes: Vec<Node> = Vec::new();
    let mut edges: Vec<Edge> = Vec::new();

    for ((source, target), _) in dfg.iter() {
        if seen_nodes.insert(source.clone()) {
            nodes.push(Node {
                id: source.clone(),
                label: source.clone(),
            });
        }

        if seen_nodes.insert(target.clone()) {
            nodes.push(Node {
                id: target.clone(),
                label: target.clone(),
            });
        }

        edges.push(Edge {
            id: format!("{}->{}", source, target),
            source: source.clone(),
            target: target.clone(),
            label: format!("Edge {}-{}", source, target),
        });
    }

    let graph = Graph { nodes, edges };
    serde_json::to_value(graph).unwrap()
}