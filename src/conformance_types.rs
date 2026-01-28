use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;
use chrono::{DateTime, FixedOffset};

#[derive(Debug, Serialize, Deserialize)]
pub struct OCPT {
    /// The root of the object-centric process tree
    pub root: OCPTNode,
}

///
/// Node in an object-centric process tree
///
#[derive(Debug, Serialize, Deserialize)]
pub enum OCPTNode {
    /// Operator node of an object-centric process tree
    Operator(OCPTOperator),
    /// Leaf node of an object-centric process tree
    Leaf(OCPTLeaf),
}

///
/// An operator node in an object-centric process tree
///
#[derive(Debug, Serialize, Deserialize)]
pub struct OCPTOperator {
    /// The node ID
    pub uuid: Uuid,
    /// The [`OCPTOperatorType`] of the tree itself
    pub operator_type: OCPTOperatorType,
    /// The children nodes of the operator node
    pub children: Vec<OCPTNode>,
}

///
/// Operator type enum for [`OCPTOperator`]
///
#[derive(Debug, Serialize, Deserialize)]
pub enum OCPTOperatorType {
    /// Sequence operator
    Sequence,
    /// Exclusive choice operator
    ExclusiveChoice,
    /// Concurrency operator
    Concurrency,
    /// Loop operator that, if given, restricts a given number of repetitions
    Loop(Option<u32>),
}

#[derive(Debug, Serialize, Deserialize)]
///
/// A leaf in an object-centric process tree
///
pub struct OCPTLeaf {
    /// The identifier of the leaf
    pub uuid: Uuid,
    /// The silent or non-silent activity label [`OCPTLeafLabel`]
    pub activity_label: OCPTLeafLabel,
    /// The related object types of the leaf
    pub related_ob_types: HashSet<ObjectType>,
    /// The divergent object types of the leaf
    pub divergent_ob_types: HashSet<ObjectType>,
    /// The convergent object types of the leaf
    pub convergent_ob_types: HashSet<ObjectType>,
    /// The deficient object types of the leaf
    pub deficient_ob_types: HashSet<ObjectType>,
}

///
/// Leaf in an object-centric process tree
///
#[derive(Debug, Serialize, Deserialize)]
pub enum OCPTLeafLabel {
    /// Non-silent activity leaf
    Activity(EventType),
    /// Silent activity leaf
    Tau,
}

/// OCEL object type
pub type ObjectType = String;

/// OCEL event type
pub type EventType = String;



// ------------------------


///
/// Object-centric Event Log
///
/// Consists of multiple [`OCELEvent`]s and [`OCELObject`]s with corresponding event and object [`OCELType`]s
///
#[derive(Serialize, Deserialize, Debug, Clone)]
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

#[derive(Serialize, Deserialize, Debug, Clone)]
/// OCEL Event/Object Type
pub struct OCELType {
    /// Name
    pub name: String,
    /// Attributes (defining the _type_ of values)
    #[serde(default)]
    pub attributes: Vec<OCELTypeAttribute>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
/// OCEL Attribute types
pub struct OCELTypeAttribute {
    /// Name of attribute
    pub name: String,
    /// Type of attribute
    #[serde(rename = "type")]
    pub value_type: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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

#[derive(Serialize, Deserialize, Debug, Clone)]
/// OCEL Event Attributes
pub struct OCELEventAttribute {
    /// Name of event attribute
    pub name: String,
    /// Value of attribute
    pub value: OCELAttributeValue,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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

#[derive(Serialize, Deserialize, Debug, Clone)]
/// OCEL Relationship (qualified; referring back to an [`OCELObject`])
pub struct OCELRelationship {
    /// ID of referenced [`OCELObject`]
    #[serde(rename = "objectId")]
    pub object_id: String,
    /// Qualifier of relationship
    pub qualifier: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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

#[derive(Serialize, Deserialize, Debug, Clone)]
/// OCEL Object Attribute
///
/// Describing a named value _at a certain point in time_
pub struct OCELObjectAttribute {
    /// Name of attribute
    pub name: String,
    /// Value of attribute
    pub value: OCELAttributeValue,
    /// Time of attribute value
    pub time: DateTime<FixedOffset>,
}

