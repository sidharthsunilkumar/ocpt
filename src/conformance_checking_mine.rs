
use crate::types::{ProcessForest, TreeNode};
use crate::add_self_loops::get_traces;
use std::collections::HashSet;

pub fn conformance_checking_mine_fitness(ocpt: &ProcessForest) {
    println!("Starting conformance checking...");

    // Get traces once outside the loop to avoid multiple calls
    let all_traces = get_traces();
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
    } else {
        println!("No traces found");
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