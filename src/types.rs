
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Deserialize)]
pub struct OcelJson {
    #[serde(rename = "ocel:global-log")]
    pub global_log: serde_json::Value,
    #[serde(rename = "ocel:events")]
    pub events: HashMap<String, Event>,
    #[serde(rename = "ocel:objects")]
    pub objects: HashMap<String, Object>,
}

#[derive(Debug, Deserialize)]
pub struct Event {
    #[serde(rename = "ocel:activity")]
    pub activity: String,
    #[serde(rename = "ocel:timestamp")]
    pub timestamp: String,
    #[serde(rename = "ocel:omap")]
    pub omap: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Object {
    #[serde(rename = "ocel:type")]
    pub object_type: String,
}

// Moved TreeNode and ProcessForest definitions
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TreeNode {
    pub label: String,
    pub children: Vec<TreeNode>,
}

pub type ProcessForest = Vec<TreeNode>;


// For format conversion of DFG to be sent a JSON response
#[derive(Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub label: String,
}

#[derive(Serialize, Deserialize)]
pub struct Edge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub label: String,
    pub cost: usize, 
}

#[derive(Serialize, Deserialize)]
pub struct Graph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct CutSuggestion {
    pub cut_type: String,
    pub set1: HashSet<String>,
    pub set2: HashSet<String>,
    pub edges_to_be_added: Vec<(String, String)>,
    pub edges_to_be_removed: Vec<(String, String)>,
    pub cost_to_add_edge: usize,
    pub total_cost: usize
}
#[derive(Serialize, Deserialize, Debug)]
pub struct CutSuggestionsList {
    pub all_activities: HashSet<String>,
    pub cuts: Vec<CutSuggestion>
}

#[derive(Serialize)]
pub struct APIResponse {
    pub OCPT: serde_json::Value,
    pub dfg: serde_json::Value,
    pub start_activities: HashSet<String>,
    pub end_activities: HashSet<String>,
    pub is_perfectly_cut: bool,
    pub cut_suggestions_list: CutSuggestionsList
}

#[derive(serde::Deserialize)]
pub struct CutSelectedAPIRequest {
    pub ocpt: serde_json::Value,
    pub dfg: serde_json::Value,
    pub start_activities: HashSet<String>,
    pub end_activities: HashSet<String>,
    pub is_perfectly_cut: bool,
    pub cut_suggestions_list: CutSuggestionsList,
    pub cut_selected: CutSuggestion
}

