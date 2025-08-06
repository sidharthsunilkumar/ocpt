
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

// OCEL 2.0 structures
#[derive(Debug, Deserialize)]
pub struct OcelJson {
    #[serde(rename = "objectTypes")]
    pub object_types: Vec<ObjectType>,
    #[serde(rename = "eventTypes")]
    pub event_types: Vec<EventType>,
    pub events: Vec<Event>,
    pub objects: Vec<Object>,
}

#[derive(Debug, Deserialize)]
pub struct ObjectType {
    pub name: String,
    pub attributes: Vec<AttributeDefinition>,
}

#[derive(Debug, Deserialize)]
pub struct EventType {
    pub name: String,
    pub attributes: Vec<AttributeDefinition>,
}

#[derive(Debug, Deserialize)]
pub struct AttributeDefinition {
    pub name: String,
    #[serde(rename = "type")]
    pub attr_type: String,
}

#[derive(Debug, Deserialize)]
pub struct Event {
    pub id: String,
    #[serde(rename = "type")]
    pub activity: String,  
    pub time: String,     
    pub attributes: Option<Vec<Attribute>>,
    pub relationships: Vec<Relationship>,
}

#[derive(Debug, Deserialize)]
pub struct Object {
    pub id: String,
    #[serde(rename = "type")]
    pub object_type: String,
    pub attributes: Option<Vec<Attribute>>,
}

#[derive(Debug, Deserialize)]
pub struct Attribute {
    pub name: String,
    pub value: serde_json::Value,  // it handle both strings and numbers
    pub time: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Relationship {
    #[serde(rename = "objectId")]
    pub object_id: String,
    pub qualifier: String,
}

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
    pub edges_to_be_added: Vec<(String, String, usize)>,
    pub edges_to_be_removed: Vec<(String, String, usize)>,
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
    pub cut_suggestions_list: CutSuggestionsList,
    pub total_edges_removed: Vec<(String, String, usize)>,
    pub total_edges_added: Vec<(String, String, usize)>
}

#[derive(serde::Deserialize)]
pub struct CutSelectedAPIRequest {
    pub ocpt: serde_json::Value,
    pub dfg: serde_json::Value,
    pub start_activities: HashSet<String>,
    pub end_activities: HashSet<String>,
    pub is_perfectly_cut: bool,
    pub cut_suggestions_list: CutSuggestionsList,
    pub cut_selected: CutSuggestion,
    pub total_edges_removed: Vec<(String, String, usize)>,
    pub total_edges_added: Vec<(String, String, usize)>
}

