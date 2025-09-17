use std::collections::{HashMap, HashSet};

pub fn get_dfg_by_object_type(
    relations: &Vec<(String, String, String, String, String)>,
    divergent_objects: &HashMap<String, Vec<String>>,
) -> HashMap<String, (HashMap<(String, String), usize>, HashSet<String>, HashSet<String>)> {

    // Response Format: HashMap<ObjectType, (DFG, StartActivities, EndActivities)>
    
    let mut dfg_sets: HashMap<String, (HashMap<(String, String), usize>, HashSet<String>, HashSet<String>)> = HashMap::new();

    // Group relations by object type (otype at index 4)
    let mut grouped_by_otype: HashMap<String, Vec<&(String, String, String, String, String)>> = HashMap::new();
    
    for relation in relations {
        grouped_by_otype
            .entry(relation.4.clone()) // otype is at index 4
            .or_insert_with(Vec::new)
            .push(relation);
    }

    // Process each object type
    for (otype, otype_relations) in grouped_by_otype {
        let mut dfg: HashMap<(String, String), usize> = HashMap::new();
        let mut start_activities: HashSet<String> = HashSet::new();
        let mut end_activities: HashSet<String> = HashSet::new();

        // Group relations by object ID (oid) within this object type
        let mut grouped_relations: HashMap<String, Vec<&(String, String, String, String, String)>> = HashMap::new();
        
        for relation in &otype_relations {
            grouped_relations
                .entry(relation.3.clone()) // oid is at index 3
                .or_insert_with(Vec::new)
                .push(*relation);
        }

        // Sort object IDs to ensure deterministic processing order
        let mut sorted_oids: Vec<_> = grouped_relations.keys().cloned().collect();
        sorted_oids.sort();

        // Process each group of relations for the same object ID in sorted order
        for oid in sorted_oids {
            let group = grouped_relations.get(&oid).unwrap();
            
            // Remove duplicates based on eid (keep first occurrence)
            let mut seen_eids: HashSet<String> = HashSet::new();
            let mut unique_relations: Vec<&(String, String, String, String, String)> = Vec::new();
            
            for relation in group {
                if seen_eids.insert(relation.0.clone()) { // eid is at index 0
                    unique_relations.push(relation);
                }
            }

            // Sort by timestamp (index 2)
            unique_relations.sort_by(|a, b| a.2.cmp(&b.2));

            // Skip empty groups or groups with only one event
            if unique_relations.is_empty() {
                continue;
            }

            // Add start activity (first event after sorting)
            let start_activity = &unique_relations[0].1; // activity is at index 1
            start_activities.insert(start_activity.clone());

            // Add end activity (last event after sorting)
            let end_activity = &unique_relations[unique_relations.len() - 1].1;
            end_activities.insert(end_activity.clone());

            // Create directly follows relationships
            for i in 0..unique_relations.len() - 1 {
                let current_relation = unique_relations[i];
                let next_relation = unique_relations[i + 1];
                
                let current_activity = &current_relation.1;
                let next_activity = &next_relation.1;
                let current_otype = &current_relation.4;
                let _next_otype = &next_relation.4;

                // Check divergence condition before adding to DFG
                let should_skip = if let (Some(current_divergent), Some(next_divergent)) = (
                    divergent_objects.get(current_activity),
                    divergent_objects.get(next_activity),
                ) {
                    current_divergent.contains(current_otype) && next_divergent.contains(current_otype)
                } else {
                    false
                };

                // Add to DFG if not divergent
                if !should_skip {
                    let edge = (current_activity.clone(), next_activity.clone());
                    *dfg.entry(edge).or_insert(0) += 1;
                }
            }
        }

        // Store the DFG for this object type
        dfg_sets.insert(otype, (dfg, start_activities, end_activities));
    }

    dfg_sets
}
