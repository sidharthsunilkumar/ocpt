use crate::types::{CutSuggestionsList, Edge, Graph, Node, ProcessForest};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use serde::de::DeserializeOwned;

pub fn dfg_to_json(dfg: &HashMap<(String, String), usize>) -> serde_json::Value {
    let mut seen_nodes: HashSet<String> = HashSet::new();
    let mut nodes: Vec<Node> = Vec::new();
    let mut edges: Vec<Edge> = Vec::new();

    for ((source, target), cost) in dfg.iter() {
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
            cost: *cost
        });
    }

    let graph = Graph { nodes, edges };
    serde_json::to_value(graph).unwrap()
}

// Convert ProcessForest to JSON 
pub fn process_forest_to_json(forest: &ProcessForest) -> serde_json::Value {
    serde_json::to_value(forest).unwrap()
}

// Convert JSON to DFG
pub fn json_to_dfg(json_val: &serde_json::Value) -> HashMap<(String, String), usize> {
    let graph: Graph = from_json_value(json_val);
    let mut dfg = HashMap::new();

    for edge in graph.edges {
        dfg.insert(
            (edge.source, edge.target),
            edge.cost
        );
    }

    dfg
}

// Convert JSON to ProcessForest
pub fn json_to_process_forest(json_string: &str) -> ProcessForest {
    serde_json::from_str(json_string).unwrap()
}

// Convert JSON to Objects
pub fn from_json_value<T: DeserializeOwned>(val: &serde_json::Value) -> T {
    serde_json::from_value(val.clone()).expect("Failed to parse JSON value")
}
