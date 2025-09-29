use std::collections::{HashMap, HashSet};
use crate::{cost_to_add, cost_to_cut::is_reachable};
use log::info;

// Returns:
// 0: bool - whether the redo cut was successful
// 1: usize - total cost (cost of edges added + cost of edges removed)
// 2: Vec<(String, String, usize)> - list of edges that were removed with their costs
// 3: Vec<(String, String, usize)> - list of edges that were added with their costs
// 4: usize - cost of edges added
// 5: usize - cost of edges removed
// 6: HashSet<String> - set1 (first partition)
// 7: HashSet<String> - set2 (second partition)
// 8: HashMap<(String, String), usize> - final DFG after modifications
pub fn best_redo_cut(
    dfg: &HashMap<(String, String), usize>, 
    all_activities: &HashSet<String>,
    start_activities: &HashSet<String>,
    end_activities: &HashSet<String>,
    cost_to_add_edges: &HashMap<(String, String), f64>
) -> (bool, usize, Vec<(String, String, usize)>, Vec<(String, String, usize)>, usize, usize, HashSet<String>, HashSet<String>, HashMap<(String, String), usize>) {

    let mut set1: HashSet<String> = HashSet::new();
    let mut set2: HashSet<String> = HashSet::new();
    let mut current_dfg = dfg.clone();
    let mut total_cost = 0usize;
    let mut cost_of_edges_added = 0usize;
    let mut cost_of_edges_removed = 0usize;
    let mut edges_removed: Vec<(String, String, usize)> = Vec::new();
    let mut edges_added: Vec<(String, String, usize)> = Vec::new();

    // Step 1: Create start_to_end_activity_pairs
    let mut start_to_end_activity_pairs: Vec<(String, String)> = Vec::new();
    for s in start_activities {
        for e in end_activities {
            if is_reachable(&dfg, &s, &e) {
                start_to_end_activity_pairs.push((s.clone(), e.clone()));
            }
        }
    }

    // Step 2: Create end_to_start_activity_pairs
    let mut end_to_start_activity_pairs: Vec<(String, String)> = Vec::new();
    for e in end_activities {
        for s in start_activities {
            if is_reachable(&dfg, &e, &s) {
                end_to_start_activity_pairs.push((e.clone(), s.clone()));
            }
        }
    }

    // Step 3: Check if all start-end and end-start pairs are reachable
    for s in start_activities {
        for e in end_activities {
            if !is_reachable(&dfg, &s, &e) || !is_reachable(&dfg, &e, &s) {
                println!("Redo cut not possible: {} to {} or {} to {}", s, e, e, s);
                return (false, 0, Vec::new(), Vec::new(), 0, 0, HashSet::new(), HashSet::new(), dfg.clone());
            }
        }
    }

    // Step 4: Put all start and end activities in set1
    for s in start_activities {
        set1.insert(s.clone());
    }
    for e in end_activities {
        set1.insert(e.clone());
    }

    // Step 5: Get remaining activities
    let mut remaining_activities: HashSet<String> = HashSet::new();
    for activity in all_activities {
        if !start_activities.contains(activity) && !end_activities.contains(activity) {
            remaining_activities.insert(activity.clone());
        }
    }

    // Step 6: Process each remaining activity
    let mut remaining_activities_clone: Vec<String> = remaining_activities.iter().cloned().collect();
    remaining_activities_clone.sort();
    
    for x in remaining_activities_clone {
        // Step 6.1 & 6.2: Check if activity is between start-end or end-start
        let activity_between_start_to_end = is_activity_between_start_end(start_activities, &x, end_activities, &current_dfg);
        let activity_between_end_to_start = is_activity_between_start_end(end_activities, &x, start_activities, &current_dfg);

        if activity_between_start_to_end && !activity_between_end_to_start {
            // Step 6.3: Put x in set1
            set1.insert(x.clone());
            let (new_dfg, cost, removed_edges) = remove_edges_for_redo(start_activities, end_activities, &current_dfg, &set1, &set2);
            current_dfg = new_dfg;
            total_cost += cost;
            cost_of_edges_removed += cost;
            edges_removed.extend(removed_edges);
        } else if !activity_between_start_to_end && activity_between_end_to_start {
            // Step 6.4: Put x in set2
            set2.insert(x.clone());
            let (new_dfg, cost, removed_edges) = remove_edges_for_redo(start_activities, end_activities, &current_dfg, &set1, &set2);
            current_dfg = new_dfg;
            total_cost += cost;
            cost_of_edges_removed += cost;
            edges_removed.extend(removed_edges);
        } else if activity_between_start_to_end && activity_between_end_to_start {
            // Step 7: Handle the case where activity is between both
            let mut test_dfg = current_dfg.clone();
            remove_activity_from_dfg(&mut test_dfg, &x);
            
            // Check which pairs become invalid after deletion
            let mut start_to_end_invalid = false;
            let mut end_to_start_invalid = false;
            let mut invalid_start_to_end_pairs: Vec<(String, String)> = Vec::new();
            let mut invalid_end_to_start_pairs: Vec<(String, String)> = Vec::new();
            
            for (s, e) in &start_to_end_activity_pairs {
                if !is_reachable(&test_dfg, s, e) {
                    start_to_end_invalid = true;
                    invalid_start_to_end_pairs.push((s.clone(), e.clone()));
                }
            }
            
            for (e, s) in &end_to_start_activity_pairs {
                if !is_reachable(&test_dfg, e, s) {
                    end_to_start_invalid = true;
                    invalid_end_to_start_pairs.push((e.clone(), s.clone()));
                }
            }

            if start_to_end_invalid && !end_to_start_invalid {
                // Step 7.1: Add to set1
                set1.insert(x.clone());
                let (new_dfg, cost, removed_edges) = remove_edges_for_redo(start_activities, end_activities, &current_dfg, &set1, &set2);
                current_dfg = new_dfg;
                total_cost += cost;
                cost_of_edges_removed += cost;
                edges_removed.extend(removed_edges);
            } else if !start_to_end_invalid && end_to_start_invalid {
                // Step 7.1: Add to set2
                set2.insert(x.clone());
                let (new_dfg, cost, removed_edges) = remove_edges_for_redo(start_activities, end_activities, &current_dfg, &set1, &set2);
                current_dfg = new_dfg;
                total_cost += cost;
                cost_of_edges_removed += cost;
                edges_removed.extend(removed_edges);
            } else  {
                // Step 7.2: Both become invalid or valid, try both cases
                let (cost1, dfg1, removed_edges1, added_edges1) = try_case_add_to_set1_new(&x, &current_dfg, start_activities, end_activities, &set1, &set2, cost_to_add_edges);
                let (cost2, dfg2, removed_edges2, added_edges2) = try_case_add_to_set2_new(&x, &current_dfg, start_activities, end_activities, &set1, &set2, cost_to_add_edges);
                
                if cost1 < cost2 {
                    set1.insert(x.clone());
                    current_dfg = dfg1;
                    total_cost += cost1;
                    // Calculate the costs from the returned values
                    let add_cost1 = added_edges1.iter().map(|(s, e, c)| *c as usize).sum::<usize>();
                    let remove_cost1 = cost1 - add_cost1;
                    cost_of_edges_added += add_cost1;
                    cost_of_edges_removed += remove_cost1;
                    edges_removed.extend(removed_edges1);
                    edges_added.extend(added_edges1);
                } else {
                    set2.insert(x.clone());
                    current_dfg = dfg2;
                    total_cost += cost2;
                    // Calculate the costs from the returned values
                    let add_cost2 = added_edges2.iter().map(|(s, e, c)| *c as usize).sum::<usize>();
                    let remove_cost2 = cost2 - add_cost2;
                    cost_of_edges_added += add_cost2;
                    cost_of_edges_removed += remove_cost2;
                    edges_removed.extend(removed_edges2);
                    edges_added.extend(added_edges2);
                }
            } 
        } else {
            // This should not happen
            println!("Redo cut not possible for activity: {}", x);
            return (false, 0, Vec::new(), Vec::new(), 0, 0, HashSet::new(), HashSet::new(), dfg.clone());
        }
    }

    // Step 7.5: Final check
    let (final_dfg, final_cost, final_removed_edges) = remove_edges_for_redo(start_activities, end_activities, &current_dfg, &set1, &set2);
    if final_cost > 0 {
        println!("final step error in best redo function");
    }

    if set1.is_empty() || set2.is_empty() {
        println!("Redo cut not possible: one of the sets is empty");
        return (false, 0, Vec::new(), Vec::new(), 0, 0, HashSet::new(), HashSet::new(), dfg.clone());
    }

    return (true, total_cost, edges_removed, edges_added, cost_of_edges_added, cost_of_edges_removed, set1, set2, current_dfg);
}

// Step 5: Function to remove edges for redo cut
fn remove_edges_for_redo(
    start_activities: &HashSet<String>,
    end_activities: &HashSet<String>,
    dfg: &HashMap<(String, String), usize>,
    set1: &HashSet<String>,
    set2: &HashSet<String>
) -> (HashMap<(String, String), usize>, usize, Vec<(String, String, usize)>) {
    let mut new_dfg = dfg.clone();
    let mut total_cost = 0usize;
    let mut edges_to_remove: Vec<(String, String, usize)> = Vec::new();
    
    for (edge, cost) in dfg.iter() {
        let a = &edge.0;
        let b = &edge.1;
        
        // Check if a and b are NOT start or end activities
        let a_not_start_end = !start_activities.contains(a) && !end_activities.contains(a);
        let b_not_start_end = !start_activities.contains(b) && !end_activities.contains(b);
        
        // Check if a and b are from different sets
        let different_sets = (set1.contains(a) && set2.contains(b)) || (set2.contains(a) && set1.contains(b));
        
        // Remove edge if both nodes are not start/end activities AND they're from different sets
        if a_not_start_end && b_not_start_end && different_sets {
            edges_to_remove.push((edge.0.clone(), edge.1.clone(), *cost));
            total_cost += cost;
        }
    }
    
    // Remove the collected edges
    for (from, to, _cost) in &edges_to_remove {
        new_dfg.remove(&(from.clone(), to.clone()));
    }
    
    (new_dfg, total_cost, edges_to_remove)
}

// Function to check if activity is between start and end activities
// This checks if there's a path from any start activity to the target activity 
// without first encountering any end activity
fn is_activity_between_start_end(
    start_set: &HashSet<String>,
    activity: &String,
    end_set: &HashSet<String>,
    dfg: &HashMap<(String, String), usize>
) -> bool {
    for start in start_set {
        if is_reachable_without_passing_through(dfg, start, activity, end_set) {
            return true;
        }
    }
    false
}

// Helper function to check if activity2 is reachable from activity1 without passing through any activity in avoid_set
fn is_reachable_without_passing_through(
    dfg: &HashMap<(String, String), usize>,
    activity1: &str,
    activity2: &str,
    avoid_set: &HashSet<String>,
) -> bool {
    let mut visited = HashSet::new();
    let mut stack = vec![activity1.to_string()];

    while let Some(current) = stack.pop() {
        if current == activity2 {
            return true;
        }

        // Skip if we've already visited this node
        if !visited.insert(current.clone()) {
            continue;
        }

        // Skip if this node is in the avoid set (but not if it's the starting node)
        if current != activity1 && avoid_set.contains(&current) {
            continue;
        }

        // Add neighbors to stack
        for ((from, to), _) in dfg {
            if from == &current {
                stack.push(to.clone());
            }
        }
    }

    false
}

// New function to add edges for redo cut
fn add_edges_for_redo(
    dfg: &HashMap<(String, String), usize>,
    set: &HashSet<String>,
    start_activities: &HashSet<String>,
    end_activities: &HashSet<String>,
    activity_x: &String,
    cost_to_add_edges: &HashMap<(String, String), f64>
) -> (HashMap<(String, String), usize>, Vec<(String, String, usize)>, usize) {
    let mut new_dfg = dfg.clone();
    let mut edges_added: Vec<(String, String, usize)> = Vec::new();
    let mut total_cost = 0usize;
    
    // Find all pairs (c,d) that should have edges added
    for ((c_from, x_to), _) in dfg.iter() {
        if x_to == activity_x {
            // Found edge (c, x)
            let c = c_from;
            
            for ((x_from, d_to), _) in dfg.iter() {
                if x_from == activity_x {
                    // Found edge (x, d)
                    let d = d_to;
                    
                    // Check if (c,d) should be added based on conditions
                    let should_add = 
                        // Condition 1: both c and d belong to set
                        (set.contains(c) && set.contains(d)) ||
                        // Condition 2: c belongs to set and d is start/end activity
                        (set.contains(c) && !set.contains(d) && 
                         (start_activities.contains(d) || end_activities.contains(d))) ||
                        // Condition 3: d belongs to set and c is start/end activity  
                        (set.contains(d) && !set.contains(c) && 
                         (start_activities.contains(c) || end_activities.contains(c)));
                    
                    // Check if (c,d) doesn't already exist in dfg
                    let edge_exists = new_dfg.contains_key(&(c.clone(), d.clone()));
                    
                    if should_add && !edge_exists {
                        let cost_to_add_edge = cost_to_add_edges.get(&(c.clone(), d.clone())).copied().unwrap_or(999999.0);
                        let cost_to_add_edge_usize = cost_to_add_edge as usize;
                        new_dfg.insert((c.clone(), d.clone()), cost_to_add_edge_usize);
                        edges_added.push((c.clone(), d.clone(), cost_to_add_edge_usize));
                        total_cost += cost_to_add_edge_usize;
                    }
                }
            }
        }
    }
    
    (new_dfg, edges_added, total_cost)
}

// Updated helper function for case 7.2.1 using new add_edges_for_redo
fn try_case_add_to_set1_new(
    x: &String,
    dfg: &HashMap<(String, String), usize>,
    start_activities: &HashSet<String>,
    end_activities: &HashSet<String>,
    set1: &HashSet<String>,
    set2: &HashSet<String>,
    cost_to_add_edges: &HashMap<(String, String), f64>
) -> (usize, HashMap<(String, String), usize>, Vec<(String, String, usize)>, Vec<(String, String, usize)>) {
    let mut test_set1 = set1.clone();
    test_set1.insert(x.clone());
    
    // Add edges using the new function
    let (dfg_with_added, added_edges, add_cost) = add_edges_for_redo(dfg, set2, start_activities, end_activities, x, cost_to_add_edges);
    
    // Remove edges
    let (final_dfg, remove_cost, removed_edges) = remove_edges_for_redo(start_activities, end_activities, &dfg_with_added, &test_set1, set2);
    
    (add_cost + remove_cost, final_dfg, removed_edges, added_edges)
}

// Updated helper function for case 7.2.2 using new add_edges_for_redo
fn try_case_add_to_set2_new(
    x: &String,
    dfg: &HashMap<(String, String), usize>,
    start_activities: &HashSet<String>,
    end_activities: &HashSet<String>,
    set1: &HashSet<String>,
    set2: &HashSet<String>,
    cost_to_add_edges: &HashMap<(String, String), f64>
) -> (usize, HashMap<(String, String), usize>, Vec<(String, String, usize)>, Vec<(String, String, usize)>) {
    let mut test_set2 = set2.clone();
    test_set2.insert(x.clone());
    
    // Add edges using the new function
    let (dfg_with_added, added_edges, add_cost) = add_edges_for_redo(dfg, set1, start_activities, end_activities, x, cost_to_add_edges);
    
    // Remove edges
    let (final_dfg, remove_cost, removed_edges) = remove_edges_for_redo(start_activities, end_activities, &dfg_with_added, set1, &test_set2);
    
    (add_cost + remove_cost, final_dfg, removed_edges, added_edges)
}

// Helper function to remove an activity and all its connected edges from DFG
fn remove_activity_from_dfg(dfg: &mut HashMap<(String, String), usize>, activity: &String) {
    let edges_to_remove: Vec<(String, String)> = dfg.keys()
        .filter(|edge| edge.0 == *activity || edge.1 == *activity)
        .cloned()
        .collect();
    
    for edge in edges_to_remove {
        dfg.remove(&edge);
    }
}