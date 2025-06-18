use std::collections::{HashMap, HashSet, VecDeque};
use crate::types::{TreeNode, ProcessForest};
use itertools::Itertools;
use log::info;
use crate::cost_to_cut::to_be_non_reachable;

/// Step 1: Tarjan's Algorithm to find SCCs
pub fn strongly_connected_components(
    dfg: &HashMap<(String, String), usize>,
    all_activities: &HashSet<String>,
) -> Vec<Vec<String>> {
    // Step 1: Build adjacency list
    let mut graph: HashMap<String, Vec<String>> = HashMap::new();
    for ((from, to), _) in dfg.iter() {
        graph.entry(from.clone()).or_default().push(to.clone());
    }
    for activity in all_activities {
        graph.entry(activity.clone()).or_default();
    }

    // Tarjan’s setup
    let mut index = 0;
    let mut indices = HashMap::new();
    let mut lowlink = HashMap::new();
    let mut stack = Vec::new();
    let mut on_stack = HashSet::new();
    let mut sccs = Vec::new();

    fn strongconnect(
        node: &String,
        graph: &HashMap<String, Vec<String>>,
        index: &mut usize,
        indices: &mut HashMap<String, usize>,
        lowlink: &mut HashMap<String, usize>,
        stack: &mut Vec<String>,
        on_stack: &mut HashSet<String>,
        sccs: &mut Vec<Vec<String>>,
    ) {
        indices.insert(node.clone(), *index);
        lowlink.insert(node.clone(), *index);
        *index += 1;
        stack.push(node.clone());
        on_stack.insert(node.clone());

        if let Some(neighbors) = graph.get(node) {
            for neighbor in neighbors {
                if !indices.contains_key(neighbor) {
                    strongconnect(
                        neighbor,
                        graph,
                        index,
                        indices,
                        lowlink,
                        stack,
                        on_stack,
                        sccs,
                    );
                    let low_n = lowlink[neighbor];
                    let low_v = lowlink[node];
                    lowlink.insert(node.clone(), low_v.min(low_n));
                } else if on_stack.contains(neighbor) {
                    let idx_n = indices[neighbor];
                    let low_v = lowlink[node];
                    lowlink.insert(node.clone(), low_v.min(idx_n));
                }
            }
        }

        if indices[node] == lowlink[node] {
            let mut scc = Vec::new();
            while let Some(top) = stack.pop() {
                on_stack.remove(&top);
                scc.push(top.clone());
                if &top == node {
                    break;
                }
            }
            sccs.push(scc);
        }
    }

    // Run Tarjan's on all nodes
    for node in all_activities {
        if !indices.contains_key(node) {
            strongconnect(
                node,
                &graph,
                &mut index,
                &mut indices,
                &mut lowlink,
                &mut stack,
                &mut on_stack,
                &mut sccs,
            );
        }
    }

    sccs
}

/// Step 2: Build SCC DAG
pub fn build_scc_dag(
    sccs: &Vec<Vec<String>>,
    dfg: &HashMap<(String, String), usize>,
) -> (HashMap<usize, HashSet<usize>>, HashMap<String, usize>) {
    let mut node_to_scc = HashMap::new();
    for (i, scc) in sccs.iter().enumerate() {
        for node in scc {
            node_to_scc.insert(node.clone(), i);
        }
    }

    let mut dag: HashMap<usize, HashSet<usize>> = HashMap::new();
    for ((from, to), _) in dfg.iter() {
        let from_scc = node_to_scc[from];
        let to_scc = node_to_scc[to];
        if from_scc != to_scc {
            dag.entry(from_scc).or_default().insert(to_scc);
        }
    }

    (dag, node_to_scc)
}

/// Step 3: Extract set1 and set2 SCCs and their activity sets
pub fn partition_scc_sets(
    dag: &HashMap<usize, HashSet<usize>>,
    sccs: &Vec<Vec<String>>,
) {
    let mut set1: HashSet<usize> = HashSet::new();
    let mut set2: HashSet<usize> = HashSet::new();

    for (&from, targets) in dag.iter() {
        set1.insert(from);
        for &to in targets {
            set1.remove(&to); // ensure it's not in set1
            set2.insert(to);
        }
    }

    println!("Set1 (SCC ids): {:?}", set1);
    println!("Set2 (SCC ids): {:?}", set2);

    let mut act_set1 = HashSet::new();
    let mut act_set2 = HashSet::new();

    for &scc_id in set1.iter() {
        for act in &sccs[scc_id] {
            act_set1.insert(act.clone());
        }
    }
    for &scc_id in set2.iter() {
        for act in &sccs[scc_id] {
            act_set2.insert(act.clone());
        }
    }

    println!("Activity Set 1: {:?}", act_set1);
    println!("Activity Set 2: {:?}", act_set2);
}


// done----------

pub fn print_strongly_connected_components(
    dfg: &HashMap<(String, String), usize>,
    all_activities: &HashSet<String>,
) {
    // Step 1: Build adjacency list
    let mut graph: HashMap<String, Vec<String>> = HashMap::new();
    for (key, _) in dfg.iter() {
        let (from, to) = key;
        graph.entry(from.clone()).or_default().push(to.clone());
    }
    for activity in all_activities {
        graph.entry(activity.clone()).or_default();
    }

    // Step 2: Tarjan's Algorithm setup
    let mut index = 0;
    let mut indices: HashMap<String, usize> = HashMap::new();
    let mut lowlink: HashMap<String, usize> = HashMap::new();
    let mut stack: Vec<String> = Vec::new();
    let mut on_stack: HashSet<String> = HashSet::new();
    let mut sccs: Vec<Vec<String>> = Vec::new();

    // Step 3: Tarjan's Recursive DFS
    fn strongconnect(
        node: &String,
        graph: &HashMap<String, Vec<String>>,
        index: &mut usize,
        indices: &mut HashMap<String, usize>,
        lowlink: &mut HashMap<String, usize>,
        stack: &mut Vec<String>,
        on_stack: &mut HashSet<String>,
        sccs: &mut Vec<Vec<String>>,
    ) {
        indices.insert(node.clone(), *index);
        lowlink.insert(node.clone(), *index);
        *index += 1;
        stack.push(node.clone());
        on_stack.insert(node.clone());

        if let Some(neighbors) = graph.get(node) {
            for neighbor in neighbors {
                if !indices.contains_key(neighbor) {
                    strongconnect(
                        neighbor,
                        graph,
                        index,
                        indices,
                        lowlink,
                        stack,
                        on_stack,
                        sccs,
                    );
                    let neighbor_low = lowlink.get(neighbor).copied().unwrap();
                    let node_low = lowlink.get(node).copied().unwrap();
                    lowlink.insert(node.clone(), node_low.min(neighbor_low));
                } else if on_stack.contains(neighbor) {
                    let neighbor_index = indices.get(neighbor).copied().unwrap();
                    let node_low = lowlink.get(node).copied().unwrap();
                    lowlink.insert(node.clone(), node_low.min(neighbor_index));
                }
            }
        }

        if indices.get(node) == lowlink.get(node) {
            let mut scc = Vec::new();
            while let Some(top) = stack.pop() {
                on_stack.remove(&top);
                scc.push(top.clone());
                if &top == node {
                    break;
                }
            }
            sccs.push(scc);
        }
    }

    // Step 4: Run Tarjan's for all nodes
    for node in all_activities {
        if !indices.contains_key(node) {
            strongconnect(
                node,
                &graph,
                &mut index,
                &mut indices,
                &mut lowlink,
                &mut stack,
                &mut on_stack,
                &mut sccs,
            );
        }
    }

    // Step 5: Print SCCs
    println!("Strongly Connected Components:");
    for (i, scc) in sccs.iter().enumerate() {
        println!("Component {}: {:?}", i + 1, scc);
    }
}

/// Main entry function
pub fn find_sequence_cut(
    dfg: &HashMap<(String, String), usize>,
    all_activities: &HashSet<String>,
) {
    if all_activities.len() <= 1 {
        return;
    }

    // Build adjacency list and reverse graph
    let mut graph: HashMap<String, Vec<String>> = HashMap::new();
    let mut reverse_graph: HashMap<String, Vec<String>> = HashMap::new();
    for ((from, to), _) in dfg {
        graph.entry(from.clone()).or_default().push(to.clone());
        reverse_graph.entry(to.clone()).or_default().push(from.clone());
    }

    // Compute SCCs
    let sccs = compute_sccs(all_activities, &graph, &reverse_graph);

    // Try each SCC as Set1 (prefix)
    for (idx, candidate_scc) in sccs.iter().enumerate() {
        let set1 = candidate_scc.clone();
        let mut reachable = HashSet::new();

        for node in &set1 {
            dfs(node, &graph, &mut reachable);
        }

        let set2: HashSet<String> = reachable
            .difference(&set1)
            .cloned()
            .collect();

        if set1.is_empty() || set2.is_empty() {
            continue;
        }

        // Condition 1: No activity in set2 can reach set1
        let mut reverse_reach = HashSet::new();
        for node in &set2 {
            dfs(node, &graph, &mut reverse_reach);
        }
        if reverse_reach.intersection(&set1).count() > 0 {
            continue;
        }

        // Condition 2: All activities in set1 can reach every activity in set2
        let mut valid = true;
        for target in &set2 {
            let mut reachable_from_any = false;
            for source in &set1 {
                let mut visited = HashSet::new();
                dfs(source, &graph, &mut visited);
                if visited.contains(target) {
                    reachable_from_any = true;
                    break;
                }
            }
            if !reachable_from_any {
                valid = false;
                break;
            }
        }

        if valid {
            println!("Sequence cut found with {:?} and {:?}", set1, set2);

            let subgraph1 = build_subgraph(dfg, &set1);
            let subgraph2 = build_subgraph(dfg, &set2);
            find_sequence_cut(&subgraph1, &set1);
            find_sequence_cut(&subgraph2, &set2);
            return;
        }
    }

    println!("No valid sequence cut found for {:?}", all_activities);
}

/// Build subgraph restricted to given activity set
fn build_subgraph(
    dfg: &HashMap<(String, String), usize>,
    activities: &HashSet<String>,
) -> HashMap<(String, String), usize> {
    dfg.iter()
        .filter_map(|((from, to), weight)| {
            if activities.contains(from) && activities.contains(to) {
                Some(((from.clone(), to.clone()), *weight))
            } else {
                None
            }
        })
        .collect()
}

/// DFS from a node in a graph
fn dfs(node: &String, graph: &HashMap<String, Vec<String>>, visited: &mut HashSet<String>) {
    let mut stack = vec![node.clone()];
    while let Some(curr) = stack.pop() {
        if visited.insert(curr.clone()) {
            if let Some(neighbors) = graph.get(&curr) {
                for neighbor in neighbors {
                    stack.push(neighbor.clone());
                }
            }
        }
    }
}

/// Compute strongly connected components using Kosaraju's algorithm
fn compute_sccs(
    all_activities: &HashSet<String>,
    graph: &HashMap<String, Vec<String>>,
    reverse_graph: &HashMap<String, Vec<String>>,
) -> Vec<HashSet<String>> {
    let mut visited = HashSet::new();
    let mut finish_stack = Vec::new();

    for node in all_activities {
        if !visited.contains(node) {
            dfs_postorder(node, graph, &mut visited, &mut finish_stack);
        }
    }

    visited.clear();
    let mut sccs = Vec::new();

    while let Some(node) = finish_stack.pop() {
        if !visited.contains(&node) {
            let mut component = HashSet::new();
            dfs_collect(&node, reverse_graph, &mut visited, &mut component);
            sccs.push(component);
        }
    }

    sccs
}

/// DFS post-order for Kosaraju step 1
fn dfs_postorder(
    node: &String,
    graph: &HashMap<String, Vec<String>>,
    visited: &mut HashSet<String>,
    stack: &mut Vec<String>,
) {
    visited.insert(node.clone());
    if let Some(neighbors) = graph.get(node) {
        for neighbor in neighbors {
            if !visited.contains(neighbor) {
                dfs_postorder(neighbor, graph, visited, stack);
            }
        }
    }
    stack.push(node.clone());
}

/// DFS collect for Kosaraju step 2
fn dfs_collect(
    node: &String,
    graph: &HashMap<String, Vec<String>>,
    visited: &mut HashSet<String>,
    component: &mut HashSet<String>,
) {
    visited.insert(node.clone());
    component.insert(node.clone());
    if let Some(neighbors) = graph.get(node) {
        for neighbor in neighbors {
            if !visited.contains(neighbor) {
                dfs_collect(neighbor, graph, visited, component);
            }
        }
    }
}



pub fn find_cuts_start(
    dfg: &HashMap<(String, String), usize>,
    all_activities: &HashSet<String>,
) {

    // find_sequence_cut(&dfg, &all_activities);

    // print_strongly_connected_components(&dfg, &all_activities);

    let sccs = strongly_connected_components(&dfg, &all_activities);
    println!("SCCs:");
    for (i, comp) in sccs.iter().enumerate() {
        println!("  SCC {}: {:?}", i, comp);
    }

    let (dag, _) = build_scc_dag(&sccs, &dfg);
    println!("SCC DAG:");
    for (from, tos) in &dag {
        for to in tos {
            println!("  SCC {} -> SCC {}", from, to);
        }
    }

    // Step 3: Create set1 and set2
    let mut set1: HashSet<usize> = HashSet::new();
    let mut set2: HashSet<usize> = HashSet::new();
    for (from, tos) in &dag {
        for to in tos {
            set1.insert(*from);
            set2.insert(*to);
        }
    }

    // Step 4: Remove overlaps from set1
    let intersection: HashSet<_> = set1.intersection(&set2).cloned().collect();
    for i in intersection {
        set1.remove(&i);
    }

    println!("\nSCC index sets:");
    println!("  Set1 (sources): {:?}", set1);
    println!("  Set2 (targets): {:?}", set2);

    // Step 5: Map SCCs to activity sets
    let mut act_set1 = HashSet::new();
    let mut act_set2 = HashSet::new();

    for i in &set1 {
        for act in &sccs[*i] {
            act_set1.insert(act.clone());
        }
    }

    for i in &set2 {
        for act in &sccs[*i] {
            act_set2.insert(act.clone());
        }
    }

    println!("\nActivity sets:");
    println!("  act_set1: {:?}", act_set1);
    println!("  act_set2: {:?}", act_set2);


    // // Initialize sets with each activity having an empty HashSet
    // let mut seq_sets: HashMap<String, HashSet<String>> = HashMap::new();
    // let mut para_sets: HashMap<String, HashSet<String>> = HashMap::new();
    // let mut excl_sets: HashMap<String, HashSet<String>> = HashMap::new();
    // let mut redo_sets: HashMap<String, HashSet<String>> = HashMap::new();

    // for activity in all_activities {
    //     seq_sets.insert(activity.clone(), HashSet::new());
    //     para_sets.insert(activity.clone(), HashSet::new());
    //     excl_sets.insert(activity.clone(), HashSet::new());
    //     redo_sets.insert(activity.clone(), HashSet::new());
    // }

    // let activities_vec: Vec<String> = all_activities.iter().cloned().collect();
    // let n = activities_vec.len();

    // for i in 0..n {
    //     for j in (i + 1)..n {
    //         let activity1 = &activities_vec[i];
    //         let activity2 = &activities_vec[j];

    //         let act1_act2 = is_reachable(dfg, activity1, activity2);
    //         let act2_act1 = is_reachable(dfg, activity2, activity1);

    //         match (act1_act2, act2_act1) {
    //             (true, false) => {
    //                 if let Some(set) = seq_sets.get_mut(activity1) {
    //                     set.insert(activity2.clone());
    //                 }
    //             }
    //             (false, true) => {
    //                 if let Some(set) = seq_sets.get_mut(activity2) {
    //                     set.insert(activity1.clone());
    //                 }
    //             }
    //             (true, true) => {
    //                 if let Some(set) = para_sets.get_mut(activity1) {
    //                     set.insert(activity2.clone());
    //                 }
    //                 if let Some(set) = para_sets.get_mut(activity2) {
    //                     set.insert(activity1.clone());
    //                 }
    //             }
    //             (false, false) => {
    //                 if let Some(set) = excl_sets.get_mut(activity1) {
    //                     set.insert(activity2.clone());
    //                 }
    //                 if let Some(set) = excl_sets.get_mut(activity2) {
    //                     set.insert(activity1.clone());
    //                 }
    //             }
    //         }
    //     }
    // }

    // // You can return or print the maps here if needed
    // // For now, just to show it's valid:
    // info!("Sequential sets: {:?}", seq_sets);
    // // info!("Parallel sets: {:?}", para_sets);
    // // info!("Exclusive sets: {:?}", excl_sets);

    //  // Sequential cut detection logic
    // let mut set1: HashSet<String> = HashSet::new();
    // let mut set2: HashSet<String> = HashSet::new();
    // let mut unknown: HashSet<String> = HashSet::new();

    // for act in all_activities {
    //     if !set1.contains(act) && !set2.contains(act) {
    //         if let Some(targets) = seq_sets.get(act) {
    //             if !targets.is_empty() {
    //                 set1.insert(act.clone());
    //                 for t in targets {
    //                     set2.insert(t.clone());
    //                 }
    //             } else {
    //                 unknown.insert(act.clone());
    //             }
    //         }
    //     }
    // }
    // info!("Set1: {:?}", set1);
    // info!("Set2: {:?}", set2);
    // info!("Unknown: {:?}", unknown);
    // if set1.len() + set2.len() == all_activities.len() {
    //     info!("Seq cut found: {:?} (->) {:?}", set1, set2);
    //     find_cuts_start(&dfg, &set1);
    //     find_cuts_start(&dfg, &set2);
    // } else {
    //     // info!("Size of all activities: {}, set1: {}, set2: {}", all_activities.len(), set1.len(), set2.len());
    //     info!("No sequential cut found");
    // }

}

fn find_best_cut(
    seq_sets: &HashMap<String, HashSet<String>>,
    all_activities: &HashSet<String>,
){
    // Sequential cut detection logic
    let mut set1: HashSet<String> = HashSet::new();
    let mut set2: HashSet<String> = HashSet::new();
    let mut unknown: HashSet<String> = HashSet::new();

    for act in all_activities {
        if !set1.contains(act) && !set2.contains(act) {
            if let Some(targets) = seq_sets.get(act) {
                if !targets.is_empty() {
                    set1.insert(act.clone());
                    for t in targets {
                        set2.insert(t.clone());
                    }
                } else {
                    unknown.insert(act.clone());
                }
            }
        }
    }
    info!("Set1: {:?}", set1);
    info!("Set2: {:?}", set2);
    info!("Unknown: {:?}", unknown);
    if set1.len() + set2.len() == all_activities.len() {
        info!("Seq cut found: {:?} (->) {:?}", set1, set2);
        find_best_cut(&seq_sets, &set1);
        find_best_cut(&seq_sets, &set2);
    } else {
        info!("Size of all activities: {}, set1: {}, set2: {}", all_activities.len(), set1.len(), set2.len());
        info!("No sequential cut found, final sets: {:?} and {:?}", set1, set2);
    }
}
    

pub fn find_cuts(
    dfg: &HashMap<(String, String), usize>,
    filtered_dfg: &HashMap<(String, String), usize>,
    all_activities: HashSet<String>,
    start_activities: &HashSet<String>,
    end_activities: &HashSet<String>
) -> ProcessForest {
    let mut forest = Vec::new();

    let activities: Vec<String> = all_activities.clone().into_iter().collect();
    let n = activities.len();

    if n == 1 {
        // Base case: single activity, create a leaf node
        let node = TreeNode {
            label: activities[0].clone(),
            children: Vec::new(),
        };
        forest.push(node);
        return forest;
    }

    for i in 1..n {
        for combo in activities.iter().combinations(i) {
            let combo_set: HashSet<String> = combo.into_iter().cloned().collect();
            let complement_set: HashSet<String> = all_activities
                .difference(&combo_set)
                .cloned()
                .collect();

            let combined_activites: HashSet<String> = combo_set.union(&complement_set).cloned().collect();
            let filtered_dfg = filter_keep_dfg(&dfg, &combined_activites);

            let (filtered_start_activites,filtered_end_activites)=get_start_and_end_activities_v2(&dfg, &combined_activites, start_activities, end_activities);
            
            // info!("Checking cut: {:?} (.....) {:?}", combo_set, complement_set);

            // let excl_cut = is_exclusive_choice_cut_possible(&filtered_dfg, &combo_set, &complement_set);
            // if excl_cut {
            //     info!("Excl parallel cut found: {:?} (X) {:?}", combo_set, complement_set);
            //     let mut node = TreeNode {
            //         label: "excl".to_string(),
            //         children: Vec::new(),
            //     };
            //     node.children.extend(find_cuts(&dfg, &filtered_dfg, combo_set, &start_activities, &end_activities));
            //     node.children.extend(find_cuts(&dfg, &filtered_dfg, complement_set, &start_activities, &end_activities));
            //     forest.push(node);
            //     return forest; // Return early if you only want the first valid cut
            // }

            // let seq_cut = is_sequence_cut_possible(&filtered_dfg, &combo_set, &complement_set);
            // if seq_cut {
            //     info!("Seq parallel cut found: {:?} (->) {:?}", combo_set, complement_set);
            //     let mut node = TreeNode {
            //         label: "seq".to_string(),
            //         children: Vec::new(),
            //     };
            //     node.children.extend(find_cuts(&dfg, &filtered_dfg, combo_set, &start_activities, &end_activities));
            //     node.children.extend(find_cuts(&dfg, &filtered_dfg, complement_set, &start_activities, &end_activities));
            //     forest.push(node);
            //     return forest; // Return early if you only want the first valid cut
            // }

            // let para_cut = is_parallel_cut_possible(&filtered_dfg, &combo_set, &complement_set, &filtered_start_activites, &filtered_end_activites);
            // if para_cut {
            //     info!("Parallel cut found: {:?} (||) {:?}", combo_set, complement_set);
            //     let mut node = TreeNode {
            //         label: "para".to_string(),
            //         children: Vec::new(),
            //     };
            //     node.children.extend(find_cuts(&dfg, &filtered_dfg, combo_set, &start_activities, &end_activities));
            //     node.children.extend(find_cuts(&dfg, &filtered_dfg, complement_set, &start_activities, &end_activities));
            //     forest.push(node);
            //     return forest; // Return early if you only want the first valid cut
            // }

            // let redo_cut = is_redo_cut_possible(&filtered_dfg, &combo_set, &complement_set, &filtered_start_activites, &filtered_end_activites);
            // if redo_cut {
            //     info!("Redo cut found: {:?} (O->) {:?}", combo_set, complement_set);
            //     let mut node = TreeNode {
            //         label: "redo".to_string(),
            //         children: Vec::new(),
            //     };
            //     node.children.extend(find_cuts(&dfg, &filtered_dfg, combo_set, &start_activities, &end_activities));
            //     node.children.extend(find_cuts(&dfg, &filtered_dfg, complement_set, &start_activities, &end_activities));
            //     forest.push(node);
            //     return forest; // Return early if you only want the first valid cut
            // }
        }
    }

    // If no valid cuts are found, return disjoint trees
    for activity in activities {
        let node = TreeNode {
            label: activity,
            children: Vec::new(),
        };
        forest.push(node);
    }

    forest
}

fn is_reachable(
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

fn filter_keep_dfg(
    dfg: &HashMap<(String, String), usize>,
    keep_list: &HashSet<String>,
) -> HashMap<(String, String), usize> {
    dfg.iter()
        .filter(|((from, to), _)| {
            keep_list.contains(from) && keep_list.contains(to)
        })
        .map(|(k, v)| (k.clone(), *v))
        .collect()
}


fn is_sequence_cut_possible(
    dfg: &HashMap<(String, String), usize>,
    set_1_activities: &HashSet<String>,
    set_2_activities: &HashSet<String>,
) -> bool {
    // Check if every activity in set_2 is reachable from every activity in set_1
    for act1 in set_1_activities {
        for act2 in set_2_activities {
            if !is_reachable(dfg, act1, act2) {
                return false;
            }
        }
    }

    // Ensure no activity in set_1 is reachable from any activity in set_2
    for act2 in set_2_activities {
        for act1 in set_1_activities {
            
            if is_reachable(dfg, act2, act1) {
                return false;
            }

        }
    }
    true
}

fn is_exclusive_choice_cut_possible(
    dfg: &HashMap<(String, String), usize>,
    set_1_activities: &HashSet<String>,
    set_2_activities: &HashSet<String>,
) -> bool {
    for act1 in set_1_activities {
        for act2 in set_2_activities {
            if dfg.contains_key(&(act1.clone(), act2.clone())) || dfg.contains_key(&(act2.clone(), act1.clone())) {
                return false;
            }
        }
    }
    true
}

fn is_parallel_cut_possible(
    dfg: &HashMap<(String, String), usize>,
    set_1_activities: &HashSet<String>,
    set_2_activities: &HashSet<String>,
    start_activities: &HashSet<String>,
    end_activities: &HashSet<String>,
) -> bool {
    // info!("Checking parallel cut...");
    // info!("Start activities: {:?}", start_activities);
    // info!("End activities: {:?}", end_activities);

    // 1. There must be some common activities between set_1_activities and start_activities
    if set_1_activities.is_disjoint(start_activities) {
        return false;
    }

    // 2. There must be some common activities between set_1_activities and end_activities
    if set_1_activities.is_disjoint(end_activities) {
        return false;
    }

    // 3. There must be some common activities between set_2_activities and start_activities
    if set_2_activities.is_disjoint(start_activities) {
        return false;
    }

    // 4. There must be some common activities between set_2_activities and end_activities
    if set_2_activities.is_disjoint(end_activities) {
        return false;
    }

    // 5. ∀ a ∈ set_1, ∀ b ∈ set_2: (a, b) ∈ dfg ∧ (b, a) ∈ dfg
    for a in set_1_activities {
        for b in set_2_activities {
            if !dfg.contains_key(&(a.clone(), b.clone())) || !dfg.contains_key(&(b.clone(), a.clone())) {
                return false;
            }
        }
    }

    true
}

fn is_redo_cut_possible(
    dfg: &HashMap<(String, String), usize>,
    set_1_activities: &HashSet<String>,
    set_2_activities: &HashSet<String>,
    start_activities: &HashSet<String>,
    end_activities: &HashSet<String>,
) -> bool {

    // info!("Checking redo...");
    // Condition 1: ignore for now as n==2

    // Condition 2: All start and end activities must be in set_1_activities
    if !start_activities.is_subset(set_1_activities) || !end_activities.is_subset(set_1_activities) {
        return false;
    }
    // info!("Condition 1 and 2 passed...");
    // Condition 3: There exists an (a ∈ end_activities, b ∈ set_2_activities) such that (a, b) ∈ dfg
    let mut condition2_met = false;
    // info!("End activities: {:?}", end_activities);
    // info!("Set 2 activities: {:?}", set_2_activities);
    // info!("DFG: {:?}", dfg);
    for a in end_activities {
        for b in set_2_activities {
            if dfg.contains_key(&(a.clone(), b.clone())) {
                // info!("Condition 3 met for {} -> {}", a, b);
                condition2_met = true;
                break;
            }
        }
        if condition2_met {
            break;
        }
    }
    if !condition2_met {
        return false;
    }

    // info!("Condition 3 passed...");

    // Condition 4: There exists an (a ∈ start_activities, b ∈ set_2_activities) such that (b, a) ∈ dfg
    let mut condition3_met = false;
    for a in start_activities {
        for b in set_2_activities {
            if dfg.contains_key(&(b.clone(), a.clone())) {
                condition3_met = true;
                break;
            }
        }
        if condition3_met {
            break;
        }
    }
    if !condition3_met {
        return false;
    }

    // info!("Condition 4 passed...");

    // Condition 5: since we are only considering 2 sets of activities, we can skip this condition.

    // Condition 6: For every a ∈ end_activities, there exists a b ∈ set_2_activities such that (a, b) ∈ dfg
    for a in end_activities {
        let mut found = false;
        for b in set_2_activities {
            if dfg.contains_key(&(a.clone(), b.clone())) {
                found = true;
                break;
            }
        }
        if !found {
            return false;
        }
    }

    // info!("Condition 5 and 6 passed...");

    // Condition 7: For every a ∈ start_activities, there exists a b ∈ set_2_activities such that (b, a) ∈ dfg
    for a in start_activities {
        let mut found = false;
        for b in set_2_activities {
            if dfg.contains_key(&(b.clone(), a.clone())) {
                found = true;
                break;
            }
        }
        if !found {
            return false;
        }
    }

    // info!("Condition 7 passed...");

    true
}


fn get_start_and_end_activities(
    dfg: &HashMap<(String, String), usize>,
    filtered_activities: &HashSet<String>,
) -> (HashSet<String>, HashSet<String>) {
    let mut start_activities = HashSet::new();
    let mut end_activities = HashSet::new();

    // info!("Getting start and end activities based on filtered set...");

    for ((from, to), _) in dfg {
        let from_in = filtered_activities.contains(from);
        let to_in = filtered_activities.contains(to);

        if to_in && !from_in {
            // 'to' is inside filtered, 'from' is outside → 'to' is a start activity
            start_activities.insert(to.clone());
        }

        if from_in && !to_in {
            // 'from' is inside filtered, 'to' is outside → 'from' is an end activity
            end_activities.insert(from.clone());
        }
    }

    // info!("Start activities: {:?}", start_activities);
    // info!("End activities: {:?}", end_activities);

    (start_activities, end_activities)
}

fn get_start_and_end_activities_v2(
    dfg: &HashMap<(String, String), usize>,
    filtered_activities: &HashSet<String>,
    global_start_activities: &HashSet<String>,
    global_end_activities: &HashSet<String>,
) -> (HashSet<String>, HashSet<String>) {
    let mut start_activities = HashSet::new();
    let mut end_activities = HashSet::new();

    for ((a, b), _) in dfg {
        let a_in = filtered_activities.contains(a);
        let b_in = filtered_activities.contains(b);

        if !a_in && b_in {
            // 'a' is outside and 'b' is inside → 'b' is a start activity
            start_activities.insert(b.clone());
        }

        if a_in && !b_in {
            // 'a' is inside and 'b' is outside → 'a' is an end activity
            end_activities.insert(a.clone());
        }
    }

    // Add common activities from global sets
    for activity in filtered_activities {
        if global_start_activities.contains(activity) {
            start_activities.insert(activity.clone());
        }
        if global_end_activities.contains(activity) {
            end_activities.insert(activity.clone());
        }
    }

    (start_activities, end_activities)
}








