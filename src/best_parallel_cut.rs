use std::collections::{HashMap, HashSet};
use rand::prelude::*;

#[derive(Debug, Clone)]
pub struct PartitionResult {
    pub minimum_cost: usize,
    pub num_edges_added: usize,
    pub edges_to_add: Vec<(String, String)>,
    pub set1: HashSet<String>,
    pub set2: HashSet<String>,
    pub new_dfg: HashMap<(String, String), usize>,
}

#[derive(Debug, Clone)]
struct Solution {
    set1: HashSet<String>,
    set2: HashSet<String>,
    cost: usize,
    edges_to_add: Vec<(String, String)>,
}

pub fn best_parallel_cut(
    dfg: &HashMap<(String, String), usize>,
    all_activities: &HashSet<String>,
) -> PartitionResult {
    println!("Starting bipartite partition for {} activities", all_activities.len());
    
    let activities: Vec<String> = all_activities.iter().cloned().collect();
    let n = activities.len();
    
    // Create edge set for fast lookup (ignoring costs)
    let edge_set: HashSet<(String, String)> = dfg.keys().cloned().collect();
    
    // Parameters for heuristic
    let num_restarts = std::cmp::min(50, std::cmp::max(10, n));
    let min_set_size = std::cmp::max(1, n / 4);
    
    let mut best_solution: Option<Solution> = None;
    let mut rng = thread_rng();
    
    println!("Performing {} random restarts", num_restarts);
    
    for restart in 0..num_restarts {
        if restart % 10 == 0 {
            println!("Restart {}/{}", restart + 1, num_restarts);
        }
        
        // STEP 1: Generate initial random partition
        let mut solution = generate_initial_partition(&activities, min_set_size, &mut rng);
        solution.cost = calculate_cost(&solution.set1, &solution.set2, &edge_set);
        solution.edges_to_add = get_edges_to_add(&solution.set1, &solution.set2, &edge_set);
        
        // STEP 2: Local search improvement
        solution = local_search_improvement(solution, &edge_set);
        
        // STEP 3: Track best solution
        if best_solution.is_none() || solution.cost < best_solution.as_ref().unwrap().cost {
            println!("New best solution found! Cost: {}", solution.cost);
            best_solution = Some(solution);
        }
    }
    
    let mut best_solution = best_solution.unwrap();
    
    // STEP 4: Final optimization
    println!("Applying final optimizations...");
    best_solution = final_optimization(best_solution, &edge_set);
    
    // Create result
    create_partition_result(best_solution, dfg)
}

fn generate_initial_partition(
    activities: &[String],
    min_set_size: usize,
    rng: &mut ThreadRng,
) -> Solution {
    let n = activities.len();
    let max_set_size = n - min_set_size;
    let set1_size = rng.gen_range(min_set_size..=max_set_size);
    
    let mut indices: Vec<usize> = (0..n).collect();
    indices.shuffle(rng);
    
    let set1: HashSet<String> = indices[..set1_size]
        .iter()
        .map(|&i| activities[i].clone())
        .collect();
    
    let set2: HashSet<String> = indices[set1_size..]
        .iter()
        .map(|&i| activities[i].clone())
        .collect();
    
    Solution {
        set1,
        set2,
        cost: 0,
        edges_to_add: Vec::new(),
    }
}

fn local_search_improvement(
    mut solution: Solution,
    edge_set: &HashSet<(String, String)>,
) -> Solution {
    let mut iteration = 0;
    let mut improvements = 0;
    
    loop {
        iteration += 1;
        let mut best_move: Option<Solution> = None;
        let current_cost = solution.cost;
        
        // Try moving each activity to the other set
        let all_activities: Vec<String> = solution.set1.iter()
            .chain(solution.set2.iter())
            .cloned()
            .collect();
        
        for activity in &all_activities {
            let new_solution = if solution.set1.contains(activity) {
                // Move from set1 to set2
                if solution.set1.len() <= 1 { continue; } // Don't empty a set
                
                let mut new_set1 = solution.set1.clone();
                let mut new_set2 = solution.set2.clone();
                new_set1.remove(activity);
                new_set2.insert(activity.clone());
                
                let cost = calculate_cost(&new_set1, &new_set2, edge_set);
                let edges_to_add = get_edges_to_add(&new_set1, &new_set2, edge_set);
                
                Solution {
                    set1: new_set1,
                    set2: new_set2,
                    cost,
                    edges_to_add,
                }
            } else {
                // Move from set2 to set1
                if solution.set2.len() <= 1 { continue; } // Don't empty a set
                
                let mut new_set1 = solution.set1.clone();
                let mut new_set2 = solution.set2.clone();
                new_set1.insert(activity.clone());
                new_set2.remove(activity);
                
                let cost = calculate_cost(&new_set1, &new_set2, edge_set);
                let edges_to_add = get_edges_to_add(&new_set1, &new_set2, edge_set);
                
                Solution {
                    set1: new_set1,
                    set2: new_set2,
                    cost,
                    edges_to_add,
                }
            };
            
            if new_solution.cost < current_cost {
                if best_move.is_none() || new_solution.cost < best_move.as_ref().unwrap().cost {
                    best_move = Some(new_solution);
                }
            }
        }
        
        // Apply best move if found
        if let Some(best) = best_move {
            solution = best;
            improvements += 1;
        } else {
            break;
        }
    }
    
    if improvements > 0 {
        println!("  Local search: {} improvements in {} iterations, final cost: {}", 
                improvements, iteration, solution.cost);
    }
    
    solution
}

fn final_optimization(
    mut solution: Solution,
    edge_set: &HashSet<(String, String)>,
) -> Solution {
    // Try activity swaps between sets
    let set1_vec: Vec<String> = solution.set1.iter().cloned().collect();
    let set2_vec: Vec<String> = solution.set2.iter().cloned().collect();
    
    let swap_limit = std::cmp::min(8, std::cmp::min(set1_vec.len(), set2_vec.len()));
    
    for i in 0..swap_limit {
        for j in 0..swap_limit {
            let activity1 = &set1_vec[i];
            let activity2 = &set2_vec[j];
            
            // Create swapped solution
            let mut new_set1 = solution.set1.clone();
            let mut new_set2 = solution.set2.clone();
            
            new_set1.remove(activity1);
            new_set1.insert(activity2.clone());
            new_set2.remove(activity2);
            new_set2.insert(activity1.clone());
            
            let new_cost = calculate_cost(&new_set1, &new_set2, edge_set);
            
            if new_cost < solution.cost {
                solution.set1 = new_set1;
                solution.set2 = new_set2;
                solution.cost = new_cost;
                solution.edges_to_add = get_edges_to_add(&solution.set1, &solution.set2, edge_set);
                println!("  Swap optimization improved cost to: {}", solution.cost);
                break;
            } else if (new_set1.len().abs_diff(new_set2.len()) < solution.set1.len().abs_diff(solution.set2.len())) {
                solution.set1 = new_set1;
                solution.set2 = new_set2;
                solution.cost = new_cost;
                solution.edges_to_add = get_edges_to_add(&solution.set1, &solution.set2, edge_set);
                println!("  Swap optimization improved cost to: {}", solution.cost);
                break;
            }
        }
    }
    
    solution
}

fn calculate_cost(
    set1: &HashSet<String>,
    set2: &HashSet<String>,
    edge_set: &HashSet<(String, String)>,
) -> usize {
    let mut cost = 0;
    
    for act1 in set1 {
        for act2 in set2 {
            // Need edge from act1 to act2
            if !edge_set.contains(&(act1.clone(), act2.clone())) {
                cost += 1;
            }
            // Need edge from act2 to act1
            if !edge_set.contains(&(act2.clone(), act1.clone())) {
                cost += 1;
            }
        }
    }
    
    cost
}

fn get_edges_to_add(
    set1: &HashSet<String>,
    set2: &HashSet<String>,
    edge_set: &HashSet<(String, String)>,
) -> Vec<(String, String)> {
    let mut edges_to_add = Vec::new();
    
    for act1 in set1 {
        for act2 in set2 {
            // Need edge from act1 to act2
            if !edge_set.contains(&(act1.clone(), act2.clone())) {
                edges_to_add.push((act1.clone(), act2.clone()));
            }
            // Need edge from act2 to act1
            if !edge_set.contains(&(act2.clone(), act1.clone())) {
                edges_to_add.push((act2.clone(), act1.clone()));
            }
        }
    }
    
    edges_to_add
}

fn create_partition_result(
    solution: Solution,
    original_dfg: &HashMap<(String, String), usize>,
) -> PartitionResult {
    let mut new_dfg = original_dfg.clone();
    
    // Add new edges with cost 1
    for (from, to) in &solution.edges_to_add {
        new_dfg.insert((from.clone(), to.clone()), 1);
    }
    
    PartitionResult {
        minimum_cost: solution.cost,
        num_edges_added: solution.edges_to_add.len(),
        edges_to_add: solution.edges_to_add,
        set1: solution.set1,
        set2: solution.set2,
        new_dfg,
    }
}
