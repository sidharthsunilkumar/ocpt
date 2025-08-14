use std::collections::{HashMap, HashSet, VecDeque};

use log::info;

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



pub fn compute_missing_edge_costsv2(
    dfg: &HashMap<(String, String), usize>,
) -> HashMap<(String, String), usize> {
    // Step 1: Count q1, q2, and total events n
    let s_obs = dfg.len();
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

    let m1 = *dfg.values().min().unwrap_or(&1);
    let m2 = *dfg.values().max().unwrap_or(&1);

    // Step 2: Base cost = (1 - probability of unseen edge) scaled between m1 and m2
    let base_cost = if n == 0 {
        m1 // if no edges at all, fallback
    } else {
        let scaled = m1 as f64
            + (1.0 - prob_unseen_edge) * (m2 - m1) as f64;
        scaled.round() as usize
    };

    // Step 3: Collect activities
    let mut activities = HashSet::new();
    for ((src, dst), _) in dfg {
        activities.insert(src.clone());
        activities.insert(dst.clone());
    }

    // Step 4: Compute in/out degrees
    let mut out_counts: HashMap<String, usize> = HashMap::new();
    let mut in_counts: HashMap<String, usize> = HashMap::new();

    for ((src, dst), _) in dfg {
        *out_counts.entry(src.clone()).or_default() += 1;
        *in_counts.entry(dst.clone()).or_default() += 1;
    }

    let avg_out = if activities.len() > 0 {
        out_counts.values().sum::<usize>() as f64 / activities.len() as f64
    } else {
        1.0
    };

    let avg_in = if activities.len() > 0 {
        in_counts.values().sum::<usize>() as f64 / activities.len() as f64
    } else {
        1.0
    };

    // Step 5: Calculate missing edges with adjusted costs
    let mut missing_edges: HashMap<(String, String), usize> = HashMap::new();
    for a in &activities {
        for b in &activities {
            if a != b && !dfg.contains_key(&(a.clone(), b.clone())) {
                let out_a = *out_counts.get(a).unwrap_or(&0) as f64 / avg_out;
                let in_b = *in_counts.get(b).unwrap_or(&0) as f64 / avg_in;

                let degree_factor = 1.0 - (out_a - in_b);
                let adjusted_cost =
                    ((base_cost as f64) * degree_factor).round().max(m1 as f64) as usize;

                missing_edges.insert((a.clone(), b.clone()), adjusted_cost);

                println!("m1: {}, m2: {}, prob_unseen_edge: {}, out_a: {}, in_b: {}, degree_factor: {}, adjusted_cost: {}", m1, m2, prob_unseen_edge, out_a, in_b, degree_factor, adjusted_cost);
            }
        }
    }

    

    missing_edges
}



pub fn compute_missing_edge_costsv1(
    dfg: &HashMap<(String, String), usize>
) -> HashMap<(String, String), usize> {
    let epsilon = 1e-6;
    let alpha = 0.1; // degree rarity scaling
    let beta = 2.0;  // novelty penalty scaling

    // --- Step 0: Extract all unique activities ---
    let mut activities: HashSet<String> = HashSet::new();
    for ((src, tgt), _) in dfg.iter() {
        activities.insert(src.clone());
        activities.insert(tgt.clone());
    }

    // --- Step 1: Precompute global novelty multiplier (species discovery) ---
    let mut q1 = 0;
    let mut q2 = 0;
    let s_obs = dfg.len();
    for &count in dfg.values() {
        if count == 1 { q1 += 1; }
        else if count == 2 { q2 += 1; }
    }
    let s_chao2 = if q2 > 0 {
        s_obs as f64 + (q1 * q1) as f64 / (2.0 * q2 as f64)
    } else {
        s_obs as f64 + ((q1 * (q1 - 1)) as f64) / 2.0
    };
    let q0 = (s_chao2 - s_obs as f64).max(0.0);
    let unseen_prob = q1 as f64 / ((q1 as f64) + q0 + epsilon);
    let novelty_multiplier = 1.0 + beta * (1.0 - unseen_prob);

    // --- Step 2: Compute all possible missing edges ---
    let mut result: HashMap<(String, String), usize> = HashMap::new();

    for a in &activities {
        for b in &activities {
            if a == b { continue; }
            if dfg.contains_key(&(a.clone(), b.clone())) { continue; }

            // Step 2.1: Local probability
            let total_out_a: usize = dfg.iter()
                .filter(|((src, _), _)| src == a)
                .map(|(_, &count)| count)
                .sum();  
            let count_ab = *dfg.get(&(a.clone(), b.clone())).unwrap_or(&0);
            let p_ab = (count_ab as f64) / (total_out_a as f64 + epsilon);
            let local_cost = -(p_ab + epsilon).log2();

            // Step 2.2: Degree rarity penalty
            let deg_out_a = dfg.keys().filter(|(src, _)| src == a).count();
            let deg_in_b = dfg.keys().filter(|(_, tgt)| tgt == b).count();
            let degree_penalty = 1.0 + alpha * (deg_out_a as f64 + deg_in_b as f64);

            // Step 2.3: Final hybrid cost
            let final_cost = (local_cost * degree_penalty * novelty_multiplier).max(1.0);
            result.insert((a.clone(), b.clone()), final_cost.ceil() as usize);
        }
    }

    result
}
