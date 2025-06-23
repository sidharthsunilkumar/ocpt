use std::collections::{HashMap, HashSet};
use crate::cost_to_cut::is_reachable;
use crate::cost_to_cut::to_be_non_reachable;
use log::info;

pub fn best_exclusive_cut(
    dfg: &HashMap<(String, String), usize>, 
    all_activities: &HashSet<String>
) -> (usize, usize, Vec<(String, String)>, HashSet<String>, HashSet<String>, HashMap<(String, String), usize>) {
    let mut best_total_cost = usize::MAX;
    let mut best_total_min_cut = 0;
    let mut best_total_cut_edges: Vec<(String, String)> = Vec::new();
    let mut best_set1: HashSet<String> = HashSet::new();
    let mut best_set2: HashSet<String> = HashSet::new();
    let mut best_size_diff = usize::MAX;
    let mut best_new_dfg: HashMap<(String, String), usize> = HashMap::new();

    // For every pair of activity(a,b)
    for activity_a in all_activities {
        for activity_b in all_activities {
            if activity_a != activity_b {
                let mut set1: HashSet<String> = HashSet::new();
                let mut set2: HashSet<String> = HashSet::new();
                set1.insert(activity_a.clone());
                set2.insert(activity_b.clone());

                // r1: make b non-reachable from a
                let (r1_min_cut, r1_cost, r1_cut_edges) = 
                    to_be_non_reachable(dfg, activity_a, activity_b);
                
                // r2: make a non-reachable from b
                let (r2_min_cut, r2_cost, r2_cut_edges) = 
                    to_be_non_reachable(dfg, activity_b, activity_a);

                let mut total_min_cut = r1_min_cut + r2_min_cut;
                let mut total_cost = r1_cost + r2_cost;
                let mut total_cut_edges = r1_cut_edges.clone();
                total_cut_edges.extend(r2_cut_edges.clone());

                // Create new_dfg with edges of r1_cut_edges and r2_cut_edges removed
                let mut new_dfg: HashMap<(String, String), usize> = dfg.clone();
                for (from, to) in &r1_cut_edges {
                    new_dfg.remove(&(from.clone(), to.clone()));
                }
                for (from, to) in &r2_cut_edges {
                    new_dfg.remove(&(from.clone(), to.clone()));
                }

                // Get remaining activities (excluding a and b)
                let mut remaining_activities: HashSet<String> = all_activities.clone();
                remaining_activities.remove(activity_a);
                remaining_activities.remove(activity_b);

                // For every remaining activity c
                for activity_c in &remaining_activities {
                    let c_a = is_reachable(&new_dfg, activity_c, activity_a);
                    let c_b = is_reachable(&new_dfg, activity_c, activity_b);
                    let a_c = is_reachable(&new_dfg, activity_a, activity_c);
                    let b_c = is_reachable(&new_dfg, activity_b, activity_c);

                    if (c_a || a_c) && !c_b && !b_c {
                        // c belongs to set1 (connected to a but not b)
                        set1.insert(activity_c.clone());
                    } else if (c_b || b_c) && !c_a && !a_c {
                        // c belongs to set2 (connected to b but not a)
                        set2.insert(activity_c.clone());
                    } else if !(c_a || a_c || c_b || b_c) {
                        // c is isolated, put in smaller set
                        if set1.len() <= set2.len() {
                            set1.insert(activity_c.clone());
                        } else {
                            set2.insert(activity_c.clone());
                        }
                    } else {
                        // c is connected to both or in conflicting ways, need to cut
                        
                        // Option 1: put c in set1 (make c and a mutually exclusive from b)
                        let (c11_min_cut, c11_cost, c11_cut_edges) = 
                            to_be_non_reachable(&new_dfg, activity_a, activity_c);
                        let mut new_dfg_c11 = new_dfg.clone();
                        for (from, to) in &c11_cut_edges {
                            new_dfg_c11.remove(&(from.clone(), to.clone()));
                        }
                        
                        let (c12_min_cut, c12_cost, c12_cut_edges) = 
                            to_be_non_reachable(&new_dfg_c11, activity_c, activity_a);
                        let c1_min_cut = c11_min_cut + c12_min_cut;
                        let c1_cost = c11_cost + c12_cost;
                        let mut c1_cut_edges = c11_cut_edges.clone();
                        c1_cut_edges.extend(c12_cut_edges.clone());
                        
                        let mut new_dfg_c1 = new_dfg_c11.clone();
                        for (from, to) in &c12_cut_edges {
                            new_dfg_c1.remove(&(from.clone(), to.clone()));
                        }

                        // Option 2: put c in set2 (make c and b mutually exclusive from a)
                        let (c21_min_cut, c21_cost, c21_cut_edges) = 
                            to_be_non_reachable(&new_dfg, activity_b, activity_c);
                        let mut new_dfg_c21 = new_dfg.clone();
                        for (from, to) in &c21_cut_edges {
                            new_dfg_c21.remove(&(from.clone(), to.clone()));
                        }
                        
                        let (c22_min_cut, c22_cost, c22_cut_edges) = 
                            to_be_non_reachable(&new_dfg_c21, activity_c, activity_b);
                        let c2_min_cut = c21_min_cut + c22_min_cut;
                        let c2_cost = c21_cost + c22_cost;
                        let mut c2_cut_edges = c21_cut_edges.clone();
                        c2_cut_edges.extend(c22_cut_edges.clone());
                        
                        let mut new_dfg_c2 = new_dfg_c21.clone();
                        for (from, to) in &c22_cut_edges {
                            new_dfg_c2.remove(&(from.clone(), to.clone()));
                        }

                        // Choose the better option
                        if c1_cost < c2_cost {
                            set1.insert(activity_c.clone());
                            total_min_cut += c1_min_cut;
                            total_cost += c1_cost;
                            total_cut_edges.extend(c1_cut_edges);
                            new_dfg = new_dfg_c1;
                        } else if c2_cost < c1_cost {
                            set2.insert(activity_c.clone());
                            total_min_cut += c2_min_cut;
                            total_cost += c2_cost;
                            total_cut_edges.extend(c2_cut_edges);
                            new_dfg = new_dfg_c2;
                        } else {
                            // Costs are equal, choose based on set size
                            if set1.len() <= set2.len() {
                                set1.insert(activity_c.clone());
                                total_min_cut += c1_min_cut;
                                total_cost += c1_cost;
                                total_cut_edges.extend(c1_cut_edges);
                                new_dfg = new_dfg_c1;
                            } else {
                                set2.insert(activity_c.clone());
                                total_min_cut += c2_min_cut;
                                total_cost += c2_cost;
                                total_cut_edges.extend(c2_cut_edges);
                                new_dfg = new_dfg_c2;
                            }
                        }
                    }
                }

                // Calculate size difference for this solution
                let size_diff = (set1.len() as isize - set2.len() as isize).abs() as usize;
                
                // Update best solution if this pair is better
                let should_update = if total_cost < best_total_cost {
                    true
                } else if total_cost == best_total_cost {
                    if size_diff < best_size_diff {
                        true
                    } else {
                        false // if size_diff is also same, keep the first one
                    }
                } else {
                    false
                };
                
                if should_update {
                    best_total_cost = total_cost;
                    best_total_min_cut = total_min_cut;
                    best_total_cut_edges = total_cut_edges;
                    best_set1 = set1;
                    best_set2 = set2;
                    best_size_diff = size_diff;
                    best_new_dfg = new_dfg;
                }
            }
        }
    }

    (best_total_cost, best_total_min_cut, best_total_cut_edges, best_set1, best_set2, best_new_dfg)
}