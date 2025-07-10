use std::collections::{HashMap, HashSet, VecDeque};

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
        self.flow.insert((to.clone(), from.clone()), 0);
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
        let current_flow = self.get_flow(from, to);
        self.flow.insert((from.to_string(), to.to_string()), current_flow + amount);
        
        let reverse_flow = self.get_flow(to, from);
        // Handle the case where reverse_flow might be less than amount
        if reverse_flow >= amount {
            self.flow.insert((to.to_string(), from.to_string()), reverse_flow - amount);
        } else {
            self.flow.insert((to.to_string(), from.to_string()), 0);
        }
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
) -> (usize, Vec<(String, String)>, HashSet<String>, HashSet<String>) {
    // Simple approach: try all possible 2^n partitions and find minimum cost
    // This is exponential but works correctly for small inputs
    
    let activities: Vec<String> = all_activities.iter().cloned().collect();
    let n = activities.len();
    
    if n == 0 {
        return (0, Vec::new(), HashSet::new(), HashSet::new());
    }
    
    let mut min_cost = usize::MAX;
    let mut best_cut_edges = Vec::new();
    let mut best_set1 = HashSet::new();
    let mut best_set2 = HashSet::new();
    
    // Try all possible partitions (2^n possibilities)
    for mask in 1..(1 << n) - 1 { // Exclude empty sets
        let mut set1 = HashSet::new();
        let mut set2 = HashSet::new();
        
        // Partition activities based on the bitmask
        for i in 0..n {
            if (mask >> i) & 1 == 1 {
                set1.insert(activities[i].clone());
            } else {
                set2.insert(activities[i].clone());
            }
        }
        
        // Calculate cost of this partition
        let mut cut_edges = Vec::new();
        let mut total_cost = 0;
        
        for ((from, to), cost) in dfg {
            let from_in_set1 = set1.contains(from);
            let to_in_set1 = set1.contains(to);
            
            // If edge crosses between sets, it needs to be cut
            if from_in_set1 != to_in_set1 {
                cut_edges.push((from.clone(), to.clone()));
                total_cost += cost;
            }
        }
        
        // Update best solution if this is better
        if total_cost < min_cost {
            min_cost = total_cost;
            best_cut_edges = cut_edges;
            best_set1 = set1;
            best_set2 = set2;
        }
    }
    
    (min_cost, best_cut_edges, best_set1, best_set2)
}
