use std::collections::{HashMap, HashSet, VecDeque};

/// Represents the final result of the partitioning.
#[derive(Debug)]
pub struct PartitionResult {
    pub min_cost: usize,
    pub num_added_edges: usize,
    pub added_edges: Vec<(String, String)>,
    pub set1: HashSet<String>,
    pub set2: HashSet<String>,
    pub new_dfg: HashMap<(String, String), usize>,
}

/// Finds an augmenting path in the residual graph using Breadth-First Search (BFS).
/// This is a helper function for the Edmonds-Karp algorithm.
///
/// # Arguments
/// * `residual_graph` - The current residual capacities between nodes.
/// * `source` - The starting node index.
/// * `sink` - The ending node index.
/// * `parent` - A mutable vector to store the path.
///
/// # Returns
/// `true` if a path is found, `false` otherwise.
fn bfs(
    residual_graph: &Vec<Vec<usize>>,
    source: usize,
    sink: usize,
    parent: &mut Vec<isize>,
) -> bool {
    let num_nodes = residual_graph.len();
    let mut visited = vec![false; num_nodes];
    let mut queue = VecDeque::new();

    queue.push_back(source);
    visited[source] = true;
    parent[source] = -1;

    while let Some(u) = queue.pop_front() {
        for v in 0..num_nodes {
            if !visited[v] && residual_graph[u][v] > 0 {
                queue.push_back(v);
                visited[v] = true;
                parent[v] = u as isize;
                if v == sink {
                    return true;
                }
            }
        }
    }
    false
}

/// Finds the minimum s-t cut in a graph using the Edmonds-Karp max-flow algorithm.
/// The capacity of the min-cut is equal to the value of the max-flow.
///
/// # Arguments
/// * `graph` - An adjacency matrix representing the weighted undirected graph.
/// * `source` - The source node index.
/// * `sink` - The sink node index.
///
/// # Returns
/// A tuple containing:
///   - The min-cut cost (which is the max-flow value).
///   - A HashSet of node indices belonging to the source side of the cut.
fn find_s_t_min_cut(
    graph: &Vec<Vec<usize>>,
    source: usize,
    sink: usize,
) -> (usize, HashSet<usize>) {
    let num_nodes = graph.len();
    let mut residual_graph = graph.clone();
    let mut parent = vec![-1; num_nodes];
    let mut max_flow = 0;

    // Augment the flow while there is a path from source to sink
    while bfs(&residual_graph, source, sink, &mut parent) {
        let mut path_flow = usize::MAX;
        let mut s = sink;
        while s != source {
            let p = parent[s] as usize;
            path_flow = path_flow.min(residual_graph[p][s]);
            s = p;
        }

        let mut v = sink;
        while v != source {
            let u = parent[v] as usize;
            residual_graph[u][v] -= path_flow;
            residual_graph[v][u] += path_flow;
            v = u;
        }

        max_flow += path_flow;
    }

    // Find the set of nodes reachable from the source in the final residual graph.
    // This forms one side of the min-cut partition.
    let mut reachable_nodes = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(source);
    reachable_nodes.insert(source);

    while let Some(u) = queue.pop_front() {
        for v in 0..num_nodes {
            if residual_graph[u][v] > 0 && !reachable_nodes.contains(&v) {
                reachable_nodes.insert(v);
                queue.push_back(v);
            }
        }
    }

    (max_flow, reachable_nodes)
}


/// Partitions a set of activities to minimize the cost of adding edges to form a complete bipartite graph.
///
/// # Arguments
/// * `dfg` - A HashMap representing the directed graph of activities. The key `(a, b)` with value `n`
///           indicates an edge from activity `a` to `b` exists (n is ignored).
/// * `all_activities` - A HashSet containing all unique activity names.
///
/// # Returns
/// A `PartitionResult` struct containing the optimal solution.
pub fn best_parallel_cut_v2(
    dfg: &HashMap<(String, String), usize>,
    all_activities: &HashSet<String>,
) -> PartitionResult {
    let activities: Vec<String> = all_activities.iter().cloned().collect();
    let num_activities = activities.len();

    if num_activities < 2 {
        return PartitionResult {
            min_cost: 0,
            num_added_edges: 0,
            added_edges: vec![],
            set1: all_activities.clone(),
            set2: HashSet::new(),
            new_dfg: dfg.clone(),
        };
    }

    // 1. Construct the weighted undirected graph `G'`.
    // The weight w(u, v) is the number of edges that must be added if u and v are separated.
    // This is the "cost" of the cut.
    let mut capacity_matrix = vec![vec![0; num_activities]; num_activities];
    for i in 0..num_activities {
        for j in (i + 1)..num_activities {
            let act_i = &activities[i];
            let act_j = &activities[j];

            // --- BUG FIX STARTS HERE ---
            // The cost is 1 for each missing edge between the pair of activities.
            // We check for the existence of the key, ignoring the associated value `n`.
            let edge_ij_exists = dfg.contains_key(&(act_i.clone(), act_j.clone()));
            let edge_ji_exists = dfg.contains_key(&(act_j.clone(), act_i.clone()));

            // cost = (1 if i->j is missing) + (1 if j->i is missing)
            let cost = (!edge_ij_exists as usize) + (!edge_ji_exists as usize);
            // --- BUG FIX ENDS HERE ---

            capacity_matrix[i][j] = cost;
            capacity_matrix[j][i] = cost;
        }
    }

    // 2. Find the Global Minimum Cut.
    // We do this by fixing a source `s` and finding the min `s-t` cut for all other `t`.
    let mut min_cost = usize::MAX;
    let mut best_partition_indices = HashSet::new();

    let source_idx = 0; // Fix the source to the first activity.
    for sink_idx in 1..num_activities {
        let (cost, partition_indices) = find_s_t_min_cut(&capacity_matrix, source_idx, sink_idx);

        if cost < min_cost {
            min_cost = cost;
            best_partition_indices = partition_indices;
        }
    }

    // 3. Interpret the results from the best cut found.
    let mut set1 = HashSet::new();
    let mut set2 = HashSet::new();
    for (i, activity) in activities.iter().enumerate() {
        if best_partition_indices.contains(&i) {
            set1.insert(activity.clone());
        } else {
            set2.insert(activity.clone());
        }
    }

    // 4. Determine which edges need to be added.
    let mut added_edges = Vec::new();
    let mut new_dfg = dfg.clone();

    // The problem requires edges between set2 and set1 (and vice-versa).
    // Let's iterate through all pairs between the two sets.
    for act1 in &set1 {
        for act2 in &set2 {
            // Check edge from set2 to set1
            if !dfg.contains_key(&(act2.clone(), act1.clone())) {
                added_edges.push((act2.clone(), act1.clone()));
                // The cost of an added edge is 1.
                new_dfg.insert((act2.clone(), act1.clone()), 1);
            }
            // Check edge from set1 to set2
            if !dfg.contains_key(&(act1.clone(), act2.clone())) {
                added_edges.push((act1.clone(), act2.clone()));
                // The cost of an added edge is 1.
                new_dfg.insert((act1.clone(), act2.clone()), 1);
            }
        }
    }

    PartitionResult {
        min_cost,
        num_added_edges: added_edges.len(),
        added_edges,
        set1,
        set2,
        new_dfg,
    }
}