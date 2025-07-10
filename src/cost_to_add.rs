use std::collections::{HashMap, HashSet, VecDeque};

use log::info;

pub fn add_edge_to_dfg(
    dfg: &HashMap<(String, String), usize>,
    activity1: &str,
    activity2: &str
) -> (HashMap<(String, String), usize>, usize) {

    let cost = 1; // Define the cost of adding an edge
    let mut totalCost = 0;

    let mut new_dfg = dfg.clone();
    let edge = (activity1.to_string(), activity2.to_string());
    new_dfg.insert(edge, cost); 
    totalCost += cost;

    (new_dfg, totalCost)
}


pub fn all_possible_edges_to_add_to_dfg(
    dfg: &HashMap<(String, String), usize>,
    all_activities: &HashSet<String>,
) -> HashMap<(String, String), f64> {
    let mut new_dfg = HashMap::new();

    // Step 1: Basic statistics
    let s_obs = dfg.len();
    let mut total_edge_occurrences = 0;
    let mut q1 = 0;
    let mut q2 = 0;

    for &count in dfg.values() {
        total_edge_occurrences += count;
        if count == 1 {
            q1 += 1;
        } else if count == 2 {
            q2 += 1;
        }
    }

    // If we have no data or no singletons, we can't estimate unseen edges reliably
    if total_edge_occurrences == 0 || q1 == 0 {
        return new_dfg;
    }

    // Step 2: Estimate species richness (Sest) with Chao2
    let s_est = if q2 > 0 {
        s_obs as f64 + (q1 * q1) as f64 / (2.0 * q2 as f64)
    } else {
        s_obs as f64 + ((q1 * (q1 - 1)) as f64) / 2.0
    };

    // Step 3: Estimate unseen species count (Q0)
    let q0 = (s_est - s_obs as f64).round(); // round to nearest whole number

    if q0 <= 0.0 {
        return new_dfg; // No unseen edges predicted
    }

    // Step 4: Estimate expected trace count per missing edge
    let prob_unseen_total = q1 as f64 / total_edge_occurrences as f64;
    let expected_count_per_missing_edge =
        prob_unseen_total / q0 * total_edge_occurrences as f64;

    // Step 5: Add all possible missing edges with estimated counts
    for from in all_activities {
        for to in all_activities {
            if from != to && !dfg.contains_key(&(from.clone(), to.clone())) {
                new_dfg.insert((from.clone(), to.clone()), expected_count_per_missing_edge);
            }
        }
    }

    new_dfg
}
