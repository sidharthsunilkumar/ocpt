use crate::cost_to_cut::to_be_non_reachable;
use crate::good_cuts::best_possible_sequence_cut;
use crate::best_sequence_cut::best_sequence_cut;
use crate::types::{ProcessForest, TreeNode};
use itertools::Itertools;
use log::info;
use std::collections::{HashMap, HashSet, VecDeque};



pub fn find_cuts_start(dfg: &HashMap<(String, String), usize>, all_activities: &HashSet<String>) {

    if all_activities.len() <= 1 {
        return;
    }


    let filtered_dfg = filter_keep_dfg(&dfg, &all_activities);
    let (start_activities, end_activities) = get_start_and_end_activities(&dfg, &all_activities);


    // ----- perform cuts--------

    

    let (excl_set1, excl_set2) = find_exclusive_choice_cut(&filtered_dfg, &all_activities);
    if(!excl_set1.is_empty() && !excl_set2.is_empty()) {
        info!("Exclusive cut found: {:?} (X) {:?}", excl_set1, excl_set2);
        find_cuts_start(&dfg, &excl_set1);
        find_cuts_start(&dfg, &excl_set2); 
        return; 
    } 

    let (set1, set2) = find_sequence_cut(&filtered_dfg, &all_activities);
    if(!set1.is_empty() && !set2.is_empty()) {
        info!("Sequence cut found: {:?} (->) {:?}", set1, set2);
        find_cuts_start(&dfg, &set1);
        find_cuts_start(&dfg, &set2);
        return;
    }

       

    let (is_parallel, para_set1, para_set2) = find_parallel_cut(&filtered_dfg, &all_activities);
    if(is_parallel && !para_set1.is_empty() && !para_set2.is_empty() && parallel_cut_condition_check(&para_set1, &para_set2, &start_activities, &end_activities)) {
        info!("Parallel cut found: {:?} (||) {:?}", para_set1, para_set2);
        find_cuts_start(&dfg, &para_set1);
        find_cuts_start(&dfg, &para_set2);  
        return;
    } 

    let (is_redo, redo_set1, redo_set2) = find_redo_cut(&filtered_dfg, &all_activities, &start_activities, &end_activities);
    if (is_redo && !redo_set2.is_empty() && !redo_set1.is_empty() && redo_cut_condition_check(&filtered_dfg, &redo_set1, &redo_set2, &start_activities, &end_activities)) {
        info!("Redo cut found: {:?} (R) {:?}", redo_set1, redo_set2);
        find_cuts_start(&dfg, &redo_set1);
        find_cuts_start(&dfg, &redo_set2); 
        return; 
    }

    info!("No further cuts found for the current set of activities: {:?}", all_activities);
    info!("Checking for best possible sequence cut...");
    // best_possible_sequence_cut(&filtered_dfg, &all_activities);
    let (min_cost, no_of_cuts, cut_edges, bs_set1, bs_set2, new_dfg) = best_sequence_cut(&filtered_dfg, &all_activities);
    info!("\n=== BEST SEQUENCE CUT RESULTS ===");
    info!("Minimum Cost: {}", min_cost);
    info!("Number of cut edges: {}", no_of_cuts);
    info!("Cut Edges: {:?}", cut_edges);
    info!("Set 1: {:?}", bs_set1);
    info!("Set 2: {:?}", bs_set2);

    find_cuts_start(&new_dfg, &bs_set1);
    find_cuts_start(&new_dfg, &bs_set2);
    return;


}

// Exclusive cut and helpers --------------
fn find_exclusive_choice_cut(
    dfg: &HashMap<(String, String), usize>,
    all_activities: &HashSet<String>,
) -> (HashSet<String>, HashSet<String>) {
    // Step 1: Convert to undirected adjacency list
    let mut undirected_graph: HashMap<String, HashSet<String>> = HashMap::new();

    for ((from, to), _) in dfg {
        undirected_graph.entry(from.clone()).or_default().insert(to.clone());
        undirected_graph.entry(to.clone()).or_default().insert(from.clone());
    }

    for activity in all_activities {
        undirected_graph.entry(activity.clone()).or_default(); // ensure isolated nodes are included
    }

    // Step 2: Find connected components using BFS
    let mut visited: HashSet<String> = HashSet::new();
    let mut components: Vec<HashSet<String>> = Vec::new();

    for activity in all_activities {
        if !visited.contains(activity) {
            let mut component = HashSet::new();
            let mut queue = VecDeque::new();
            queue.push_back(activity.clone());
            visited.insert(activity.clone());

            while let Some(current) = queue.pop_front() {
                component.insert(current.clone());
                for neighbor in undirected_graph.get(&current).unwrap_or(&HashSet::new()) {
                    if !visited.contains(neighbor) {
                        visited.insert(neighbor.clone());
                        queue.push_back(neighbor.clone());
                    }
                }
            }

            components.push(component);
        }
    }

    // // Step 3: Print disjoint components
    // info!("Disjoint components:");
    // for (i, comp) in components.iter().enumerate() {
    //     info!("Component {}: {:?}", i + 1, comp);
    // }

    // Step 4: Assign first component to set1, rest to set2
    let mut set1 = HashSet::new();
    let mut set2 = HashSet::new();

    if !components.is_empty() {
        set1 = components[0].clone();
        for comp in components.iter().skip(1) {
            set2.extend(comp.iter().cloned());
        }
    }

    // Step 5: Print the final sets
    // info!("\nSet 1: {:?}", set1);
    // info!("Set 2: {:?}", set2);
    (set1, set2)
}

// ------- Sequence cut and helpers ------------
fn find_sequence_cut(dfg: &HashMap<(String, String), usize>, all_activities: &HashSet<String>)
-> (HashSet<String>, HashSet<String>) {
    let sccs = strongly_connected_components(&dfg, &all_activities);
    // println!("SCCs:");
    // for (i, comp) in sccs.iter().enumerate() {
    //     println!("  SCC {}: {:?}", i, comp);
    // }

    let (dag, _) = build_scc_dag(&sccs, &dfg);
    // println!("SCC DAG:");
    // for (from, tos) in &dag {
    //     for to in tos {
    //         println!("  SCC {} -> SCC {}", from, to);
    //     }
    // }

    let (set1, set2) = partition_scc_sets(&dag, &sccs);

    // println!("Set1 (sources): {:?}", set1);
    // println!("Set2 (targets): {:?}", set2);

    (set1, set2)
}

/// Step 1: Tarjan's Algorithm to find SCCs
fn strongly_connected_components(
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
                        neighbor, graph, index, indices, lowlink, stack, on_stack, sccs,
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
) -> (HashSet<String>, HashSet<String>) {
    // Step 3: Create set1 and set2
    let mut set1: HashSet<usize> = HashSet::new();
    let mut set2: HashSet<usize> = HashSet::new();
    for (from, tos) in dag {
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

    // println!("\nSCC index sets:");
    // println!("  Set1 (sources): {:?}", set1);
    // println!("  Set2 (targets): {:?}", set2);

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

    // println!("\nActivity sets:");
    // println!("  act_set1: {:?}", act_set1);
    // println!("  act_set2: {:?}", act_set2);

    (act_set1, act_set2)
}

// --------------------- Parallel cut and helpers ---------------------

fn find_parallel_cut(
    dfg: &HashMap<(String, String), usize>,
    all_activities: &HashSet<String>,
) -> (bool, HashSet<String>, HashSet<String>) {
    let mut set1: HashSet<String> = HashSet::new();
    let mut set2: HashSet<String> = HashSet::new();

    for act in all_activities {
        if set1.is_empty() {
            set1.insert(act.clone());
            continue;
        }

        let mut singleton = HashSet::new();
        singleton.insert(act.clone());

        if check_bi_direction_sets(dfg, &singleton, &set1)
            && check_bi_direction_sets(dfg, &set1, &singleton)
        {
            set2.insert(act.clone());
        } else {
            if set2.is_empty() {
                set1.insert(act.clone());
                continue;
            }

            if check_bi_direction_sets(dfg, &singleton, &set2)
                && check_bi_direction_sets(dfg, &set2, &singleton)
            {
                set1.insert(act.clone());
            } else {
                return (false, set1, set2);
            }
        }
    }

    (true, set1, set2)
}

fn parallel_cut_condition_check(
    set1: &HashSet<String>,
    set2: &HashSet<String>,
    start_activities: &HashSet<String>,
    end_activities: &HashSet<String>,
) -> bool {
    let cond1 = !set1.is_disjoint(start_activities);
    let cond2 = !set1.is_disjoint(end_activities);
    let cond3 = !set2.is_disjoint(start_activities);
    let cond4 = !set2.is_disjoint(end_activities);

    cond1 && cond2 && cond3 && cond4
}

// --------------------- Redo cut and helpers ---------------------
fn find_redo_cut(
    dfg: &HashMap<(String, String), usize>,
    all_activities: &HashSet<String>,
    start_activities: &HashSet<String>,
    end_activities: &HashSet<String>,
) -> (bool, HashSet<String>, HashSet<String>) {
    let mut set1: HashSet<String> = HashSet::new();
    let mut set2: HashSet<String> = HashSet::new();

    // Add start and end activities to set1
    set1.extend(start_activities.iter().cloned());
    set1.extend(end_activities.iter().cloned());

    for x in all_activities {
        if set1.contains(x) {
            continue;
        }

        let is_s1_redo = is_reachable_before_end_activity(start_activities, x, end_activities, dfg);
        let is_s2_redo = is_reachable_before_end_activity(end_activities, x, start_activities, dfg);

        if is_s1_redo && !is_s2_redo{
            set1.insert(x.clone());
        } else if (!is_s1_redo && is_s2_redo) {
            set2.insert(x.clone());            
        } else {
            return (false, set1, set2);
        }
    }

    (true, set1, set2)
}

fn redo_cut_condition_check(
    dfg: &HashMap<(String, String), usize>,
    set1: &HashSet<String>,
    set2: &HashSet<String>,
    start_activities: &HashSet<String>,
    end_activities: &HashSet<String>,
) -> bool {
    // 1. All start_activities and end_activities must be in set1
    if !start_activities.is_subset(set1) || !end_activities.is_subset(set1) {
        return false;
    }

    // 2. There exists (e, x) ∈ dfg where e ∈ end_activities and x ∈ set2
    let mut cond2 = false;
    for e in end_activities {
        for x in set2 {
            if dfg.contains_key(&(e.clone(), x.clone())) {
                cond2 = true;
                break;
            }
        }
        if cond2 {
            break;
        }
    }
    if !cond2 {
        return false;
    }

    // 3. There exists (x, s) ∈ dfg where x ∈ set2 and s ∈ start_activities
    let mut cond3 = false;
    for x in set2 {
        for s in start_activities {
            if dfg.contains_key(&(x.clone(), s.clone())) {
                cond3 = true;
                break;
            }
        }
        if cond3 {
            break;
        }
    }
    if !cond3 {
        return false;
    }

    // 4. For every e ∈ end_activities, there exists b ∈ set2 such that (e, b) ∈ dfg
    for e in end_activities {
        let mut found = false;
        for b in set2 {
            if dfg.contains_key(&(e.clone(), b.clone())) {
                found = true;
                break;
            }
        }
        if !found {
            return false;
        }
    }

    // 5. For every s ∈ start_activities, there exists b ∈ set2 such that (b, s) ∈ dfg
    for s in start_activities {
        let mut found = false;
        for b in set2 {
            if dfg.contains_key(&(b.clone(), s.clone())) {
                found = true;
                break;
            }
        }
        if !found {
            return false;
        }
    }

    true
}

// --------------------- common helpers ---------------------

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

fn check_bi_direction_sets(
    dfg: &HashMap<(String, String), usize>,
    set1: &HashSet<String>,
    set2: &HashSet<String>,
) -> bool {
    for m in set1 {
        for n in set2 {
            if !dfg.contains_key(&(m.clone(), n.clone())) || !dfg.contains_key(&(n.clone(), m.clone())) {
                return false;
            }
        }
    }
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

fn is_reachable_before_end_activity(
    start_activities: &HashSet<String>,
    target: &String,
    end_activities: &HashSet<String>,
    dfg: &HashMap<(String, String), usize>,
) -> bool {
    fn dfs(
        current: &String,
        target: &String,
        end_activities: &HashSet<String>,
        dfg: &HashMap<(String, String), usize>,
        visited: &mut HashSet<String>,
    ) -> bool {
        if current == target {
            return true;
        }

        if visited.contains(current) || end_activities.contains(current) {
            return false;
        }

        visited.insert(current.clone());

        for (src, dst) in dfg.keys() {
            if src == current {
                if dfs(dst, target, end_activities, dfg, visited) {
                    return true;
                }
            }
        }

        false
    }

    for start in start_activities {
        let mut visited = HashSet::new();
        if dfs(start, target, end_activities, dfg, &mut visited) {
            return true;
        }
    }

    false
}