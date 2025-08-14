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
mod best_parallel_cut_v3;
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
    let file_content = tokiofs::read_to_string("ddfg-diagrams/ddfg_order-management.json")
        .await
        .expect("Failed to read dfg.json");
    let json: Value =
        serde_json::from_str(&file_content).expect("Failed to parse JSON from dfg.json");

    Json(json)
}


async fn getInitialResponse() -> Json<Value> {

    println!("Starting...");

    // Changed to use OCEL 2.0 format
    let file_name ="order-management";
    let file_path = "data/order-management.json";
    // let file_name ="ContainerLogistics";
    // let file_path = "data/ContainerLogistics.json";
    // let file_name ="ocel2-p2p";
    // let file_path = "data/ocel2-p2p.json";
    // let file_name ="age_of_empires_ocel2";
    // let file_path = "data/age_of_empires_ocel2.json";

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

    let remove_list = vec![];
    // let remove_list = vec!["failed delivery".to_string(),"payment reminder".to_string()];
    // let remove_list = vec!["reopened".to_string()];
    let filtered_dfg = filter_dfg(&dfg, &remove_list);
    let filtered_activities = filter_activities(&all_activities, &remove_list);


    let json_dfg: Value = format_conversion::dfg_to_json(&dfg);

    // Save to file
    let dfs_path = format!("ddfg-diagrams/ddfg_{}.json", file_name);
    let mut file = File::create(&dfs_path).expect("Failed to create file");
    let json_string = serde_json::to_string(&json_dfg).expect("Failed to serialize DFG to JSON");
    file.write_all(json_string.as_bytes()).expect("Failed to write to file");

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
        total_edges_added: Vec::new(),
        total_edges_removed: Vec::new(),
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
) -> (Json<Value>) {
    println!("Received cut-selected request: {:?}", payload.cut_selected);

    let mut ocpt: ProcessForest = from_json_value(&payload.ocpt);
    let mut dfg: HashMap<(String, String), usize> = json_to_dfg(&payload.dfg);
    let global_start_activities: HashSet<String> = payload.start_activities;
    let global_end_activities: HashSet<String> = payload.end_activities;
    let cut_suggestions_list: CutSuggestionsList = payload.cut_suggestions_list;
    let cut_selected: CutSuggestion = payload.cut_selected;
    let mut total_edges_removed: Vec<(String, String, usize)> = payload.total_edges_removed;
    let mut total_edges_added: Vec<(String, String, usize)> = payload.total_edges_added;

    println!("old dfg:\n ");
    print_dfg(&dfg);


    // 1. Modify the DFG according to the selected cut
    // Remove edges that need to be cut
    for (from, to, _) in &cut_selected.edges_to_be_removed {
        dfg.remove(&(from.clone(), to.clone()));
    }
    
    // Add new edges with default weight of 1
    for (from, to, _) in &cut_selected.edges_to_be_added {
        dfg.insert((from.clone(), to.clone()), (cut_selected.cost_to_add_edge).clone());
    }

    println!("new dfg:\n ");
    print_dfg(&dfg);

    let new_tree_node = create_new_tree_node_by_cut_selection(
        &dfg,
        &cut_selected,
        &global_start_activities,
        &global_end_activities,
    );
            
    

    println!("old ocpt:\n ");
    print_process_forest(&ocpt);

    // 2. Modify the process forest using the new function
    ocpt = modify_process_forest(ocpt, &cut_suggestions_list.all_activities, &cut_selected, &new_tree_node);

    println!("new ocpt:\n ");
    print_process_forest(&ocpt);



    let json_dfg: Value = format_conversion::dfg_to_json(&dfg);

    // 3. Add to total edges removed and added from cut selected
    total_edges_removed.extend(cut_selected.edges_to_be_removed.clone());
    total_edges_added.extend(cut_selected.edges_to_be_added.clone());

    let mut response:APIResponse = APIResponse {
        OCPT: serde_json::Value::String(String::new()),
        dfg: json_dfg,
        start_activities: global_start_activities.clone(),
        end_activities: global_end_activities.clone(),
        is_perfectly_cut: true,
        cut_suggestions_list: CutSuggestionsList {
            all_activities: HashSet::new(),
            cuts: Vec::new(),
        },
        total_edges_added: total_edges_added.clone(),
        total_edges_removed: total_edges_removed.clone(),
    };

    // // Convert to JSON string
    let ocpt_json_string = process_forest_to_json(&ocpt);
    println!("OCPT JSON string:\n{}", ocpt_json_string);
    response.OCPT = ocpt_json_string;

    let (found_disjoint, disjoint_activities) = collect_disjoint_activities(&ocpt);

    // Print disjoint activities
    if found_disjoint {
        println!(
            "Disjoint activities found in OCPT: {:?}",
            disjoint_activities
        );
        response.is_perfectly_cut = false;
        let cut_suggestions_list = start_cuts_opti_v2::find_best_possible_cuts(
            &dfg,
            &disjoint_activities,
            &global_start_activities,
            &global_end_activities,
        );
        response.cut_suggestions_list = cut_suggestions_list;
    } else {
        println!("No disjoint activities found in the OCPT");
    }


    Json(serde_json::to_value(response).unwrap())
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

    // Process each node in the forest
    for node in ocpt.iter_mut() {
        match node.label.as_str() {
            // For sequence, parallel, exclusive, redo nodes
            label @ ("sequence" | "parallel" | "exclusive" | "redo") => {
                if !node.children.is_empty() {
                    node.children = modify_process_forest(
                        node.children.clone(), 
                        disjoint_activities, 
                        cut_selected,
                        new_tree_node
                    );
                }
            },
            // For flower nodes
            "flower" => {
                // Collect labels of all children
                let list_d: HashSet<String> = node.children.iter()
                    .map(|child| child.label.clone())
                    .collect();

                // If children match disjoint activities or list is empty, replace with new tree node
                if list_d.is_empty() || list_d == *disjoint_activities {
                    *node = new_tree_node.clone();
                }
            },
            _ => {} // Skip other nodes
        }
    }
    
    ocpt
}

fn create_new_tree_node_by_cut_selection(
    dfg: &HashMap<(String, String), usize>,
    cut_selected: &CutSuggestion,
    global_start_activities: &HashSet<String>,
    global_end_activities: &HashSet<String>
) -> TreeNode {
    let process_forest_set1 = start_cuts_opti_v2::find_cuts_start(
        dfg,
        &cut_selected.set1,
        global_start_activities,
        global_end_activities,
    );

    let process_forest_set2 = start_cuts_opti_v2::find_cuts_start(
        dfg,
        &cut_selected.set2,
        global_start_activities,
        global_end_activities,
    );

    println!("Process Forest Set 1:\n ");
    print_process_forest(&process_forest_set1);

    println!("Process Forest Set 2:\n ");
    print_process_forest(&process_forest_set2);

    let mut children = Vec::new();
    children.extend(process_forest_set1);
    children.extend(process_forest_set2);
    TreeNode {
        label: cut_selected.cut_type.clone(),
        children,
    }
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
        .route("/dfg", get(hello))
        .route("/cut-selected", axum::routing::post(cut_selected_handler))
        .layer(cors);
    
    println!("Server running on http://localhost:1080");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:1080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
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

// Function to collect disjoint activities from flower node
fn collect_disjoint_activities(forest: &ProcessForest) -> (bool, HashSet<String>) {
    let mut disjoint_activities: HashSet<String> = HashSet::new();

    fn find_flower_node(nodes: &[TreeNode], disjoint_set: &mut HashSet<String>) -> bool {
        for node in nodes {
            // Check if this is a flower node
            if node.label == "flower" {
                // Add all child activity names to the disjoint set
                for child in &node.children {
                    disjoint_set.insert(child.label.clone());
                }
                return true; // Found flower node, return immediately
            }

            // Recursively check children
            if !node.children.is_empty() {
                if find_flower_node(&node.children, disjoint_set) {
                    return true; // Found in children, propagate up
                }
            }
        }
        false // No flower node found
    }

    let found = find_flower_node(forest, &mut disjoint_activities);
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
