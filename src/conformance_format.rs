use std::collections::{HashMap, HashSet};
use crate::types::{OCELEvent, OCELObject};
use crate::conformance_types::{OCEL, OCELType, OCPT, OCPTNode, OCPTOperator, OCPTOperatorType, OCPTLeaf, OCPTLeafLabel};
use std::fs::File;
use std::io::{Write, Read};
use serde::Deserialize;
use uuid::Uuid;



pub fn build_ocel_format_for_conformance(
    events: &Vec<OCELEvent>,
    objects: &Vec<OCELObject>,
    file_name: &str
) -> OCEL {

    // Create a HashMap for quick object lookup
    let object_map: HashMap<String, &OCELObject> = objects.iter()
        .map(|obj| (obj.id.clone(), obj))
        .collect();

    let mut new_events = Vec::new();
    let mut event_types_names = HashSet::new();

    for event in events {
        let mut valid_relationships = Vec::new();
        for relationship in &event.relationships {
            if object_map.contains_key(&relationship.object_id) {
                valid_relationships.push(relationship.clone());
            }
        }
        
        let mut new_event = event.clone();
        new_event.relationships = valid_relationships;
        new_events.push(new_event);
        
        event_types_names.insert(event.event_type.clone());
    }

    let mut object_types_names = HashSet::new();
    for obj in objects {
        object_types_names.insert(obj.object_type.clone());
    }

    let event_types: Vec<OCELType> = event_types_names.into_iter()
        .map(|name| OCELType { name, attributes: Vec::new() })
        .collect();

    let object_types: Vec<OCELType> = object_types_names.into_iter()
        .map(|name| OCELType { name, attributes: Vec::new() })
        .collect();

    // Convert types to conformance types
    let new_events_json = serde_json::to_value(&new_events).expect("Failed to serialize events");
    let converted_events: Vec<crate::conformance_types::OCELEvent> = serde_json::from_value(new_events_json).expect("Failed to convert events");
    
    let objects_json = serde_json::to_value(&objects).expect("Failed to serialize objects");
    let converted_objects: Vec<crate::conformance_types::OCELObject> = serde_json::from_value(objects_json).expect("Failed to convert objects");

    let ocel = OCEL {
        event_types,
        object_types,
        events: converted_events,
        objects: converted_objects,
    };

    let json_string = serde_json::to_string(&ocel).expect("Failed to serialize OCEL to JSON");
    let file_path = format!("conformance_files/{}-ocel-data.json", file_name);
    
    // Ensure the directory exists
    if let Some(parent) = std::path::Path::new(&file_path).parent() {
        std::fs::create_dir_all(parent).expect("Failed to create directory");
    }

    let mut file = File::create(file_path).expect("Failed to create file");
    file.write_all(json_string.as_bytes()).expect("Failed to write to file");

    ocel
}

pub fn saveInteractionPatterns(
    div: &HashMap<String, Vec<String>>,
    con: &HashMap<String, Vec<String>>,
    rel: &HashMap<String, Vec<String>>,
    defi: &HashMap<String, Vec<String>>,
    file_name: &str
) {
    let json_data = serde_json::json!({
        "divergent": div,
        "convergent": con,
        "relational": rel,
        "deficient": defi
    });

    let json_string = serde_json::to_string_pretty(&json_data).expect("Failed to serialize interaction patterns");
    let file_path = format!("conformance_files/{}-ip-data.json", file_name);

    if let Some(parent) = std::path::Path::new(&file_path).parent() {
        std::fs::create_dir_all(parent).expect("Failed to create directory");
    }

    let mut file = File::create(file_path).expect("Failed to create file");
    file.write_all(json_string.as_bytes()).expect("Failed to write to file");
}

pub fn build_ocpt_format_for_conformance(file_name: &str) -> OCPT {
    
    // Read IP data first
    let ip_file_path = format!("conformance_files/{}-ip-data.json", file_name);
    let mut ip_file = File::open(&ip_file_path).expect(&format!("Failed to open file: {}", ip_file_path));
    let mut ip_contents = String::new();
    ip_file.read_to_string(&mut ip_contents).expect("Failed to read IP file");
    let interaction_patterns: InteractionPatterns = serde_json::from_str(&ip_contents).expect("Failed to parse IP JSON");

    let file_path = format!("conformance_files/{}-ocpt-data.json", file_name);
    let mut file = File::open(&file_path).expect(&format!("Failed to open file: {}", file_path));
    let mut contents = String::new();
    file.read_to_string(&mut contents).expect("Failed to read file");

    let nodes: Vec<InputNode> = serde_json::from_str(&contents).expect("Failed to parse JSON");
    
    if nodes.is_empty() {
        panic!("JSON array is empty");
    }

    // The first node in the array is the root, pass interaction patterns
    let root_node = convert_node(nodes.into_iter().next().unwrap(), &interaction_patterns);

    let ocpt = OCPT {
        root: root_node,
    };

    let json_string = serde_json::to_string_pretty(&ocpt).expect("Failed to serialize OCPT to JSON");
    let output_file_path = format!("conformance_files/{}-ocpt-conformance-data.json", file_name);
    let mut output_file = File::create(output_file_path).expect("Failed to create output file");
    output_file.write_all(json_string.as_bytes()).expect("Failed to write to output file");

    ocpt
}

pub fn build_ocpt_format_for_conformance_from_json(file_name: &str) -> OCPT {
    let file_path = format!("conformance_files/{}-ocpt-conformance-data.json", file_name);
    let mut file = File::open(&file_path).expect(&format!("Failed to open file: {}", file_path));
    let mut contents = String::new();
    file.read_to_string(&mut contents).expect("Failed to read file");

    let ocpt: OCPT = serde_json::from_str(&contents).expect("Failed to parse OCPT JSON");
    ocpt
}

pub fn build_ocel_format_for_conformance_from_json(file_name: &str) -> OCEL {
    let file_path = format!("conformance_files/{}-ocel-data.json", file_name);
    let mut file = File::open(&file_path).expect(&format!("Failed to open file: {}", file_path));
    let mut contents = String::new();
    file.read_to_string(&mut contents).expect("Failed to read file");

    let ocel: OCEL = serde_json::from_str(&contents).expect("Failed to parse OCEL JSON");
    ocel
}

#[derive(Deserialize)]
struct InputNode {
    id: String,
    label: String,
    #[serde(default)]
    children: Vec<InputNode>,
}

#[derive(Deserialize)]
struct InteractionPatterns {
    divergent: HashMap<String, Vec<String>>,
    convergent: HashMap<String, Vec<String>>,
    relational: HashMap<String, Vec<String>>,
    deficient: HashMap<String, Vec<String>>,
}

fn convert_node(node: InputNode, patterns: &InteractionPatterns) -> OCPTNode {
    let InputNode { id, label, children } = node;
    // Use the provided ID if valid, otherwise generate a new one
    let uuid = Uuid::parse_str(&id).unwrap_or_else(|_| Uuid::new_v4());

    match label.as_str() {
        "sequence" => {
            let children = children.into_iter().map(|c| convert_node(c, patterns)).collect();
            OCPTNode::Operator(OCPTOperator {
                uuid,
                operator_type: OCPTOperatorType::Sequence,
                children,
            })
        },
        "exclusive" => {
            let children = children.into_iter().map(|c| convert_node(c, patterns)).collect();
            OCPTNode::Operator(OCPTOperator {
                uuid,
                operator_type: OCPTOperatorType::ExclusiveChoice,
                children,
            })
        },
        "parallel" => {
            let children = children.into_iter().map(|c| convert_node(c, patterns)).collect();
            OCPTNode::Operator(OCPTOperator {
                uuid,
                operator_type: OCPTOperatorType::Concurrency,
                children,
            })
        },
        "redo" => {
            let children = children.into_iter().map(|c| convert_node(c, patterns)).collect();
            OCPTNode::Operator(OCPTOperator {
                uuid,
                operator_type: OCPTOperatorType::Loop(None),
                children,
            })
        },
        label_str => {
             // For leaf, children should be empty usually
             let activity_label = if label_str.eq_ignore_ascii_case("tau") {
                 OCPTLeafLabel::Tau
             } else {
                 OCPTLeafLabel::Activity(label_str.to_string())
             };
             
             let (related, divergent, convergent, deficient) = if let OCPTLeafLabel::Activity(act_name) = &activity_label {
                 (
                     patterns.relational.get(act_name).cloned().unwrap_or_default().into_iter().collect(),
                     patterns.divergent.get(act_name).cloned().unwrap_or_default().into_iter().collect(),
                     patterns.convergent.get(act_name).cloned().unwrap_or_default().into_iter().collect(),
                     patterns.deficient.get(act_name).cloned().unwrap_or_default().into_iter().collect(),
                 )
             } else {
                 (HashSet::new(), HashSet::new(), HashSet::new(), HashSet::new())
             };

             OCPTNode::Leaf(OCPTLeaf {
                 uuid,
                 activity_label,
                 related_ob_types: related,
                 divergent_ob_types: divergent,
                 convergent_ob_types: convergent,
                 deficient_ob_types: deficient,
             })
        }
    }
}