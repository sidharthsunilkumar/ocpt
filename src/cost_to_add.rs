use std::collections::{HashMap, HashSet};
use chrono::{DateTime, Utc};


pub fn cost_of_adding_edge(
    relations: &Vec<(String, String, String, String, String)>,
    divergent_objects: &HashMap<String, Vec<String>>,
    dfg: &HashMap<(String, String), usize>
) -> HashMap<(String, String), f64> {

    // relations format: (event_id, event_type, timestamp, object_id, object_type)
    ////// from relations, build a list of traces
    
    println!("Using provided divergent objects:");
    for (activity, object_types) in divergent_objects {
        println!("  {}: {:?}", activity, object_types);
    }

    // print all unique object type for debugging
    let mut object_types = HashSet::new();
    for relation in relations {
        object_types.insert(relation.4.clone()); // object_type is at index 4
    }       
    println!("Unique object types in relations:");
    for obj_type in &object_types { 
        println!("  {}", obj_type); 
    }   

    // Group relations by object ID (oid)
    let mut grouped_relations: HashMap<String, Vec<&(String, String, String, String, String)>> = HashMap::new();
    
    for relation in relations {
        grouped_relations
            .entry(relation.3.clone()) // oid is at index 3
            .or_insert_with(Vec::new)
            .push(relation);
    }

    // Step 1: Create empty array called traces
    let mut traces: Vec<Vec<(String, String, String, String, String)>> = Vec::new();


    // Step 2 & 3: Sort every list inside grouped_relations by timestamp and put in traces
    // (Commented out for testing with hardcoded data)
    
    for (_oid, mut relations_group) in grouped_relations {
        // Sort by timestamp (index 2)
        relations_group.sort_by(|a, b| a.2.cmp(&b.2));
        
        // Convert from references back to owned data and add to traces
        let sorted_trace: Vec<(String, String, String, String, String)> = relations_group
            .into_iter()
            .map(|relation| relation.clone())
            .collect();
        
        // Check if any two event types in the trace have common divergent objects
        let mut should_skip_trace = false;
        let mut trace_object_types: HashSet<String> = HashSet::new();
        let mut trace_event_types: HashSet<String> = HashSet::new();
        
        // Collect all object types and event types in this trace
        for event in &sorted_trace {
            trace_object_types.insert(event.4.clone()); // object_type is at index 4
            trace_event_types.insert(event.1.clone()); // event_type is at index 1
        }
        
        // Check if any two event types in the trace has any common divergent_objects, 
        // and the trace in question is dealing with those common divergent object types
        let event_types_vec: Vec<String> = trace_event_types.into_iter().collect();
        'outer: for i in 0..event_types_vec.len() {
            for j in (i + 1)..event_types_vec.len() {
                let event_type1 = &event_types_vec[i];
                let event_type2 = &event_types_vec[j];
                
                if let (Some(divergent_types1), Some(divergent_types2)) = 
                    (divergent_objects.get(event_type1), divergent_objects.get(event_type2)) {
                    
                    // Find common divergent object types between the two event types
                    let common_divergent_types: HashSet<_> = divergent_types1
                        .iter()
                        .filter(|&dt| divergent_types2.contains(dt))
                        .collect();
                    
                    // Check if any of the common divergent types are present in the trace
                    for common_divergent_type in &common_divergent_types {
                        if trace_object_types.contains(*common_divergent_type) {
                            should_skip_trace = true;
                            break 'outer;
                        }
                    }
                }
            }
        }
        
        // Only push the trace if it doesn't have divergent object types
        if !should_skip_trace {
            traces.push(sorted_trace);
        }
    }
    

    // Step 4: Split traces when duplicate event types are found
    // This handles cases where an object goes through multiple complete process instances
    // Example: If a trace has [create_order, ship_order, complete_order, create_order, ship_order]
    // It gets split into: [create_order, ship_order, complete_order] and [create_order, ship_order]
    let mut split_traces: Vec<Vec<(String, String, String, String, String)>> = Vec::new();
    
    for (_trace_idx, trace) in traces.iter().enumerate() {
        let mut current_trace: Vec<(String, String, String, String, String)> = Vec::new();
        let mut seen_event_types: HashSet<String> = HashSet::new();
        
        for (_event_idx, event) in trace.iter().enumerate() {
            let event_type = &event.1; // event_type is at index 1
            
            // If we've seen this event type before, start a new trace
            if seen_event_types.contains(event_type) {
                // Save the current trace if it's not empty
                if !current_trace.is_empty() {
                    split_traces.push(current_trace.clone());
                }
                
                // Start a new trace with this event
                current_trace.clear();
                seen_event_types.clear();
            }
            
            // Add the event to current trace and mark event type as seen
            current_trace.push(event.clone());
            seen_event_types.insert(event_type.clone());
        }
        
        // Don't forget to add the last trace if it's not empty
        if !current_trace.is_empty() {
            split_traces.push(current_trace);
        }
    }

    // Normalised timestamp traces
    let mut normalized_split_traces: Vec<Vec<(String, String, String, String, String)>> = Vec::new();
    
    for trace in &split_traces {
        if !trace.is_empty() {
            // Get start_time from the first event
            let start_time_str = &trace[0].2;
            
            // Parse the start time (format: 2023-11-16T12:48:30+00:00)
            if let Ok(start_time) = start_time_str.parse::<chrono::DateTime<chrono::Utc>>() {
                // Create normalized trace by subtracting start_time from every timestamp
                let mut normalized_trace = Vec::new();
                for event in trace {
                    if let Ok(event_time) = event.2.parse::<chrono::DateTime<chrono::Utc>>() {
                        // Calculate time difference in seconds from start
                        let duration = event_time.signed_duration_since(start_time);
                        let seconds_from_start = duration.num_seconds();
                        
                        // Create new event with normalized timestamp
                        let normalized_event = (
                            event.0.clone(),  // event_id
                            event.1.clone(),  // event_type
                            seconds_from_start.to_string(), // normalized timestamp as seconds from start
                            event.3.clone(),  // object_id
                            event.4.clone()   // object_type
                        );
                        normalized_trace.push(normalized_event);
                    }
                }
                
                if !normalized_trace.is_empty() {
                    normalized_split_traces.push(normalized_trace);
                }
            } else {
                // If parsing fails, use original trace
                normalized_split_traces.push(trace.clone());
            }
        }
    }
    
    // // keep track of unique object types seen
    // let mut unique_object_types_seen: HashSet<String> = HashSet::new(); 

    // // Now split_traces contains all the split traces
    // println!("\n=== FINAL RESULTS ===");
    // println!("Number of traces after splitting: {}", split_traces.len());
    // println!("\nSplit traces:");
    // for (i, trace) in split_traces.iter().enumerate() {
    //     println!("Split Trace {}:", i);
    //     for (j, event) in trace.iter().enumerate() {
    //         println!("  Event {}: {} , {} , {} , {}, {}", j, event.0, event.1, event.2, event.3, event.4);
    //         unique_object_types_seen.insert(event.4.clone());
    //     }
    //     println!();
    // }
    // println!("Unique object types seen in split traces:");
    // for obj_type in &unique_object_types_seen { 
    //     println!("  {}", obj_type); 
    // }   

    // Get all activities from dfg
    let mut activities = HashSet::new();
    for ((a, b), _) in dfg {
        activities.insert(a.clone());
        activities.insert(b.clone());
    }

    // Create missing_edge_dfg hashmap
    let mut missing_edge_dfg: HashMap<(String, String), f64> = HashMap::new();

    // Find missing edges and add them with cost 9999999
    for a in &activities {
        for b in &activities {
            if a != b && !dfg.contains_key(&(a.clone(), b.clone())) {
                missing_edge_dfg.insert((a.clone(), b.clone()), 9999999.0);
            }
        }
    }

    // Compute average dfg edge cost
    let total_dfg_cost: usize = dfg.values().sum();
    let avg_dfg_cost: f64 = if !dfg.is_empty() {
        total_dfg_cost as f64 / dfg.len() as f64
    } else {
        1.0
    };

    // Precompute: group all events by activity type and sort them by timestamp once
    let mut events_per_activity: HashMap<String, Vec<(String, String, String, String, String)>> = HashMap::new();

    for trace in &normalized_split_traces {
        for event in trace {
            // event.1 is the activity label
            events_per_activity.entry(event.1.clone())
                .or_default()
                .push(event.clone());
        }
    }

    // Sort each activity’s events by timestamp (event.2)
    for events in events_per_activity.values_mut() {
        events.sort_by(|event1, event2| {
            let timestamp1: f64 = event1.2.parse().unwrap_or(0.0);
            let timestamp2: f64 = event2.2.parse().unwrap_or(0.0);
            timestamp1
                .partial_cmp(&timestamp2)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    // Pre-parse timestamps to avoid repeated parsing in inner loops
    let mut events_with_timestamps: HashMap<String, Vec<f64>> = HashMap::new();
    for (activity, events) in &events_per_activity {
        let timestamps: Vec<f64> = events.iter()
            .map(|event| event.2.parse().unwrap_or(0.0))
            .collect();
        events_with_timestamps.insert(activity.clone(), timestamps);
    }

    // Process each missing edge to calculate probability scores
    let mut edge_updates: Vec<((String, String), f64)> = Vec::new(); // Store updates to apply later
    
    // Pre-calculate constants to avoid repeated computation
    let max_cost = avg_dfg_cost * 2.0;
    let min_cost = 1.0;
    let cost_range = max_cost - min_cost;
    
    for (a, b) in missing_edge_dfg.keys() {
        let a_timestamps = events_with_timestamps.get(a);
        let b_timestamps = events_with_timestamps.get(b);

        let mut sum = 0.0;

        // Skip calculation if either event type has no occurrences
        if let (Some(a_times), Some(b_times)) = (a_timestamps, b_timestamps) {
            if !a_times.is_empty() && !b_times.is_empty() {
                let a_events_len = a_times.len();
                let b_events_len = b_times.len();
                let p1 = 1.0 / a_events_len as f64; // Calculate once outside loop

                for &a_timestamp in a_times {
                    // Use binary search to find first b event after a_timestamp
                    // Since b_times is already sorted, this is O(log n) instead of O(n)
                    let bn = b_times.partition_point(|&b_timestamp| b_timestamp <= a_timestamp);
                    
                    // Probability that 'b' happens after this 'a' event
                    let p2 = (b_events_len - bn) as f64 / b_events_len as f64;

                    // Add to the total probability sum
                    sum += p1 * p2;
                }
            }
        }

        println!("Missing edge {} -> {}, score: {}", a, b, sum);

        // Update missing_edge_dfg with the calculated score
        if sum > 0.0 {
            // If sum is above 1, clamp it to 1
            let clamped_sum = sum.min(1.0);
            
            // Convert sum to new value using inverse range
            // sum = 0 -> new_value = max_cost (avg_dfg_cost * 2)
            // sum = 1 -> new_value = min_cost (1)
            // Formula: new_value = max_cost - (clamped_sum * (max_cost - min_cost))
            let new_value = max_cost - (clamped_sum * cost_range);
            
            // Store the update to apply later (avoid cloning strings)
            edge_updates.push(((a.clone(), b.clone()), new_value));
        }
    }
    
    // Apply all the updates to missing_edge_dfg
    for ((a, b), new_cost) in edge_updates {
        if let Some(edge_cost) = missing_edge_dfg.get_mut(&(a, b)) {
            *edge_cost = new_cost;
        }
    }

    // Print missing edges with their final costs
    println!("\n=== FINAL MISSING EDGE COSTS ===");
    for (edge, cost) in &missing_edge_dfg {
        println!("Missing Edge: {:?} -> {:?}, Cost: {:.2}", edge.0, edge.1, cost);
    }

    // Return the missing edge dfg
    missing_edge_dfg
}
