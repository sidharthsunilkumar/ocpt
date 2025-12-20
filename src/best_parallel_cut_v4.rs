use std::collections::{HashMap, HashSet, VecDeque};

pub fn best_parallel_cut_v4(
    dfg: &HashMap<(String, String), usize>,
    all_activities: &HashSet<String>,
    cost_to_add_edges: &HashMap<(String, String), f64>,
    start_activities: &HashSet<String>, 
    end_activities: &HashSet<String>
) -> (usize, usize, Vec<(String, String, usize)>, HashSet<String>, HashSet<String>, HashMap<(String, String), usize>) {
    
    // 1. Validation: Impossible to split if we don't have at least 2 starts and 2 ends
    if start_activities.len() < 2 || end_activities.len() < 2 {
        // Return a default "no solution" state or handle error appropriately
        return (usize::MAX, 0, Vec::new(), HashSet::new(), HashSet::new(), dfg.clone());
    }

    let mut min_cost = usize::MAX;
    let mut best_no_of_added_edges = 0;
    let mut best_added_edges: Vec<(String, String, usize)> = Vec::new();
    let mut best_set1: HashSet<String> = HashSet::new();
    let mut best_set2: HashSet<String> = HashSet::new();
    let mut best_new_dfg: HashMap<(String, String), usize> = HashMap::new();
    
    // Create base missing_dfg
    let (base_missing_dfg, edge_to_missing_map) = create_missing_dfg(dfg, all_activities, cost_to_add_edges);

    // Convert sets to vecs for indexing
    let start_vec: Vec<String> = start_activities.iter().cloned().collect();
    let end_vec: Vec<String> = end_activities.iter().cloned().collect();

    // 2. Iterate through specific Start/End combinations
    // We need: Set1 having (s1, e1) and Set2 having (s2, e2)
    // Complexity: O(|Start|^2 * |End|^2 * MaxFlow). Since Start/End sets are usually small (1-5), this is fast.
    
    for i in 0..start_vec.len() {
        for j in (i + 1)..start_vec.len() {
            let s1 = &start_vec[i];
            let s2 = &start_vec[j];

            for k in 0..end_vec.len() {
                for l in (k + 1)..end_vec.len() {
                    let e1 = &end_vec[k];
                    let e2 = &end_vec[l];

                    // We must test two configurations because s1/s2 are distinct, but e1/e2 pairing matters.
                    // Config A: Set1 has (s1, e1), Set2 has (s2, e2)
                    // Config B: Set1 has (s1, e2), Set2 has (s2, e1)
                    
                    let configs = vec![
                        (s1, e1, s2, e2),
                        (s1, e2, s2, e1)
                    ];

                    for (source_start, source_end, sink_start, sink_end) in configs {
                        
                        // 3. Construct Graph with Virtual Nodes
                        // We clone the base graph and add infinite capacity edges
                        let mut current_missing_dfg = base_missing_dfg.clone();
                        let super_source = "__SUPER_SOURCE__".to_string();
                        let super_sink = "__SUPER_SINK__".to_string();
                        let infinity_cost = 999_999_999; // Sufficiently large number

                        // Force source_start and source_end to be on the Source side
                        current_missing_dfg.insert((super_source.clone(), source_start.clone()), infinity_cost);
                        current_missing_dfg.insert((super_source.clone(), source_end.clone()), infinity_cost);

                        // Force sink_start and sink_end to be on the Sink side
                        current_missing_dfg.insert((sink_start.clone(), super_sink.clone()), infinity_cost);
                        current_missing_dfg.insert((sink_end.clone(), super_sink.clone()), infinity_cost);
                        
                        // Note: We need to make sure the algorithm handles these keys correctly.
                        // Since max_flow_min_cut builds an undirected graph from these keys, 
                        // (SUPER -> node) is enough to create the bidirectional link with capacity.

                        // 4. Run Max Flow between Super Source and Super Sink
                        let all_activities_with_virtual: HashSet<String> = all_activities.iter().cloned()
                            .chain(std::iter::once(super_source.clone()))
                            .chain(std::iter::once(super_sink.clone()))
                            .collect();

                        let (max_flow_value, mut cut_set1, mut cut_set2, added_edges) = max_flow_min_cut(
                            &current_missing_dfg, 
                            &edge_to_missing_map, 
                            &all_activities_with_virtual, 
                            &super_source, 
                            &super_sink, 
                            cost_to_add_edges
                        );

                        // 5. Cleanup Results
                        // Remove the virtual nodes from the result sets
                        cut_set1.remove(&super_source);
                        cut_set2.remove(&super_sink);
                        
                        // The cost returned includes the internal graph edges, which is correct.
                        // It should NOT include the infinity edges because the cut shouldn't cross them 
                        // (if it crosses an infinity edge, it means a valid split was impossible).
                        if max_flow_value >= infinity_cost {
                            continue; // Impossible configuration
                        }

                        let cost = max_flow_value;
                        let no_of_added_edges = added_edges.len();

                        // 6. Update Best Solution
                        let current_size_diff = cut_set1.len().abs_diff(cut_set2.len());
                        let best_size_diff = best_set1.len().abs_diff(best_set2.len());

                        if cost < min_cost || (cost == min_cost && current_size_diff < best_size_diff) {
                            min_cost = cost;
                            best_no_of_added_edges = no_of_added_edges;
                            best_added_edges = added_edges;
                            best_set1 = cut_set1;
                            best_set2 = cut_set2;
                            
                            best_new_dfg = dfg.clone();
                            for (a, b, _cost) in &best_added_edges {
                                best_new_dfg.insert((a.clone(), b.clone()), 1);
                            }
                        }
                    }
                }
            }
        }
    }

    (min_cost, best_no_of_added_edges, best_added_edges, best_set1, best_set2, best_new_dfg)
}


fn create_missing_dfg(
    dfg: &HashMap<(String, String), usize>,
    all_activities: &HashSet<String>,
    cost_to_add_edges: &HashMap<(String, String), f64>
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
                        let edge_ba_cost = cost_to_add_edges.get(&(b.clone(), a.clone())).copied().unwrap_or(1.0);
                        missing_edges.push((b.clone(), a.clone()));
                        edge_ba_cost as usize
                    },  // (b,a) is missing
                    (false, true) => {
                        let edge_ab_cost = cost_to_add_edges.get(&(a.clone(), b.clone())).copied().unwrap_or(1.0);
                        missing_edges.push((a.clone(), b.clone()));
                        edge_ab_cost as usize
                    },  // (a,b) is missing
                    (false, false) => {
                        missing_edges.push((a.clone(), b.clone()));
                        missing_edges.push((b.clone(), a.clone()));
                        let edge_ba_cost = cost_to_add_edges.get(&(b.clone(), a.clone())).copied().unwrap_or(1.0);
                        let edge_ab_cost = cost_to_add_edges.get(&(a.clone(), b.clone())).copied().unwrap_or(1.0);
                        let sum_ab_ba = edge_ab_cost + edge_ba_cost;
                        sum_ab_ba as usize
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
    cost_to_add_edges: &HashMap<(String, String), f64>
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
                        let cost_to_add_edge = cost_to_add_edges.get(&(from.clone(), to.clone())).copied().unwrap_or(999999.0);
                        added_edges.push((from.clone(), to.clone(), cost_to_add_edge as usize));
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