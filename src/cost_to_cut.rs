use std::collections::{HashMap, HashSet, VecDeque};
use std::cmp::min;

pub fn to_be_non_reachable(
    dfg: &HashMap<(String, String), usize>,
    activity1: &str,
    activity2: &str,
) -> (usize, usize, Vec<(String, String)>) {
    // First check if activity2 is reachable from activity1
    if !is_reachable(dfg, activity1, activity2) {
        return (0, 0, Vec::new()); // Already non-reachable
    }
    
    // Find all simple paths from activity1 to activity2
    let all_paths = find_all_paths(dfg, activity1, activity2);
    
    if all_paths.is_empty() {
        return (0, 0, Vec::new());
    }
    
    // Find minimum edge cut using a greedy approach
    // We'll use a more sophisticated algorithm: find minimum vertex cut
    find_min_edge_cut(dfg, &all_paths)
}

fn find_all_paths(
    dfg: &HashMap<(String, String), usize>,
    start: &str,
    end: &str,
) -> Vec<Vec<String>> {
    let mut paths = Vec::new();
    let mut current_path = vec![start.to_string()];
    let mut visited = HashSet::new();
    
    find_paths_dfs(dfg, start, end, &mut current_path, &mut visited, &mut paths);
    paths
}

fn find_paths_dfs(
    dfg: &HashMap<(String, String), usize>,
    current: &str,
    target: &str,
    current_path: &mut Vec<String>,
    visited: &mut HashSet<String>,
    all_paths: &mut Vec<Vec<String>>,
) {
    if current == target {
        all_paths.push(current_path.clone());
        return;
    }
    
    if all_paths.len() > 1000 { // Prevent infinite paths in complex graphs
        return;
    }
    
    visited.insert(current.to_string());
    
    for ((from, to), _) in dfg {
        if from == current && !visited.contains(to) {
            current_path.push(to.clone());
            find_paths_dfs(dfg, to, target, current_path, visited, all_paths);
            current_path.pop();
        }
    }
    
    visited.remove(current);
}

fn find_min_edge_cut(
    dfg: &HashMap<(String, String), usize>,
    paths: &[Vec<String>], // Not used in Edmonds-Karp, but kept for signature compatibility
) -> (usize, usize, Vec<(String, String)>) {
    use std::collections::VecDeque;

    // Build adjacency list and capacity map
    let mut capacity = HashMap::new();
    let mut adj = HashMap::<String, Vec<String>>::new();
    for ((from, to), &cap) in dfg.iter() {
        capacity.insert((from.clone(), to.clone()), cap);
        adj.entry(from.clone()).or_default().push(to.clone());
        adj.entry(to.clone()).or_default(); // Ensure all nodes are present
    }

    let source = if let Some(path) = paths.first() {
        path.first().cloned().unwrap_or_default()
    } else {
        return (0, 0, Vec::new());
    };
    let sink = if let Some(path) = paths.first() {
        path.last().cloned().unwrap_or_default()
    } else {
        return (0, 0, Vec::new());
    };

    let mut flow = 0;
    let mut residual = capacity.clone();

    // Edmonds-Karp BFS to find augmenting paths
    loop {
        let mut parent = HashMap::<String, String>::new();
        let mut q = VecDeque::new();
        q.push_back(source.clone());

        while let Some(u) = q.pop_front() {
            for v in adj.get(&u).unwrap() {
                if !parent.contains_key(v) && *residual.get(&(u.clone(), v.clone())).unwrap_or(&0) > 0 && v != &source {
                    parent.insert(v.clone(), u.clone());
                    q.push_back(v.clone());
                }
            }
        }

        if !parent.contains_key(&sink) {
            break;
        }

        // Find minimum residual capacity along the path
        let mut v = sink.clone();
        let mut path_flow = usize::MAX;
        while let Some(u) = parent.get(&v) {
            let cap = *residual.get(&(u.clone(), v.clone())).unwrap_or(&0);
            path_flow = path_flow.min(cap);
            v = u.clone();
        }

        // Update residual capacities
        let mut v = sink.clone();
        while let Some(u) = parent.get(&v) {
            *residual.get_mut(&(u.clone(), v.clone())).unwrap() -= path_flow;
            *residual.entry((v.clone(), u.clone())).or_insert(0) += path_flow;
            v = u.clone();
        }

        flow += path_flow;
    }

    // After max-flow, find reachable vertices from source in residual graph
    let mut visited = HashSet::new();
    let mut q = VecDeque::new();
    q.push_back(source.clone());
    while let Some(u) = q.pop_front() {
        if visited.insert(u.clone()) {
            for v in adj.get(&u).unwrap() {
                if *residual.get(&(u.clone(), v.clone())).unwrap_or(&0) > 0 && !visited.contains(v) {
                    q.push_back(v.clone());
                }
            }
        }
    }

    // Edges from visited to unvisited are the min-cut
    let mut cut_edges = Vec::new();
    let mut cut_weight = 0;
    for ((u, v), &cap) in capacity.iter() {
        if visited.contains(u) && !visited.contains(v) {
            cut_edges.push((u.clone(), v.clone()));
            cut_weight += cap;
        }
    }

    (cut_edges.len(), cut_weight, cut_edges)
}

fn find_min_edge_cut_old(
    dfg: &HashMap<(String, String), usize>,
    paths: &[Vec<String>],
) -> (usize, usize, Vec<(String, String)>) {
    // Extract all edges from the original graph
    let mut edges: Vec<((String, String), usize)> = dfg.iter()
        .map(|((from, to), weight)| ((from.clone(), to.clone()), *weight))
        .collect();
    
    // Sort edges by weight (greedy approach - try cheapest first)
    edges.sort_by_key(|(_, weight)| *weight);
    
    let mut min_edges = usize::MAX;
    let mut min_weight = usize::MAX;
    let mut best_edges = Vec::new();
    
    // Try all possible combinations of edges to remove
    let n = edges.len();
    
    // Use bit manipulation to try all subsets
    for mask in 1..(1 << n) {
        let mut removed_edges = HashMap::new();
        let mut edge_list = Vec::new();
        let mut edge_count = 0;
        let mut total_weight = 0;
        
        for i in 0..n {
            if (mask >> i) & 1 == 1 {
                let ((from, to), weight) = &edges[i];
                removed_edges.insert((from.clone(), to.clone()), *weight);
                edge_list.push((from.clone(), to.clone()));
                edge_count += 1;
                total_weight += weight;
            }
        }
        
        // Check if removing these edges disconnects all paths
        if is_disconnected(dfg, &removed_edges, paths) {
            if edge_count < min_edges || (edge_count == min_edges && total_weight < min_weight) {
                min_edges = edge_count;
                min_weight = total_weight;
                best_edges = edge_list;
            }
        }
    }
    
    if min_edges == usize::MAX {
        (0, 0, Vec::new()) // Shouldn't happen if reachable
    } else {
        (min_edges, min_weight, best_edges)
    }
}

fn is_disconnected(
    dfg: &HashMap<(String, String), usize>,
    removed_edges: &HashMap<(String, String), usize>,
    paths: &[Vec<String>],
) -> bool {
    // Check if any path is still valid after removing edges
    for path in paths {
        let mut path_blocked = false;
        
        for i in 0..path.len() - 1 {
            let edge = (path[i].clone(), path[i + 1].clone());
            if removed_edges.contains_key(&edge) {
                path_blocked = true;
                break;
            }
        }
        
        if !path_blocked {
            return false; // At least one path is still open
        }
    }
    
    true // All paths are blocked
}

// Helper function to check reachability (same as your original)
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
