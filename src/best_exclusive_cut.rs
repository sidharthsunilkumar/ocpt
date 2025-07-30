use std::collections::{HashMap, HashSet, VecDeque};
use log::info;

#[derive(Debug, Clone)]
struct Graph {
    capacity: HashMap<(String, String), usize>,
    flow: HashMap<(String, String), usize>,
    nodes: HashSet<String>,
}

impl Graph {
    fn new() -> Self {
        Graph {
            capacity: HashMap::new(),
            flow: HashMap::new(),
            nodes: HashSet::new(),
        }
    }

    fn add_edge(&mut self, from: String, to: String, cap: usize) {
        self.capacity.insert((from.clone(), to.clone()), cap);
        self.flow.insert((from.clone(), to.clone()), 0);
        
        // Initialize reverse flow if not exists
        if !self.flow.contains_key(&(to.clone(), from.clone())) {
            self.flow.insert((to.clone(), from.clone()), 0);
        }
        
        self.nodes.insert(from);
        self.nodes.insert(to);
    }

    fn get_capacity(&self, from: &str, to: &str) -> usize {
        self.capacity.get(&(from.to_string(), to.to_string())).copied().unwrap_or(0)
    }

    fn get_flow(&self, from: &str, to: &str) -> usize {
        self.flow.get(&(from.to_string(), to.to_string())).copied().unwrap_or(0)
    }

    fn get_residual_capacity(&self, from: &str, to: &str) -> usize {
        self.get_capacity(from, to) - self.get_flow(from, to)
    }

    fn push_flow(&mut self, from: &str, to: &str, amount: usize) {
        // Update forward flow
        let current_flow = self.get_flow(from, to);
        self.flow.insert((from.to_string(), to.to_string()), current_flow + amount);
        
        // Update backward flow (residual edge)
        let reverse_flow = self.get_flow(to, from);
        self.flow.insert((to.to_string(), from.to_string()), reverse_flow + amount);
    }

    fn bfs_find_path(&self, source: &str, sink: &str) -> Option<Vec<String>> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut parent: HashMap<String, String> = HashMap::new();

        queue.push_back(source.to_string());
        visited.insert(source.to_string());

        while let Some(current) = queue.pop_front() {
            if current == sink {
                // Reconstruct path
                let mut path = Vec::new();
                let mut node = sink.to_string();
                path.push(node.clone());

                while let Some(p) = parent.get(&node) {
                    path.push(p.clone());
                    node = p.clone();
                }
                
                path.reverse();
                return Some(path);
            }

            // Check all possible neighbors
            for next in &self.nodes {
                if !visited.contains(next) && self.get_residual_capacity(&current, next) > 0 {
                    visited.insert(next.clone());
                    parent.insert(next.clone(), current.clone());
                    queue.push_back(next.clone());
                }
            }
        }

        None
    }

    fn max_flow(&mut self, source: &str, sink: &str) -> usize {
        let mut total_flow = 0;

        while let Some(path) = self.bfs_find_path(source, sink) {
            // Find minimum residual capacity along the path
            let mut min_capacity = usize::MAX;
            for i in 0..path.len() - 1 {
                let cap = self.get_residual_capacity(&path[i], &path[i + 1]);
                if cap < min_capacity {
                    min_capacity = cap;
                }
            }

            // Push flow along the path
            for i in 0..path.len() - 1 {
                self.push_flow(&path[i], &path[i + 1], min_capacity);
            }

            total_flow += min_capacity;
        }

        total_flow
    }

    fn find_reachable_from_source(&self, source: &str) -> HashSet<String> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        queue.push_back(source.to_string());
        visited.insert(source.to_string());

        while let Some(current) = queue.pop_front() {
            for next in &self.nodes {
                if !visited.contains(next) && self.get_residual_capacity(&current, next) > 0 {
                    visited.insert(next.clone());
                    queue.push_back(next.clone());
                }
            }
        }

        visited
    }
}

pub fn best_exclusive_cut(
    dfg: &HashMap<(String, String), usize>,
    all_activities: &HashSet<String>,
) -> (usize, Vec<(String, String)>, HashSet<String>, HashSet<String>, HashMap<(String, String), usize>) {

    //info!("Starting best_exclusive_cut...");
    if all_activities.len() < 2 {
        return (0, Vec::new(), all_activities.clone(), HashSet::new(), dfg.clone());
    }
    
    let activities: Vec<String> = all_activities.iter().cloned().collect();
    let mut min_cost = usize::MAX;
    let mut best_cut_edges = Vec::new();
    let mut best_set1 = HashSet::new();
    let mut best_set2 = HashSet::new();
    
    // Calculate a large value for infinite capacity
    let inf_capacity = dfg.values().sum::<usize>() * 2 + 1000;
    
    // Try different ways to partition by fixing different activities in different sets
    for i in 0..activities.len() {
        for j in (i+1)..activities.len() {
            let activity_s = &activities[i];
            let activity_t = &activities[j];
            
            //info!("Trying partition with {} in source set, {} in sink set", activity_s, activity_t);

            let mut graph = Graph::new();
            
            let source = "SOURCE".to_string();
            let sink = "SINK".to_string();
            
            // Add all nodes
            graph.nodes.insert(source.clone());
            graph.nodes.insert(sink.clone());
            for activity in all_activities {
                graph.nodes.insert(activity.clone());
            }
            
            // Connect fixed activities to source and sink
            graph.add_edge(source.clone(), activity_s.clone(), inf_capacity);
            graph.add_edge(activity_t.clone(), sink.clone(), inf_capacity);

            
            // Add all DFG edges as undirected edges in the graph
            for ((from, to), weight) in dfg {
                // Only add if both nodes are in our activity set
                if all_activities.contains(from) && all_activities.contains(to) {
                    graph.add_edge(from.clone(), to.clone(), *weight);
                    graph.add_edge(to.clone(), from.clone(), *weight);
                }
            }
            
            // Find maximum flow = minimum cut
            let max_flow_value = graph.max_flow(&source, &sink);
            //info!("Max flow value: {}", max_flow_value);
            
            // Find the cut by determining reachable nodes from source
            let reachable_from_source = graph.find_reachable_from_source(&source);
            //info!("Reachable from source: {:?}", reachable_from_source);
            
            // Partition activities
            let mut set1 = HashSet::new();
            let mut set2 = HashSet::new();
            
            for activity in all_activities {
                if reachable_from_source.contains(activity) {
                    set1.insert(activity.clone());
                } else {
                    set2.insert(activity.clone());
                }
            }
            
            // Skip if either set is empty
            if set1.is_empty() || set2.is_empty() {
                //info!("Skipping: empty partition");
                continue;
            }
            
            // Calculate actual cut cost from original DFG
            let mut cut_edges = Vec::new();
            let mut total_cut_cost = 0;
            
            for ((from, to), cost) in dfg {
                let from_in_set1 = set1.contains(from);
                let to_in_set1 = set1.contains(to);
                
                if from_in_set1 != to_in_set1 {
                    cut_edges.push((from.clone(), to.clone()));
                    total_cut_cost += cost;
                }
            }
            
            //info!("Cut cost: {}, Cut edges: {:?}", total_cut_cost, cut_edges);
            //info!("Set1: {:?}, Set2: {:?}", set1, set2);
            
            // Update best solution
            if total_cut_cost < min_cost {
                min_cost = total_cut_cost;
                best_cut_edges = cut_edges;
                best_set1 = set1;
                best_set2 = set2;
            }
        }
    }
    
    // Create new DFG with cut edges removed
    let mut new_dfg = dfg.clone();
    for (from, to) in &best_cut_edges {
        new_dfg.remove(&(from.clone(), to.clone()));
    }
    
    //info!("Final result - Min cost: {}, Cut edges: {:?}", min_cost, best_cut_edges);
    //info!("Set1: {:?}, Set2: {:?}", best_set1, best_set2);
    
    (min_cost, best_cut_edges, best_set1, best_set2, new_dfg)
}
