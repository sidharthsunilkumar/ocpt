
use crate::types::{ProcessForest, TreeNode};
use crate::add_self_loops::get_traces;
use std::collections::{HashSet, HashMap};

fn format_traces_for_precision(traces: Vec<Vec<(String, String, String, String, String)>>, self_loop_activities: &[String]) -> Vec<Vec<String>> {
    // println!("Formatting {} traces for precision calculation...", traces.len());
    
    let formatted_traces: Vec<Vec<String>> = traces.into_iter()
        .map(|trace| {
            trace.into_iter()
                .map(|(_, activity, _, _, _)| activity)
                .collect()
        })
        .collect();
    
    // println!("Formatted traces:");
    // for (i, trace) in formatted_traces.iter().enumerate() {
    //     println!("Trace {}: {:?}", i + 1, trace);
    // }

    // keep only unique traces
    let original_count = formatted_traces.len();
    let unique_traces: Vec<Vec<String>> = {
        let mut seen = HashSet::new();
        formatted_traces.into_iter()
            .filter(|trace| seen.insert(trace.clone()))
            .collect()
    };

    // print unique traces
    // for (i, trace) in unique_traces.iter().enumerate() {
    //     println!("Unique Trace {}: {:?}", i + 1, trace);
    // }

    // code - remove consecutive duplicate self-loop activities
    let self_loop_set: HashSet<String> = self_loop_activities.iter().cloned().collect();
    
    let cleaned_traces: Vec<Vec<String>> = unique_traces.into_iter()
        .map(|trace| {
            let mut cleaned_trace = Vec::new();
            let mut prev_activity: Option<String> = None;
            
            for activity in trace {
                // If this activity is a self-loop activity and it's the same as the previous one, skip it
                if let Some(ref prev) = prev_activity {
                    if self_loop_set.contains(&activity) && &activity == prev {
                        continue; // Skip consecutive duplicate self-loop activity
                    }
                }
                
                cleaned_trace.push(activity.clone());
                prev_activity = Some(activity);
            }
            
            cleaned_trace
        })
        .collect();
    
    // println!("Cleaned {} traces by removing consecutive self-loop duplicates", cleaned_traces.len());
    // println!("Self-loop activities considered: {:?}", self_loop_activities);
    
    // Generate additional traces with self-loop repetitions
    let mut expanded_traces = cleaned_traces.clone();
    
    for self_loop_activity in self_loop_activities {
        for trace in &cleaned_traces {
            // Find all positions where this self-loop activity occurs
            let positions: Vec<usize> = trace.iter().enumerate()
                .filter_map(|(i, activity)| {
                    if activity == self_loop_activity {
                        Some(i)
                    } else {
                        None
                    }
                })
                .collect();
            
            // For each position, generate two additional traces with extra repetitions
            for &pos in &positions {
                // Generate trace with one additional repetition (e.g., C -> C,C)
                let mut trace_with_one_extra = trace.clone();
                trace_with_one_extra.insert(pos + 1, self_loop_activity.clone());
                expanded_traces.push(trace_with_one_extra);
                
                // Generate trace with two additional repetitions (e.g., C -> C,C,C)
                let mut trace_with_two_extra = trace.clone();
                trace_with_two_extra.insert(pos + 1, self_loop_activity.clone());
                trace_with_two_extra.insert(pos + 2, self_loop_activity.clone());
                expanded_traces.push(trace_with_two_extra);
            }
        }
    }
    
    println!("Generated {} total traces (original {} + expanded {})", 
             expanded_traces.len(), cleaned_traces.len(), expanded_traces.len() - cleaned_traces.len());
    
    println!("Kept {} unique traces out of original {} traces", cleaned_traces.len(), original_count);
    
    expanded_traces
}

pub fn conformance_checking_mine_precision(ocpt: &ProcessForest, self_loop_activities: &[String], file_name: &str) -> f64 {
    println!("Calculating all possible executions...");
    
    if ocpt.is_empty() {
        println!("Empty process tree, returning empty result");
        return 0.0;
    }
    
    let all_executions = generate_all_executions(&ocpt[0]);
    
    println!("Total possible executions: {}", all_executions.len());
    // for (i, execution) in all_executions.iter().enumerate() {
    //     println!("Execution {}: {:?}", i + 1, execution);
    // }

    // Get traces and format them for precision calculation
    let raw_traces = get_traces(file_name);
    let traces = format_traces_for_precision(raw_traces, &self_loop_activities);

    // Check which executions are present in traces
    let mut executions_found_in_traces = 0;
    let mut executions_not_found = Vec::new();

    println!("\n--- Checking executions against traces ---");
    for (i, execution) in all_executions.iter().enumerate() {
        let found_in_traces = traces.contains(execution);
        if found_in_traces {
            executions_found_in_traces += 1;
            // println!("✓ Execution {}: {:?} - FOUND in traces", i + 1, execution);
        } else {
            executions_not_found.push(execution.clone());
            // println!("✗ Execution {}: {:?} - NOT FOUND in traces", i + 1, execution);
        }
    }

    // Print executions not found in traces
    if !executions_not_found.is_empty() {
        println!("\n--- Executions NOT found in traces ({}) ---", executions_not_found.len());
        // for (i, execution) in executions_not_found.iter().enumerate() {
        //     println!("Missing {}: {:?}", i + 1, execution);
        // }
    } else {
        println!("\n✓ All possible executions are present in the traces!");
    }

    // Calculate and print precision value
    let total_executions = all_executions.len();
    let precision_percentage = if total_executions > 0 {
        (executions_found_in_traces as f64 / total_executions as f64) * 100.0
    } else {
        0.0
    };

    println!("\n--- Precision Results ---");
    println!("Total possible executions: {}", total_executions);
    println!("Executions found in traces: {}", executions_found_in_traces);
    println!("Executions not found in traces: {}", executions_not_found.len());
    println!("Precision value: {:.2}%", precision_percentage);

    
    precision_percentage
}

fn generate_all_executions(node: &TreeNode) -> Vec<Vec<String>> {
    // Control flow operators to exclude from executions
    let control_flow_operators = ["sequence", "parallel", "exclusive", "redo", "tau"];
    
    // If this is a leaf node (activity) and not a control flow operator
    if node.children.is_empty() && !control_flow_operators.contains(&node.label.as_str()) {
        return vec![vec![node.label.clone()]];
    }
    
    // If it's a tau (silent) node, return empty execution
    if node.label == "tau" {
        return vec![vec![]];
    }
    
    // Handle control flow nodes
    match node.label.as_str() {
        "sequence" => {
            if node.children.len() >= 2 {
                let left_executions = generate_all_executions(&node.children[0]);
                let right_executions = generate_all_executions(&node.children[1]);
                
                let mut result = Vec::new();
                for left_exec in &left_executions {
                    for right_exec in &right_executions {
                        let mut combined = left_exec.clone();
                        combined.extend(right_exec.clone());
                        result.push(combined);
                    }
                }
                result
            } else {
                vec![vec![]]
            }
        },
        "exclusive" => {
            if node.children.len() >= 2 {
                let mut result = Vec::new();
                let left_executions = generate_all_executions(&node.children[0]);
                let right_executions = generate_all_executions(&node.children[1]);
                
                result.extend(left_executions);
                result.extend(right_executions);
                result
            } else {
                vec![vec![]]
            }
        },
        "parallel" => {
            if node.children.len() >= 2 {
                let left_executions = generate_all_executions(&node.children[0]);
                let right_executions = generate_all_executions(&node.children[1]);
                
                let mut result = Vec::new();
                for left_exec in &left_executions {
                    for right_exec in &right_executions {
                        // Generate all interleavings of left and right executions
                        let interleavings = generate_interleavings(left_exec, right_exec);
                        result.extend(interleavings);
                    }
                }
                result
            } else {
                vec![vec![]]
            }
        },
        "redo" => {
            if node.children.len() >= 2 {
                let left_executions = generate_all_executions(&node.children[0]);
                let right_executions = generate_all_executions(&node.children[1]);
                
                let mut result = Vec::new();
                
                // Possibility 1: Only left child executes
                result.extend(left_executions.clone());
                
                // Possibility 2: left -> right -> left
                for left_exec1 in &left_executions {
                    for right_exec in &right_executions {
                        for left_exec2 in &left_executions {
                            let mut combined = left_exec1.clone();
                            combined.extend(right_exec.clone());
                            combined.extend(left_exec2.clone());
                            result.push(combined);
                        }
                    }
                }
                
                // Possibility 3: left -> right -> left -> right -> left
                for left_exec1 in &left_executions {
                    for right_exec1 in &right_executions {
                        for left_exec2 in &left_executions {
                            for right_exec2 in &right_executions {
                                for left_exec3 in &left_executions {
                                    let mut combined = left_exec1.clone();
                                    combined.extend(right_exec1.clone());
                                    combined.extend(left_exec2.clone());
                                    combined.extend(right_exec2.clone());
                                    combined.extend(left_exec3.clone());
                                    result.push(combined);
                                }
                            }
                        }
                    }
                }
                result
            } else {
                vec![vec![]]
            }
        },
        _ => {
            // For unknown operators or leaf nodes with children, return empty
            vec![vec![]]
        }
    }
}

fn generate_interleavings(seq1: &[String], seq2: &[String]) -> Vec<Vec<String>> {
    if seq1.is_empty() {
        return vec![seq2.to_vec()];
    }
    if seq2.is_empty() {
        return vec![seq1.to_vec()];
    }
    
    let mut result = Vec::new();
    
    // Take first element from seq1 and interleave with rest
    let mut with_first_from_seq1 = vec![seq1[0].clone()];
    let remaining_seq1 = &seq1[1..];
    let sub_interleavings = generate_interleavings(remaining_seq1, seq2);
    
    for sub_interleaving in sub_interleavings {
        let mut combined = with_first_from_seq1.clone();
        combined.extend(sub_interleaving);
        result.push(combined);
    }
    
    // Take first element from seq2 and interleave with rest
    let mut with_first_from_seq2 = vec![seq2[0].clone()];
    let remaining_seq2 = &seq2[1..];
    let sub_interleavings = generate_interleavings(seq1, remaining_seq2);
    
    for sub_interleaving in sub_interleavings {
        let mut combined = with_first_from_seq2.clone();
        combined.extend(sub_interleaving);
        result.push(combined);
    }
    
    result
}

pub fn conformance_checking_mine_fitness(ocpt: &ProcessForest, file_name: &str) -> f64 {
    println!("Starting conformance checking...");

    // Get traces once outside the loop to avoid multiple calls
    let all_traces = get_traces(file_name);
    let total_traces = all_traces.len();
    let mut incorrect_traces = 0;

    for trace in &all_traces {
        if !ocpt.is_empty() {
            if !check_node(&ocpt[0], trace) {
                incorrect_traces += 1;
            }
        }
    }

    println!("Total traces: {}", total_traces);
    println!("Incorrect traces: {}", incorrect_traces);
    
    if total_traces > 0 {
        let fitness_percentage = (incorrect_traces as f64 / total_traces as f64) * 100.0;
        println!("My fitness value is {:.2}%", fitness_percentage);
        fitness_percentage
    } else {
        println!("No traces found");
        0.0
    }
}

fn check_node(node: &TreeNode, trace: &Vec<(String, String, String, String, String)>) -> bool {
    if node.label == "sequence" {
        if node.children.len() >= 2 {
            let left_activities = get_descendant_activities(&node.children[0]);
            let right_activities = get_descendant_activities(&node.children[1]);
            
            // Check if any left or right activity occurs in the trace
            if trace_contains_any_activity(trace, &left_activities) || 
               trace_contains_any_activity(trace, &right_activities) {
                
                if !check_sequence(&left_activities, &right_activities, trace) {
                    return false;
                } else {
                    // Check both children recursively
                    return check_node(&node.children[0], trace) && 
                           check_node(&node.children[1], trace);
                }
            }
        }
    } else if node.label == "exclusive" {
        if node.children.len() >= 2 {
            let left_activities = get_descendant_activities(&node.children[0]);
            let right_activities = get_descendant_activities(&node.children[1]);
            
            // Check if any left or right activity occurs in the trace
            if trace_contains_any_activity(trace, &left_activities) || 
               trace_contains_any_activity(trace, &right_activities) {
                
                if !check_exclusive(&left_activities, &right_activities, trace) {
                    return false;
                } else {
                    // Check both children recursively
                    return check_node(&node.children[0], trace) && 
                           check_node(&node.children[1], trace);
                }
            }
        }
    } 
    // else if node.label == "parallel" {
    //     // For parallel nodes, all children should be valid
    //     for child in &node.children {
    //         if !check_node(child, trace) {
    //             return false;
    //         }
    //     }
    // } else if node.label == "redo" {
    //     // For redo nodes, check all children
    //     for child in &node.children {
    //         if !check_node(child, trace) {
    //             return false;
    //         }
    //     }
    // }
    // For leaf nodes (activities) or other cases, return true
    true
}

fn check_sequence(left_activities: &HashSet<String>, right_activities: &HashSet<String>, 
                 trace: &Vec<(String, String, String, String, String)>) -> bool {
    // Find all positions of left and right activities
    let mut left_positions = Vec::new();
    let mut right_positions = Vec::new();

    for (i, (_, activity, _, _, _)) in trace.iter().enumerate() {
        if left_activities.contains(activity) {
            left_positions.push(i);
        }
        if right_activities.contains(activity) {
            right_positions.push(i);
        }
    }

    // If we have both left and right activities, check sequence constraint
    if !left_positions.is_empty() && !right_positions.is_empty() {
        // For sequence, every right activity should come after at least one left activity
        // Check if any right activity comes before all left activities
        let earliest_right = right_positions.iter().min().unwrap();
        let latest_left = left_positions.iter().max().unwrap();
        
        // If the earliest right activity comes before the latest left activity, sequence is violated
        return latest_left < earliest_right;
    }

    // If only one side or neither side has activities, it's valid
    true
}

fn check_exclusive(left_activities: &HashSet<String>, right_activities: &HashSet<String>, 
                  trace: &Vec<(String, String, String, String, String)>) -> bool {
    // If any right activity and any left activity both exist in the trace, return false
    let has_left = trace.iter().any(|(_, activity, _, _, _)| left_activities.contains(activity));
    let has_right = trace.iter().any(|(_, activity, _, _, _)| right_activities.contains(activity));

    // Return false if both sides have activities (exclusive violation)
    !(has_left && has_right)
}

fn get_descendant_activities(node: &TreeNode) -> HashSet<String> {
    let mut activities = HashSet::new();
    
    // Control flow operators to exclude
    let control_flow_operators = ["sequence", "parallel", "exclusive", "redo", "tau"];
    
    // If this is a leaf node (activity) and not a control flow operator, add its label
    if node.children.is_empty() && !control_flow_operators.contains(&node.label.as_str()) {
        activities.insert(node.label.clone());
    } else {
        // Recursively get activities from all children
        for child in &node.children {
            activities.extend(get_descendant_activities(child));
        }
    }
    
    activities
}

fn trace_contains_any_activity(trace: &Vec<(String, String, String, String, String)>, 
                              activities: &HashSet<String>) -> bool {
    trace.iter().any(|(_, activity, _, _, _)| activities.contains(activity))
}

fn remove_consecutive_self_loops(trace: Vec<String>) -> Vec<String> {
    if trace.is_empty() {
        return trace;
    }
    
    let mut cleaned_trace = Vec::new();
    let mut prev_activity: Option<String> = None;
    
    for activity in trace {
        // Only add the activity if it's different from the previous one
        if let Some(ref prev) = prev_activity {
            if &activity != prev {
                cleaned_trace.push(activity.clone());
            }
        } else {
            // First activity, always add it
            cleaned_trace.push(activity.clone());
        }
        prev_activity = Some(activity);
    }
    
    cleaned_trace
}

pub fn find_fitness_and_precision(ocpt: &ProcessForest, file_name: &str) -> (usize, usize, usize, usize, f64, f64, f64) {
    println!("Starting find_fitness_and_precision...");
    
    // Get traces from the file
    let raw_traces = get_traces(file_name);
    let total_traces = raw_traces.len();
    
    // Step 1: Format traces to extract only activity names
    let activity_traces: Vec<Vec<String>> = raw_traces.into_iter()
        .map(|trace| {
            trace.into_iter()
                .map(|(_, activity, _, _, _)| activity)
                .collect()
        })
        .collect();
    
    // Step 2: Remove consecutive self-loops from each trace
    let traces: Vec<Vec<String>> = activity_traces.into_iter()
        .map(|trace| remove_consecutive_self_loops(trace))
        .collect();
    
    // Check for activities that appear multiple times in traces (after self-loop removal)
    println!("\n--- Checking for repeated activities in traces (after self-loop removal) ---");
    let mut traces_with_repeats = 0;
    let mut total_repeat_instances = 0;
    
    for (trace_idx, trace) in traces.iter().enumerate() {
        let mut activity_counts: HashMap<String, usize> = HashMap::new();
        
        // Count occurrences of each activity in this trace
        for activity in trace {
            *activity_counts.entry(activity.clone()).or_insert(0) += 1;
        }
        
        // Check for activities that appear more than once
        let repeated_activities: Vec<(String, usize)> = activity_counts.iter()
            .filter(|(_, count)| **count > 1)
            .map(|(activity, count)| (activity.clone(), *count))
            .collect();
        
        if !repeated_activities.is_empty() {
            traces_with_repeats += 1;
            total_repeat_instances += repeated_activities.len();
            
            println!("Trace #{}: Activities appearing multiple times:", trace_idx + 1);
            for (activity, count) in &repeated_activities {
                println!("  - '{}' appears {} times", activity, count);
            }
            println!("  Full trace: {:?}", trace);
        }
    }
    
    if traces_with_repeats > 0 {
        println!("Summary: {} out of {} traces contain repeated activities", traces_with_repeats, traces.len());
        println!("Total repeated activity instances found: {}", total_repeat_instances);
    } else {
        println!("No traces contain repeated activities");
    }
    
    // Remove traces that contain repeated activities
    let original_traces_count = traces.len();
    let filtered_traces: Vec<Vec<String>> = traces.into_iter()
        .filter(|trace| {
            let mut activity_counts: HashMap<String, usize> = HashMap::new();
            
            // Count occurrences of each activity in this trace
            for activity in trace {
                *activity_counts.entry(activity.clone()).or_insert(0) += 1;
            }
            
            // Keep only traces without repeated activities
            !activity_counts.values().any(|&count| count > 1)
        })
        .collect();
    
    println!("Removed {} traces with repeated activities. {} traces remaining.", 
             original_traces_count - filtered_traces.len(), filtered_traces.len());
    println!("--- End of repeated activities check ---\n");
    
    // Generate all possible executions from the model
    let all_executions = if ocpt.is_empty() {
        Vec::new()
    } else {
        generate_all_executions(&ocpt[0])
    };
    
    let total_executions = all_executions.len();
    
    // Calculate x: number of executions for which a trace also exists
    let mut x = 0;
    for execution in &all_executions {
        if filtered_traces.contains(execution) {
            x += 1;
        }
    }
    
    // Calculate t: number of traces for which a possible execution exists
    let mut t = 0;
    for trace in &filtered_traces {
        if all_executions.contains(trace) {
            t += 1;
        }
    }

    //Fitness and Precision calculations
    let precision = if total_executions > 0 {
        x as f64 / total_executions as f64
    } else {
        0.0
    };

    let fitness = if total_traces > 0 {
        t as f64 / total_traces as f64
    } else {
        0.0
    };

    // Calculate F1 Score = 2 * (Precision * Fitness) / (Precision + Fitness)
    let f_score = if (precision + fitness) > 0.0 {
        2.0 * (precision * fitness) / (precision + fitness)
    } else {
        0.0
    };

    // Print the results
    println!("=== Find Fitness and Precision Results ===");
    println!("Total number of executions: {}", total_executions);
    println!("Total number of traces: {}", total_traces);
    // println!("Total number of unique traces: {}", unique_traces.len());
    println!("x (executions with corresponding trace): {}", x);
    println!("t (traces with corresponding execution): {}", t);
    println!("Fitness: {:.5}", fitness);
    println!("Precision: {:.5}", precision);
    println!("F-Score: {:.5}", f_score);
    println!("==========================================");
    
    // Return all calculated variables
    (total_executions, total_traces, x, t, fitness, precision, f_score)
}

