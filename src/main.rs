use crate::format_conversion::{from_json_value, json_to_dfg, json_to_process_forest, process_forest_to_json, json_to_cost_to_add_edges};
use crate::types::{APIResponse, CutSelectedAPIRequest, CutSuggestion, CutSuggestionsList, OCEL, ProcessForest, TreeNode, OCPTWithMetrics};
use serde::Deserialize;
use simplelog::*;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs as stdfs;
use std::fs::File;
use std::io::Write;
use std::sync::Arc;
mod add_self_loops;
mod build_relations_fns;
mod divergence_free_dfg;
mod format_conversion;
mod get_dfg_by_object_type;
mod interaction_patterns;
mod start_cuts;
mod start_cuts_opti_v1;
mod start_cuts_opti_v2;
mod types;
use log::info;
use add_self_loops::add_self_loops;
mod best_exclusive_cut;
mod best_parallel_cut;
mod best_parallel_cut_exhaustive;
mod best_parallel_cut_v2;
mod best_parallel_cut_v3;
mod best_parallel_cut_v4;
mod best_redo_cuts;
mod best_sequence_cut;
mod best_sequence_cut_v2;
mod cost_to_add;
mod cost_to_cut;
mod good_cuts;
use crate::cost_to_add::cost_of_adding_edge;
use axum::extract::{DefaultBodyLimit, Json as AxumJson, Multipart, Path};
use axum::http::StatusCode;
use tokio::io::AsyncWriteExt;

// Include the conformance checking modules
mod conformance_checking;
mod conformance_checking_mine;
use conformance_checking::{calculate_conformance_metrics, ConformanceMetrics};
use conformance_checking_mine::{conformance_checking_mine_fitness, conformance_checking_mine_precision, find_fitness_and_precision};

//For REST API server
use axum::Json;
use tokio::fs as tokiofs;
// use axum::{
//     routing::{get, post},
//     Router,
// };
use axum::{Router, response::Html, routing::get};
use tower_http::cors::{Any, CorsLayer};
// use tower_http::limit::RequestBodyLimitLayer;
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


// Handler for POST /upload
// Expects multipart/form-data with one field named "file".
async fn upload_handler(mut multipart: Multipart) -> Result<Json<Value>, StatusCode> {
    println!("Upload handler called - processing multipart upload...");

    stdfs::create_dir_all("data").map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    while let Some(field) = multipart.next_field().await.map_err(|e| {
        println!("Multipart error getting next field: {:?}", e);
        StatusCode::BAD_REQUEST
    })? {
        let name = field.name().unwrap_or("");
        println!("Processing field: {}", name);

        if name != "file" {
            continue;
        }

        let filename = field
            .file_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "upload.json".to_string());

        if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
            return Ok(Json(serde_json::json!({
                "success": false,
                "message": "Invalid filename"
            })));
        }

        if !filename.ends_with(".json") && !filename.ends_with(".jsonocel") {
            return Ok(Json(serde_json::json!({
                "success": false,
                "message": "Only JSON and JSONOCEL files are allowed"
            })));
        }

        let file_path = format!("data/{}", filename);
        let tmp_path = format!("{}.uploading", file_path);
        println!("Saving upload to temp file: {}", tmp_path);

    let mut out = tokiofs::File::create(&tmp_path)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let mut total_bytes: u64 = 0;
        let mut field = field;

        loop {
            match field.chunk().await {
                Ok(Some(chunk)) => {
                    total_bytes += chunk.len() as u64;
                    if let Err(e) = out.write_all(&chunk).await {
                        println!("Failed writing upload chunk: {:?}", e);
                        let _ = tokiofs::remove_file(&tmp_path).await;
                        return Ok(Json(serde_json::json!({
                            "success": false,
                            "message": "Failed writing uploaded file",
                            "received_bytes": total_bytes,
                            "tmp_path": tmp_path
                        })));
                    }
                }
                Ok(None) => {
                    // End of stream
                    break;
                }
                Err(e) => {
                    // This is the case you're hitting: stream aborted mid-upload.
                    println!("Multipart stream aborted: {:?}", e);
                    let _ = tokiofs::remove_file(&tmp_path).await;
                    return Ok(Json(serde_json::json!({
                        "success": false,
                        "message": "Upload stream aborted before completion",
                        "received_bytes": total_bytes,
                        "error": format!("{}", e)
                    })));
                }
            }
        }

        if let Err(e) = out.flush().await {
            println!("Failed flushing file: {:?}", e);
            let _ = tokiofs::remove_file(&tmp_path).await;
            return Ok(Json(serde_json::json!({
                "success": false,
                "message": "Failed flushing uploaded file",
                "received_bytes": total_bytes
            })));
        }

        // Publish the file only after fully written.
        let _ = tokiofs::remove_file(&file_path).await;
        if let Err(e) = tokiofs::rename(&tmp_path, &file_path).await {
            println!("Failed renaming temp upload: {:?}", e);
            let _ = tokiofs::remove_file(&tmp_path).await;
            return Ok(Json(serde_json::json!({
                "success": false,
                "message": "Failed finalizing uploaded file",
                "received_bytes": total_bytes
            })));
        }

        let saved_size = tokiofs::metadata(&file_path)
            .await
            .map(|m| m.len())
            .unwrap_or(0);

        println!(
            "Upload saved: {} (received {} bytes, saved {} bytes)",
            file_path, total_bytes, saved_size
        );

        return Ok(Json(serde_json::json!({
            "success": true,
            "message": "File uploaded successfully!",
            "filename": filename,
            "path": file_path,
            "received_bytes": total_bytes,
            "saved_bytes": saved_size
        })));
    }

    Ok(Json(serde_json::json!({
        "success": false,
        "message": "No 'file' field found in multipart upload"
    })))
}

async fn get_initial_response(Path(file_name): Path<String>) -> Json<Value> {
    process_response(file_name).await
}

async fn get_initial_response_default() -> Json<Value> {
    process_response("order-management".to_string()).await
}

async fn process_response(file_name_input: String) -> Json<Value> {

    println!("Starting...");

    // Changed to use OCEL 2.0 format
    let file_name = if file_name_input.is_empty() {
        "order-management"
    } else {
        &file_name_input
    };
    
    let file_path = format!("data/{}.json", file_name);

    let file_content = stdfs::read_to_string(&file_path).unwrap();
    let ocel: OCEL = serde_json::from_str(&file_content).unwrap();

    let relations = build_relations_fns::build_relations(&ocel.events, &ocel.objects);
    // println!("size of relations: {}", relations.len());

    let (div, con, rel, defi, all_activities, all_object_types) =
        interaction_patterns::get_interaction_patterns(&relations, &ocel);

    // Get DFGs by object type
    let dfg_sets = get_dfg_by_object_type::get_dfg_by_object_type(&relations, &div);

    // print first 5 relations tuple
    println!("First 5 relations:");
    for relation in &relations[..5] {
        println!("{:?}", relation);
    }
    //print div
    println!("Divergent: {:?}", div);

    // Print information about DFGs by object type
    println!("DFGs by object type:");
    for (otype, (dfg_otype, start_acts_otype, end_acts_otype)) in &dfg_sets {
        println!("\nObject Type: {}", otype);
        println!("Number of edges in DFG: {}", dfg_otype.len());
        println!("Start activities: {:?}", start_acts_otype);
        println!("End activities: {:?}", end_acts_otype);
        if !dfg_otype.is_empty() {
            println!("DFG edges:");
            print_dfg(dfg_otype);
        }
    }

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
        cost_to_add_edges: serde_json::json!({}),
        precision: 0.0,
        fitness: 0.0,
        f_score: 0.0,
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
        // Get costs to add edges
        let cost_to_add_edges = cost_of_adding_edge(&relations, &div, &filtered_dfg);
        let json_cost_to_add_edges: Value = format_conversion::cost_to_add_edges_to_json(&cost_to_add_edges);

        response.is_perfectly_cut = false;
        response.cost_to_add_edges = json_cost_to_add_edges;
        let cut_suggestions_list = start_cuts_opti_v2::find_best_possible_cuts(
            &filtered_dfg,
            &disjoint_activities,
            &start_acts,
            &end_acts,
            &cost_to_add_edges
        );
        response.cut_suggestions_list = cut_suggestions_list;
    } else {
        println!("No disjoint activities found in the OCPT");

        // Get the modified OCPT with self-loops added
        let (modified_ocpt, self_loop_activities) = add_self_loops(&dfg.clone(), &process_forest, file_name);
        println!("Self-loop activities processed: {:?}", self_loop_activities);
        
        // Update the response with the modified OCPT
        let modified_ocpt_json_string = process_forest_to_json(&modified_ocpt);
        response.OCPT = modified_ocpt_json_string;

        // // Perform custom conformance checking
        // println!("\n--- Custom Conformance Analysis ---");
        // let fitness_percentage = conformance_checking_mine_fitness(&modified_ocpt, file_name);
        
        // // Calculate precision (all possible executions)
        // println!("\n--- Precision Analysis ---");
        // let precision_percentage = conformance_checking_mine_precision(&modified_ocpt, &self_loop_activities, file_name);
        
        // // Calculate F1 Score = 2 * (Precision * Fitness) / (Precision + Fitness)
        // let f_score = if (precision_percentage + fitness_percentage) > 0.0 {
        //     2.0 * (precision_percentage * fitness_percentage) / (precision_percentage + fitness_percentage)
        // } else {
        //     0.0
        // };

        // println!("Summary: Fitness = {:.2}%, Precision = {:.2}%, F1-Score = {:.2}%", 
        //          fitness_percentage, precision_percentage, f_score);
        
        // Call find_fitness_and_precision function
        println!("\n--- Find Fitness and Precision Analysis ---");
        let (_, _, _, _, fitness, precision, f_score) = find_fitness_and_precision(&modified_ocpt, file_name);
        
        response.fitness = fitness;
        response.precision = precision;
        response.f_score = f_score;
    }

    Json(serde_json::to_value(response).unwrap())
}


fn test_conformance(ocpt_data: serde_json::Value) {
    // Placeholder for conformance checking test function

     // Read OCEL log
    let file_name = "order-management";
    let file_path = format!("data/{}.json", file_name);
    
    println!("Reading OCEL log from: {}", file_path);
    let file_content = stdfs::read_to_string(&file_path)
        .expect("Failed to read OCEL file");
    
    let ocel: OCEL = serde_json::from_str(&file_content)
        .expect("Failed to parse OCEL JSON");
    
    println!("✓ OCEL log loaded successfully");
    println!("  - Events: {}", ocel.events.len());
    println!("  - Objects: {}", ocel.objects.len());
    println!();

    let process_tree: ProcessForest = serde_json::from_value(ocpt_data)
        .expect("Failed to parse OCPT JSON");
    
    println!("✓ Process tree (OCPT) loaded successfully");
    println!();
    
    // Calculate conformance metrics
    println!("Calculating conformance metrics...");
    println!();
    
    let metrics = calculate_conformance_metrics(&ocel, &process_tree);
    
    // Print results
    println!("{}", metrics);
    
    // Additional detailed output
    println!();
    println!("Interpretation:");
    if metrics.fitness >= 0.9 {
        println!("✓ High fitness: The log fits the model very well");
    } else if metrics.fitness >= 0.7 {
        println!("⚠ Moderate fitness: Some traces deviate from the model");
    } else {
        println!("✗ Low fitness: Significant deviations between log and model");
    }
    
    if metrics.precision >= 0.9 {
        println!("✓ High precision: The model is not overly general");
    } else if metrics.precision >= 0.7 {
        println!("⚠ Moderate precision: The model allows some extra behavior");
    } else {
        println!("✗ Low precision: The model is too general");
    }
}

// New function to generate all possible OCPTs
async fn all_possible_ocpts() -> Json<Value> {
    println!("Starting all_possible_ocpts...");

    // Initial setup - same as getInitialResponse
    let file_name = "order-management";
    // let file_name = "ocel2-p2p";
    // let file_name = "ContainerLogistics";
    // let file_name = "age_of_empires_ocel2";
    let file_path = format!("data/{}.json", file_name);

    let file_content = stdfs::read_to_string(&file_path).unwrap();
    let ocel: OCEL = serde_json::from_str(&file_content).unwrap();

    let relations = build_relations_fns::build_relations(&ocel.events, &ocel.objects);
    let (div, _con, _rel, _defi, all_activities, _all_object_types) =
        interaction_patterns::get_interaction_patterns(&relations, &ocel);

    let (dfg, start_acts, end_acts) =
        divergence_free_dfg::get_divergence_free_graph_v2(&relations, &div);

    let remove_list = vec![];
    let filtered_dfg = filter_dfg(&dfg, &remove_list);
    let filtered_activities = filter_activities(&all_activities, &remove_list);

    // Get initial process forest
    let initial_process_forest = start_cuts_opti_v2::find_cuts_start(
        &filtered_dfg,
        &filtered_activities,
        &start_acts,
        &end_acts,
    );

    // Structure to track state for recursion
    #[derive(Clone)]
    struct OCPTState {
        ocpt: ProcessForest,
        dfg: HashMap<(String, String), usize>,
        total_edges_added: Vec<(String, String, usize)>,
        total_edges_removed: Vec<(String, String, usize)>,
        cost_to_add_edges: HashMap<(String, String), f64>,
        sequence_of_choices: Vec<String>,
    }

    let initial_cost_to_add_edges = cost_of_adding_edge(&relations, &div, &filtered_dfg);
    
    let initial_state = OCPTState {
        ocpt: initial_process_forest.clone(),
        dfg: filtered_dfg.clone(),
        total_edges_added: Vec::new(),
        total_edges_removed: Vec::new(),
        cost_to_add_edges: initial_cost_to_add_edges,
        sequence_of_choices: Vec::new(),
    };

    let mut self_loop_activities_list: Vec<String> = Vec::new();

    let mut all_final_ocpts: Vec<ProcessForest> = Vec::new();
    let mut all_final_sequences: Vec<Vec<String>> = Vec::new();
    let mut ocpts_with_metrics: Vec<OCPTWithMetrics> = Vec::new();
    let mut states_to_process: Vec<OCPTState> = vec![initial_state];
    let mut ocpt_counter = 1;

    println!("Starting recursive OCPT generation...\n");

    while let Some(current_state) = states_to_process.pop() {
        println!("=== Processing OCPT #{} ===", ocpt_counter);
        print_process_forest(&current_state.ocpt);

        let (found_disjoint, disjoint_activities) = collect_disjoint_activities(&current_state.ocpt);

        if found_disjoint {
            println!("Disjoint activities found: {:?}", disjoint_activities);
            
            // Get cut suggestions for this state
            let cut_suggestions_list = start_cuts_opti_v2::find_best_possible_cuts(
                &current_state.dfg,
                &disjoint_activities,
                &start_acts,
                &end_acts,
                &current_state.cost_to_add_edges
            );

            if cut_suggestions_list.cuts.is_empty() {
                println!("No cut suggestions available. Adding to final OCPTs.");
                all_final_ocpts.push(current_state.ocpt.clone());
                all_final_sequences.push(current_state.sequence_of_choices.clone());
            } else {
                println!("Found {} cut suggestions. Applying each...", cut_suggestions_list.cuts.len());
                
                // Apply each cut suggestion and add to queue for further processing
                for (cut_idx, cut_suggestion) in cut_suggestions_list.cuts.iter().enumerate() {
                    println!("  Applying cut suggestion {}/{}: {:?}", cut_idx + 1, cut_suggestions_list.cuts.len(), cut_suggestion.cut_type);
                    
                    // Clone current state for modification
                    let mut new_state = current_state.clone();

                    // Apply the cut (modify DFG)
                    for (from, to, _) in &cut_suggestion.edges_to_be_removed {
                        new_state.dfg.remove(&(from.clone(), to.clone()));
                    }
                    
                    for (from, to, cost) in &cut_suggestion.edges_to_be_added {
                        new_state.dfg.insert((from.clone(), to.clone()), cost.clone());
                    }

                    // Update cost_to_add_edges
                    for (from, to, _) in &cut_suggestion.edges_to_be_removed {
                        new_state.cost_to_add_edges.remove(&(from.clone(), to.clone()));
                    }

                    // Create new tree node based on the cut
                    let new_tree_node = create_new_tree_node_by_cut_selection(
                        &new_state.dfg,
                        cut_suggestion,
                        &start_acts,
                        &end_acts,
                    );

                    // Modify the process forest
                    new_state.ocpt = modify_process_forest(
                        new_state.ocpt,
                        &cut_suggestions_list.all_activities,
                        cut_suggestion,
                        &new_tree_node
                    );

                    // Update tracking info
                    new_state.total_edges_removed.extend(cut_suggestion.edges_to_be_removed.clone());
                    new_state.total_edges_added.extend(cut_suggestion.edges_to_be_added.clone());
                    
                    // Add the choice to the sequence of choices
                    new_state.sequence_of_choices.push(cut_suggestion.cut_type.clone());

                    // Add to queue for further processing
                    states_to_process.push(new_state);
                }
            }
        } else {
            println!("No disjoint activities found. Adding to final OCPTs.");
            
            // // Apply self-loops for complete process forest
            // let (final_ocpt, self_loop_activities) = add_self_loops(&current_state.dfg, &current_state.ocpt, file_name);
            // self_loop_activities_list = self_loop_activities.clone();
            // // println!("Self-loop activities processed: {:?}", self_loop_activities);
            // all_final_ocpts.push(final_ocpt);
            // all_final_sequences.push(current_state.sequence_of_choices.clone());

            // TEMP
            all_final_ocpts.push(current_state.ocpt);
            all_final_sequences.push(current_state.sequence_of_choices.clone());
        }

        ocpt_counter += 1;
        println!();
    }

    // Print all final OCPTs
    println!("\n{}", "=".repeat(60));
    println!("FINAL RESULTS: Found {} distinct process forests", all_final_ocpts.len());
    println!("{}", "=".repeat(60));

    for (i, (final_ocpt, sequence_of_choices)) in all_final_ocpts.iter().zip(all_final_sequences.iter()).enumerate() {
        println!("\n>>> FINAL OCPT #{} <<<", i + 1);
        println!("Sequence of choices: {:?}", sequence_of_choices);
        // print_process_forest(final_ocpt);
        // println!("JSON representation:");
        let json_string = process_forest_to_json(final_ocpt);
        // println!("{}", json_string);
        
        // // Perform conformance checking for this OCPT
        // println!("\n--- Conformance Analysis for OCPT #{} ---", i + 1);
        // test_conformance(json_string.clone());
        
        // // Perform custom conformance checking
        // println!("\n--- Custom Conformance Analysis for OCPT #{} ---", i + 1);
        // let fitness_percentage = conformance_checking_mine_fitness(final_ocpt, file_name);
        
        // // Calculate precision (all possible executions)
        // println!("\n--- Precision Analysis for OCPT #{} ---", i + 1);
        // let precision_percentage = conformance_checking_mine_precision(final_ocpt, &self_loop_activities_list, file_name);
        
        // // Calculate F1 Score = 2 * (Precision * Fitness) / (Precision + Fitness)
        // let f_score = if (precision_percentage + fitness_percentage) > 0.0 {
        //     2.0 * (precision_percentage * fitness_percentage) / (precision_percentage + fitness_percentage)
        // } else {
        //     0.0
        // };

        // println!("Summary for OCPT #{}: Fitness = {:.2}%, Precision = {:.2}%, F1-Score = {:.2}%", 
        //          i + 1, fitness_percentage, precision_percentage, f_score);

        
        // Call find_fitness_and_precision function
        println!("\n--- Find Fitness and Precision Analysis for OCPT #{} ---", i + 1);
        let (total_executions, total_traces, x, t, fitness, precision, f_score) = find_fitness_and_precision(final_ocpt, file_name);
        
        // Convert to percentages for compatibility
        let fitness_percentage = fitness * 100.0;
        let precision_percentage = precision * 100.0;

        // Create OCPTWithMetrics object and add to list
        let ocpt_with_metrics = OCPTWithMetrics {
            ocpt: final_ocpt.clone(),
            fitness_percentage,
            precision_percentage,
            f_score,
            sequence_of_choices: sequence_of_choices.clone(),
        };
        ocpts_with_metrics.push(ocpt_with_metrics);
        
        println!("{}", "-".repeat(40));
    }

    // Find and print best OCPTs by specific metrics
    if !ocpts_with_metrics.is_empty() {
        println!("\n{}", "=".repeat(60));
        println!("BEST PERFORMING OCPTs");
        println!("{}", "=".repeat(60));
        
        // Find OCPT with best fitness
        let best_fitness_ocpt = ocpts_with_metrics.iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.fitness_percentage.partial_cmp(&b.fitness_percentage).unwrap());
        
        if let Some((best_fitness_idx, best_fitness_metrics)) = best_fitness_ocpt {
            println!("🏆 BEST FITNESS: OCPT #{} - Fitness = {:.2}%, Precision = {:.2}%, F1-Score = {:.2}%", 
                     best_fitness_idx + 1, 
                     best_fitness_metrics.fitness_percentage, 
                     best_fitness_metrics.precision_percentage,
                     best_fitness_metrics.f_score);
            println!("   Sequence of choices: {:?}", best_fitness_metrics.sequence_of_choices);
        }
        
        // Find OCPT with best precision
        let best_precision_ocpt = ocpts_with_metrics.iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.precision_percentage.partial_cmp(&b.precision_percentage).unwrap());
        
        if let Some((best_precision_idx, best_precision_metrics)) = best_precision_ocpt {
            println!("� BEST PRECISION: OCPT #{} - Fitness = {:.2}%, Precision = {:.2}%, F1-Score = {:.2}%", 
                     best_precision_idx + 1, 
                     best_precision_metrics.fitness_percentage, 
                     best_precision_metrics.precision_percentage,
                     best_precision_metrics.f_score);
            println!("   Sequence of choices: {:?}", best_precision_metrics.sequence_of_choices);
        }

        // Find OCPT with best F1 score
        let best_f_score_ocpt = ocpts_with_metrics.iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.f_score.partial_cmp(&b.f_score).unwrap());
        
        if let Some((best_f_score_idx, best_f_score_metrics)) = best_f_score_ocpt {
            println!("⚖️ BEST F1-SCORE: OCPT #{} - Fitness = {:.2}%, Precision = {:.2}%, F1-Score = {:.2}%", 
                     best_f_score_idx + 1, 
                     best_f_score_metrics.fitness_percentage, 
                     best_f_score_metrics.precision_percentage,
                     best_f_score_metrics.f_score);
            println!("   Sequence of choices: {:?}", best_f_score_metrics.sequence_of_choices);
        }
    }

    // Return summary response
    let response = serde_json::json!({
        "total_ocpts": all_final_ocpts.len(),
        "ocpts": all_final_ocpts.iter().map(|ocpt| process_forest_to_json(ocpt)).collect::<Vec<_>>(),
        "ocpts_with_metrics": ocpts_with_metrics.iter().map(|ocpt_metrics| {
            serde_json::json!({
                "ocpt": process_forest_to_json(&ocpt_metrics.ocpt),
                "fitness_percentage": ocpt_metrics.fitness_percentage,
                "precision_percentage": ocpt_metrics.precision_percentage,
                "f_score": ocpt_metrics.f_score,
                "sequence_of_choices": ocpt_metrics.sequence_of_choices,
                "average_score": (ocpt_metrics.fitness_percentage + ocpt_metrics.precision_percentage) / 2.0
            })
        }).collect::<Vec<_>>(),
        "message": format!("Generated {} distinct process forests with conformance metrics", all_final_ocpts.len())
    });

    Json(response)
}

// Handler for POST /cut-selected/:file_name
async fn cut_selected_handler(
    Path(file_name): Path<String>,
    AxumJson(payload): AxumJson<CutSelectedAPIRequest>,
) -> Json<Value> {
    process_cut_selected(file_name, payload).await
}

// Handler for POST /cut-selected (default)
async fn cut_selected_handler_default(
    AxumJson(payload): AxumJson<CutSelectedAPIRequest>,
) -> Json<Value> {
    process_cut_selected("order-management".to_string(), payload).await
}

async fn process_cut_selected(
    file_name_input: String,
    payload: CutSelectedAPIRequest,
) -> Json<Value> {
    println!("Received cut-selected request: {:?}", payload.cut_selected);

    let file_name = if file_name_input.is_empty() {
        "order-management"
    } else {
        &file_name_input
    };
    
    let mut ocpt: ProcessForest = from_json_value(&payload.ocpt);
    let mut dfg: HashMap<(String, String), usize> = json_to_dfg(&payload.dfg);
    let global_start_activities: HashSet<String> = payload.start_activities;
    let global_end_activities: HashSet<String> = payload.end_activities;
    let cut_suggestions_list: CutSuggestionsList = payload.cut_suggestions_list;
    let cut_selected: CutSuggestion = payload.cut_selected;
    let mut total_edges_removed: Vec<(String, String, usize)> = payload.total_edges_removed;
    let mut total_edges_added: Vec<(String, String, usize)> = payload.total_edges_added;
    let mut cost_to_add_edges: HashMap<(String, String), f64> = json_to_cost_to_add_edges(&payload.cost_to_add_edges);

    println!("old dfg:\n ");
    print_dfg(&dfg);


    // 1. Modify the DFG according to the selected cut
    // Remove edges that need to be cut
    for (from, to, _) in &cut_selected.edges_to_be_removed {
        dfg.remove(&(from.clone(), to.clone()));
    }
    
    // Add new edges
    for (from, to, cost) in &cut_selected.edges_to_be_added {
        dfg.insert((from.clone(), to.clone()), cost.clone());
    }

    // Remove edges from cost_to_add_edges that are no longer needed
    for (from, to, _) in &cut_selected.edges_to_be_removed {
        cost_to_add_edges.remove(&(from.clone(), to.clone()));
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

    let json_cost_to_add_edges: Value = format_conversion::cost_to_add_edges_to_json(&cost_to_add_edges);

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
        cost_to_add_edges: json_cost_to_add_edges,
        precision: 0.0,
        fitness: 0.0,
        f_score: 0.0,
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
            &cost_to_add_edges
        );
        response.cut_suggestions_list = cut_suggestions_list;
    } else {
        println!("No disjoint activities found in the OCPT");

        // // Get the modified OCPT with self-loops added
        // let (modified_ocpt, self_loop_activities) = add_self_loops(&dfg.clone(), &ocpt, file_name);
        // println!("Self-loop activities processed: {:?}", self_loop_activities);
        
        // // Update the response with the modified OCPT
        // let modified_ocpt_json_string = process_forest_to_json(&modified_ocpt);
        // response.OCPT = modified_ocpt_json_string;

        // // add code to find precision
        // println!("\n--- Precision Analysis ---");
        // let precision_percentage = conformance_checking_mine_precision(&modified_ocpt, &self_loop_activities, file_name);
        
        // println!("Final OCPT Precision: {:.2}%", precision_percentage);

        // test_conformance(response.OCPT.clone());

        

        // Call find_fitness_and_precision function
        println!("\n--- Find Fitness and Precision Analysis ---");
        let (_, _, _, _, fitness, precision, f_score) = find_fitness_and_precision(&ocpt, file_name);

        // Get the modified OCPT with self-loops added
        let (modified_ocpt, self_loop_activities) = add_self_loops(&dfg.clone(), &ocpt, file_name);
        println!("Self-loop activities processed: {:?}", self_loop_activities);
        
        // TEMP
         // Update the response with the modified OCPT
        let modified_ocpt_json_string = process_forest_to_json(&modified_ocpt);
        response.OCPT = modified_ocpt_json_string;

        response.fitness = fitness;
        response.precision = precision;
        response.f_score = f_score;
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

    println!("Configuring routes...");
    let app = Router::new()
        .route("/", get(get_initial_response_default))
        .route("/:file_name", get(get_initial_response))
        .route("/dfg", get(hello))
        .route("/all-possible-ocpts", get(all_possible_ocpts))
        .route("/cut-selected", axum::routing::post(cut_selected_handler_default))
        .route("/cut-selected/:file_name", axum::routing::post(cut_selected_handler))
        .route("/upload", axum::routing::post(upload_handler))
        .layer(DefaultBodyLimit::max(200 * 1024 * 1024)) // 200MB limit
        .layer(cors);
    
    println!("Routes configured:");
    println!("  GET /");
    println!("  GET /dfg");
    println!("  GET /all-possible-ocpts");
    println!("  POST /cut-selected");
    println!("  POST /upload");
    println!("Server running on http://localhost:1080");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:1080").await.unwrap();
    
    // Configure the server with appropriate settings for large uploads
    axum::serve(listener, app)
        .tcp_nodelay(true)
        .await
        .unwrap();
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
