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

pub fn cost_to_add_edges_to_json(dfg: &HashMap<(String, String), f64>) -> serde_json::Value {
    let mut json_map = serde_json::Map::new();
    
    for ((source, target), cost) in dfg.iter() {
        let key = format!("{}:{}", source, target);
        json_map.insert(key, serde_json::Value::Number(serde_json::Number::from_f64(*cost).unwrap()));
    }
    
    serde_json::Value::Object(json_map)
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

// Convert JSON to cost_to_add_edges HashMap
pub fn json_to_cost_to_add_edges(json_val: &serde_json::Value) -> HashMap<(String, String), f64> {
    let mut dfg = HashMap::new();
    
    if let serde_json::Value::Object(map) = json_val {
        for (key, value) in map.iter() {
            if let Some(colon_pos) = key.find(':') {
                let source = key[..colon_pos].to_string();
                let target = key[colon_pos + 1..].to_string();
                
                if let serde_json::Value::Number(num) = value {
                    if let Some(cost) = num.as_f64() {
                        dfg.insert((source, target), cost);
                    }
                }
            }
        }
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
