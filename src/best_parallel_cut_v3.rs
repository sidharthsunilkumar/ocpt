use std::collections::{HashMap, HashSet, VecDeque};

pub fn best_parallel_cut_v3(
    dfg: &HashMap<(String, String), usize>,
    all_activities: &HashSet<String>,
    cost_to_add_edge: &usize
) -> (usize, usize, Vec<(String, String, usize)>, HashSet<String>, HashSet<String>, HashMap<(String, String), usize>) {
    let mut min_cost = usize::MAX;
    let mut best_no_of_added_edges = 0;
    let mut best_added_edges: Vec<(String, String, usize)> = Vec::new();
    let mut best_set1: HashSet<String> = HashSet::new();
    let mut best_set2: HashSet<String> = HashSet::new();
    let mut best_new_dfg: HashMap<(String, String), usize> = HashMap::new();
    
    // Create missing_dfg - undirected graph with costs for missing edges
    let (missing_dfg, edge_to_missing_map) = create_missing_dfg(dfg, all_activities, cost_to_add_edge);
    
    // Try each activity as a potential source for min-cut
    let activities: Vec<String> = all_activities.iter().cloned().collect();
    
    for i in 0..activities.len() {
        for j in (i + 1)..activities.len() {
            let source = &activities[i];
            let sink = &activities[j];
            
            // Run max flow / min cut algorithm: uses Ford-Fulkerson with BFS (Edmonds-Karp) to find the minimum cut
            let (max_flow_value, cut_set1, cut_set2, added_edges) = 
                max_flow_min_cut(&missing_dfg, &edge_to_missing_map, all_activities, source, sink, cost_to_add_edge);
            
            let cost = max_flow_value;
            let no_of_added_edges = added_edges.len();
            
            // Update best solution if this is better
            if cost < min_cost {
                min_cost = cost;
                best_no_of_added_edges = no_of_added_edges;
                best_added_edges = added_edges.clone();
                best_set1 = cut_set1;
                best_set2 = cut_set2;
                
                // Create new DFG with added edges
                best_new_dfg = dfg.clone();
                for (a, b, _cost) in &added_edges {
                    best_new_dfg.insert((a.clone(), b.clone()), 1);
                }
            }
        }
    }
    
    (min_cost, best_no_of_added_edges, best_added_edges, best_set1, best_set2, best_new_dfg)
}

fn create_missing_dfg(
    dfg: &HashMap<(String, String), usize>,
    all_activities: &HashSet<String>,
    cost_to_add_edge: &usize
) -> (HashMap<(String, String), usize>, HashMap<(String, String), Vec<(String, String)>>) {
    let mut missing_dfg = HashMap::new();
    let mut edge_to_missing_map = HashMap::new(); // Maps undirected edge to missing directed edges
    
    // For every pair of activities (a,b), check if edges exist
    for a in all_activities {
        for b in all_activities {
            if a < b {  // Only process each pair once for undirected graph
                let edge_ab = dfg.contains_key(&(a.clone(), b.clone()));
                let edge_ba = dfg.contains_key(&(b.clone(), a.clone()));
                
                let mut missing_edges = Vec::new();
                
                let cost = match (edge_ab, edge_ba) {
                    (true, true) => 0,  // Both edges exist
                    (true, false) => {
                        missing_edges.push((b.clone(), a.clone()));
                        *cost_to_add_edge
                    },  // (b,a) is missing
                    (false, true) => {
                        missing_edges.push((a.clone(), b.clone()));
                        *cost_to_add_edge
                    },  // (a,b) is missing
                    (false, false) => {
                        missing_edges.push((a.clone(), b.clone()));
                        missing_edges.push((b.clone(), a.clone()));
                        2 * cost_to_add_edge
                    },  // Both edges missing
                };
                
                // Only add to missing_dfg if cost > 0 (there are missing edges)
                if cost > 0 {
                    missing_dfg.insert((a.clone(), b.clone()), cost);
                    edge_to_missing_map.insert((a.clone(), b.clone()), missing_edges);
                }
            }
        }
    }
    
    (missing_dfg, edge_to_missing_map)
}

fn max_flow_min_cut(
    missing_dfg: &HashMap<(String, String), usize>,
    edge_to_missing_map: &HashMap<(String, String), Vec<(String, String)>>,
    all_activities: &HashSet<String>,
    source: &String,
    sink: &String,
    cost_to_add_edge: &usize
) -> (usize, HashSet<String>, HashSet<String>, Vec<(String, String, usize)>) {
    
    // Create adjacency list representation
    let mut graph = HashMap::new();
    let mut capacity = HashMap::new();
    
    // Initialize graph
    for activity in all_activities {
        graph.insert(activity.clone(), Vec::new());
    }
    
    // Build undirected graph from missing_dfg
    for ((a, b), cost) in missing_dfg {
        // Add edge in both directions for undirected graph
        graph.get_mut(a).unwrap().push(b.clone());
        graph.get_mut(b).unwrap().push(a.clone());
        
        capacity.insert((a.clone(), b.clone()), *cost);
        capacity.insert((b.clone(), a.clone()), *cost);
    }
    
    // Ford-Fulkerson with BFS (Edmonds-Karp)
    let mut residual_capacity = capacity.clone();
    let mut max_flow = 0;
    
    loop {
        // BFS to find augmenting path
        let path = bfs_find_path(&graph, &residual_capacity, source, sink);
        
        if path.is_empty() {
            break;
        }
        
        // Find bottleneck capacity along the path
        let mut path_flow = usize::MAX;
        for i in 0..(path.len() - 1) {
            let u = &path[i];
            let v = &path[i + 1];
            let cap = residual_capacity.get(&(u.clone(), v.clone())).unwrap_or(&0);
            path_flow = path_flow.min(*cap);
        }
        
        // Update residual capacities
        for i in 0..(path.len() - 1) {
            let u = &path[i];
            let v = &path[i + 1];
            
            // Reduce forward edge
            let forward_key = (u.clone(), v.clone());
            let current_forward = residual_capacity.get(&forward_key).unwrap_or(&0);
            residual_capacity.insert(forward_key, current_forward - path_flow);
            
            // Increase backward edge
            let backward_key = (v.clone(), u.clone());
            let current_backward = residual_capacity.get(&backward_key).unwrap_or(&0);
            residual_capacity.insert(backward_key, current_backward + path_flow);
        }
        
        max_flow += path_flow;
    }
    
    // Find min cut by doing BFS from source in residual graph
    let reachable = bfs_reachable(&graph, &residual_capacity, source);
    
    let mut set1 = HashSet::new();
    let mut set2 = HashSet::new();
    
    for activity in all_activities {
        if reachable.contains(activity) {
            set1.insert(activity.clone());
        } else {
            set2.insert(activity.clone());
        }
    }
    
    // Find edges that need to be added (cut edges)
    let mut added_edges: Vec<(String, String, usize)> = Vec::new();
    for activity1 in &set1 {
        for activity2 in &set2 {
            // Check both possible orderings since we stored undirected edges as (min, max)
            let key1 = if activity1 < activity2 { 
                (activity1.clone(), activity2.clone()) 
            } else { 
                (activity2.clone(), activity1.clone()) 
            };
            
            if let Some(missing_edges) = edge_to_missing_map.get(&key1) {
                // Add only the specific missing directed edges that cross the cut
                for missing_edge in missing_edges {
                    let (from, to) = missing_edge;
                    // Check if this edge crosses the cut
                    if (set1.contains(from) && set2.contains(to)) || 
                       (set2.contains(from) && set1.contains(to)) {
                        added_edges.push((from.clone(), to.clone(), *cost_to_add_edge));
                    }
                }
            }
        }
    }
    
    (max_flow, set1, set2, added_edges)
}

fn bfs_find_path(
    graph: &HashMap<String, Vec<String>>,
    residual_capacity: &HashMap<(String, String), usize>,
    source: &String,
    sink: &String
) -> Vec<String> {
    let mut queue = VecDeque::new();
    let mut visited = HashSet::new();
    let mut parent: HashMap<String, String> = HashMap::new();
    
    queue.push_back(source.clone());
    visited.insert(source.clone());
    
    while let Some(current) = queue.pop_front() {
        if current == *sink {
            // Reconstruct path
            let mut path = Vec::new();
            let mut node = sink.clone();
            
            while node != *source {
                path.push(node.clone());
                node = parent[&node].clone();
            }
            path.push(source.clone());
            path.reverse();
            
            return path;
        }
        
        if let Some(neighbors) = graph.get(&current) {
            for neighbor in neighbors {
                let capacity_key = (current.clone(), neighbor.clone());
                let capacity = residual_capacity.get(&capacity_key).unwrap_or(&0);
                
                if !visited.contains(neighbor) && *capacity > 0 {
                    visited.insert(neighbor.clone());
                    parent.insert(neighbor.clone(), current.clone());
                    queue.push_back(neighbor.clone());
                }
            }
        }
    }
    
    Vec::new() // No path found
}

fn bfs_reachable(
    graph: &HashMap<String, Vec<String>>,
    residual_capacity: &HashMap<(String, String), usize>,
    source: &String
) -> HashSet<String> {
    let mut queue = VecDeque::new();
    let mut visited = HashSet::new();
    
    queue.push_back(source.clone());
    visited.insert(source.clone());
    
    while let Some(current) = queue.pop_front() {
        if let Some(neighbors) = graph.get(&current) {
            for neighbor in neighbors {
                let capacity_key = (current.clone(), neighbor.clone());
                let capacity = residual_capacity.get(&capacity_key).unwrap_or(&0);
                
                if !visited.contains(neighbor) && *capacity > 0 {
                    visited.insert(neighbor.clone());
                    queue.push_back(neighbor.clone());
                }
            }
        }
    }
    
    visited
}