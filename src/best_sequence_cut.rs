use std::collections::{HashMap, HashSet};
use crate::cost_to_cut::is_reachable;
use crate::cost_to_cut::to_be_non_reachable;
use log::info;

pub fn best_sequence_cut(
    dfg: &HashMap<(String, String), usize>, 
    all_activities: &HashSet<String>
) -> (usize, usize, Vec<(String, String)>, HashSet<String>, HashSet<String>, HashMap<(String, String), usize>) {
    let mut min_cost = usize::MAX;
    let mut best_min_cut = 0;
    let mut best_cut_edges: Vec<(String, String)> = Vec::new();
    let mut best_set1: HashSet<String> = HashSet::new();
    let mut best_set2: HashSet<String> = HashSet::new();
    let mut best_size_diff = usize::MAX;
    let mut best_new_dfg: HashMap<(String, String), usize> = HashMap::new();

    // println!("DFG:");
    // for (key, value) in dfg {
    //     println!("{} -> {} : {}", key.0, key.1, value);
    // }

    
    // create a nested loop for every pair of activities
    for activity1 in all_activities {
        for activity2 in all_activities {
            if activity1 != activity2 {
                // Call the function to find the minimum edge cut
                // this is taking around 70ms
                let (min_cut, cost, cut_edges) =
                    to_be_non_reachable(dfg, activity1, activity2);

                
                
                // Print the results
                // println!(
                //     "To make {} non-reachable from {}: min cut = {}, min cost = {}, cut edges = {:?}",
                //     activity2, activity1, min_cut, cost, cut_edges
                // );
                
                // create a new dfg and delete the edges in cut_edges
                let mut new_dfg: HashMap<(String, String), usize> = dfg.clone();
                for (from, to) in &cut_edges {
                    new_dfg.remove(&(from.clone(), to.clone()));
                }
                
                // print the updated dfg
                // println!("Updated DFG after cutting edges:");
                // for (key, value) in &new_dfg {
                //     println!("{} -> {} : {}", key.0, key.1, value);
                // }
                
                let mut set1: HashSet<String> = HashSet::new();
                set1.insert(activity2.clone());
                let mut set2: HashSet<String> = HashSet::new();
                set2.insert(activity1.clone());
                
                // get all the activities that are not activity1 and activity2
                let mut remaining_activities: HashSet<String> = all_activities.clone();
                remaining_activities.remove(activity1);
                remaining_activities.remove(activity2);
                
                // loop for every activity in remaining_activities, check if it is reachable from activity2
                for activity in &remaining_activities {
                    if is_reachable(&new_dfg, activity, activity2) {
                        set1.insert(activity.clone());
                    } else {
                        set2.insert(activity.clone());
                    }
                }
                
                // print the sets
                // println!("Set1: {:?}", set1);
                // println!("Set2: {:?}", set2);
                
                let size_diff = (set1.len() as isize - set2.len() as isize).abs() as usize;
                
                // Update best solution if this one is better
                let should_update = if cost < min_cost {
                    true
                } else if cost == min_cost {
                    if size_diff < best_size_diff {
                        true
                    } else {
                        false // if size_diff is also same, keep the first one
                    }
                } else {
                    false
                };
                
                if should_update {
                    min_cost = cost;
                    best_min_cut = min_cut;
                    best_cut_edges = cut_edges;
                    best_set1 = set1;
                    best_set2 = set2;
                    best_size_diff = size_diff;
                    best_new_dfg = new_dfg;
                }

                
            }
        }
    }
    
    
    (min_cost, best_min_cut, best_cut_edges, best_set1, best_set2, best_new_dfg)
}