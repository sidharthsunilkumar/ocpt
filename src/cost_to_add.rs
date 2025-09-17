use std::collections::{HashMap, HashSet};

pub fn cost_of_adding_edge(dfg: &HashMap<(String, String), usize>) -> usize {
    
    let s_obs = dfg.len();  // Number of edges in the DFG
    let mut q1 = 0; // Number of edges with count 1
    let mut q2 = 0; // Number of edges with count 2
    let mut n: usize = 0;
    
    for &count in dfg.values() {
        if count == 1 {
            q1 += 1;
        } else if count == 2 {
            q2 += 1;
        }
        n += count;
    }

    let prob_unseen_edge = q1 as f64 / n as f64;
    
    // Chao2 estimator - yeilds an estimate of speicies richness i.e. the number of species (seen and unseen)
    // i.e estimate of total edges
    let s_chao2 = if q2 > 0 {
        s_obs as f64 + (q1 * q1) as f64 / (2.0 * q2 as f64)
    } else {
        s_obs as f64 + ((q1 * (q1 - 1)) as f64) / 2.0
    };

    // Estimate of unseen edges
    let q0 = s_chao2 - s_obs as f64;

    let est_cost: usize = if q0 > 0.0 {
        (q1 as f64 / q0 as f64).ceil() as usize
    } else {
        1 // Default to 1 if q0 is 0 or negative
    };

    // Ensure minimum cost is 1
    let final_cost = est_cost.max(1);

    println!("Cost to add; s_obs: {}, q1: {}, q2: {}, n: {}, s_chao2: {}, q0: {}, est_cost: {}, final_cost: {}\n", 
          s_obs, q1, q2, n, s_chao2, q0, est_cost, final_cost);
    
    final_cost
}


pub fn compute_missing_edge_costs(
    dfg: &HashMap<(String, String), usize>,
) -> HashMap<(String, String), usize> {
    // Step 1: Count q1, q2, and total events n
    let mut q1 = 0;
    let mut q2 = 0;
    let mut n: usize = 0;

    for &count in dfg.values() {
        if count == 1 {
            q1 += 1;
        } else if count == 2 {
            q2 += 1;
        }
        n += count;
    }

    let prob_unseen_edge = if n > 0 {
        q1 as f64 / n as f64
    } else {
        0.0
    };

    // Step 2: Collect all activities
    let mut activities = HashSet::new();
    for ((src, dst), _) in dfg {
        activities.insert(src.clone());
        activities.insert(dst.clone());
    }

    // Step 3: Precompute avg outgoing cost per source
    let mut out_cost_sum: HashMap<String, usize> = HashMap::new();
    let mut out_count: HashMap<String, usize> = HashMap::new();

    // Step 4: Precompute avg incoming cost per target
    let mut in_cost_sum: HashMap<String, usize> = HashMap::new();
    let mut in_count: HashMap<String, usize> = HashMap::new();

    for ((src, dst), &cost) in dfg {
        *out_cost_sum.entry(src.clone()).or_default() += cost;
        *out_count.entry(src.clone()).or_default() += 1;

        *in_cost_sum.entry(dst.clone()).or_default() += cost;
        *in_count.entry(dst.clone()).or_default() += 1;
    }

    // Step 5: Compute degree ratios
    let mut out_degree_ratio: HashMap<String, f64> = HashMap::new();
    let mut in_degree_ratio: HashMap<String, f64> = HashMap::new();

    let avg_out_degree = if activities.len() > 0 {
        out_count.values().sum::<usize>() as f64 / activities.len() as f64
    } else {
        1.0
    };

    let avg_in_degree = if activities.len() > 0 {
        in_count.values().sum::<usize>() as f64 / activities.len() as f64
    } else {
        1.0
    };

    for act in &activities {
        out_degree_ratio.insert(
            act.clone(),
            *out_count.get(act).unwrap_or(&0) as f64 / avg_out_degree,
        );
        in_degree_ratio.insert(
            act.clone(),
            *in_count.get(act).unwrap_or(&0) as f64 / avg_in_degree,
        );
    }

    // Step 6: Calculate missing edges
    let mut missing_edges: HashMap<(String, String), usize> = HashMap::new();
    for a in &activities {
        for b in &activities {
            if a != b && !dfg.contains_key(&(a.clone(), b.clone())) {
                let avg_out_cost_a = if let Some(sum) = out_cost_sum.get(a) {
                    *sum as f64 / *out_count.get(a).unwrap_or(&1) as f64
                } else {
                    1.0
                };

                let avg_in_cost_b = if let Some(sum) = in_cost_sum.get(b) {
                    *sum as f64 / *in_count.get(b).unwrap_or(&1) as f64
                } else {
                    1.0
                };

                // Base cost from local averages
                let mut cost = (avg_out_cost_a + avg_in_cost_b) / 2.0;

                // Degree adjustment: lower cost if out_degree(A) high and in_degree(B) low
                let out_ratio = *out_degree_ratio.get(a).unwrap_or(&1.0);
                let in_ratio = *in_degree_ratio.get(b).unwrap_or(&1.0);
                let degree_factor = 1.0 - (out_ratio - in_ratio);

                cost *= degree_factor;

                // Prob unseen edge adjustment
                cost *= 1.0 - prob_unseen_edge;

                // Ensure cost is at least 1
                let final_cost = cost.round().max(1.0) as usize;

                missing_edges.insert((a.clone(), b.clone()), final_cost);

                println!("degree_factor: {}, final_cost: {}, cost: {}", degree_factor, final_cost, cost);
            }
        }
    }

    missing_edges
}

// // Return a missing edge dfg
// // Algo no. 3
// pub fn cost_of_adding_edge_by_object_type(
//     dfg: &HashMap<(String, String), usize>,
//     _dfg_sets: &HashMap<String, (HashMap<(String, String), usize>, HashSet<String>, HashSet<String>)>
// ) -> (
//     HashMap<(String, String), usize>, 
//     f64, 
//     HashMap<(String, String), f64>, 
//     HashMap<(String, String), f64>
// ) {

//     // Step 1: Get all activities from dfg
//     let mut activities = HashSet::new();
//     for ((a, b), _) in dfg {
//         activities.insert(a.clone());
//         activities.insert(b.clone());
//     }

//     // Step 2: Create missing_edge_dfg hashmap
//     let mut missing_edge_dfg: HashMap<(String, String), f64> = HashMap::new();

//     // Step 3: Find missing edges and add them with cost 1
//     for a in &activities {
//         for b in &activities {
//             if a != b && !dfg.contains_key(&(a.clone(), b.clone())) {
//                 missing_edge_dfg.insert((a.clone(), b.clone()), 1.0);
//             }
//         }
//     }

//     // Step 4: Calculate values directly from dfg instead of dfg_sets
//     let k = if !dfg.is_empty() {
//         *dfg.values().min().unwrap()
//     } else {
//         1
//     };
    
//     let count_with_k = dfg.values().filter(|&&cost| cost == k).count();
    
//     // Step 5: Get total number of traces from dfg
//     let n: usize = dfg.values().sum();
//     println!("DFG: {:?}", dfg);

//     // Step 6: Create probability_of_missing_edge variable
//     let probability_of_missing_edge = if n > 0 && k > 0 {
//         (count_with_k as f64 / n as f64) * (1.0 / k as f64)
//     } else {
//         0.0
//     };
    
//     println!("n: {}, k: {}, count_with_k: {}", n, k, count_with_k);

    
//     // Step 7: Calculate frequencies
//     let mut frequencies: HashMap<String, usize> = HashMap::new();
    
//     // Initialize all activities with frequency 0
//     for activity in &activities {
//         frequencies.insert(activity.clone(), 0);
//     }
    
//     // For every edge in dfg, add n to frequencies
//     for ((a, b), n) in dfg {
//         *frequencies.entry(a.clone()).or_insert(0) += n;
//         *frequencies.entry(b.clone()).or_insert(0) += n;
//     }

//     // Step 8: Calculate rarity score of edges
//     let mut rarity_score: HashMap<(String, String), f64> = HashMap::new();
//     let mut max_rarity_score = 0.0;
    
//     for a in &activities {
//         for b in &activities {
//             if a != b && !rarity_score.contains_key(&(b.clone(), a.clone())) {
//                 let freq_a = *frequencies.get(a).unwrap_or(&0) as f64;
//                 let freq_b = *frequencies.get(b).unwrap_or(&0) as f64;
                
//                 let score = if freq_a > 0.0 && freq_b > 0.0 {
//                     1.0 / (freq_a * freq_b).sqrt()
//                 } else {
//                     0.0
//                 };
                
//                 rarity_score.insert((a.clone(), b.clone()), score);
                
//                 if score > max_rarity_score {
//                     max_rarity_score = score;
//                 }
//             }
//         }
//     }

//     // Step 9: Calculate normalized rarity score
//     let mut normalised_rarity_score: HashMap<(String, String), f64> = HashMap::new();
    
//     for ((a, b), score) in &rarity_score {
//         let normalized_score = if max_rarity_score > 0.0 {
//             (score / max_rarity_score ) * 2.0 // Change this to 2
//         } else {
//             0.0
//         };
//         normalised_rarity_score.insert((a.clone(), b.clone()), normalized_score);
//     }

//     // Step 10: Update missing_edge_dfg with probability and rarity scores
//     for ((a, b), value) in missing_edge_dfg.iter_mut() {
//         let rarity_value = normalised_rarity_score.get(&(a.clone(), b.clone()))
//             .or_else(|| normalised_rarity_score.get(&(b.clone(), a.clone())))
//             .unwrap_or(&0.0);

//         let new_value = probability_of_missing_edge * rarity_value * 1.0;
//         println!("Edge: ({}, {}) rarity: {} * prob: {} = {}", a, b, rarity_value, probability_of_missing_edge, new_value);
        
//         *value = new_value;
//     }

//     //print any 1 value of missing edge
//     if let Some((edge, value)) = missing_edge_dfg.iter().next() {
//         println!("Missing edge: {:?}, value: {}", edge, value);
//     }

//     // Step 11: Get maximum cost of edges in dfg
//     let max_traces_of_edge = dfg.values().max().copied().unwrap_or(0);

//     // Step 12: Update missing_edge_dfg with final calculation
//     for ((_a, _b), value) in missing_edge_dfg.iter_mut() {
//         let current_value = *value;
//         let new_value = 1.0 + (max_traces_of_edge as f64 - 1.0) * (1.0 - current_value);    // need to change this to 2 as well
//         *value = new_value;
//     }

//     // print any 1 value of missing edge
//     if let Some((edge, value)) = missing_edge_dfg.iter().next() {
//         println!("Missing edge: {:?}, value: {}", edge, value);
//     }

//     // Convert to usize HashMap for return
//     let missing_edge_dfg_usize: HashMap<(String, String), usize> = missing_edge_dfg
//         .iter()
//         .map(|((a, b), &value)| ((a.clone(), b.clone()), value.round().max(1.0) as usize))
//         .collect();

//     // calling cost_of_adding_edge_by_object_type_cl
//     let missing_edge_dfg_cl = cost_of_adding_edge_by_object_type_cl(dfg);

//     (missing_edge_dfg_usize, probability_of_missing_edge, rarity_score, normalised_rarity_score)
// }


// Return a missing edge dfg
// Algo no. 4
pub fn cost_of_adding_edge_by_object_type(
    dfg: &HashMap<(String, String), usize>,
    dfg_sets: &HashMap<String, (HashMap<(String, String), usize>, HashSet<String>, HashSet<String>)>
) -> (
    HashMap<(String, String), usize>, 
    f64, 
    HashMap<(String, String), f64>, 
    HashMap<(String, String), f64>
) {

    // Step 1: Get all activities from dfg
    let mut activities = HashSet::new();
    for ((a, b), _) in dfg {
        activities.insert(a.clone());
        activities.insert(b.clone());
    }

    // Step 2: Create missing_edge_dfg hashmap
    let mut missing_edge_dfg: HashMap<(String, String), f64> = HashMap::new();

    // Step 3: Find missing edges and add them with cost 1
    for a in &activities {
        for b in &activities {
            if a != b && !dfg.contains_key(&(a.clone(), b.clone())) {
                missing_edge_dfg.insert((a.clone(), b.clone()), 1.0);
            }
        }
    }

    // Step 4: Process dfg_sets to find least cost edges by dfg_by_object_types
    let mut least_trace_edge: HashMap<String, (usize, usize)> = HashMap::new();
    let mut least_k: Option<(String, usize, usize)> = None;
    
    for (object_type, (dfg, _, _)) in dfg_sets {
        if !dfg.is_empty() {
            // Find the lowest cost edge k
            let k = *dfg.values().min().unwrap();
            
            // Count how many edges have cost k
            let count_with_k = dfg.values().filter(|&&cost| cost == k).count();

            least_trace_edge.insert(object_type.clone(), (k, count_with_k));
            
            // Update least_k if this is the smallest k seen so far
            match least_k {
                None => least_k = Some((object_type.clone(), k, count_with_k)),
                Some((_, current_k, _)) if k < current_k => {
                    least_k = Some((object_type.clone(), k, count_with_k));
                }
                _ => {}
            }
        }
    }

    // Step 5: Get total number of traces of dfg with the edge containing least no. of traces
    let mut n = 0;
    if let Some((ref object_type, _, _)) = least_k {
        if let Some((dfg_for_object, _, _)) = dfg_sets.get(object_type) {
            n = dfg_for_object.values().sum();
            // print that dfg
            println!("DFG for object type {}: {:?}", object_type, dfg_for_object);
        }
    }

    // Step 6: Create probability_of_missing_edge variable
    let probability_of_missing_edge = if let Some((_, k, count_with_k)) = least_k {
        if n > 0 && k > 0 {
            (count_with_k as f64 / n as f64) * (1.0 / k as f64)
        } else {
            0.0
        }
    } else {
        0.0
    };
    // print n,k, count_with_k of least_k
    if let Some((_, k, count_with_k)) = least_k {
        println!("n: {}, k: {}, count_with_k: {}", n, k, count_with_k);
    }

    
    // Step 7: Calculate frequencies and total_frequency
    let mut total_frequency = 0;
    let mut frequencies: HashMap<String, usize> = HashMap::new();
    
    // Initialize all activities with frequency 0
    for activity in &activities {
        frequencies.insert(activity.clone(), 0);
    }
    
    // For every edge in dfg, add n to frequencies and 2*n to total_frequency
    for ((a, b), n) in dfg {
        *frequencies.entry(a.clone()).or_insert(0) += n;
        *frequencies.entry(b.clone()).or_insert(0) += n;
        total_frequency += 2 * n;
    }

    // Step 8: Calculate rarity score of edges
    let mut rarity_score: HashMap<(String, String), f64> = HashMap::new();
    let mut sum_rarity_score = 0.0;
    
    for a in &activities {
        for b in &activities {
            if a != b && !rarity_score.contains_key(&(b.clone(), a.clone())) {
                let freq_a = *frequencies.get(a).unwrap_or(&0) as f64;
                let freq_b = *frequencies.get(b).unwrap_or(&0) as f64;
                
                let score = if freq_a > 0.0 && freq_b > 0.0 {
                    1.0 / (freq_a * freq_b).sqrt()
                } else {
                    0.0
                };
                
                rarity_score.insert((a.clone(), b.clone()), score);
                
                sum_rarity_score += score;
            }
        }
    }

    // Step 9: Calculate normalized rarity score
    let mut normalised_rarity_score: HashMap<(String, String), f64> = HashMap::new();
    
    for ((a, b), score) in &rarity_score {
        let normalized_score = if sum_rarity_score > 0.0 {
            (score / sum_rarity_score ) * 1.0 // Change this to 2
        } else {
            0.0
        };
        normalised_rarity_score.insert((a.clone(), b.clone()), normalized_score);
    }

    // Step 10: Update missing_edge_dfg with probability and rarity scores
    for ((a, b), value) in missing_edge_dfg.iter_mut() {
        let rarity_value = normalised_rarity_score.get(&(a.clone(), b.clone()))
            .or_else(|| normalised_rarity_score.get(&(b.clone(), a.clone())))
            .unwrap_or(&0.0);

        let new_value = probability_of_missing_edge * rarity_value * 1.0;
        println!("Edge: ({}, {}) rarity: {} * prob: {} = {}", a, b, rarity_value, probability_of_missing_edge, new_value);
        
        *value = new_value;
    }

    //print any 1 value of missing edge
    if let Some((edge, value)) = missing_edge_dfg.iter().next() {
        println!("Missing edge: {:?}, value: {}", edge, value);
    }

    // Step 11: Get maximum cost of edges in dfg
    let max_traces_of_edge = dfg.values().max().copied().unwrap_or(0);

    // Get average cost of edges in dfg
    let avg_traces_of_edge = if !dfg.is_empty() {
        dfg.values().sum::<usize>() as f64 / dfg.len() as f64
    } else {
        1.0
    };

    // Get median cost of edges in dfg
    let mut costs: Vec<usize> = dfg.values().cloned().collect();
    costs.sort_unstable();
    let median_traces_of_edge = if !costs.is_empty() {
        let mid = costs.len() / 2;
        if costs.len() % 2 == 1 {
            costs[mid] as f64
        } else {
            (costs[mid - 1] + costs[mid]) as f64 / 2.0
        }
    } else {
        1.0
    };

    // print all 3
    println!("Max cost of edges in DFG: {}", max_traces_of_edge);
    println!("Avg cost of edges in DFG: {:.2}", avg_traces_of_edge);
    println!("Median cost of edges in DFG: {:.2}", median_traces_of_edge);

    // Step 12: Update missing_edge_dfg with final calculation
    for ((a, b), value) in missing_edge_dfg.iter_mut() {
        let current_value = *value;
        let new_value = 1.0 + (avg_traces_of_edge*2.0 - 1.0) * (1.0 - current_value);    // need to change this to 2 as well
        *value = new_value;
    }

    // print any 1 value of missing edge
    if let Some((edge, value)) = missing_edge_dfg.iter().next() {
        println!("Missing edge: {:?}, value: {}", edge, value);
    }

    // Convert to usize HashMap for return
    let missing_edge_dfg_usize: HashMap<(String, String), usize> = missing_edge_dfg
        .iter()
        .map(|((a, b), &value)| ((a.clone(), b.clone()), value.round().max(1.0) as usize))
        .collect();

    (missing_edge_dfg_usize, probability_of_missing_edge, rarity_score, normalised_rarity_score)
}


// // Return a missing edge dfg
// // Algo no. 5
// pub fn cost_of_adding_edge_by_object_type(
//     dfg: &HashMap<(String, String), usize>,
//     dfg_sets: &HashMap<String, (HashMap<(String, String), usize>, HashSet<String>, HashSet<String>)>
// ) -> (
//     HashMap<(String, String), usize>, 
//     f64, 
//     HashMap<(String, String), f64>, 
//     HashMap<(String, String), f64>
// ) {

//     // Step 1: Get all activities from dfg
//     let mut activities = HashSet::new();
//     for ((a, b), _) in dfg {
//         activities.insert(a.clone());
//         activities.insert(b.clone());
//     }

//     // Step 2: Create missing_edge_dfg hashmap
//     let mut missing_edge_dfg: HashMap<(String, String), f64> = HashMap::new();

//     // Step 3: Find missing edges and add them with cost 1
//     for a in &activities {
//         for b in &activities {
//             if a != b && !dfg.contains_key(&(a.clone(), b.clone())) {
//                 missing_edge_dfg.insert((a.clone(), b.clone()), 1.0);
//             }
//         }
//     }

//     // Step 4: Process dfg_sets to find least cost edges by dfg_by_object_types
//     let mut least_trace_edge: HashMap<String, (usize, usize)> = HashMap::new();
//     let mut least_k: Option<(String, usize, usize)> = None;
    
//     for (object_type, (dfg, _, _)) in dfg_sets {
//         if !dfg.is_empty() {
//             // Find the lowest cost edge k
//             let k = *dfg.values().min().unwrap();
            
//             // Count how many edges have cost k
//             let count_with_k = dfg.values().filter(|&&cost| cost == k).count();

//             least_trace_edge.insert(object_type.clone(), (k, count_with_k));
            
//             // Update least_k if this is the smallest k seen so far
//             match least_k {
//                 None => least_k = Some((object_type.clone(), k, count_with_k)),
//                 Some((_, current_k, _)) if k < current_k => {
//                     least_k = Some((object_type.clone(), k, count_with_k));
//                 }
//                 _ => {}
//             }
//         }
//     }
    
//     // Step 5 & 6: Calculate highest probability_of_missing_edge by comparing every dfg in dfg_sets
//     let mut probability_of_missing_edge = 0.0;
//     let mut highest_prob_object_type = String::new();
    
//     for (object_type, (dfg_for_object, _, _)) in dfg_sets {
//         if !dfg_for_object.is_empty() {
//             // Calculate 'n' - total number of traces (cost) in this dfg
//             let n: usize = dfg_for_object.values().sum();
            
//             // Find k - the lowest cost edge in this dfg
//             let k = *dfg_for_object.values().min().unwrap();
            
//             // Find count_with_k - how many edges have cost k in this dfg
//             let count_with_k = dfg_for_object.values().filter(|&&cost| cost == k).count();
            
//             // Calculate probability_of_missing_edge for this dfg
//             let current_prob = if n > 0 && k > 0 {
//                 (count_with_k as f64 / n as f64) * (1.0 / k as f64)
//             } else {
//                 0.0
//             };
            
//             println!("Object type: {}, n: {}, k: {}, count_with_k: {}, prob: {}", 
//                      object_type, n, k, count_with_k, current_prob);
            
//             // Take the highest probability_of_missing_edge and track the object type
//             if current_prob > probability_of_missing_edge {
//                 probability_of_missing_edge = current_prob;
//                 highest_prob_object_type = object_type.clone();
//             }
//         }
//     }
    
//     println!("Highest probability_of_missing_edge: {} (from object type: {})", 
//              probability_of_missing_edge, highest_prob_object_type);

//     // Step 7: Calculate frequencies and total_frequency
//     let mut total_frequency = 0;
//     let mut frequencies: HashMap<String, usize> = HashMap::new();
    
//     // Initialize all activities with frequency 0
//     for activity in &activities {
//         frequencies.insert(activity.clone(), 0);
//     }
    
//     // For every edge in dfg, add n to frequencies and 2*n to total_frequency
//     for ((a, b), n) in dfg {
//         *frequencies.entry(a.clone()).or_insert(0) += n;
//         *frequencies.entry(b.clone()).or_insert(0) += n;
//         total_frequency += 2 * n;
//     }

//     // Step 8: Calculate rarity score of edges
//     let mut rarity_score: HashMap<(String, String), f64> = HashMap::new();
//     let mut max_rarity_score = 0.0;
    
//     for a in &activities {
//         for b in &activities {
//             if a != b && !rarity_score.contains_key(&(b.clone(), a.clone())) {
//                 let freq_a = *frequencies.get(a).unwrap_or(&0) as f64;
//                 let freq_b = *frequencies.get(b).unwrap_or(&0) as f64;
                
//                 let score = if freq_a > 0.0 && freq_b > 0.0 {
//                     1.0 / (freq_a * freq_b).sqrt()
//                 } else {
//                     0.0
//                 };
                
//                 rarity_score.insert((a.clone(), b.clone()), score);
                
//                 if score > max_rarity_score {
//                     max_rarity_score = score;
//                 }
//             }
//         }
//     }

//     // Step 9: Calculate normalized rarity score
//     let mut normalised_rarity_score: HashMap<(String, String), f64> = HashMap::new();
    
//     for ((a, b), score) in &rarity_score {
//         let normalized_score = if max_rarity_score > 0.0 {
//             (score / max_rarity_score ) * 2.0 // Change this to 2
//         } else {
//             0.0
//         };
//         normalised_rarity_score.insert((a.clone(), b.clone()), normalized_score);
//     }

//     // Step 10: Update missing_edge_dfg with probability and rarity scores
//     for ((a, b), value) in missing_edge_dfg.iter_mut() {
//         let rarity_value = normalised_rarity_score.get(&(a.clone(), b.clone()))
//             .or_else(|| normalised_rarity_score.get(&(b.clone(), a.clone())))
//             .unwrap_or(&0.0);

//         let new_value = probability_of_missing_edge * rarity_value * 1.0;
//         println!("Edge: ({}, {}) rarity: {} * prob: {} = {}", a, b, rarity_value, probability_of_missing_edge, new_value);
        
//         *value = new_value;
//     }

//     //print any 1 value of missing edge
//     if let Some((edge, value)) = missing_edge_dfg.iter().next() {
//         println!("Missing edge: {:?}, value: {}", edge, value);
//     }

//     // Step 11: Get maximum cost of edges in dfg
//     let max_traces_of_edge = dfg.values().max().copied().unwrap_or(0);

//     // Step 12: Update missing_edge_dfg with final calculation
//     for ((a, b), value) in missing_edge_dfg.iter_mut() {
//         let current_value = *value;
//         let new_value = 1.0 + (max_traces_of_edge as f64 - 1.0) * (1.0 - current_value);    // need to change this to 2 as well
//         *value = new_value;
//     }

//     // print any 1 value of missing edge
//     if let Some((edge, value)) = missing_edge_dfg.iter().next() {
//         println!("Missing edge: {:?}, value: {}", edge, value);
//     }

//     // Convert to usize HashMap for return
//     let missing_edge_dfg_usize: HashMap<(String, String), usize> = missing_edge_dfg
//         .iter()
//         .map(|((a, b), &value)| ((a.clone(), b.clone()), value.round().max(1.0) as usize))
//         .collect();

//     (missing_edge_dfg_usize, probability_of_missing_edge, rarity_score, normalised_rarity_score)
// }


// Complex link prediction for missing edges
// Returns probability scores for edges that don't exist yet
pub fn cost_of_adding_edge_by_object_type_cl(
    dfg: &HashMap<(String, String), usize>
) -> HashMap<(String, String), f64> {
    
    // Get all activities from the DFG
    let mut nodes = HashSet::new();
    for ((from, to), _) in dfg.iter() {
        nodes.insert(from.clone());
        nodes.insert(to.clone());
    }
    println!("Found {} unique activities in the DFG", nodes.len());
    
    // Find all missing edges - ones that could exist but don't
    let mut missing_dfg: HashMap<(String, String), usize> = HashMap::new();
    for from in &nodes {
        for to in &nodes {
            let edge = (from.clone(), to.clone());
            // Skip self-loops and existing edges
            if from != to && !dfg.contains_key(&edge) {
                missing_dfg.insert(edge, 0);
            }
        }
    }
    println!("Identified {} missing edges out of {} possible edges", 
             missing_dfg.len(), nodes.len() * (nodes.len() - 1));
    
    // Calculate scores for each missing edge
    let missing_edge_scores = calculate_missing_edge_score(&dfg, &missing_dfg);
    
    println!("-------------------------------");
    println!("MISSING EDGE SCORES:");
    println!("Found {} missing edges with calculated scores:", missing_edge_scores.len());
    println!("-------------------------------");

    for ((a, b), score) in &missing_edge_scores {
        println!("Missing edge: ({} -> {}), score: {:.6}", a, b, score);
    }
    
    missing_edge_scores
}

// Calculate scores for missing edges using 5 different factors
// Each factor gets equal 20% weight in the final score
fn calculate_missing_edge_score(
    dfg: &HashMap<(String, String), usize>,
    missing_dfg: &HashMap<(String, String), usize>
) -> HashMap<(String, String), f64> {
    
    // Get all nodes
    let mut nodes = HashSet::new();
    for ((from, to), _) in dfg.iter() {
        nodes.insert(from.clone());
        nodes.insert(to.clone());
    }
    
    // Calculate graph stats
    let stats = calculate_graph_statistics(dfg, &nodes);

    // Get maximum cost of edges in dfg
    let max_traces_of_edge = dfg.values().max().copied().unwrap_or(0);
    
    let mut result = HashMap::new();
    
    for ((from, to), _) in missing_dfg.iter() {
        println!("Calculating score for missing edge: {} -> {}", from, to);
        
        // Factor 1: Common neighbors (20%)
        let jaccard_score = calculate_jaccard_coefficient(from, to, dfg);
        println!("  Jaccard coefficient: {:.4}", jaccard_score);
        
        // Factor 2: Source probability (20%)
        let source_score = calculate_source_transition_probability(from, to, &stats);
        println!("  Source transition probability: {:.4}", source_score);
        
        // Factor 3: Target probability (20%)
        let target_score = calculate_target_reception_probability(from, to, &stats);
        println!("  Target reception probability: {:.4}", target_score);
        
        // Factor 4: Preferential attachment (20%)
        let pref_attach_score = calculate_preferential_attachment(from, to, &stats);
        println!("  Preferential attachment score: {:.4}", pref_attach_score);
        
        // Factor 5: Path analysis (20%)
        let indirect_score = calculate_path_based_score(from, to, dfg);
        println!("  Indirect path score: {:.4}", indirect_score);
        
        // Combine all factors with equal weights
        let final_score = 0.2 * jaccard_score +
                         0.2 * source_score +
                         0.2 * target_score +
                         0.2 * pref_attach_score +
                         0.2 * indirect_score;
        
        println!("  Final combined score: {:.4}", final_score);

        // Translate score to cost (1 to max_traces_of_edge)
        // Higher score = lower cost, lower score = higher cost
        let cost = if max_traces_of_edge > 1 {
            1.0 + (max_traces_of_edge as f64 - 1.0) * (1.0 - final_score)
        } else {
            1.0
        };
        
        println!("  Translated to cost: {:.4}", cost);
        println!();
        
        result.insert((from.clone(), to.clone()), cost);
    }
    
    result
}

// Jaccard coefficient - measures how similar two nodes are based on their neighbors
// Returns value between 0 and 1
fn calculate_jaccard_coefficient(
    from: &str,
    to: &str,
    dfg: &HashMap<(String, String), usize>
) -> f64 {
    // Get all neighbors of 'from' node
    let mut from_neighbors = HashSet::new();
    
    // Outgoing neighbors
    for ((edge_from, edge_to), _) in dfg.iter() {
        if edge_from == from {
            from_neighbors.insert(edge_to.clone());
        }
    }
    
    // Incoming neighbors
    for ((edge_from, edge_to), _) in dfg.iter() {
        if edge_to == from {
            from_neighbors.insert(edge_from.clone());
        }
    }
    
    // Get all neighbors of 'to' node
    let mut to_neighbors = HashSet::new();
    
    // Outgoing neighbors
    for ((edge_from, edge_to), _) in dfg.iter() {
        if edge_from == to {
            to_neighbors.insert(edge_to.clone());
        }
    }
    
    // Incoming neighbors
    for ((edge_from, edge_to), _) in dfg.iter() {
        if edge_to == to {
            to_neighbors.insert(edge_from.clone());
        }
    }
    
    // Calculate intersection and union
    let intersection: HashSet<_> = from_neighbors.intersection(&to_neighbors).collect();
    let union: HashSet<_> = from_neighbors.union(&to_neighbors).collect();
    
    // Return Jaccard coefficient
    if union.is_empty() {
        0.0
    } else {
        intersection.len() as f64 / union.len() as f64
    }
}

// Path-based scoring - checks if nodes are connected through other nodes
// Fewer intermediate nodes = higher score
fn calculate_path_based_score(
    from: &str,
    to: &str,
    dfg: &HashMap<(String, String), usize>
) -> f64 {
    // Check if there's any path from source to target
    if !is_reachable(dfg, from, to) {
        return 0.0;
    }
    
    // Count nodes in between
    let intermediate_count = count_intermediate_nodes(dfg, from, to);
    
    // Convert to score - fewer intermediates = higher score
    let score = (-(intermediate_count as f64) / 2.0).exp();
    
    score
}

// Check if activity2 can be reached from activity1
pub fn is_reachable(
    dfg: &HashMap<(String, String), usize>,
    activity1: &str,
    activity2: &str,
) -> bool {
    let mut visited = HashSet::new();
    let mut stack = vec![activity1.to_string()];

    while let Some(current) = stack.pop() {
        if current == activity2 {
            return true;
        }

        if visited.insert(current.clone()) {
            for ((from, to), _) in dfg {
                if from == &current {
                    stack.push(to.clone());
                }
            }
        }
    }

    false
}

// Count nodes between source and target using shortest path
fn count_intermediate_nodes(
    dfg: &HashMap<(String, String), usize>,
    source: &str,
    target: &str,
) -> usize {
    use std::collections::VecDeque;
    
    let mut queue = VecDeque::new();
    let mut visited = HashSet::new();
    let mut parent: HashMap<String, String> = HashMap::new();
    
    queue.push_back(source.to_string());
    visited.insert(source.to_string());
    
    // BFS to find shortest path
    while let Some(current) = queue.pop_front() {
        if current == target {
            // Reconstruct path and count intermediates
            let mut path = Vec::new();
            let mut node = target.to_string();
            
            while let Some(p) = parent.get(&node) {
                path.push(node.clone());
                node = p.clone();
            }
            path.push(source.to_string());
            path.reverse();
            
            // Return intermediate count (path length - 2)
            return if path.len() >= 2 { path.len() - 2 } else { 0 };
        }
        
        // Explore neighbors
        for ((from, to), _) in dfg {
            if from == &current && !visited.contains(to) {
                visited.insert(to.clone());
                parent.insert(to.clone(), current.clone());
                queue.push_back(to.clone());
            }
        }
    }
    
    // No path found
    usize::MAX
}

// Graph statistics - stores info about node connections and traces
#[allow(dead_code)]
struct GraphStats {
    // How many outgoing edges each node has
    out_degree: HashMap<String, usize>,
    
    // How many incoming edges each node has
    in_degree: HashMap<String, usize>,
    
    // Total traces flowing out of each node
    out_trace_sum: HashMap<String, usize>,
    
    // Total traces flowing into each node
    in_trace_sum: HashMap<String, usize>,
    
    // Total traces in the whole graph
    total_traces: usize,
    
    // Average traces per edge
    avg_traces_per_edge: f64,
    
    // Average out-degree
    avg_out_degree: f64,
    
    // Average in-degree
    avg_in_degree: f64,
}

// Calculate various stats about the graph structure
fn calculate_graph_statistics(
    dfg: &HashMap<(String, String), usize>,
    nodes: &HashSet<String>
) -> GraphStats {
    // Initialize counters
    let mut out_degree = HashMap::new();
    let mut in_degree = HashMap::new();
    let mut out_trace_sum = HashMap::new();
    let mut in_trace_sum = HashMap::new();
    let mut total_traces = 0;
    
    // Set all nodes to 0 initially
    for node in nodes {
        out_degree.insert(node.clone(), 0);
        in_degree.insert(node.clone(), 0);
        out_trace_sum.insert(node.clone(), 0);
        in_trace_sum.insert(node.clone(), 0);
    }
    
    // Process each edge
    for ((from, to), traces) in dfg.iter() {
        // Update connection counts
        *out_degree.get_mut(from).unwrap() += 1;
        *in_degree.get_mut(to).unwrap() += 1;
        
        // Update trace counts
        *out_trace_sum.get_mut(from).unwrap() += traces;
        *in_trace_sum.get_mut(to).unwrap() += traces;
        
        total_traces += traces;
    }
    
    // Calculate averages
    let avg_traces_per_edge = if !dfg.is_empty() {
        total_traces as f64 / dfg.len() as f64
    } else {
        1.0
    };
    
    let avg_out_degree = if !nodes.is_empty() {
        out_degree.values().sum::<usize>() as f64 / nodes.len() as f64
    } else {
        0.0
    };
    
    let avg_in_degree = if !nodes.is_empty() {
        in_degree.values().sum::<usize>() as f64 / nodes.len() as f64
    } else {
        0.0
    };
    
    println!("Graph Statistics:");
    println!("  - Total nodes: {}", nodes.len());
    println!("  - Total edges: {}", dfg.len());
    println!("  - Total traces: {}", total_traces);
    println!("  - Average traces per edge: {:.2}", avg_traces_per_edge);
    println!("  - Average out-degree: {:.2}", avg_out_degree);
    println!("  - Average in-degree: {:.2}", avg_in_degree);
    
    GraphStats {
        out_degree,
        in_degree,
        out_trace_sum,
        in_trace_sum,
        total_traces,
        avg_traces_per_edge,
        avg_out_degree,
        avg_in_degree,
    }
}



// How likely is a source node to create new outgoing edges
// Nodes with more connections tend to make more connections
fn calculate_source_transition_probability(
    from: &str,
    _to: &str,  // Prefixed with _ to suppress unused warning
    stats: &GraphStats
) -> f64 {
    // Get outgoing edges for this node
    let out_degree = *stats.out_degree.get(from).unwrap_or(&0) as f64;
    let avg_out_degree = stats.avg_out_degree;
    
    // Empty graph case
    if avg_out_degree == 0.0 {
        return 0.5;
    }
    
    // Compare to average - more connections = higher score
    let degree_ratio = out_degree / avg_out_degree;
    
    // Normalize to 0-1 range
    let score = degree_ratio / (degree_ratio + 1.0);
    
    score.max(0.01)
}

// How likely is a target node to receive new incoming edges
// Popular nodes tend to get more connections
fn calculate_target_reception_probability(
    _from: &str,  // Prefixed with _ to suppress unused warning
    to: &str,
    stats: &GraphStats
) -> f64 {
    // Get incoming edges for this node
    let in_degree = *stats.in_degree.get(to).unwrap_or(&0) as f64;
    let avg_in_degree = stats.avg_in_degree;
    
    // Empty graph case
    if avg_in_degree == 0.0 {
        return 0.5;
    }
    
    // Compare to average - more connections = higher score
    let degree_ratio = in_degree / avg_in_degree;
    
    // Normalize to 0-1 range
    let score = degree_ratio / (degree_ratio + 1.0);
    
    score.max(0.01)
}



// Preferential attachment - "rich get richer" 
// Popular nodes are more likely to connect to other popular nodes
fn calculate_preferential_attachment(
    from: &str,
    to: &str,
    stats: &GraphStats
) -> f64 {
    // Get connection counts
    let from_degree = *stats.out_degree.get(from).unwrap_or(&0) as f64;
    let to_degree = *stats.in_degree.get(to).unwrap_or(&0) as f64;
    
    // Give small score to nodes with no connections
    if from_degree == 0.0 || to_degree == 0.0 {
        return 0.01;
    }
    
    // Calculate score using geometric mean
    let score = (from_degree * to_degree).sqrt();
    
    // Normalize by max possible score
    let max_out_degree = stats.out_degree.values().max().unwrap_or(&1);
    let max_in_degree = stats.in_degree.values().max().unwrap_or(&1);
    let max_possible = (*max_out_degree * *max_in_degree) as f64;
    
    if max_possible > 0.0 {
        (score / max_possible.sqrt()).min(1.0)
    } else {
        0.0
    }
}

