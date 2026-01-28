use process_mining::conformance::object_centric::object_centric_language_abstraction::{
    compute_fitness_precision, OCLanguageAbstraction,
};

use std::fs::File;
use std::io::{Write, Read};
use process_mining::core::event_data::object_centric::linked_ocel::IndexLinkedOCEL;
use process_mining::core::OCEL;
use process_mining::core::process_models::object_centric::ocpt::OCPT;
use crate::conformance_types::OCPT as ConformanceOCPT;

// use crate::conformance_format::{build_ocel_format_for_conformance_from_json, build_ocpt_format_for_conformance_from_json};

/// Computes fitness and precision for a given Object-Centric Process Tree (OCPT)
/// and an Object-Centric Event Log (OCEL).
pub fn calculate_model_quality(tree: &OCPT, ocel: OCEL) -> (f64, f64) {
    // 1. Preprocess the log: Remove objects that have no events linked to them.
    // This is required because the abstraction assumes a connected graph.
    let preprocessed_ocel = ocel.remove_orphan_objects();

    // 2. Convert to IndexLinkedOCEL: This struct provides optimized index access 
    // to the log data, which is necessary for creating the language abstraction.
    let locel = IndexLinkedOCEL::from_ocel(preprocessed_ocel);

    // 3. Create Language Abstractions:
    // Convert the tree model into its behavioral abstraction.
    let abstraction_tree = OCLanguageAbstraction::create_from_oc_process_tree(tree);
    // Convert the event log into its behavioral abstraction.
    let abstraction_log = OCLanguageAbstraction::create_from_ocel(&locel);

    // 4. Compute Metrics:
    // Compare the two abstractions to get fitness and precision.
    let (fitness, precision) = compute_fitness_precision(&abstraction_log, &abstraction_tree);

    (fitness, precision)
}

pub fn calculate_metrics(file_name: &str, tree: &ConformanceOCPT) -> (f64, f64) {
    println!("\n--- R4PM Library Conformance Analysis ---");
    
    let json_val = serde_json::to_value(tree).expect("Failed to serialize OCPT");
    let pm_tree: OCPT = serde_json::from_value(json_val).expect("Failed to convert OCPT");

    // let file_name = "order-management-2";
    // let file_name = "ocel2-p2p-no-se";
    // let file_name = "small-example-v1";
    // let tree = build_ocpt_format_for_conformance_from_json(file_name);
    let ocel = build_ocel_format_for_conformance_from_json(file_name);

    let (fit, pre) = calculate_model_quality(&pm_tree, ocel);

    println!("R4PM Fitness: {:.4}", fit);
    println!("R4PM Precision: {:.4}", pre);

    (fit, pre)
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
