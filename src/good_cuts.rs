
use std::collections::{HashMap, HashSet, VecDeque, BTreeMap};
use log::info;

use petgraph::graphmap::DiGraphMap;
use petgraph::graph::{Graph, NodeIndex};
use petgraph::algo::ford_fulkerson;
use petgraph::visit::EdgeRef;
use petgraph::Direction;

/// Entry function to perform the minimum cut on a directed graph with edge costs.
/// 
/// # Arguments
/// * `dfg` - A reference to a HashMap representing the directed edges and their costs.
/// * `all_activities` - A reference to a HashSet of all activity names (nodes).
// pub fn best_possible_sequence_cut(dfg: &HashMap<(String, String), usize>, all_activities: &HashSet<String>) {
//     let nodes: Vec<String> = all_activities.iter().cloned().collect();
//     let mut best_cut_cost = usize::MAX;
//     let mut best_cut_edges = Vec::new();
//     let mut best_set1 = HashSet::new();
//     let mut best_set2 = HashSet::new();

//     // Try every pair of nodes (s, t)
//     for s in &nodes {
//         for t in &nodes {
//             if s == t {
//                 continue;
//             }

//             // Build a residual graph from the original dfg
//             let (cut_cost, cut_edges, set1) = min_st_cut(s, t, dfg, all_activities);

//             if cut_cost < best_cut_cost {
//                 best_cut_cost = cut_cost;
//                 best_cut_edges = cut_edges;
//                 best_set1 = set1;
//                 best_set2 = all_activities.difference(&best_set1).cloned().collect();
//             }
//         }
//     }

//     info!("Minimum cut cost: {}", best_cut_cost);
//     info!("Edges to remove:");
//     for (a, b) in best_cut_edges {
//         info!("  {} -> {}", a, b);
//     }

//     info!("Set 1: {:?}", best_set1);
//     info!("Set 2: {:?}", best_set2);
// }

// /// Implements a simplified Edmonds-Karp algorithm to find the min s-t cut.
// /// Returns (cut_cost, edges_to_remove, set1_nodes)
// fn min_st_cut(
//     s: &String,
//     t: &String,
//     dfg: &HashMap<(String, String), usize>,
//     all_nodes: &HashSet<String>,
// ) -> (usize, Vec<(String, String)>, HashSet<String>) {
//     use std::collections::BTreeMap;

//     // Step 1: Build adjacency list
//     let mut capacity: HashMap<String, HashMap<String, usize>> = HashMap::new();
//     for ((u, v), &cost) in dfg {
//         capacity.entry(u.clone()).or_default().insert(v.clone(), cost);
//     }

//     // Step 2: BFS to find reachable nodes from s (residual graph)
//     let mut reachable = HashSet::new();
//     let mut queue = VecDeque::new();
//     queue.push_back(s.clone());
//     reachable.insert(s.clone());

//     while let Some(node) = queue.pop_front() {
//         if let Some(neighbors) = capacity.get(&node) {
//             for (neighbor, &cap) in neighbors {
//                 if cap > 0 && !reachable.contains(neighbor) {
//                     reachable.insert(neighbor.clone());
//                     queue.push_back(neighbor.clone());
//                 }
//             }
//         }
//     }

//     // Step 3: Determine cut edges (from reachable to unreachable nodes)
//     let mut cut_edges = Vec::new();
//     let mut cut_cost = 0;

//     for ((u, v), &cost) in dfg {
//         if reachable.contains(&u[..]) && !reachable.contains(&v[..]) {
//             cut_edges.push((u.clone(), v.clone()));
//             cut_cost += cost;
//         }
//     }

//     (cut_cost, cut_edges, reachable)
// }

// pub fn best_possible_sequence_cut(dfg: &HashMap<(String, String), usize>, all_activities: &HashSet<String>){
    
// }

//claude code-
use std::cmp;

#[derive(Debug, Clone)]
pub struct CutResult {
    pub total_cost: usize,
    pub edges_to_cut: Vec<((String, String), usize)>,
    pub set1: Vec<String>,
    pub set2: Vec<String>,
}

pub struct ActivityPartitioner {
    activities: Vec<String>,
    activity_to_idx: HashMap<String, usize>,
    graph: Vec<Vec<usize>>,
    cost_matrix: Vec<Vec<usize>>,
    n: usize,
}

impl ActivityPartitioner {
    pub fn new(dfg: &HashMap<(String, String), usize>, all_activities: &HashSet<String>) -> Self {
        let activities: Vec<String> = all_activities.iter().cloned().collect();
        let n = activities.len();
        
        let mut activity_to_idx = HashMap::new();
        for (i, activity) in activities.iter().enumerate() {
            activity_to_idx.insert(activity.clone(), i);
        }
        
        let mut graph = vec![Vec::new(); n];
        let mut cost_matrix = vec![vec![0; n]; n];
        
        // Build adjacency list and cost matrix
        for ((start, end), cost) in dfg.iter() {
            if let (Some(&start_idx), Some(&end_idx)) = 
                (activity_to_idx.get(start), activity_to_idx.get(end)) {
                graph[start_idx].push(end_idx);
                cost_matrix[start_idx][end_idx] = *cost;
            }
        }
        
        ActivityPartitioner {
            activities,
            activity_to_idx,
            graph,
            cost_matrix,
            n,
        }
    }
    
    pub fn solve(&self) -> Option<CutResult> {
        let mut best_cost = usize::MAX;
        let mut best_result: Option<CutResult> = None;
        
        // Try all possible source-sink pairs
        for source in 0..self.n {
            for sink in 0..self.n {
                if source == sink {
                    continue;
                }
                
                if let Some(result) = self.try_source_sink(source, sink) {
                    if !result.set1.is_empty() && !result.set2.is_empty() && 
                       result.total_cost < best_cost &&
                       self.validate_partition(&result.set1, &result.set2) {
                        best_cost = result.total_cost;
                        best_result = Some(result);
                    }
                }
            }
        }
        
        best_result
    }
    
    fn try_source_sink(&self, source: usize, sink: usize) -> Option<CutResult> {
        // Create capacity matrix for flow network
        let mut capacity = vec![vec![0; self.n]; self.n];
        
        // Set capacities based on edge costs
        for i in 0..self.n {
            for &j in &self.graph[i] {
                capacity[i][j] = self.cost_matrix[i][j];
            }
        }
        
        // Find max flow using Edmonds-Karp
        let (max_flow, residual) = self.edmonds_karp(source, sink, capacity);
        
        // Find the actual cut edges
        let mut cut_edges = Vec::new();
        for i in 0..self.n {
            for j in 0..self.n {
                // If there was originally an edge but now no residual capacity
                if self.cost_matrix[i][j] > 0 && residual[i][j] == 0 {
                    cut_edges.push((
                        (self.activities[i].clone(), self.activities[j].clone()),
                        self.cost_matrix[i][j]
                    ));
                }
            }
        }
        
        // Find reachable nodes from source in residual graph
        let reachable = self.find_reachable(source, &residual);
        
        let mut set1 = Vec::new();
        let mut set2 = Vec::new();
        
        for i in 0..self.n {
            if reachable.contains(&i) {
                set1.push(self.activities[i].clone());
            } else {
                set2.push(self.activities[i].clone());
            }
        }
        
        Some(CutResult {
            total_cost: max_flow,
            edges_to_cut: cut_edges,
            set1,
            set2,
        })
    }
    
    fn edmonds_karp(&self, source: usize, sink: usize, mut capacity: Vec<Vec<usize>>) 
        -> (usize, Vec<Vec<usize>>) {
        
        let mut max_flow = 0;
        
        loop {
            // BFS to find augmenting path
            let mut parent = vec![-1i32; self.n];
            let mut visited = vec![false; self.n];
            let mut queue = VecDeque::new();
            
            queue.push_back(source);
            visited[source] = true;
            
            let mut found_path = false;
            
            while let Some(u) = queue.pop_front() {
                if u == sink {
                    found_path = true;
                    break;
                }
                
                for v in 0..self.n {
                    if !visited[v] && capacity[u][v] > 0 {
                        visited[v] = true;
                        parent[v] = u as i32;
                        queue.push_back(v);
                    }
                }
            }
            
            if !found_path {
                break;
            }
            
            // Find minimum capacity along the path
            let mut path_flow = usize::MAX;
            let mut v = sink;
            
            while parent[v] != -1 {
                let u = parent[v] as usize;
                path_flow = cmp::min(path_flow, capacity[u][v]);
                v = u;
            }
            
            // Update residual capacities
            v = sink;
            while parent[v] != -1 {
                let u = parent[v] as usize;
                capacity[u][v] -= path_flow;
                capacity[v][u] += path_flow;
                v = u;
            }
            
            max_flow += path_flow;
        }
        
        (max_flow, capacity)
    }
    
    fn find_reachable(&self, source: usize, residual: &[Vec<usize>]) -> HashSet<usize> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        
        queue.push_back(source);
        visited.insert(source);
        
        while let Some(u) = queue.pop_front() {
            for v in 0..self.n {
                if !visited.contains(&v) && residual[u][v] > 0 {
                    visited.insert(v);
                    queue.push_back(v);
                }
            }
        }
        
        visited
    }
    
    fn validate_partition(&self, set1: &[String], set2: &[String]) -> bool {
        if set1.is_empty() || set2.is_empty() {
            return false;
        }
        
        let set1_indices: HashSet<usize> = set1.iter()
            .filter_map(|act| self.activity_to_idx.get(act))
            .cloned()
            .collect();
        
        let set2_indices: HashSet<usize> = set2.iter()
            .filter_map(|act| self.activity_to_idx.get(act))
            .cloned()
            .collect();
        
        // Check constraint: Every node in set1 should be able to reach every node in set2
        for &s1_idx in &set1_indices {
            let reachable_from_s1 = self.get_reachable_nodes(s1_idx);
            if !set2_indices.iter().all(|&idx| reachable_from_s1.contains(&idx)) {
                return false;
            }
        }
        
        true
    }
    
    fn get_reachable_nodes(&self, start: usize) -> HashSet<usize> {
        let mut visited = HashSet::new();
        let mut stack = vec![start];
        
        while let Some(node) = stack.pop() {
            if visited.contains(&node) {
                continue;
            }
            visited.insert(node);
            
            for &neighbor in &self.graph[node] {
                if !visited.contains(&neighbor) {
                    stack.push(neighbor);
                }
            }
        }
        
        visited
    }
}

pub fn perform_cut(
    dfg: &HashMap<(String, String), usize>,
    all_activities: &HashSet<String>
) -> Option<CutResult> {
    // Validate inputs
    if all_activities.len() < 2 {
        info!("Error: Need at least 2 activities to partition");
        return None;
    }
    
    let partitioner = ActivityPartitioner::new(dfg, all_activities);
    
    match partitioner.solve() {
        Some(result) => {
            // Print results
            info!("=== OPTIMAL PARTITION FOUND ===");
            info!("Total cost of cutting: {}", result.total_cost);
            info!("Number of edges to cut: {}", result.edges_to_cut.len());
            
            info!("\nSet 1 (can reach Set 2): {:?}", result.set1);
            info!("Set 2 (cannot reach Set 1): {:?}", result.set2);
            
            info!("\nEdges to cut:");
            for ((start, end), cost) in &result.edges_to_cut {
                info!("  {} -> {} (cost: {})", start, end, cost);
            }
            
            Some(result)
        }
        None => {
            info!("No valid partition found that satisfies all constraints");
            None
        }
    }
}

// Enhanced version with better error handling and optimizations
pub fn best_possible_sequence_cut(
    dfg: &HashMap<(String, String), usize>,
    all_activities: &HashSet<String>
) -> Result<CutResult, String> {
    // Validate inputs
    if all_activities.len() < 2 {
        return Err("Need at least 2 activities to partition".to_string());
    }
    
    // Check if all edges reference valid activities
    for (start, end) in dfg.keys() {
        if !all_activities.contains(start) || !all_activities.contains(end) {
            return Err(format!("Edge references unknown activity: {} -> {}", start, end));
        }
    }
    
    let partitioner = ActivityPartitioner::new(dfg, all_activities);
    
    match partitioner.solve() {
        Some(result) => {
            // Print results
            info!("=== OPTIMAL PARTITION FOUND ===");
            info!("Total cost of cutting: {}", result.total_cost);
            info!("Number of edges to cut: {}", result.edges_to_cut.len());
            
            info!("\nSet 1 (can reach Set 2):");
            for activity in &result.set1 {
                info!("  - {}", activity);
            }
            
            info!("\nSet 2 (cannot reach Set 1):");
            for activity in &result.set2 {
                info!("  - {}", activity);
            }
            
            info!("\nEdges to cut:");
            for ((start, end), cost) in &result.edges_to_cut {
                info!("  {} -> {} (cost: {})", start, end, cost);
            }
            
            // Additional statistics
            let total_edges = dfg.len();
            let cut_percentage = if total_edges > 0 {
                (result.edges_to_cut.len() as f64 / total_edges as f64) * 100.0
            } else {
                0.0
            };
            
            info!("\n=== STATISTICS ===");
            info!("Total edges in graph: {}", total_edges);
            info!("Edges cut: {} ({:.1}%)", result.edges_to_cut.len(), cut_percentage);
            info!("Set 1 size: {}", result.set1.len());
            info!("Set 2 size: {}", result.set2.len());
            info!("Set 1 activities: {:?}", result.set1);
            info!("Set 2 activities: {:?}", result.set2);
            
            Ok(result)
        }
        None => {
            Err("No valid partition found that satisfies all constraints".to_string())
        }
    }
}

