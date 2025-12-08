use std::collections::{HashMap, HashSet};
use std::fs as stdfs;
use crate::types::{OCEL, ProcessForest, TreeNode};
use crate::build_relations_fns;
use crate::interaction_patterns;

/// Adds self-loops to a DFG and prints it
/// 
/// This function takes a DFG (Directly-Follows Graph), OCPT (Process Forest), and file name and prints their structure,
/// including all nodes and edges. Returns the modified OCPT with self-loops added and the list of self-loop activities.
pub fn add_self_loops(dfg: &HashMap<(String, String), usize>, ocpt: &ProcessForest, file_name: &str) -> (ProcessForest, Vec<String>) {
    println!("Checking for self-loops in DFG...");
    
    // Extract unique activities (nodes) from the DFG
    let mut activities = std::collections::HashSet::new();
    for ((from, to), _) in dfg {
        activities.insert(from.clone());
        activities.insert(to.clone());
    }
    
    // Find self-loops
    let self_loops: Vec<_> = dfg.iter()
        .filter(|((from, to), _)| from == to)
        .collect();
    
    // create variable self_loop_activities and collect the from the self_loops
    let self_loop_activities: Vec<String> = self_loops.iter()
        .map(|((from, _), _)| from.clone())
        .collect();
    
    println!("Found {} self-loop(s): {:?}", self_loop_activities.len(), self_loop_activities);
    
    // Adding self loop algorithm
    if self_loop_activities.is_empty() {
        println!("No self-loops found. Returning original OCPT.");
        return (ocpt.clone(), self_loop_activities);
    }
    
    println!("Processing {} self-loop(s) and modifying OCPT...", self_loop_activities.len());
    
    // Get traces once outside the loop to avoid multiple calls
    let all_traces = get_traces(file_name);
    
    // Start with the original OCPT and progressively modify it
    let mut current_ocpt = ocpt.clone();
    let mut processed_count = 0;

    // For each self-loop activity
    for self_loop_activity in &self_loop_activities {
        // Step 1: Traverse the OCPT, find the parent node of the self-loop activity, and get all of its children other than the self-loop activity
        if let Some((parent_node, self_loop_activity_siblings_and_its_decendants)) = find_parent_and_siblings(&current_ocpt, self_loop_activity) {
            
            // Step 2: if parent node is 'sequence' or 'parallel', get all the traces containing any of the descendants. check if self-loop activity is present in all of those traces. if so, create 3 nodes - 'redo', 'tau', and the self loop activity. the redo must be the parent whose first child should be the self-loop activity and second child should be 'tau'. replace the original self loop activity with the 'redo' node in the OCPT. but if the self loop activity is not ovsered in any 1 of those traces, make the first child of the parent 'redo' as 'tau' and second child as the self-loop activity.
            if matches!(parent_node.as_str(), "sequence" | "parallel") {
                // Get all traces containing any of the descendants
                let relevant_traces: Vec<_> = all_traces.iter()
                    .filter(|trace| {
                        trace.iter().any(|event| self_loop_activity_siblings_and_its_decendants.contains(&event.1))
                    })
                    .collect();
                
                // Check if self-loop activity is present in ALL of those traces
                let self_loop_in_all_traces = relevant_traces.iter()
                    .all(|trace| {
                        trace.iter().any(|event| &event.1 == self_loop_activity)
                    });
                
                if self_loop_in_all_traces {
                    current_ocpt = modify_ocpt_with_redo(&current_ocpt, self_loop_activity, true);
                } else {
                    current_ocpt = modify_ocpt_with_redo(&current_ocpt, self_loop_activity, false);
                }
                processed_count += 1;
            } else if matches!(parent_node.as_str(), "exclusive") {
                let (first_group, second_group) = find_descendants_of_non_exclusive_ancestor(&current_ocpt, self_loop_activity).unwrap();

                let other_branch_activities_of_pseudo_root = second_group;
                
                // Create set for efficient lookup of items to exclude
                let mut exclude_set: HashSet<String> = HashSet::new();
                exclude_set.insert(self_loop_activity.clone());
                for activity in &self_loop_activity_siblings_and_its_decendants {
                    exclude_set.insert(activity.clone());
                }
                
                let self_loop_activity_ancestors_of_same_branch: Vec<String> = first_group
                    .into_iter()
                    .filter(|activity| !exclude_set.contains(activity))
                    .collect();
                
                // Create relevant_traces based on other_branch_activities_of_pseudo_root
                let mut relevant_traces: Vec<_> = if other_branch_activities_of_pseudo_root.is_empty() {
                    // If other_branch_activities_of_pseudo_root is empty, use all traces
                    all_traces.iter().collect()
                } else {
                    // Filter traces containing any activity from other_branch_activities_of_pseudo_root
                    all_traces.iter()
                        .filter(|trace| {
                            trace.iter().any(|event| other_branch_activities_of_pseudo_root.contains(&event.1))
                        })
                        .collect()
                };
                
                // Further filter relevant_traces by removing traces that contain any activity from self_loop_activity_ancestors_of_same_branch
                // but only if self_loop_activity_ancestors_of_same_branch is not empty
                if !self_loop_activity_ancestors_of_same_branch.is_empty() {
                    relevant_traces = relevant_traces
                        .into_iter()
                        .filter(|trace| {
                            // Keep traces that do NOT contain any activity from self_loop_activity_ancestors_of_same_branch
                            !trace.iter().any(|event| self_loop_activity_ancestors_of_same_branch.contains(&event.1))
                        })
                        .collect();
                }
                
                // Further filter relevant_traces by removing traces that contain any activity from self_loop_activity_siblings_and_its_decendants
                // but only if self_loop_activity_siblings_and_its_decendants is not empty
                if !self_loop_activity_siblings_and_its_decendants.is_empty() {
                    relevant_traces = relevant_traces
                        .into_iter()
                        .filter(|trace| {
                            // Keep traces that do NOT contain any activity from self_loop_activity_siblings_and_its_decendants
                            !trace.iter().any(|event| self_loop_activity_siblings_and_its_decendants.contains(&event.1))
                        })
                        .collect();
                }
                
                // Check if relevant_traces is empty or if every trace contains the self-loop activity
                if relevant_traces.is_empty() {
                    current_ocpt = modify_ocpt_with_redo(&current_ocpt, self_loop_activity, true);
                } else {
                    // Check if self-loop activity is present in ALL of the relevant traces
                    let self_loop_in_all_relevant_traces = relevant_traces.iter()
                        .all(|trace| {
                            trace.iter().any(|event| &event.1 == self_loop_activity)
                        });
                    
                    if self_loop_in_all_relevant_traces {
                        current_ocpt = modify_ocpt_with_redo(&current_ocpt, self_loop_activity, true);
                    } else {
                        current_ocpt = modify_ocpt_with_redo(&current_ocpt, self_loop_activity, false);
                    }
                }
                processed_count += 1;





            }
        }
    }
    
    // Return the final modified OCPT and the self-loop activities
    println!("Successfully processed and added {} self-loop(s) to OCPT.", processed_count);
    (current_ocpt, self_loop_activities)
}
pub fn get_traces(file_name: &str) -> Vec<Vec<(String, String, String, String, String)>> {
    let file_path = format!("data/{}.json", file_name);

    let file_content = stdfs::read_to_string(&file_path).unwrap();
    let ocel: OCEL = serde_json::from_str(&file_content).unwrap();

    let relations = build_relations_fns::build_relations(&ocel.events, &ocel.objects);

    let (divergent_objects, con, rel, defi, all_activities, all_object_types) =
        interaction_patterns::get_interaction_patterns(&relations, &ocel);

    // Group relations by object ID (oid)
    let mut grouped_relations: HashMap<String, Vec<(String, String, String, String, String)>> = HashMap::new();
    
    for relation in relations {
        grouped_relations
            .entry(relation.3.clone()) // oid is at index 3
            .or_insert_with(Vec::new)
            .push(relation);
    }

    // Step 1: Create empty array called traces
    let mut traces: Vec<Vec<(String, String, String, String, String)>> = Vec::new();


    // Step 2 & 3: Sort every list inside grouped_relations by timestamp and put in traces
    // (Commented out for testing with hardcoded data)
    
    for (_oid, mut relations_group) in grouped_relations {
        // Sort by timestamp (index 2)
        relations_group.sort_by(|a, b| a.2.cmp(&b.2));
        
        // The relations_group already contains owned data, no need to clone
        let sorted_trace: Vec<(String, String, String, String, String)> = relations_group;
        
        // Check if any two event types in the trace have common divergent objects
        let mut should_skip_trace = false;
        let mut trace_object_types: HashSet<String> = HashSet::new();
        let mut trace_event_types: HashSet<String> = HashSet::new();
        
        // Collect all object types and event types in this trace
        for event in &sorted_trace {
            trace_object_types.insert(event.4.clone()); // object_type is at index 4
            trace_event_types.insert(event.1.clone()); // event_type is at index 1
        }
        
        // Check if any two event types in the trace has any common divergent_objects, 
        // and the trace in question is dealing with those common divergent object types
        let event_types_vec: Vec<String> = trace_event_types.into_iter().collect();
        'outer: for i in 0..event_types_vec.len() {
            for j in (i + 1)..event_types_vec.len() {
                let event_type1 = &event_types_vec[i];
                let event_type2 = &event_types_vec[j];
                
                if let (Some(divergent_types1), Some(divergent_types2)) = 
                    (divergent_objects.get(event_type1), divergent_objects.get(event_type2)) {
                    
                    // Find common divergent object types between the two event types
                    let common_divergent_types: HashSet<_> = divergent_types1
                        .iter()
                        .filter(|&dt| divergent_types2.contains(dt))
                        .collect();
                    
                    // Check if any of the common divergent types are present in the trace
                    for common_divergent_type in &common_divergent_types {
                        if trace_object_types.contains(*common_divergent_type) {
                            should_skip_trace = true;
                            break 'outer;
                        }
                    }
                }
            }
        }
        
        // Only push the trace if it doesn't have divergent object types
        if !should_skip_trace {
            traces.push(sorted_trace);
        }
    }
    
    traces
}

fn find_parent_and_siblings(forest: &ProcessForest, target_activity: &str) -> Option<(String, Vec<String>)> {
    for tree in forest {
        if let Some(result) = find_parent_and_siblings_recursive(tree, target_activity) {
            return Some(result);
        }
    }
    None
}

fn find_parent_and_siblings_recursive(node: &TreeNode, target_activity: &str) -> Option<(String, Vec<String>)> {
    // Check if any direct child matches the target activity
    for child in &node.children {
        if child.label == target_activity {
            // Found the target activity as a direct child
            let mut self_loop_activity_siblings_and_its_decendants = Vec::new();
            
            // Collect all descendants from all children except the target activity
            for sibling in &node.children {
                if sibling.label != target_activity {
                    collect_all_descendants(sibling, &mut self_loop_activity_siblings_and_its_decendants);
                }
            }
            
            return Some((node.label.clone(), self_loop_activity_siblings_and_its_decendants));
        }
    }
    
    // Recursively search in children
    for child in &node.children {
        if let Some(result) = find_parent_and_siblings_recursive(child, target_activity) {
            return Some(result);
        }
    }
    
    None
}

fn collect_all_descendants(node: &TreeNode, descendants: &mut Vec<String>) {
    // Add the current node only if it's not a control flow node and not 'tau'
    if !matches!(node.label.as_str(), "parallel" | "sequence" | "redo" | "exclusive" | "tau") {
        descendants.push(node.label.clone());
    }
    
    // Recursively add all children
    for child in &node.children {
        collect_all_descendants(child, descendants);
    }
}

fn modify_ocpt_with_redo(ocpt: &ProcessForest, self_loop_activity: &str, self_loop_first: bool) -> ProcessForest {
    let mut modified_ocpt = ocpt.clone();
    
    for tree in &mut modified_ocpt {
        modify_tree_with_redo(tree, self_loop_activity, self_loop_first);
    }
    
    modified_ocpt
}

fn modify_tree_with_redo(node: &mut TreeNode, self_loop_activity: &str, self_loop_first: bool) -> bool {
    // Check if any direct child matches the self-loop activity
    for (i, child) in node.children.iter().enumerate() {
        if child.label == self_loop_activity {
            // Found the self-loop activity as a direct child
            let self_loop_node = TreeNode {
                label: self_loop_activity.to_string(),
                children: vec![],
            };
            
            let tau_node = TreeNode {
                label: "tau".to_string(),
                children: vec![],
            };
            
            let redo_children = if self_loop_first {
                vec![self_loop_node, tau_node]
            } else {
                vec![tau_node, self_loop_node]
            };
            
            let redo_node = TreeNode {
                label: "redo".to_string(),
                children: redo_children,
            };
            
            // Replace the original self-loop activity with the redo node
            node.children[i] = redo_node;
            return true;
        }
    }
    
    // Recursively search and modify children
    for child in &mut node.children {
        if modify_tree_with_redo(child, self_loop_activity, self_loop_first) {
            return true;
        }
    }
    
    false
}

fn find_descendants_of_non_exclusive_ancestor(ocpt: &ProcessForest, target_node_label: &str) -> Option<(Vec<String>, Vec<String>)> {
    for tree in ocpt {
        if let Some(result) = find_non_exclusive_ancestor_recursive(tree, target_node_label, &vec![]) {
            return Some(result);
        }
    }
    
    // If no non-exclusive ancestor found, return all activities as first_group and empty second_group
    for tree in ocpt {
        if contains_target_node(tree, target_node_label) {
            let mut first_group = Vec::new();
            let second_group = Vec::new();  // Empty second group
            
            // Collect all activities from the entire tree
            collect_filtered_descendants(tree, &mut first_group);
            
            return Some((first_group, second_group));
        }
    }
    
    None
}

fn find_non_exclusive_ancestor_recursive(
    node: &TreeNode, 
    target_label: &str, 
    path: &Vec<&TreeNode>
) -> Option<(Vec<String>, Vec<String>)> {
    // Check if current node matches the target
    if node.label == target_label {
        // Go up the path to find first non-exclusive ancestor
        for ancestor in path.iter().rev() {
            if ancestor.label != "exclusive" {
                // Found non-exclusive ancestor, get descendants of its first two children
                if ancestor.children.len() >= 2 {
                    let mut first_group = Vec::new();
                    let mut second_group = Vec::new();
                    
                    // Collect descendants from first child
                    collect_filtered_descendants(&ancestor.children[0], &mut first_group);
                    
                    // Collect descendants from second child
                    collect_filtered_descendants(&ancestor.children[1], &mut second_group);
                    
                    return Some((first_group, second_group));
                }
            }
        }
        
        // If no non-exclusive ancestor found in path, check if current node itself is non-exclusive
        if node.label != "exclusive" && node.children.len() >= 2 {
            let mut first_group = Vec::new();
            let mut second_group = Vec::new();
            
            collect_filtered_descendants(&node.children[0], &mut first_group);
            collect_filtered_descendants(&node.children[1], &mut second_group);
            
            return Some((first_group, second_group));
        }
    }
    
    // Continue searching in children, adding current node to path
    let mut new_path = path.clone();
    new_path.push(node);
    
    for child in &node.children {
        if let Some(result) = find_non_exclusive_ancestor_recursive(child, target_label, &new_path) {
            return Some(result);
        }
    }
    
    None
}

fn collect_filtered_descendants(node: &TreeNode, descendants: &mut Vec<String>) {
    // Add the current node only if it's not a control flow node and not 'tau'
    if !matches!(node.label.as_str(), "parallel" | "sequence" | "redo" | "exclusive" | "tau") {
        descendants.push(node.label.clone());
    }
    
    // Recursively add all children
    for child in &node.children {
        collect_filtered_descendants(child, descendants);
    }
}

fn contains_target_node(node: &TreeNode, target_label: &str) -> bool {
    if node.label == target_label {
        return true;
    }
    
    for child in &node.children {
        if contains_target_node(child, target_label) {
            return true;
        }
    }
    
    false
}


