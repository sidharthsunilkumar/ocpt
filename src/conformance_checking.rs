use std::collections::{HashMap, HashSet, VecDeque};
use serde_json;

// Import your types
use crate::types::{OCEL, OCELEvent, TreeNode, ProcessForest};

/// Extract traces from OCEL log (flattened event sequences per object)
fn extract_traces(ocel: &OCEL) -> Vec<Vec<String>> {
    let mut object_traces: HashMap<String, Vec<(String, chrono::DateTime<chrono::FixedOffset>)>> = HashMap::new();
    
    // Group events by objects
    for event in &ocel.events {
        let activity = event.event_type.clone();
        let timestamp = event.time;
        
        for relationship in &event.relationships {
            let obj_id = &relationship.object_id;
            object_traces
                .entry(obj_id.clone())
                .or_insert_with(Vec::new)
                .push((activity.clone(), timestamp));
        }
    }
    
    // Sort by timestamp and extract activity sequences
    let mut traces = Vec::new();
    for (_obj_id, mut events) in object_traces {
        events.sort_by_key(|e| e.1);
        let trace: Vec<String> = events.into_iter().map(|e| e.0).collect();
        if !trace.is_empty() {
            traces.push(trace);
        }
    }
    
    traces
}

/// Extract all activities from the process tree
fn extract_activities_from_tree(tree: &TreeNode) -> HashSet<String> {
    let mut activities = HashSet::new();
    
    fn traverse(node: &TreeNode, activities: &mut HashSet<String>) {
        if node.children.is_empty() {
            // Leaf node - it's an activity
            if !["sequence", "parallel", "exclusive", "loop"].contains(&node.label.as_str()) {
                activities.insert(node.label.clone());
            }
        } else {
            // Internal node - traverse children
            for child in &node.children {
                traverse(child, activities);
            }
        }
    }
    
    traverse(tree, &mut activities);
    activities
}

/// Check if a trace can be replayed on the process tree
fn can_replay_trace(trace: &[String], tree: &TreeNode) -> (bool, usize, usize) {
    // Returns (is_fit, consumed_tokens, missing_tokens, remaining_tokens)
    // This is a simplified implementation
    
    let (success, consumed, missing) = replay_on_tree(trace, tree, 0);
    (success && consumed == trace.len(), consumed, missing)
}

/// Recursive replay on process tree
fn replay_on_tree(trace: &[String], node: &TreeNode, start_idx: usize) -> (bool, usize, usize) {
    if start_idx >= trace.len() {
        return (true, 0, 0);
    }
    
    match node.label.as_str() {
        "sequence" => {
            let mut idx = start_idx;
            let mut total_consumed = 0;
            let mut total_missing = 0;
            
            for child in &node.children {
                let (success, consumed, missing) = replay_on_tree(trace, child, idx);
                total_consumed += consumed;
                total_missing += missing;
                idx += consumed;
                
                if !success {
                    return (false, total_consumed, total_missing);
                }
            }
            (true, total_consumed, total_missing)
        }
        "parallel" => {
            // Simplified: try to match all children in any order
            let mut remaining_trace: Vec<String> = trace[start_idx..].to_vec();
            let mut total_consumed = 0;
            let mut total_missing = 0;
            
            for child in &node.children {
                let (_, consumed, missing) = replay_on_tree(&remaining_trace, child, 0);
                total_consumed += consumed;
                total_missing += missing;
                
                // Remove consumed activities
                for _ in 0..consumed {
                    if !remaining_trace.is_empty() {
                        remaining_trace.remove(0);
                    }
                }
            }
            (true, total_consumed, total_missing)
        }
        "exclusive" => {
            // Try each child branch
            for child in &node.children {
                let (success, consumed, missing) = replay_on_tree(trace, child, start_idx);
                if success && consumed > 0 {
                    return (true, consumed, missing);
                }
            }
            (false, 0, 1)
        }
        "loop" => {
            // Simplified loop handling
            if node.children.is_empty() {
                return (true, 0, 0);
            }
            
            let mut idx = start_idx;
            let mut total_consumed = 0;
            let mut total_missing = 0;
            
            // Try to execute the loop body at least once
            loop {
                let (success, consumed, missing) = replay_on_tree(trace, &node.children[0], idx);
                total_consumed += consumed;
                total_missing += missing;
                idx += consumed;
                
                if !success || consumed == 0 || idx >= trace.len() {
                    break;
                }
            }
            
            (true, total_consumed, total_missing)
        }
        _ => {
            // Leaf node - activity
            if start_idx < trace.len() && trace[start_idx] == node.label {
                (true, 1, 0)
            } else {
                (false, 0, 1)
            }
        }
    }
}

/// Calculate fitness using token-based replay
pub fn calculate_fitness(ocel: &OCEL, process_tree: &ProcessForest) -> f64 {
    if process_tree.is_empty() {
        return 0.0;
    }
    
    let tree = &process_tree[0]; // Assuming single root
    let traces = extract_traces(ocel);
    
    if traces.is_empty() {
        return 0.0;
    }
    
    let mut total_fit = 0;
    let mut total_traces = traces.len();
    
    for trace in &traces {
        let (is_fit, consumed, missing) = can_replay_trace(trace, tree);
        
        if is_fit {
            total_fit += 1;
        } else {
            // Partial fitness based on how much was consumed
            let trace_fitness = consumed as f64 / (consumed + missing).max(1) as f64;
            total_fit += trace_fitness as usize;
        }
    }
    
    total_fit as f64 / total_traces as f64
}

/// Calculate precision (how much behavior in model is observed in log)
pub fn calculate_precision(ocel: &OCEL, process_tree: &ProcessForest) -> f64 {
    if process_tree.is_empty() {
        return 0.0;
    }
    
    let tree = &process_tree[0];
    let traces = extract_traces(ocel);
    
    if traces.is_empty() {
        return 0.0;
    }
    
    // Get all activities from model
    let model_activities = extract_activities_from_tree(tree);
    
    // Get all activities from log
    let mut log_activities = HashSet::new();
    for trace in &traces {
        for activity in trace {
            log_activities.insert(activity.clone());
        }
    }
    
    if model_activities.is_empty() {
        return 0.0;
    }
    
    // Simple precision: ratio of log activities that are in the model
    let overlap = log_activities.intersection(&model_activities).count();
    let precision = overlap as f64 / model_activities.len() as f64;
    
    precision.min(1.0)
}

/// Calculate both metrics and return detailed results
pub fn calculate_conformance_metrics(ocel: &OCEL, process_tree: &ProcessForest) -> ConformanceMetrics {
    let fitness = calculate_fitness(ocel, process_tree);
    let precision = calculate_precision(ocel, process_tree);    
    let traces = extract_traces(ocel);
    
    let model_activities = if !process_tree.is_empty() {
        extract_activities_from_tree(&process_tree[0])
    } else {
        HashSet::new()
    };
    
    let mut log_activities = HashSet::new();
    for trace in &traces {
        for activity in trace {
            log_activities.insert(activity.clone());
        }
    }
    
    ConformanceMetrics {
        fitness,
        precision,
        num_traces: traces.len(),
        num_events: ocel.events.len(),
        model_activities: model_activities.len(),
        log_activities: log_activities.len(),
    }
}

#[derive(Debug, Clone)]
pub struct ConformanceMetrics {
    pub fitness: f64,
    pub precision: f64,
    pub num_traces: usize,
    pub num_events: usize,
    pub model_activities: usize,
    pub log_activities: usize,
}

impl std::fmt::Display for ConformanceMetrics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "=== Conformance Checking Results ===\n\
             Fitness:    {:.4} ({:.2}%)\n\
             Precision:  {:.4} ({:.2}%)\n\
             \n\
             Log Statistics:\n\
             - Number of traces: {}\n\
             - Number of events: {}\n\
             - Log activities: {}\n\
             - Model activities: {}\n\
             ===================================",
            self.fitness,
            self.fitness * 100.0,
            self.precision,
            self.precision * 100.0,
            self.num_traces,
            self.num_events,
            self.log_activities,
            self.model_activities
        )
    }
}