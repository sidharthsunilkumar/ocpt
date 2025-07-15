use crate::best_exclusive_cut::best_exclusive_cut;
use crate::best_parallel_cut::best_parallel_cut;
use crate::best_parallel_cut_exhaustive::best_parallel_cut_exhaustive;
use crate::best_parallel_cut_v2::best_parallel_cut_v2;
use crate::best_redo_cuts::best_redo_cut;
use crate::best_sequence_cut::best_sequence_cut;
use crate::best_sequence_cut_v2;
use crate::cost_to_cut::is_reachable;
use crate::cost_to_cut::to_be_non_reachable;
use crate::good_cuts::best_possible_sequence_cut;
use crate::start_cuts::{is_exclusive_choice_cut_possible, is_sequence_cut_possible};
use crate::types::CutSuggestion;
use crate::types::CutSuggestionsList;
use crate::types::{ProcessForest, TreeNode};
use itertools::Itertools;
use log::info;
use std::collections::{HashMap, HashSet, VecDeque};

pub fn find_cuts_start(
    dfg: &HashMap<(String, String), usize>,
    all_activities: &HashSet<String>,
    start_activities: &HashSet<String>,
    end_activities: &HashSet<String>,
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

    let filtered_dfg = filter_keep_dfg(&dfg, &all_activities);
    let (start_activities, end_activities) =
        get_start_and_end_activities(&dfg, &all_activities, &start_activities, &end_activities);

    // ----- perform cuts--------

    let (excl_set1, excl_set2) = find_exclusive_choice_cut(&filtered_dfg, &all_activities);
    if (!excl_set1.is_empty()
        && !excl_set2.is_empty()
        && is_exclusive_choice_cut_possible(&filtered_dfg, &excl_set1, &excl_set2))
    {
        info!("Exclusive cut found: {:?} (X) {:?}", excl_set1, excl_set2);
        let mut node = TreeNode {
            label: "exclusive".to_string(),
            children: Vec::new(),
        };
        node.children.extend(find_cuts_start(
            &dfg,
            &excl_set1,
            &start_activities,
            &end_activities,
        ));
        node.children.extend(find_cuts_start(
            &dfg,
            &excl_set2,
            &start_activities,
            &end_activities,
        ));
        forest.push(node);
        return forest;
    }

    let (set1, set2) = find_sequence_cut(&filtered_dfg, &all_activities);
    if (!set1.is_empty()
        && !set2.is_empty()
        && is_sequence_cut_possible(&filtered_dfg, &set1, &set2))
    {
        info!("Sequence cut found: {:?} (->) {:?}", set1, set2);
        let mut node = TreeNode {
            label: "sequence".to_string(),
            children: Vec::new(),
        };
        node.children.extend(find_cuts_start(
            &dfg,
            &set1,
            &start_activities,
            &end_activities,
        ));
        node.children.extend(find_cuts_start(
            &dfg,
            &set2,
            &start_activities,
            &end_activities,
        ));
        forest.push(node);
        return forest;
    }

    let (is_parallel, para_set1, para_set2) = find_parallel_cut(&filtered_dfg, &all_activities);
    if (is_parallel
        && !para_set1.is_empty()
        && !para_set2.is_empty()
        && parallel_cut_condition_check(&para_set1, &para_set2, &start_activities, &end_activities))
    {
        info!("Parallel cut found: {:?} (||) {:?}", para_set1, para_set2);
        let mut node = TreeNode {
            label: "parallel".to_string(),
            children: Vec::new(),
        };
        node.children.extend(find_cuts_start(
            &dfg,
            &para_set1,
            &start_activities,
            &end_activities,
        ));
        node.children.extend(find_cuts_start(
            &dfg,
            &para_set2,
            &start_activities,
            &end_activities,
        ));
        forest.push(node);
        return forest;
    }

    let (is_redo, redo_set1, redo_set2) = find_redo_cut(
        &filtered_dfg,
        &all_activities,
        &start_activities,
        &end_activities,
    );
    if (is_redo
        && !redo_set2.is_empty()
        && !redo_set1.is_empty()
        && redo_cut_condition_check(
            &filtered_dfg,
            &redo_set1,
            &redo_set2,
            &start_activities,
            &end_activities,
        ))
    {
        info!("Redo cut found: {:?} (R) {:?}", redo_set1, redo_set2);
        let mut node = TreeNode {
            label: "redo".to_string(),
            children: Vec::new(),
        };
        node.children.extend(find_cuts_start(
            &dfg,
            &redo_set1,
            &start_activities,
            &end_activities,
        ));
        node.children.extend(find_cuts_start(
            &dfg,
            &redo_set2,
            &start_activities,
            &end_activities,
        ));
        forest.push(node);
        return forest;
    }

    info!(
        "No further cuts found for the current set of activities: {:?}",
        all_activities
    );

    // If no valid cuts are found, return disjoint trees
    for activity in activities {
        let node = TreeNode {
            label: activity,
            children: Vec::new(),
        };
        forest.push(node);
    }

    return forest;

    // info!("Checking for best possible sequence cut...");
    // let (
    //     bs_min_cost,
    //     bs_no_of_cuts,
    //     bs_cut_edges,
    //     bs_no_of_added_edges,
    //     bs_added_edges,
    //     bs_set1,
    //     bs_set2,
    //     bs_new_dfg,
    // ) = best_sequence_cut(&filtered_dfg, &all_activities);
    // info!("\n=== BEST SEQUENCE CUT RESULTS ===");
    // info!("Minimum Cost: {}", bs_min_cost);
    // info!("Number of cut edges: {}", bs_no_of_cuts);
    // info!("Cut Edges: {:?}", bs_cut_edges);
    // info!("Number of added edges: {}", bs_no_of_added_edges);
    // info!("Added Edges: {:?}", bs_added_edges);
    // info!("Set 1: {:?}", bs_set1);
    // info!("Set 2: {:?}", bs_set2);
    // // info!("new dfg: {:?}", bs_new_dfg);
    // let (is_sequence, failures) = sequence_cut_condition_check(&bs_new_dfg, &bs_set1, &bs_set2);
    // if (!is_sequence) {
    //     // I dont think this is possible, but just in case
    //     // because, for 'a' in set1, and 'b' in set2, we would definitely have a forced sequence cut
    //     info!(
    //         "Sequence cut condition failed for sets: {:?} and {:?}",
    //         bs_set1, bs_set2
    //     );
    //     for (a, b, r1, r2) in failures {
    //         info!(
    //             "Condition failure: {} -> {} (reachable: {}, {})",
    //             a, b, r1, r2
    //         );
    //     }
    // }

    // find_cuts_start(&bs_new_dfg, &bs_set1, &start_activities, &end_activities);
    // find_cuts_start(&bs_new_dfg, &bs_set2, &start_activities, &end_activities);
    // return;

    // info!("Checking for best possible sequence cut V2...");
    // let (bsv2_min_cost, bsv2_cut_cost, bsv2_add_cost, bsv2_cutedges, bsv2_added_edges, bsv2_set1, bsv2_set2) = optimal_partition(&filtered_dfg, &all_activities);
    // info!("\n=== BEST SEQUENCE CUT V2 RESULTS ===");
    // info!("Minimum Cost: {}", bsv2_min_cost);
    // info!("Cut Cost: {}", bsv2_cut_cost);
    // info!("Add Cost: {}", bsv2_add_cost);
    // info!("Cut Edges: {:?}", bsv2_cutedges);
    // info!("Added Edges: {:?}", bsv2_added_edges);
    // info!("Set 1: {:?}", bsv2_set1);
    // info!("Set 2: {:?}", bsv2_set2);

    // info!("Checking for best possible exclusive cut...");
    // let (be_min_cost, be_cut_edges, be_set1, be_set2, be_new_dfg) = best_exclusive_cut(&dfg, &all_activities);
    // info!("\n=== BEST EXCLUSIVE CUT RESULTS ===");
    // info!("Minimum Cost: {}", be_min_cost);
    // info!("Cut Edges: {:?}", be_cut_edges);
    // info!("Set 1: {:?}", be_set1);
    // info!("Set 2: {:?}", be_set2);
    // let (is_exclusive, be_failures) = exclusive_cut_condition_check(&be_new_dfg, &be_set1, &be_set2);
    // if(!is_exclusive) {
    //     // I dont think this is possible, but just in case
    //     // because, for 'a' in set1, and 'b' in set2, we would definitely have a forced sequence cut
    //     info!("Exclusive cut condition failed for sets: {:?} and {:?}", be_set1, be_set2);
    //     for (a, b, r1, r2) in be_failures {
    //         info!("Condition failure: {} -> {} (reachable: {}, {})", a, b, r1, r2);
    //     }
    // }

    // find_cuts_start(&be_new_dfg, &be_set1, &start_activities, &end_activities);
    // find_cuts_start(&be_new_dfg, &be_set2, &start_activities, &end_activities);
    // return;

    // info!("Checking for best possible parallel cut...");
    // let result = best_parallel_cut(&dfg, &all_activities);
    // info!("\n=== BEST PARALLEL CUT RESULTS ===");
    // info!("Minimum cost: {}", result.minimum_cost);
    // info!("Number of edges added: {}", result.num_edges_added);
    // info!("Set1: {:?}", result.set1);
    // info!("Set2: {:?}", result.set2);
    // find_cuts_start(&result.new_dfg, &result.set1);
    // find_cuts_start(&result.new_dfg, &result.set2);
    // return;

    // info!("Checking for best possible exhaustive parallel cut...");
    // let result = best_parallel_cut_exhaustive(&dfg, &all_activities);
    // info!("\n=== BEST PARALLEL CUT RESULTS ===");
    // info!("Minimum cost: {}", result.minimum_cost);
    // info!("Number of edges added: {}", result.num_edges_added);
    // info!("Set1: {:?}", result.set1);
    // info!("Set2: {:?}", result.set2);
    // info!("Original edges: {}", dfg.len());
    // info!("New DFG edges: {}", result.new_dfg.len());

    // info!("Checking for best possible parallel cut vs gemini...");
    // let result = best_parallel_cut_v2(&dfg, &all_activities);
    // info!("\n=== BEST PARALLEL CUT RESULTS ===");
    // info!("Minimum cost: {}", result.min_cost);
    // info!("Total Number of Edges to Add: {}", result.num_added_edges);
    // info!("Set1: {:?}", result.set1);
    // info!("Set2: {:?}", result.set2);
    // for edge in result.added_edges {
    //     info!("  - From '{}' to '{}'", edge.0, edge.1);
    // }

    // info!("Checking for best possible redo cut...");
    // let (br_is_redo, br_min_cost, br_no_of_cuts,br_cut_edges, br_set1, br_set2, br_new_dfg) = best_redo_cut(&filtered_dfg, &all_activities, &start_activities, &end_activities);
    // if br_is_redo {
    //     info!("\n=== BEST REDO CUT RESULTS ===");
    //     info!("Minimum Cost: {}", br_min_cost);
    //     info!("Number of cut edges: {}", br_no_of_cuts);
    //     info!("Cut Edges: {:?}", br_cut_edges);
    //     info!("Set 1: {:?}", br_set1);
    //     info!("Set 2: {:?}", br_set2);

    //     // find_cuts_start(&br_new_dfg, &br_set1, &start_activities, &end_activities);
    //     // find_cuts_start(&br_new_dfg, &br_set2, &start_activities, &end_activities);
    //     // return;
    // } else {
    //     info!("No best redo cut found");
    // }
}

pub fn find_best_possible_cuts(
    dfg: &HashMap<(String, String), usize>,
    all_activities: &HashSet<String>,
    start_activities: &HashSet<String>,
    end_activities: &HashSet<String>,
) -> CutSuggestionsList{
    let filtered_dfg = filter_keep_dfg(&dfg, &all_activities);
    let (start_activities, end_activities) =
        get_start_and_end_activities(&dfg, &all_activities, &start_activities, &end_activities);

    info!(
        "Best the best possible cuts for set of activities: {:?}",
        all_activities
    );

    let mut cuts: Vec<CutSuggestion> = Vec::new();

    let mut cost_to_add_edge: usize = 1;

    info!("Checking for best possible sequence cut...");
    let (
        bs_min_cost,
        bs_no_of_cuts,
        bs_cut_edges,
        bs_no_of_added_edges,
        bs_added_edges,
        bs_set1,
        bs_set2,
        bs_new_dfg,
    ) = best_sequence_cut(&filtered_dfg, &all_activities);
    info!("\n=== BEST SEQUENCE CUT RESULTS ===");
    info!("Minimum Cost: {}", bs_min_cost);
    info!("Number of cut edges: {}", bs_no_of_cuts);
    info!("Cut Edges: {:?}", bs_cut_edges);
    info!("Number of added edges: {}", bs_no_of_added_edges);
    info!("Added Edges: {:?}", bs_added_edges);
    info!("Set 1: {:?}", bs_set1);
    info!("Set 2: {:?}", bs_set2);
    // info!("new dfg: {:?}", bs_new_dfg);
    let (is_sequence, failures) = sequence_cut_condition_check(&bs_new_dfg, &bs_set1, &bs_set2);
    if (!is_sequence) {
        // I dont think this is possible, but just in case
        // because, for 'a' in set1, and 'b' in set2, we would definitely have a forced sequence cut
        info!(
            "Sequence cut condition failed for sets: {:?} and {:?}",
            bs_set1, bs_set2
        );
        for (a, b, r1, r2) in failures {
            info!(
                "Condition failure: {} -> {} (reachable: {}, {})",
                a, b, r1, r2
            );
        }
    }
    // Add sequence cut suggestion
    cuts.push(CutSuggestion {
        cut_type: "sequence".to_string(),
        set1: bs_set1,
        set2: bs_set2,
        edges_to_be_added: bs_added_edges,
        edges_to_be_removed: bs_cut_edges,
        cost_to_add_edge: cost_to_add_edge,
        total_cost: bs_min_cost,
    });


    info!("Checking for best possible parallel cut vs gemini...");
    let result = best_parallel_cut_v2(&dfg, &all_activities);
    info!("\n=== BEST PARALLEL CUT RESULTS ===");
    info!("Minimum cost: {}", result.min_cost);
    info!("Total Number of Edges to Add: {}", result.num_added_edges);
    info!("Set1: {:?}", result.set1);
    info!("Set2: {:?}", result.set2);
    // Add parallel cut suggestion
    cuts.push(CutSuggestion {
        cut_type: "parallel".to_string(),
        set1: result.set1,
        set2: result.set2,
        edges_to_be_added: result.added_edges,
        edges_to_be_removed: Vec::new(), 
        cost_to_add_edge: cost_to_add_edge,
        total_cost: result.min_cost,
    });

     // Create the final result structure
    let cut_suggestions_list: CutSuggestionsList = CutSuggestionsList {
        all_activities: all_activities.clone(),
        cuts,
    };

    cut_suggestions_list
    
}

// Exclusive cut and helpers --------------
fn find_exclusive_choice_cut(
    dfg: &HashMap<(String, String), usize>,
    all_activities: &HashSet<String>,
) -> (HashSet<String>, HashSet<String>) {
    // Step 1: Convert to undirected adjacency list
    let mut undirected_graph: HashMap<String, HashSet<String>> = HashMap::new();

    for ((from, to), _) in dfg {
        undirected_graph
            .entry(from.clone())
            .or_default()
            .insert(to.clone());
        undirected_graph
            .entry(to.clone())
            .or_default()
            .insert(from.clone());
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

fn exclusive_cut_condition_check(
    dfg: &HashMap<(String, String), usize>,
    set1: &HashSet<String>,
    set2: &HashSet<String>,
) -> (bool, Vec<(String, String, bool, bool)>) {
    let mut failures = Vec::new();
    for a in set1 {
        for b in set2 {
            let r1 = is_reachable(dfg, a, b);
            let r2 = is_reachable(dfg, b, a);
            if (r1 || r2) {
                failures.push((a.clone(), b.clone(), r1, r2));
            }
        }
    }
    (failures.is_empty(), failures)
}

// ------- Sequence cut and helpers ------------
fn find_sequence_cut(
    dfg: &HashMap<(String, String), usize>,
    all_activities: &HashSet<String>,
) -> (HashSet<String>, HashSet<String>) {
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
    // Create set1 and set2
    let mut set1: HashSet<usize> = HashSet::new();
    let mut set2: HashSet<usize> = HashSet::new();
    for (from, tos) in dag {
        for to in tos {
            set1.insert(*from);
            set2.insert(*to);
        }
    }

    // Find common activities and remove them from both sets
    let intersection: HashSet<_> = set1.intersection(&set2).cloned().collect();
    let mut common_activities = intersection.clone();

    for i in &intersection {
        set1.remove(i);
        set2.remove(i);
    }

    // For each common activity, decide whether to put it in set1 or set2
    for c in common_activities {
        let mut all_can_reach_and_c_cannot_reach_back = true;

        // Check if every activity 't' in set1 can reach 'c', and 'c' cannot reach 't'
        for t in &set1 {
            if !is_reachable_in_dag(dag, *t, c) || is_reachable_in_dag(dag, c, *t) {
                all_can_reach_and_c_cannot_reach_back = false;
                break;
            }
        }

        if all_can_reach_and_c_cannot_reach_back {
            set2.insert(c);
        } else {
            set1.insert(c);
        }
    }

    // Map SCCs to activity sets
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

    (act_set1, act_set2)
}

pub fn is_reachable_in_dag(
    dag: &HashMap<usize, HashSet<usize>>,
    activity1: usize,
    activity2: usize,
) -> bool {
    let mut visited = HashSet::new();
    let mut stack = vec![activity1];

    while let Some(current) = stack.pop() {
        if current == activity2 {
            return true;
        }
        if visited.insert(current) {
            if let Some(neighbors) = dag.get(&current) {
                for &neighbor in neighbors {
                    stack.push(neighbor);
                }
            }
        }
    }
    false
}

fn sequence_cut_condition_check(
    dfg: &HashMap<(String, String), usize>,
    set1: &HashSet<String>,
    set2: &HashSet<String>,
) -> (bool, Vec<(String, String, bool, bool)>) {
    let mut failures = Vec::new();
    for a in set1 {
        for b in set2 {
            let r1 = is_reachable(dfg, a, b);
            let r2 = is_reachable(dfg, b, a);
            if !(r1 && !r2) {
                failures.push((a.clone(), b.clone(), r1, r2));
            }
        }
    }
    (failures.is_empty(), failures)
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

        if is_s1_redo && !is_s2_redo {
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
        .filter(|((from, to), _)| keep_list.contains(from) && keep_list.contains(to))
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
            if !dfg.contains_key(&(m.clone(), n.clone()))
                || !dfg.contains_key(&(n.clone(), m.clone()))
            {
                return false;
            }
        }
    }
    true
}

fn get_start_and_end_activities(
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

pub fn is_reachable_before_end_activity(
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
