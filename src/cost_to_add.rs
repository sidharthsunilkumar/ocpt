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
    
    // Chao2 estimator - yeilds an estimate of speicies richness i.e. the number of species (seen and unseen)
    // i.e estimate of total edges
    let s_chao2 = if q2 > 0 {
        s_obs as f64 + (q1 * q1) as f64 / (2.0 * q2 as f64)
    } else {
        s_obs as f64 + ((q1 * (q1 - 1)) as f64) / 2.0
    };

    // Estimate of unseen edges
    let q0 = s_chao2 - s_obs as f64;

    let est_cost: usize = (q1 as f64 / q0 as f64).ceil() as usize; //round up

    println!("Cost to add; s_obs: {}, q1: {}, q2: {}, n: {}, s_chao2: {}, q0: {}, est_cost: {}\n", 
          s_obs, q1, q2, n, s_chao2, q0, est_cost);
    
    est_cost
}
