use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs as stdfs;
use std::fs::File;
use simplelog::*;
use std::io::Write;
use crate::types::{OcelJson, Event, Object, ProcessForest, TreeNode};
mod build_relations_fns;
mod types;
mod interaction_patterns;
mod divergence_free_dfg;
mod start_cuts;
mod start_cuts_opti_v1;
mod start_cuts_opti_v2;
mod format_conversion;
use log::info;
mod cost_to_cut;
mod good_cuts;
mod best_sequence_cut;

//For REST API server
use tokio::fs as tokiofs;
use axum::Json;
// use axum::{
//     routing::{get, post},
//     Router,
// };
use tower_http::cors::{Any, CorsLayer};
use axum::{Router, routing::get, response::Html};
// use tower::ServiceExt;
// use axum::http::{HeaderValue, Method};
// use tower_http::cors::CorsLayer;
use serde_json::Value;

// GET / — serves the content of dfg.json as JSON
async fn hello() -> Json<Value> {
    let file_content = tokiofs::read_to_string("dfs-diagrams/dfg_github_pm4py.json")
        .await
        .expect("Failed to read dfg.json");
    let json: Value = serde_json::from_str(&file_content)
        .expect("Failed to parse JSON from dfg.json");

    Json(json)
}


#[tokio::main]
async fn main1() {
    // Configure CORS
    let cors = CorsLayer::new()
        .allow_methods(Any)
        .allow_origin(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/", get(hello))
        .layer(cors);
    
    println!("Server running on http://localhost:1080");
    println!("GET  / - Hello World");
    println!("POST /print - Prints body to console");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:1080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}


fn main() {

    println!("Starting...");

    CombinedLogger::init(vec![
        TermLogger::new(LevelFilter::Info, Config::default(), TerminalMode::Mixed, ColorChoice::Auto),
        WriteLogger::new(LevelFilter::Info, Config::default(), File::create("process.log").unwrap()),
    ]).unwrap();

    // let file_path = "data/small-example-v7.jsonocel";
    // let file_path = "data/running-example.jsonocel";
    let file_path = "data/github_pm4py.jsonocel";

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

    // let json_dfg = format_conversion::dfg_to_json(&dfg);
    // let json_string = serde_json::to_string_pretty(&json_dfg).unwrap();
    // // Save to file
    // let mut file = File::create("dfs-diagrams/dfg_github_pm4py.json").expect("Failed to create file");
    // file.write_all(json_string.as_bytes()).expect("Failed to write to file");

    let start_time = std::time::Instant::now();

    // let process_forest = start_cuts_gem::find_cuts(&dfg, &dfg, all_activities, &start_acts, &end_acts);
    // In case of filtering activities in the begining
    // let process_forest = start_cuts::find_cuts(&filtered_dfg, &filtered_dfg, filtered_activities, &start_acts, &end_acts);
    let process_forest = start_cuts_opti_v2::find_cuts_start(&filtered_dfg, &filtered_activities);


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
    keys.sort_by(|(a1, b1), (a2, b2)| {
        a1.cmp(a2).then_with(|| b1.cmp(b2))
    });

    for (a, b) in keys {
        if let Some(count) = dfg.get(&(a.clone(), b.clone())) {
            info!("{} -> {} : {}", a, b, count);
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
        .filter(|((from, to), _)| {
            !remove_list.contains(from) && !remove_list.contains(to)
        })
        .map(|(k, v)| (k.clone(), *v))
        .collect()
}

fn filter_activities(
    all_activities: &Vec<String>,
    remove_list: &Vec<String>,
) -> HashSet<String> {
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
