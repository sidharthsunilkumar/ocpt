use std::collections::{HashMap, HashSet};
use std::ptr::null;
use crate::cost_to_cut::is_reachable;
use crate::cost_to_cut::to_be_non_reachable;
use crate::start_cuts_opti_v2::is_reachable_before_end_activity;
use log::info;

pub fn best_redo_cut(
    dfg: &HashMap<(String, String), usize>, 
    all_activities: &HashSet<String>,
    start_activities: &HashSet<String>,
    end_activities: &HashSet<String>
) -> (bool, usize, usize, Vec<(String, String)>, HashSet<String>, HashSet<String>, HashMap<(String, String), usize>) {

    let mut start_end_pairs: Vec<(String, String)> = Vec::new();
    let mut end_start_pairs: Vec<(String, String)> = Vec::new();
    let mut set1: HashSet<String> = HashSet::new();
    let mut set2: HashSet<String> = HashSet::new();
    let mut current_dfg = dfg.clone();
    let mut total_cost = 0usize;
    let mut total_edges_removed = 0usize;
    let mut edges_removed: Vec<(String, String)> = Vec::new();

    // info!("Start and end activities: {:?}, {:?}", start_activities, end_activities);

    //Step 1: For all start activities 's' and end activities 'e', check if 'e' is reachable from 's'
    for s in start_activities {
        for e in end_activities {
            let s_e: bool = is_reachable(&dfg, &s, &e);
            let e_s: bool = is_reachable(&dfg, &e, &s);
            if !e_s {
                return (false, 0, 0, Vec::new(), HashSet::new(), HashSet::new(), dfg.clone());
            }
            if s_e {
                start_end_pairs.push((s.clone(), e.clone()));
            }
            if e_s {
                end_start_pairs.push((e.clone(), s.clone()));
            }
        }
    }
    // info!("After step 1, start_end_pairs: {:?}, end_start_pairs: {:?}", start_end_pairs, end_start_pairs);

    // Step 2: Put all start and activities into set1
    for s in start_activities {
        set1.insert(s.clone());
    }
    for e in end_activities {
        set1.insert(e.clone());
    }

    // info!("After step 2, set1: {:?}, set2: {:?}", set1, set2);

    // Step 3: put the remaining activities into remaining_activities
    let mut remaining_activities: HashSet<String> = HashSet::new();
    for activity in all_activities {
        if !start_activities.contains(activity) && !end_activities.contains(activity) {
            remaining_activities.insert(activity.clone());
        }
    }

    // info!("After step 3, remaining_activities: {:?}", remaining_activities);

    // Step 4: Assign the remaining activities to set1 or set2
    let mut remaining_activities_clone: Vec<String> = remaining_activities.iter().cloned().collect();
    remaining_activities_clone.sort();
    for x in remaining_activities_clone {
        let btw_start_and_end = is_reachable_before_end_activity(start_activities, &x, end_activities, dfg);
        let btw_end_and_start = is_reachable_before_end_activity(end_activities, &x, start_activities, dfg);

        // info!("In step 4, Checking activity: {}, btw_start_and_end: {}, btw_end_and_start: {}", x, btw_start_and_end, btw_end_and_start);
        // info!("Current set1: {:?}, set2: {:?}", set1, set2);
        // info!("Current dfg: {:?}", current_dfg);

        if btw_start_and_end && !btw_end_and_start{
            set1.insert(x.clone());
        } else if (!btw_start_and_end && btw_end_and_start) {
            set2.insert(x.clone());            
        } else if(btw_start_and_end && btw_end_and_start) {
            // Step 4.1: remove x from a clone of dfg and check is every start_end pair is still reachable by using is_reachable function. if so Assign true to variable is_start_end_pairs_valid
            let mut test_dfg = current_dfg.clone();
            remove_activity_from_dfg(&mut test_dfg, &x);
            
            let mut is_start_end_pairs_valid = true;
            for (s, e) in &start_end_pairs {
                if !is_reachable(&test_dfg, s, e) {
                    is_start_end_pairs_valid = false;
                    break;
                }
            }

            // Step 4.2: remove x from a clone of dfg and check is every end_start pair is still reachable by using is_reachable function. if so Assign true to variable is_end_start_pairs_valid
            let mut is_end_start_pairs_valid = true;
            for (e, s) in &end_start_pairs {
                if !is_reachable(&test_dfg, e, s) {
                    is_end_start_pairs_valid = false;
                    break;
                }
            }


            // Step 4.3: if !is_start_end_pairs_valid and is_end_start_pairs_valid, then add x to set1, remove from remaining_Activities and call remove_edges(dfg,x,set2), else if is_start_end_pairs_valid and !is_end_start_pairs_valid, then add x to set2, remove from remaining_activities and call remove_edges(dfg,x,set1), 
            //  If both are invalid, then return false. else, do dfg1,cost1= remove_edges_dfg(dfg, &x, &set1) and dfg2,cost2=remove_edges_dfg(dfg, &x, &set2), then check if cost1 < cost2, if so add x to set2 and use dfg1, else add x to set1 and use dfg2.
            
            // after remove_edges(dfg,x,set2) returns a new dfg, make sure thenext x uses this new dfg, not the original dfg.
            if !is_start_end_pairs_valid && is_end_start_pairs_valid {
                set1.insert(x.clone());
                remaining_activities.remove(&x);
                let (new_dfg, cost, removed_edges) = remove_edges_dfg(&current_dfg, &x, &set2);
                current_dfg = new_dfg;
                total_cost += cost as usize;
                total_edges_removed += removed_edges.len();
                edges_removed.extend(removed_edges);
            } else if is_start_end_pairs_valid && !is_end_start_pairs_valid {
                set2.insert(x.clone());
                remaining_activities.remove(&x);
                let (new_dfg, cost, removed_edges) = remove_edges_dfg(&current_dfg, &x, &set1);
                current_dfg = new_dfg;
                total_cost += cost as usize;
                total_edges_removed += removed_edges.len();
                edges_removed.extend(removed_edges);
            } else if !is_start_end_pairs_valid && !is_end_start_pairs_valid {
                // Both are invalid, return false
                return (false, 0, 0, Vec::new(), HashSet::new(), HashSet::new(), dfg.clone());
            } else {
                // Both are valid, choose based on cost
                let (dfg1, cost1, removed_edges1) = remove_edges_dfg(&current_dfg, &x, &set1);
                let (dfg2, cost2, removed_edges2) = remove_edges_dfg(&current_dfg, &x, &set2);
                
                if cost1 < cost2 {
                    set2.insert(x.clone());
                    current_dfg = dfg1;
                    total_cost += cost1 as usize;
                    total_edges_removed += removed_edges1.len();
                    edges_removed.extend(removed_edges1);
                } else {
                    set1.insert(x.clone());
                    current_dfg = dfg2;
                    total_cost += cost2 as usize;
                    total_edges_removed += removed_edges2.len();
                    edges_removed.extend(removed_edges2);
                }
                remaining_activities.remove(&x);
            }
        
        } else {
            // This is impossible, but just in case :)
            return (false, 0, 0, Vec::new(), HashSet::new(), HashSet::new(), dfg.clone());
        }

        
    }

    if set1.is_empty() || set2.is_empty() {
        return (false, 0, 0, Vec::new(), HashSet::new(), HashSet::new(), dfg.clone());
    }

    return (true, total_cost, total_edges_removed, edges_removed, set1, set2, current_dfg);
}

fn remove_edges_dfg(
    dfg: &HashMap<(String, String), usize>,
    activity: &String,
    set: &HashSet<String>
) -> (HashMap<(String, String), usize>, i32, Vec<(String, String)>) {
    let mut new_dfg = dfg.clone();
    let mut total_cost = 0i32;
    
    // Collect edges to remove (to avoid borrowing issues)
    let mut edges_to_remove: Vec<(String, String)> = Vec::new();
    
    for (edge, cost) in dfg.iter() {
        // Remove edges between activity and any node in the set
        if (edge.0 == *activity && set.contains(&edge.1)) || 
           (edge.1 == *activity && set.contains(&edge.0)) {
            edges_to_remove.push(edge.clone());
            total_cost += *cost as i32;
        }
    }
    
    // Remove the collected edges
    for edge in &edges_to_remove {
        new_dfg.remove(edge);
    }
    
    (new_dfg, total_cost, edges_to_remove)
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