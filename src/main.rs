use crate::format_conversion::{from_json_value, json_to_dfg, json_to_process_forest, process_forest_to_json};
use crate::types::{APIResponse, CutSelectedAPIRequest, CutSuggestion, CutSuggestionsList, Event, Object, OcelJson, ProcessForest, TreeNode};
use serde::Deserialize;
use simplelog::*;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs as stdfs;
use std::fs::File;
use std::io::Write;
use std::sync::Arc;
mod build_relations_fns;
mod divergence_free_dfg;
mod format_conversion;
mod interaction_patterns;
mod start_cuts;
mod start_cuts_opti_v1;
mod start_cuts_opti_v2;
mod types;
use log::info;
mod best_exclusive_cut;
mod best_parallel_cut;
mod best_parallel_cut_exhaustive;
mod best_parallel_cut_v2;
mod best_redo_cuts;
mod best_sequence_cut;
mod best_sequence_cut_v2;
mod cost_to_add;
mod cost_to_cut;
mod good_cuts;
use axum::extract::Json as AxumJson;
use axum::http::StatusCode;

//For REST API server
use axum::Json;
use tokio::fs as tokiofs;
// use axum::{
//     routing::{get, post},
//     Router,
// };
use axum::{Router, response::Html, routing::get};
use tower_http::cors::{Any, CorsLayer};
// use tower::ServiceExt;
// use axum::http::{HeaderValue, Method};
// use tower_http::cors::CorsLayer;
use serde_json::Value;

// GET / — serves the content of dfg.json as JSON
async fn hello() -> Json<Value> {
    let file_content = tokiofs::read_to_string("data/order_management_tree.json")
        .await
        .expect("Failed to read dfg.json");
    let json: Value =
        serde_json::from_str(&file_content).expect("Failed to parse JSON from dfg.json");

    Json(json)
}

#[tokio::main]
async fn main23() {
    // Configure CORS
    let cors = CorsLayer::new()
        .allow_methods(Any)
        .allow_origin(Any)
        .allow_headers(Any);

    let app = Router::new().route("/", get(hello)).layer(cors);

    println!("Server running on http://localhost:1080");
    println!("GET  / - Hello World");
    println!("POST /print - Prints body to console");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:1080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn getInitialResponse() -> Json<Value> {

    println!("Starting...");

    let file_path = "data/github_pm4py.jsonocel";
    // let file_path = "data/o2c.jsonocel";

    let file_content = stdfs::read_to_string(&file_path).unwrap();
    let ocel: OcelJson = serde_json::from_str(&file_content).unwrap();

    let relations = build_relations_fns::build_relations(&ocel.events, &ocel.objects);
    // println!("size of relations: {}", relations.len());

    let (div, con, rel, defi, all_activities, all_object_types) =
        interaction_patterns::get_interaction_patterns(&relations, &ocel);

    // println!("Divergent: {:?}",div);
    // println!("Convergent: {:?}",con);
    // println!("Relational: {:?}",rel);
    // println!("Deficiency {:?}",defi);
    // log_sorted_map("Divergent", &div);
    // log_sorted_map("Convergent", &con);
    // log_sorted_map("Relational", &rel);
    // log_sorted_map("Deficiency", &defi);

    let (dfg, start_acts, end_acts) =
        divergence_free_dfg::get_divergence_free_graph_v2(&relations, &div);

    println!("created DFG!");

    print_dfg(&dfg);

    // let remove_list = vec![];
    // let remove_list = vec!["failed delivery".to_string(),"payment reminder".to_string()];
    let remove_list = vec!["reopened".to_string()];
    let filtered_dfg = filter_dfg(&dfg, &remove_list);
    let filtered_activities = filter_activities(&all_activities, &remove_list);


    let json_dfg: Value = format_conversion::dfg_to_json(&dfg);

    let start_time = std::time::Instant::now();

    // In case of filtering activities in the begining
    // let process_forest = start_cuts::find_cuts(&filtered_dfg, &filtered_dfg, filtered_activities, &start_acts, &end_acts);
    let process_forest = start_cuts_opti_v2::find_cuts_start(
        &filtered_dfg,
        &filtered_activities,
        &start_acts,
        &end_acts,
    );

    let mut response:APIResponse = APIResponse {
        OCPT: serde_json::Value::String(String::new()),
        dfg: json_dfg,
        start_activities: start_acts.clone(),
        end_activities: end_acts.clone(),
        is_perfectly_cut: true,
        cut_suggestions_list: CutSuggestionsList {
            all_activities: HashSet::new(),
            cuts: Vec::new(),
        },
    };

    // // Convert to JSON string
    let ocpt_json_string = process_forest_to_json(&process_forest);
    println!("OCPT JSON string:\n{}", ocpt_json_string);
    response.OCPT = ocpt_json_string;

    // // Convert back to ProcessForest
    // let parsed_forest = json_to_process_forest(&json_string);
    // println!("\nParsed ProcessForest:");
    // print_process_forest(&parsed_forest);

    let (found_disjoint, disjoint_activities) = collect_disjoint_activities(&process_forest);

    // Print disjoint activities
    if found_disjoint {
        println!(
            "Disjoint activities found in OCPT: {:?}",
            disjoint_activities
        );
        response.is_perfectly_cut = false;
        let cut_suggestions_list = start_cuts_opti_v2::find_best_possible_cuts(
            &filtered_dfg,
            &disjoint_activities,
            &start_acts,
            &end_acts,
        );
        response.cut_suggestions_list = cut_suggestions_list;
    } else {
        println!("No disjoint activities found in the OCPT");
    }

    Json(serde_json::to_value(response).unwrap())
}

// Handler for POST /cut-selected
async fn cut_selected_handler(
    AxumJson(payload): AxumJson<CutSelectedAPIRequest>,
) -> (StatusCode, AxumJson<serde_json::Value>) {
    println!("Received cut-selected request: {:?}", payload.cut_selected);

    let mut ocpt: ProcessForest = from_json_value(&payload.ocpt);
    let mut dfg: HashMap<(String, String), usize> = json_to_dfg(&payload.dfg);
    let global_start_activities: HashSet<String> = payload.start_activities;
    let global_end_activities: HashSet<String> = payload.end_activities;
    let cut_suggestions_list: CutSuggestionsList = payload.cut_suggestions_list;
    let cut_selected: CutSuggestion = payload.cut_selected;

    println!("old dfg:\n ");
    print_dfg(&dfg);


    // 1. Modify the DFG according to the selected cut
    // Remove edges that need to be cut
    for (from, to) in &cut_selected.edges_to_be_removed {
        dfg.remove(&(from.clone(), to.clone()));
    }
    
    // Add new edges with default weight of 1
    for (from, to) in &cut_selected.edges_to_be_added {
        dfg.insert((from.clone(), to.clone()), (cut_selected.cost_to_add_edge).clone());
    }

    println!("new dfg:\n ");
    print_dfg(&dfg);

    let process_forest_set1 = start_cuts_opti_v2::find_cuts_start(
        &dfg,
        &cut_selected.set1,
        &global_start_activities,
        &global_end_activities,
    );

    let process_forest_set2 = start_cuts_opti_v2::find_cuts_start(
        &dfg,
        &cut_selected.set2,
        &global_start_activities,
        &global_end_activities,
    );

    println!("Process Forest Set 1:\n ");
    print_process_forest(&process_forest_set1);

    println!("Process Forest Set 2:\n ");
    print_process_forest(&process_forest_set2);

    let mut children = Vec::new();
    children.extend(process_forest_set1);
    children.extend(process_forest_set2);
    let new_tree_node: TreeNode = TreeNode {
        label: cut_selected.cut_type.clone(),
        children,
    };
            
    

    println!("old ocpt:\n ");
    print_process_forest(&ocpt);

    // 2. Modify the process forest using the new function
    ocpt = modify_process_forest(ocpt, &cut_suggestions_list.all_activities, &cut_selected, &new_tree_node);

    println!("new ocpt:\n ");
    print_process_forest(&ocpt);

    // 3. Create the response with the updated DFG and OCPT
    let response = serde_json::json!({
        "status": "success",
        "message": "Cut operation performed",
        "OCPT": process_forest_to_json(&ocpt),
        "dfg": format_conversion::dfg_to_json(&dfg),
        "is_perfectly_cut": true
    });

    (StatusCode::OK, AxumJson(response))
}

fn modify_process_forest(
    mut ocpt: ProcessForest,
    disjoint_activities: &HashSet<String>,
    cut_selected: &CutSuggestion,
    new_tree_node: &TreeNode
) -> ProcessForest {
    // Return if empty
    if ocpt.is_empty() {
        return ocpt;
    }

    // First pass: process children of objects with special labels
    for node in ocpt.iter_mut() {
        if matches!(node.label.as_str(), "sequence" | "parallel" | "exclusive" | "redo") {
            if !node.children.is_empty() {
                node.children = modify_process_forest(node.children.clone(), disjoint_activities, cut_selected, new_tree_node);
            }
        }
    }

   

    // Collect labels of objects that don't have special labels
    let list_d: HashSet<String> = ocpt.iter()
        .filter(|node| !matches!(node.label.as_str(), "sequence" | "parallel" | "exclusive" | "redo"))
        .map(|node| node.label.clone())
        .collect();

    // println!("list_d: {:?}", list_d);

    // Return original if no match or empty list_d
    if list_d.is_empty() || list_d != *disjoint_activities {
        return ocpt;
    }

    // if list_d == *disjoint_activities {
    //     // If the disjoint activities match, we can proceed with the cut
    //     println!("Disjoint activities match: {:?}", list_d);
    // } 

    

    // Find cut object and its placement
    let mut cut_object = None;
    let mut cut_object_placement = None;

    // Check first and last positions for cut object
    if ocpt.first().map_or(false, |node| {
        matches!(node.label.as_str(), "sequence" | "parallel" | "exclusive" | "redo")
    }) {
        cut_object = Some(ocpt.remove(0));
        cut_object_placement = Some("beginning");
    } else if ocpt.last().map_or(false, |node| {
        matches!(node.label.as_str(), "sequence" | "parallel" | "exclusive" | "redo")
    }) {
        let idx = ocpt.len() - 1;
        cut_object = Some(ocpt.remove(idx));
        cut_object_placement = Some("last");
    }

    



    // Create new OCPT based on cut_object placement
    let mut new_ocpt = Vec::new();
    
    match (cut_object, cut_object_placement) {
        (Some(obj), Some("beginning")) => {
            new_ocpt.push(obj);
            new_ocpt.push(new_tree_node.clone());
        }
        (Some(obj), Some("last")) => {
            new_ocpt.push(new_tree_node.clone());
            new_ocpt.push(obj);
        }
        (_, _) => {
            new_ocpt.push(new_tree_node.clone());
        }
    }
    
    new_ocpt
}

#[tokio::main]
async fn main() {
    // Configure CORS
    let cors = CorsLayer::new()
        .allow_methods(Any)
        .allow_origin(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/", get(getInitialResponse))
        .route("/cut-selected", axum::routing::post(cut_selected_handler))
        .layer(cors);
    
    println!("Server running on http://localhost:1080");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:1080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

fn mainlk() {
    println!("Starting...");

    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Info,
            Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        WriteLogger::new(
            LevelFilter::Info,
            Config::default(),
            File::create("process.log").unwrap(),
        ),
    ])
    .unwrap();

    // let file_path = "data/small-example-v4.jsonocel";
    // let file_path = "data/running-example.jsonocel";
    let file_path = "data/github_pm4py.jsonocel";
    // let file_path = "data/o2c.jsonocel";

    let file_content = stdfs::read_to_string(&file_path).unwrap();
    let ocel: OcelJson = serde_json::from_str(&file_content).unwrap();

    let relations = build_relations_fns::build_relations(&ocel.events, &ocel.objects);
    // info!("size of relations: {}", relations.len());

    let (div, con, rel, defi, all_activities, all_object_types) =
        interaction_patterns::get_interaction_patterns(&relations, &ocel);

    // info!("Divergent: {:?}",div);
    // info!("Convergent: {:?}",con);
    // info!("Relational: {:?}",rel);
    // info!("Deficiency {:?}",defi);
    // log_sorted_map("Divergent", &div);
    // log_sorted_map("Convergent", &con);
    // log_sorted_map("Relational", &rel);
    // log_sorted_map("Deficiency", &defi);

    let (dfg, start_acts, end_acts) =
        divergence_free_dfg::get_divergence_free_graph_v2(&relations, &div);

    println!("created DFG!");

    print_dfg(&dfg);

    // let remove_list = vec![];
    // let remove_list = vec!["failed delivery".to_string(),"payment reminder".to_string()];
    let remove_list = vec!["reopened".to_string()];
    let filtered_dfg = filter_dfg(&dfg, &remove_list);
    let filtered_activities = filter_activities(&all_activities, &remove_list);

    // let temp = cost_to_add::all_possible_edges_to_add_to_dfg(&filtered_dfg, &filtered_activities);
    // for (edge, cost) in &temp {
    //     info!("Edge: {:?} with cost: {}", edge, cost);
    // }

    let json_dfg = format_conversion::dfg_to_json(&dfg);
    // // Save to file
    // let mut file = File::create("dfs-diagrams/dfg_o2c.json").expect("Failed to create file");
    // file.write_all(json_string.as_bytes()).expect("Failed to write to file");

    let start_time = std::time::Instant::now();

    // let process_forest = start_cuts_gem::find_cuts(&dfg, &dfg, all_activities, &start_acts, &end_acts);
    // In case of filtering activities in the begining
    // let process_forest = start_cuts::find_cuts(&filtered_dfg, &filtered_dfg, filtered_activities, &start_acts, &end_acts);
    let process_forest = start_cuts_opti_v2::find_cuts_start(
        &filtered_dfg,
        &filtered_activities,
        &start_acts,
        &end_acts,
    );

    let mut response:APIResponse = APIResponse {
        OCPT: serde_json::Value::String(String::new()),
        dfg: json_dfg,
        start_activities: start_acts.clone(),
        end_activities: end_acts.clone(),
        is_perfectly_cut: true,
        cut_suggestions_list: CutSuggestionsList {
            all_activities: HashSet::new(),
            cuts: Vec::new(),
        },
    };

    // // Convert to JSON string
    let ocpt_json_string = process_forest_to_json(&process_forest);
    info!("OCPT JSON string:\n{}", ocpt_json_string);
    response.OCPT = ocpt_json_string;

    // // Convert back to ProcessForest
    // let parsed_forest = json_to_process_forest(&json_string);
    // info!("\nParsed ProcessForest:");
    // print_process_forest(&parsed_forest);

    let (found_disjoint, disjoint_activities) = collect_disjoint_activities(&process_forest);


    // Print disjoint activities
    if found_disjoint {
        info!(
            "Disjoint activities found in OCPT: {:?}",
            disjoint_activities
        );
        response.is_perfectly_cut = false;
        let cut_suggestions_list = start_cuts_opti_v2::find_best_possible_cuts(
            &filtered_dfg,
            &disjoint_activities,
            &start_acts,
            &end_acts,
        );
        response.cut_suggestions_list = cut_suggestions_list;
    } else {
        info!("No disjoint activities found in the OCPT");
    }

    let elapsed = start_time.elapsed();
    println!("Time taken to form process_forest: {:.2?}", elapsed);

    // println!("=== Object list === num: {}", all_object_types.len());
    // for object in &all_object_types {
    //     println!("{}", object);
    // }

    // println!("=== Activity list === num: {}", all_activities.len());
    // for activity in &all_activities {
    //     println!("{}", activity);
    // }

    // println!("\nStart Activities: {:?}", start_acts);
    // println!("End Activities: {:?}", end_acts);

    // println!("\n=== Process Forest ===");
    // print_process_forest(&process_forest);
}

fn log_sorted_map<T: std::fmt::Debug + Ord, U: std::fmt::Debug>(
    label: &str,
    map: &std::collections::HashMap<T, U>,
) {
    let mut items: Vec<_> = map.iter().collect();
    items.sort_by(|a, b| a.0.cmp(b.0));
    info!("{}: {{", label);
    for (k, v) in items {
        info!("  {:?}: {:?}", k, v);
    }
    info!("}}");
}

fn print_dfg(dfg: &HashMap<(String, String), usize>) {
    let mut keys: Vec<_> = dfg.keys().collect();
    keys.sort_by(|(a1, b1), (a2, b2)| a1.cmp(a2).then_with(|| b1.cmp(b2)));

    for (a, b) in keys {
        if let Some(count) = dfg.get(&(a.clone(), b.clone())) {
            println!("{} -> {} : {}", a, b, count);
        }
    }
}

fn print_process_tree(tree: &TreeNode, depth: usize) {
    let indent = "  ".repeat(depth);
    println!("{}{}", indent, tree.label);
    for child in &tree.children {
        print_process_tree(child, depth + 1);
    }
}

fn print_process_forest(forest: &ProcessForest) {
    for tree in forest {
        print_process_tree(tree, 0);
    }
}

fn filter_dfg(
    dfg: &HashMap<(String, String), usize>,
    remove_list: &Vec<String>,
) -> HashMap<(String, String), usize> {
    dfg.iter()
        .filter(|((from, to), _)| !remove_list.contains(from) && !remove_list.contains(to))
        .map(|(k, v)| (k.clone(), *v))
        .collect()
}

fn filter_activities(all_activities: &Vec<String>, remove_list: &Vec<String>) -> HashSet<String> {
    all_activities
        .iter()
        .filter(|activity| !remove_list.contains(*activity))
        .cloned()
        .collect()
}

fn get_start_and_end_activities_from_dfg(
    dfg: &HashMap<(String, String), usize>,
    all_activities: &HashSet<String>,
) -> (HashSet<String>, HashSet<String>) {
    let mut start_activities = HashSet::new();
    let mut end_activities = HashSet::new();

    for ((a, b), _) in dfg {
        let a_in = all_activities.contains(a);
        let b_in = all_activities.contains(b);

        if !a_in && b_in {
            // 'a' is outside, 'b' is inside → 'b' is a start activity
            start_activities.insert(b.clone());
        }

        if a_in && !b_in {
            // 'a' is inside, 'b' is outside → 'a' is an end activity
            end_activities.insert(a.clone());
        }
    }

    (start_activities, end_activities)
}

// Function to collect disjoint activities
fn collect_disjoint_activities(forest: &ProcessForest) -> (bool, HashSet<String>) {
    let mut disjoint_activities: HashSet<String> = HashSet::new();

    fn find_first_disjoint(nodes: &[TreeNode], disjoint_set: &mut HashSet<String>) -> bool {
        for node in nodes {
            // Check if this node has more than 2 children (indicating disjoint activities)
            if node.children.len() > 2 {
                // Add all child activity names to the disjoint set
                for child in &node.children {
                    // Only add leaf nodes (activities without children)
                    if child.children.is_empty() {
                        disjoint_set.insert(child.label.clone());
                    }
                }
                return true; // Found first disjoint set, return immediately
            }

            // Recursively check children
            if !node.children.is_empty() {
                if find_first_disjoint(&node.children, disjoint_set) {
                    return true; // Found in children, propagate up
                }
            }
        }
        false // No disjoint set found
    }

    let found = find_first_disjoint(forest, &mut disjoint_activities);
    (found, disjoint_activities)
}

fn mainkoi() {
    println!("Starting example...");
    CombinedLogger::init(vec![
        TermLogger::new(
            LevelFilter::Info,
            Config::default(),
            TerminalMode::Mixed,
            ColorChoice::Auto,
        ),
        WriteLogger::new(
            LevelFilter::Info,
            Config::default(),
            File::create("process.log").unwrap(),
        ),
    ])
    .unwrap();

    // Example with 4 nodes: A, B, C, D
    let mut dfg: HashMap<(String, String), usize> = HashMap::new();
    dfg.insert(("A".to_string(), "D".to_string()), 1);
    dfg.insert(("C".to_string(), "B".to_string()), 1);
    dfg.insert(("C".to_string(), "D".to_string()), 1);

    let all_activities: HashSet<String> =
        ["A", "B", "C", "D"].iter().map(|s| s.to_string()).collect();

    let start_acts: HashSet<String> = ["A", "C"].iter().map(|s| s.to_string()).collect();

    let end_acts: HashSet<String> = ["B", "D"].iter().map(|s| s.to_string()).collect();

    // start_cuts::find_cuts(&dfg, &dfg, all_activities, &start_acts, &end_acts);
    start_cuts_opti_v2::find_cuts_start(&dfg, &all_activities, &start_acts, &end_acts);

    // let (min_cost, cut_edges, set1, set2, new_dfg) = best_exclusive_cut::best_exclusive_cut(&dfg, &all_activities);

    // info!("== Best Exclusive Cut Result ===");
    // info!("Min cost: {}", min_cost);
    // info!("Cut edges: {:?}", cut_edges);
    // info!("Set 1: {:?}", set1);
    // info!("Set 2: {:?}", set2);
    // info!("New DFG: {:?}", new_dfg);
}
