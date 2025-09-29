
use serde::{Deserialize, Serialize};
use chrono::{DateTime, FixedOffset};
use std::collections::{HashMap, HashSet};

// OCEL 2.0 structures

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct OCEL {
    /// Event Types in OCEL
    #[serde(rename = "eventTypes")]
    pub event_types: Vec<OCELType>,
    /// Object Types in OCEL
    #[serde(rename = "objectTypes")]
    pub object_types: Vec<OCELType>,
    /// Events contained in OCEL
    #[serde(default)]
    pub events: Vec<OCELEvent>,
    /// Objects contained in OCEL
    #[serde(default)]
    pub objects: Vec<OCELObject>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
/// OCEL Event/Object Type
pub struct OCELType {
    /// Name
    pub name: String,
    /// Attributes (defining the _type_ of values)
    #[serde(default)]
    pub attributes: Vec<OCELTypeAttribute>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
/// OCEL Attribute types
pub struct OCELTypeAttribute {
    /// Name of attribute
    pub name: String,
    /// Type of attribute
    #[serde(rename = "type")]
    pub value_type: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
/// OCEL Event
pub struct OCELEvent {
    /// Event ID
    pub id: String,
    /// Event Type (referring back to the `name` of an [`OCELType`])
    #[serde(rename = "type")]
    pub event_type: String,
    /// `DateTime` when event occured
    pub time: DateTime<FixedOffset>,
    /// Event attributes
    #[serde(default)]
    pub attributes: Vec<OCELEventAttribute>,
    /// E2O (Event-to-Object) relationships
    #[serde(default)]
    pub relationships: Vec<OCELRelationship>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
/// OCEL Event Attributes
pub struct OCELEventAttribute {
    /// Name of event attribute
    pub name: String,
    /// Value of attribute
    pub value: OCELAttributeValue,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
/// OCEL Object
pub struct OCELObject {
    /// Object ID
    pub id: String,
    /// Object Type (referring back to thte `name` of an [`OCELType`])
    #[serde(rename = "type")]
    pub object_type: String,
    /// Object attributes
    #[serde(default)]
    pub attributes: Vec<OCELObjectAttribute>,
    /// O2O (Object-to-Object) relationships
    #[serde(default)]
    pub relationships: Vec<OCELRelationship>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
/// OCEL Object Attribute
///
/// Describing a named value _at a certain point in time_
pub struct OCELObjectAttribute {
    /// Name of attribute
    pub name: String,
    /// Value of attribute
    pub value: OCELAttributeValue,
    /// Time of attribute value
    #[serde(deserialize_with = "robust_timestamp_parsing")]
    pub time: DateTime<FixedOffset>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
/// OCEL Relationship (qualified; referring back to an [`OCELObject`])
pub struct OCELRelationship {
    /// ID of referenced [`OCELObject`]
    #[serde(rename = "objectId")]
    pub object_id: String,
    /// Qualifier of relationship
    pub qualifier: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
/// OCEL Attribute Values
pub enum OCELAttributeValue {
    /// `DateTime`
    Time(DateTime<FixedOffset>),
    /// Integer
    Integer(i64),
    /// Float
    Float(f64),
    /// Boolean
    Boolean(bool),
    /// String
    String(String),
    /// Placeholder for invalid values
    Null,
}

fn robust_timestamp_parsing<'de, D>(deserializer: D) -> Result<DateTime<FixedOffset>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let time: String = Deserialize::deserialize(deserializer)?;
    if let Ok(dt) = DateTime::parse_from_rfc3339(&time) {
        return Ok(dt);
    }
    if let Ok(dt) = DateTime::parse_from_rfc2822(&time) {
        return Ok(dt);
    }
    // eprintln!("Encountered weird datetime format: {:?}", time);

    // Some logs have this date: "2023-10-06 09:30:21.890421"
    // Assuming that this is UTC
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&time, "%F %T%.f") {
        return Ok(dt.and_utc().into());
    }

    // Also handle "2024-10-02T07:55:15.348555" as well as "2022-01-09T15:00:00"
    // Assuming UTC time zone
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&time, "%FT%T%.f") {
        return Ok(dt.and_utc().into());
    }

    // export_path
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&time, "%F %T UTC") {
        return Ok(dt.and_utc().into());
    }

    // Who made me do this? 🫣
    // Some logs have this date: "Mon Apr 03 2023 12:08:18 GMT+0200 (Mitteleuropäische Sommerzeit)"
    // Below ignores the first "Mon " part (%Z) parses the rest (only if "GMT") and then parses the timezone (+0200)
    // The rest of the input is ignored
    if let Ok((dt, _)) = DateTime::parse_and_remainder(&time, "%Z %b %d %Y %T GMT%z") {
        return Ok(dt);
    }
    Err(serde::de::Error::custom("Unexpected Date Format"))
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
    pub total_edges_added: Vec<(String, String, usize)>,
    pub cost_to_add_edges: serde_json::Value
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
    pub total_edges_added: Vec<(String, String, usize)>,
    pub cost_to_add_edges: serde_json::Value
}

