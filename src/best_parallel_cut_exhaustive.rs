use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct PartitionResult {
    pub minimum_cost: usize,
    pub num_edges_added: usize,
    pub edges_to_add: Vec<(String, String)>,
    pub set1: Vec<String>,
    pub set2: Vec<String>,
    pub new_dfg: HashMap<(String, String), usize>,
}

/// Exhaustive search for optimal bipartite partition
/// WARNING: Exponential time complexity O(2^n) - only suitable for small inputs (n ≤ 20)
pub fn best_parallel_cut_exhaustive(
    dfg: &HashMap<(String, String), usize>,
    all_activities: &HashSet<String>,
) -> PartitionResult {
    let activities: Vec<String> = all_activities.iter().cloned().collect();
    let n = activities.len();
    
    println!("Starting exhaustive search for {} activities", n);
    
    if n > 20 {
        println!("WARNING: Exhaustive search with {} activities will take very long!", n);
        println!("Consider using the heuristic approach instead.");
        println!("Estimated combinations to check: {}", (1u64 << n) - 2);
    }
    
    // Create edge set for fast lookup (ignoring costs)
    let edge_set: HashSet<(String, String)> = dfg.keys().cloned().collect();
    
    let mut best_cost = usize::MAX;
    let mut best_set1: Vec<String> = Vec::new();
    let mut best_set2: Vec<String> = Vec::new();
    let mut best_edges_to_add: Vec<(String, String)> = Vec::new();
    
    let total_partitions = (1u64 << n) - 2; // Exclude empty sets
    let mut checked_partitions = 0u64;
    let report_interval = std::cmp::max(1, total_partitions / 100); // Report every 1%
    
    println!("Checking {} possible partitions...", total_partitions);
    
    // Try all possible non-empty partitions
    // Bit mask from 1 to 2^n - 2 (excluding 0 and 2^n - 1 which would create empty sets)
    for mask in 1..(1u64 << n) - 1 {
        checked_partitions += 1;
        
        // Progress reporting
        if checked_partitions % report_interval == 0 {
            let progress = (checked_partitions as f64 / total_partitions as f64) * 100.0;
            println!("Progress: {:.1}% ({}/{}), Current best cost: {}", 
                    progress, checked_partitions, total_partitions, best_cost);
        }
        
        // Create partition based on bit mask
        let mut set1 = Vec::new();
        let mut set2 = Vec::new();
        
        for i in 0..n {
            if mask & (1u64 << i) != 0 {
                set1.push(activities[i].clone());
            } else {
                set2.push(activities[i].clone());
            }
        }
        
        // Calculate cost for this partition
        let (cost, edges_to_add) = calculate_partition_cost(&set1, &set2, &edge_set);
        
        // Update best solution if this is better
        if cost < best_cost {
            best_cost = cost;
            best_set1 = set1;
            best_set2 = set2;
            best_edges_to_add = edges_to_add;
            
            println!("New best solution found! Cost: {}, Set1 size: {}, Set2 size: {}", 
                    best_cost, best_set1.len(), best_set2.len());
        }
    }
    
    println!("Exhaustive search completed!");
    println!("Checked {} partitions", checked_partitions);
    println!("Optimal cost: {}", best_cost);
    
    // Create the new DFG with added edges
    let mut new_dfg = dfg.clone();
    for (from, to) in &best_edges_to_add {
        new_dfg.insert((from.clone(), to.clone()), 1);
    }
    
    PartitionResult {
        minimum_cost: best_cost,
        num_edges_added: best_edges_to_add.len(),
        edges_to_add: best_edges_to_add,
        set1: best_set1,
        set2: best_set2,
        new_dfg,
    }
}

/// Calculate the cost of a specific partition and return edges to add
fn calculate_partition_cost(
    set1: &[String],
    set2: &[String],
    edge_set: &HashSet<(String, String)>,
) -> (usize, Vec<(String, String)>) {
    let mut cost = 0;
    let mut edges_to_add = Vec::new();
    
    // Check all required edges between set1 and set2
    for act1 in set1 {
        for act2 in set2 {
            // Need edge from act1 to act2
            if !edge_set.contains(&(act1.clone(), act2.clone())) {
                edges_to_add.push((act1.clone(), act2.clone()));
                cost += 1;
            }
            
            // Need edge from act2 to act1
            if !edge_set.contains(&(act2.clone(), act1.clone())) {
                edges_to_add.push((act2.clone(), act1.clone()));
                cost += 1;
            }
        }
    }
    
    (cost, edges_to_add)
}

/// Optimized exhaustive search using bit manipulation tricks
/// Slightly faster for larger inputs within the feasible range
pub fn best_parallel_cut_exhaustive_optimized(
    dfg: &HashMap<(String, String), usize>,
    all_activities: &HashSet<String>,
) -> PartitionResult {
    let activities: Vec<String> = all_activities.iter().cloned().collect();
    let n = activities.len();
    
    println!("Starting optimized exhaustive search for {} activities", n);
    
    if n > 20 {
        println!("WARNING: Even optimized exhaustive search with {} activities will take very long!", n);
        return best_parallel_cut_exhaustive(dfg, all_activities); // Fall back to regular version
    }
    
    // Create edge lookup matrix for faster access
    let mut edge_matrix = vec![vec![false; n]; n];
    let activity_to_index: HashMap<String, usize> = activities.iter()
        .enumerate()
        .map(|(i, act)| (act.clone(), i))
        .collect();
    
    for (from, to) in dfg.keys() {
        if let (Some(&from_idx), Some(&to_idx)) = (activity_to_index.get(from), activity_to_index.get(to)) {
            edge_matrix[from_idx][to_idx] = true;
        }
    }
    
    let mut best_cost = usize::MAX;
    let mut best_mask = 0u64;
    
    let total_partitions = (1u64 << n) - 2;
    let mut checked = 0u64;
    
    println!("Checking {} partitions with optimized approach...", total_partitions);
    
    // Try all masks, but with optimizations
    for mask in 1..(1u64 << n) - 1 {
        checked += 1;
        
        if checked % 100000 == 0 {
            let progress = (checked as f64 / total_partitions as f64) * 100.0;
            println!("Progress: {:.1}%, Current best: {}", progress, best_cost);
        }
        
        // Quick calculation using precomputed matrix
        let cost = calculate_cost_optimized(mask, n, &edge_matrix);
        
        if cost < best_cost {
            best_cost = cost;
            best_mask = mask;
            
            let set1_size = mask.count_ones();
            let set2_size = n as u32 - set1_size;
            println!("New best: cost={}, set1_size={}, set2_size={}", 
                    cost, set1_size, set2_size);
        }
    }
    
    // Reconstruct the best solution
    let mut best_set1 = Vec::new();
    let mut best_set2 = Vec::new();
    
    for i in 0..n {
        if best_mask & (1u64 << i) != 0 {
            best_set1.push(activities[i].clone());
        } else {
            best_set2.push(activities[i].clone());
        }
    }
    
    // Calculate edges to add for the best solution
    let edge_set: HashSet<(String, String)> = dfg.keys().cloned().collect();
    let (_, best_edges_to_add) = calculate_partition_cost(&best_set1, &best_set2, &edge_set);
    
    // Create new DFG
    let mut new_dfg = dfg.clone();
    for (from, to) in &best_edges_to_add {
        new_dfg.insert((from.clone(), to.clone()), 1);
    }
    
    println!("Optimized exhaustive search completed!");
    println!("Optimal cost: {}", best_cost);
    
    PartitionResult {
        minimum_cost: best_cost,
        num_edges_added: best_edges_to_add.len(),
        edges_to_add: best_edges_to_add,
        set1: best_set1,
        set2: best_set2,
        new_dfg,
    }
}

/// Fast cost calculation using bit manipulation and precomputed matrix
fn calculate_cost_optimized(mask: u64, n: usize, edge_matrix: &[Vec<bool>]) -> usize {
    let mut cost = 0;
    
    for i in 0..n {
        for j in 0..n {
            // Check if i and j are in different sets
            let i_in_set1 = (mask & (1u64 << i)) != 0;
            let j_in_set1 = (mask & (1u64 << j)) != 0;
            
            if i_in_set1 != j_in_set1 {
                // They're in different sets, so we need an edge i->j
                if !edge_matrix[i][j] {
                    cost += 1;
                }
            }
        }
    }
    
    cost
}

/// Memory-efficient exhaustive search for very small inputs
/// Uses iterative deepening to find good solutions quickly
pub fn best_parallel_cut_exhaustive_memory_efficient(
    dfg: &HashMap<(String, String), usize>,
    all_activities: &HashSet<String>,
) -> PartitionResult {
    let activities: Vec<String> = all_activities.iter().cloned().collect();
    let n = activities.len();
    
    if n > 15 {
        println!("Memory-efficient version recommended only for n ≤ 15");
        return best_parallel_cut_exhaustive(dfg, all_activities);
    }
    
    println!("Starting memory-efficient exhaustive search for {} activities", n);
    
    let edge_set: HashSet<(String, String)> = dfg.keys().cloned().collect();
    
    // Use recursive approach with pruning
    let mut best_cost = usize::MAX;
    let mut best_solution = None;
    
    // Try different set1 sizes
    for set1_size in 1..n {
        println!("Trying partitions with set1_size = {}", set1_size);
        
        search_combinations(
            &activities,
            &edge_set,
            set1_size,
            0,
            &mut Vec::new(),
            &mut best_cost,
            &mut best_solution,
        );
    }
    
    let (best_set1, best_set2, best_edges_to_add) = best_solution.unwrap();
    
    let mut new_dfg = dfg.clone();
    for (from, to) in &best_edges_to_add {
        new_dfg.insert((from.clone(), to.clone()), 1);
    }
    
    println!("Memory-efficient exhaustive search completed!");
    println!("Optimal cost: {}", best_cost);
    
    PartitionResult {
        minimum_cost: best_cost,
        num_edges_added: best_edges_to_add.len(),
        edges_to_add: best_edges_to_add,
        set1: best_set1,
        set2: best_set2,
        new_dfg,
    }
}

/// Recursive combination search with pruning
fn search_combinations(
    activities: &[String],
    edge_set: &HashSet<(String, String)>,
    target_size: usize,
    start_idx: usize,
    current_set1: &mut Vec<String>,
    best_cost: &mut usize,
    best_solution: &mut Option<(Vec<String>, Vec<String>, Vec<(String, String)>)>,
) {
    // If we have enough elements, evaluate this partition
    if current_set1.len() == target_size {
        let set2: Vec<String> = activities.iter()
            .filter(|act| !current_set1.contains(act))
            .cloned()
            .collect();
        
        let (cost, edges_to_add) = calculate_partition_cost(current_set1, &set2, edge_set);
        
        if cost < *best_cost {
            *best_cost = cost;
            *best_solution = Some((current_set1.clone(), set2, edges_to_add));
        }
        return;
    }
    
    // Pruning: if we can't reach target_size, return
    let remaining_needed = target_size - current_set1.len();
    let remaining_available = activities.len() - start_idx;
    if remaining_available < remaining_needed {
        return;
    }
    
    // Try adding each remaining activity
    for i in start_idx..activities.len() {
        current_set1.push(activities[i].clone());
        search_combinations(activities, edge_set, target_size, i + 1, current_set1, best_cost, best_solution);
        current_set1.pop();
    }
}
