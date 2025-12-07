use std::collections::{HashMap, HashSet};
use chrono::{DateTime, Utc};
use plotters::prelude::*;


pub fn cost_of_adding_edge_1(
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

    //Temporary code to delete later
    return missing_edge_dfg;

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

    // Pre-compute all timestamps to avoid repeated parsing
    let mut activity_timestamps: HashMap<String, Vec<f64>> = HashMap::new();
    for (activity, events) in &events_per_activity {
        let timestamps: Vec<f64> = events.iter()
            .map(|event| event.2.parse().unwrap_or(0.0))
            .collect();
        activity_timestamps.insert(activity.clone(), timestamps);
    }

    for (a, b) in missing_edge_dfg.keys() {
        let empty_vec: Vec<(String, String, String, String, String)> = Vec::new();
        let a_events = events_per_activity.get(a).unwrap_or(&empty_vec);
        let b_events = events_per_activity.get(b).unwrap_or(&empty_vec);

        let a_events_len = a_events.len();
        let b_events_len = b_events.len();
        let mut sum = 0.0;

        // Skip calculation if either event type has no occurrences
        if a_events_len > 0 && b_events_len > 0 {
            let mut an = 0;

            while an < a_events_len {
                // Probability that 'a' happens at this specific time
                let p1 = 1.0 / a_events_len as f64;

                // Get the timestamp of this 'a' event
                let a_timestamp: f64 = a_events[an].2.parse().unwrap_or(0.0);

                // Find the first 'b' event that occurs after this 'a' event
                let mut bn = 0;
                while bn < b_events_len {
                    let b_timestamp: f64 = b_events[bn].2.parse().unwrap_or(0.0);
                    if b_timestamp > a_timestamp {
                        break;
                    }
                    bn += 1;
                }

                // Probability that 'b' happens after this 'a' event
                let p2 = (b_events_len - bn) as f64 / b_events_len as f64;

                // Add to the total probability sum
                sum += p1 * p2;
                an += 1;
            }
        }

        // Calculate probability of finding both activities a and b in traces
        let activity_names = vec![a.clone(), b.clone()];
        let probability_a_b_in_traces = are_activities_in_trace(&activity_names, &normalized_split_traces);
        sum *= probability_a_b_in_traces;
        let total_traces = normalized_split_traces.len();
        sum=1.0-sum;
        sum = sum.powf(total_traces as f64);
        sum = 1.0-sum;

        println!("Missing edge {} -> {}, score: {:e}", a, b, sum);

        // Update missing_edge_dfg with the calculated score
        if sum > 0.0 {
            // Create range from 1 to avg_dfg_cost * 2
            let max_cost = avg_dfg_cost * 2.0;
            let min_cost = 1.0;
            
            // If sum is above 1, clamp it to 1
            let clamped_sum = if sum > 1.0 { 1.0 } else { sum };
            
            // Convert sum to new value using inverse range
            // sum = 0 -> new_value = max_cost (avg_dfg_cost * 2)
            // sum = 1 -> new_value = min_cost (1)
            // Formula: new_value = max_cost - (clamped_sum * (max_cost - min_cost))
            let new_value = max_cost - (clamped_sum * (max_cost - min_cost));
            
            // Store the update to apply later
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




// // seperated by object type
// pub fn cost_of_adding_edge(
//     relations: &Vec<(String, String, String, String, String)>,
//     divergent_objects: &HashMap<String, Vec<String>>,
//     dfg: &HashMap<(String, String), usize>
// ) -> HashMap<(String, String), f64> {

//     // relations format: (event_id, event_type, timestamp, object_id, object_type)
//     ////// from relations, build a list of traces
    
//     println!("Using provided divergent objects:");
//     for (activity, object_types) in divergent_objects {
//         println!("  {}: {:?}", activity, object_types);
//     }

//     // print all unique object type for debugging
//     let mut object_types = HashSet::new();
//     for relation in relations {
//         object_types.insert(relation.4.clone()); // object_type is at index 4
//     }       
//     println!("Unique object types in relations:");
//     for obj_type in &object_types { 
//         println!("  {}", obj_type); 
//     }   

//     // Group relations by object ID (oid)
//     let mut grouped_relations: HashMap<String, Vec<&(String, String, String, String, String)>> = HashMap::new();
    
//     for relation in relations {
//         grouped_relations
//             .entry(relation.3.clone()) // oid is at index 3
//             .or_insert_with(Vec::new)
//             .push(relation);
//     }

//     // Step 1: Create empty array called traces
//     let mut traces: Vec<Vec<(String, String, String, String, String)>> = Vec::new();


//     // Step 2 & 3: Sort every list inside grouped_relations by timestamp and put in traces
//     // (Commented out for testing with hardcoded data)
    
//     for (_oid, mut relations_group) in grouped_relations {
//         // Sort by timestamp (index 2)
//         relations_group.sort_by(|a, b| a.2.cmp(&b.2));
        
//         // Convert from references back to owned data and add to traces
//         let sorted_trace: Vec<(String, String, String, String, String)> = relations_group
//             .into_iter()
//             .map(|relation| relation.clone())
//             .collect();
        
//         // Check if any two event types in the trace have common divergent objects
//         let mut should_skip_trace = false;
//         let mut trace_object_types: HashSet<String> = HashSet::new();
//         let mut trace_event_types: HashSet<String> = HashSet::new();
        
//         // Collect all object types and event types in this trace
//         for event in &sorted_trace {
//             trace_object_types.insert(event.4.clone()); // object_type is at index 4
//             trace_event_types.insert(event.1.clone()); // event_type is at index 1
//         }
        
//         // Check if any two event types in the trace has any common divergent_objects, 
//         // and the trace in question is dealing with those common divergent object types
//         let event_types_vec: Vec<String> = trace_event_types.into_iter().collect();
//         'outer: for i in 0..event_types_vec.len() {
//             for j in (i + 1)..event_types_vec.len() {
//                 let event_type1 = &event_types_vec[i];
//                 let event_type2 = &event_types_vec[j];
                
//                 if let (Some(divergent_types1), Some(divergent_types2)) = 
//                     (divergent_objects.get(event_type1), divergent_objects.get(event_type2)) {
                    
//                     // Find common divergent object types between the two event types
//                     let common_divergent_types: HashSet<_> = divergent_types1
//                         .iter()
//                         .filter(|&dt| divergent_types2.contains(dt))
//                         .collect();
                    
//                     // Check if any of the common divergent types are present in the trace
//                     for common_divergent_type in &common_divergent_types {
//                         if trace_object_types.contains(*common_divergent_type) {
//                             should_skip_trace = true;
//                             break 'outer;
//                         }
//                     }
//                 }
//             }
//         }
        
//         // Only push the trace if it doesn't have divergent object types
//         if !should_skip_trace {
//             traces.push(sorted_trace);
//         }
//     }
    

//     // Step 4: Split traces when duplicate event types are found
//     // This handles cases where an object goes through multiple complete process instances
//     // Example: If a trace has [create_order, ship_order, complete_order, create_order, ship_order]
//     // It gets split into: [create_order, ship_order, complete_order] and [create_order, ship_order]
//     let mut split_traces: Vec<Vec<(String, String, String, String, String)>> = Vec::new();
    
//     for (_trace_idx, trace) in traces.iter().enumerate() {
//         let mut current_trace: Vec<(String, String, String, String, String)> = Vec::new();
//         let mut seen_event_types: HashSet<String> = HashSet::new();
        
//         for (_event_idx, event) in trace.iter().enumerate() {
//             let event_type = &event.1; // event_type is at index 1
            
//             // If we've seen this event type before, start a new trace
//             if seen_event_types.contains(event_type) {
//                 // Save the current trace if it's not empty
//                 if !current_trace.is_empty() {
//                     split_traces.push(current_trace.clone());
//                 }
                
//                 // Start a new trace with this event
//                 current_trace.clear();
//                 seen_event_types.clear();
//             }
            
//             // Add the event to current trace and mark event type as seen
//             current_trace.push(event.clone());
//             seen_event_types.insert(event_type.clone());
//         }
        
//         // Don't forget to add the last trace if it's not empty
//         if !current_trace.is_empty() {
//             split_traces.push(current_trace);
//         }
//     }

//     // Normalised timestamp traces
//     let mut normalized_split_traces: Vec<Vec<(String, String, String, String, String)>> = Vec::new();
    
//     for trace in &split_traces {
//         if !trace.is_empty() {
//             // Get start_time from the first event
//             let start_time_str = &trace[0].2;
            
//             // Parse the start time (format: 2023-11-16T12:48:30+00:00)
//             if let Ok(start_time) = start_time_str.parse::<chrono::DateTime<chrono::Utc>>() {
//                 // Create normalized trace by subtracting start_time from every timestamp
//                 let mut normalized_trace = Vec::new();
//                 for event in trace {
//                     if let Ok(event_time) = event.2.parse::<chrono::DateTime<chrono::Utc>>() {
//                         // Calculate time difference in seconds from start
//                         let duration = event_time.signed_duration_since(start_time);
//                         let seconds_from_start = duration.num_seconds();
                        
//                         // Create new event with normalized timestamp
//                         let normalized_event = (
//                             event.0.clone(),  // event_id
//                             event.1.clone(),  // event_type
//                             seconds_from_start.to_string(), // normalized timestamp as seconds from start
//                             event.3.clone(),  // object_id
//                             event.4.clone()   // object_type
//                         );
//                         normalized_trace.push(normalized_event);
//                     }
//                 }
                
//                 if !normalized_trace.is_empty() {
//                     normalized_split_traces.push(normalized_trace);
//                 }
//             } else {
//                 // If parsing fails, use original trace
//                 normalized_split_traces.push(trace.clone());
//             }
//         }
//     }
    
//     // // keep track of unique object types seen
//     // let mut unique_object_types_seen: HashSet<String> = HashSet::new(); 

//     // // Now split_traces contains all the split traces
//     // println!("\n=== FINAL RESULTS ===");
//     // println!("Number of traces after splitting: {}", split_traces.len());
//     // println!("\nSplit traces:");
//     // for (i, trace) in split_traces.iter().enumerate() {
//     //     println!("Split Trace {}:", i);
//     //     for (j, event) in trace.iter().enumerate() {
//     //         println!("  Event {}: {} , {} , {} , {}, {}", j, event.0, event.1, event.2, event.3, event.4);
//     //         unique_object_types_seen.insert(event.4.clone());
//     //     }
//     //     println!();
//     // }
//     // println!("Unique object types seen in split traces:");
//     // for obj_type in &unique_object_types_seen { 
//     //     println!("  {}", obj_type); 
//     // }   

//     // Get all activities from dfg
//     let mut activities = HashSet::new();
//     for ((a, b), _) in dfg {
//         activities.insert(a.clone());
//         activities.insert(b.clone());
//     }

//     // Create missing_edge_dfg hashmap
//     let mut missing_edge_dfg: HashMap<(String, String), f64> = HashMap::new();

//     // Find missing edges and add them with cost 9999999
//     for a in &activities {
//         for b in &activities {
//             if a != b && !dfg.contains_key(&(a.clone(), b.clone())) {
//                 missing_edge_dfg.insert((a.clone(), b.clone()), 9999999.0);
//             }
//         }
//     }

//     // Compute average dfg edge cost
//     let total_dfg_cost: usize = dfg.values().sum();
//     let avg_dfg_cost: f64 = if !dfg.is_empty() {
//         total_dfg_cost as f64 / dfg.len() as f64
//     } else {
//         1.0
//     };

//     // Precompute: group all events by activity type and sort them by timestamp once
//     let mut events_per_activity: HashMap<String, Vec<(String, String, String, String, String)>> = HashMap::new();

//     for trace in &normalized_split_traces {
//         for event in trace {
//             // event.1 is the activity label
//             events_per_activity.entry(event.1.clone())
//                 .or_default()
//                 .push(event.clone());
//         }
//     }

//     // Sort each activity’s events by timestamp (event.2)
//     for events in events_per_activity.values_mut() {
//         events.sort_by(|event1, event2| {
//             let timestamp1: f64 = event1.2.parse().unwrap_or(0.0);
//             let timestamp2: f64 = event2.2.parse().unwrap_or(0.0);
//             timestamp1
//                 .partial_cmp(&timestamp2)
//                 .unwrap_or(std::cmp::Ordering::Equal)
//         });
//     }

//     // Pre-parse timestamps to avoid repeated parsing in inner loops
//     let mut events_with_timestamps: HashMap<String, Vec<f64>> = HashMap::new();
//     for (activity, events) in &events_per_activity {
//         let timestamps: Vec<f64> = events.iter()
//             .map(|event| event.2.parse().unwrap_or(0.0))
//             .collect();
//         events_with_timestamps.insert(activity.clone(), timestamps);
//     }

//     // Process each missing edge to calculate probability scores
//     let mut edge_updates: Vec<((String, String), f64)> = Vec::new(); // Store updates to apply later
    
//     // Pre-calculate constants to avoid repeated computation
//     let max_cost = avg_dfg_cost * 2.0;
//     let min_cost = 1.0;
//     let cost_range = max_cost - min_cost;

//     // Get all unique object types from normalized traces
//     let mut all_object_types = HashSet::new();
//     for trace in &normalized_split_traces {
//         for event in trace {
//             all_object_types.insert(event.4.clone()); // event.4 is the object_type
//         }
//     }

//     // Split normalized_split_traces by object type - keep complete traces that belong to each object type
//     let mut traces_by_object_type: HashMap<String, Vec<Vec<(String, String, String, String, String)>>> = HashMap::new();
//     for object_type in &all_object_types {
//         let mut filtered_traces = Vec::new();
//         for trace in &normalized_split_traces {
//             // Check if ALL events in this trace belong to the same object type
//             let trace_object_types: HashSet<String> = trace.iter().map(|event| event.4.clone()).collect();
            
//             // Only include this trace if it exclusively contains events of the current object type
//             if trace_object_types.len() == 1 && trace_object_types.contains(object_type) {
//                 filtered_traces.push(trace.clone());
//             }
//         }
//         traces_by_object_type.insert(object_type.clone(), filtered_traces.clone());
        
//         // Debug print to show which traces belong to each object type
//         println!("Object type '{}' has {} complete traces", object_type, filtered_traces.len());
//     }

//     // Build events_per_activity_per_object_type: HashMap<object_type, HashMap<activity, Vec<events>>>
//     let mut events_per_activity_per_object_type: HashMap<String, HashMap<String, Vec<(String, String, String, String, String)>>> = HashMap::new();

//     for (object_type, traces) in &traces_by_object_type {
//         let mut events_per_activity_local: HashMap<String, Vec<(String, String, String, String, String)>> = HashMap::new();
        
//         for trace in traces {
//             for event in trace {
//                 // event.1 is the activity label
//                 events_per_activity_local.entry(event.1.clone())
//                     .or_default()
//                     .push(event.clone());
//             }
//         }

//         // Sort each activity's events by timestamp (event.2)
//         for events in events_per_activity_local.values_mut() {
//             events.sort_by(|event1, event2| {
//                 let timestamp1: f64 = event1.2.parse().unwrap_or(0.0);
//                 let timestamp2: f64 = event2.2.parse().unwrap_or(0.0);
//                 timestamp1
//                     .partial_cmp(&timestamp2)
//                     .unwrap_or(std::cmp::Ordering::Equal)
//             });
//         }

//         events_per_activity_per_object_type.insert(object_type.clone(), events_per_activity_local);
//     }

//     // Pre-compute all timestamps to avoid repeated parsing
//     let mut activity_timestamps: HashMap<String, Vec<f64>> = HashMap::new();
//     for (activity, events) in &events_per_activity {
//         let timestamps: Vec<f64> = events.iter()
//             .map(|event| event.2.parse().unwrap_or(0.0))
//             .collect();
//         activity_timestamps.insert(activity.clone(), timestamps);
//     }

//     for (a, b) in missing_edge_dfg.keys() {
//         let mut max_sum = 0.0;

//         // Calculate score for each object type separately
//         for object_type in &all_object_types {
//             let empty_map: HashMap<String, Vec<(String, String, String, String, String)>> = HashMap::new();
//             let events_per_activity_for_type = events_per_activity_per_object_type.get(object_type).unwrap_or(&empty_map);
            
//             let empty_vec: Vec<(String, String, String, String, String)> = Vec::new();
//             let a_events = events_per_activity_for_type.get(a).unwrap_or(&empty_vec);
//             let b_events = events_per_activity_for_type.get(b).unwrap_or(&empty_vec);

//             let a_events_len = a_events.len();
//             let b_events_len = b_events.len();
//             let mut sum = 0.0;

//             // Skip calculation if either event type has no occurrences for this object type
//             if a_events_len > 0 && b_events_len > 0 {
//                 let mut an = 0;

//                 while an < a_events_len {
//                     // Probability that 'a' happens at this specific time
//                     let p1 = 1.0 / a_events_len as f64;

//                     // Get the timestamp of this 'a' event
//                     let a_timestamp: f64 = a_events[an].2.parse().unwrap_or(0.0);

//                     // Find the first 'b' event that occurs after this 'a' event
//                     let mut bn = 0;
//                     while bn < b_events_len {
//                         let b_timestamp: f64 = b_events[bn].2.parse().unwrap_or(0.0);
//                         if b_timestamp > a_timestamp {
//                             break;
//                         }
//                         bn += 1;
//                     }

//                     // Probability that 'b' happens after this 'a' event
//                     let p2 = (b_events_len - bn) as f64 / b_events_len as f64;

//                     // Add to the total probability sum
//                     sum += p1 * p2;
//                     an += 1;
//                 }
//             }

//             // Calculate probability of finding both activities a and b in traces for this object type
//             let empty_traces_vec = Vec::new();
//             let traces_for_object_type = traces_by_object_type.get(object_type).unwrap_or(&empty_traces_vec);
//             let activity_names = vec![a.clone(), b.clone()];
//             let probability_a_b_in_traces = are_activities_in_trace(&activity_names, traces_for_object_type);
//             sum *= probability_a_b_in_traces;
//             // if sum==0.0{
//             //     sum=0.0000001;
//             // }
//             let total_traces_for_type = traces_for_object_type.len();
//             sum = 1.0 - sum;
//             sum = sum.powf(total_traces_for_type as f64);
//             sum = 1.0 - sum;

//             // print top 1 of a_events and b_events
//             if let (Some(a_event), Some(b_event)) = (a_events.get(0), b_events.get(0)) {
//                 println!("Top 1 of {}: {:?}", a, a_event);
//                 println!("Top 1 of {}: {:?}", b, b_event);
//             } else {
//                 println!("size of {}: {}, size of {}: {}", a, a_events.len(), b, b_events.len());
//             }

//             println!("Missing edge {} -> {}, score: {}, Object type: {}, Total traces: {}", a, b, sum, object_type, total_traces_for_type);

//             // Keep track of the maximum score
//             if sum > max_sum {
//                 max_sum = sum;
//             }
//         }

//         // Use the highest score across all object types
//         let final_sum = max_sum;

//         println!("Missing edge {} -> {}, highest score: {:}", a, b, final_sum);

//         // ===== COMPARISON: Calculate with original algorithm using all normalized_split_traces =====
//         let empty_vec: Vec<(String, String, String, String, String)> = Vec::new();
//         let a_events_original = events_per_activity.get(a).unwrap_or(&empty_vec);
//         let b_events_original = events_per_activity.get(b).unwrap_or(&empty_vec);

//         let a_events_len_original = a_events_original.len();
//         let b_events_len_original = b_events_original.len();
//         let mut sum_original = 0.0;

//         // Skip calculation if either event type has no occurrences
//         if a_events_len_original > 0 && b_events_len_original > 0 {
//             let mut an = 0;

//             while an < a_events_len_original {
//                 // Probability that 'a' happens at this specific time
//                 let p1 = 1.0 / a_events_len_original as f64;

//                 // Get the timestamp of this 'a' event
//                 let a_timestamp: f64 = a_events_original[an].2.parse().unwrap_or(0.0);

//                 // Find the first 'b' event that occurs after this 'a' event
//                 let mut bn = 0;
//                 while bn < b_events_len_original {
//                     let b_timestamp: f64 = b_events_original[bn].2.parse().unwrap_or(0.0);
//                     if b_timestamp > a_timestamp {
//                         break;
//                     }
//                     bn += 1;
//                 }

//                 // Probability that 'b' happens after this 'a' event
//                 let p2 = (b_events_len_original - bn) as f64 / b_events_len_original as f64;

//                 // Add to the total probability sum
//                 sum_original += p1 * p2;
//                 an += 1;
//             }
//         }

//         // Calculate probability of finding both activities a and b in traces (original method)
//         let activity_names_original = vec![a.clone(), b.clone()];
//         let probability_a_b_in_traces_original = are_activities_in_trace(&activity_names_original, &normalized_split_traces);
//         sum_original *= probability_a_b_in_traces_original;
//         // if sum_original == 0.0 {
//         //     sum_original = 0.0000001;
//         // }
//         let total_traces_original = normalized_split_traces.len();
//         if total_traces_original > 0 {
//             sum_original = 1.0 - sum_original;
//             sum_original = sum_original.powf(total_traces_original as f64);
//         }
//         sum_original = 1.0 - sum_original;

//         println!("Missing edge {} -> {}, ORIGINAL score: {}, Total traces: {}", a, b, sum_original, total_traces_original);
//         println!("--- Comparison: Object-type highest: {} vs Original: {} ---", final_sum, sum_original);

//         // Update missing_edge_dfg with the calculated score
//         if final_sum > 0.0 {
//             // Create range from 1 to avg_dfg_cost * 2
//             let max_cost = avg_dfg_cost * 2.0;
//             let min_cost = 1.0;
            
//             // If sum is above 1, clamp it to 1
//             let clamped_sum = if final_sum > 1.0 { 1.0 } else { final_sum };
            
//             // Convert sum to new value using inverse range
//             // sum = 0 -> new_value = max_cost (avg_dfg_cost * 2)
//             // sum = 1 -> new_value = min_cost (1)
//             // Formula: new_value = max_cost - (clamped_sum * (max_cost - min_cost))
//             let new_value = max_cost - (clamped_sum * (max_cost - min_cost));
            
//             // Store the update to apply later
//             edge_updates.push(((a.clone(), b.clone()), new_value));
//         }
//     }




//     // Apply all the updates to missing_edge_dfg
//     for ((a, b), new_cost) in edge_updates {
//         if let Some(edge_cost) = missing_edge_dfg.get_mut(&(a, b)) {
//             *edge_cost = new_cost;
//         }
//     }    

//     // Print missing edges with their final costs
//     println!("\n=== FINAL MISSING EDGE COSTS ===");
//     for (edge, cost) in &missing_edge_dfg {
//         println!("Missing Edge: {:?} -> {:?}, Cost: {:.2}", edge.0, edge.1, cost);
//     }

//     // Return the missing edge dfg
//     missing_edge_dfg
// }






/// Binary search function to find index based on third item of tuple
/// 
/// # Arguments
/// * `x` - The number to search for
/// * `a_list` - Vector of tuples sorted by the third item (timestamp as string representing a number)
/// * `arg` - Either "upper" or "lower" to specify search direction
/// 
/// # Returns
/// * If arg is "lower": index of tuple where third item is just lower than x, or -2 if no item is lower
/// * If arg is "upper": index of tuple where third item is just higher than x, or -1 if no item is higher
pub fn binary_search(
    x: f64, 
    a_list: &Vec<(String, String, String, String, String)>, 
    arg: &str
) -> i32 {
    if a_list.is_empty() {
        return if arg == "lower" { -2 } else { -1 };
    }
    
    let mut left = 0;
    let mut right = a_list.len();
    let mut result_index = if arg == "lower" { -2 } else { -1 };
    
    while left < right {
        let mid = left + (right - left) / 2;
        
        // Parse the third item (timestamp) as a number
        let mid_value = match a_list[mid].2.parse::<f64>() {
            Ok(val) => val,
            Err(_) => {
                // If parsing fails, skip this element
                if arg == "lower" {
                    right = mid;
                } else {
                    left = mid + 1;
                }
                continue;
            }
        };
        
        if arg == "lower" {
            // Looking for the largest value that is smaller than x
            if mid_value < x {
                result_index = mid as i32;
                left = mid + 1;
            } else {
                right = mid;
            }
        } else if arg == "upper" {
            // Looking for the smallest value that is larger than x
            if mid_value > x {
                result_index = mid as i32;
                right = mid;
            } else {
                left = mid + 1;
            }
        }
    }
    
    result_index
}

/// Function to calculate the probability of finding all specified activities in traces
/// 
/// # Arguments
/// * `activity_names` - List of activity names to search for
/// * `normalized_split_traces` - The traces to search in
/// 
/// # Returns
/// * Probability (0.0 to 1.0) of finding all activities in a trace
pub fn are_activities_in_trace(
    activity_names: &Vec<String>,
    normalized_split_traces: &Vec<Vec<(String, String, String, String, String)>>
) -> f64 {
    if activity_names.is_empty() || normalized_split_traces.is_empty() {
        return 0.0;
    }
    
    let mut traces_containing_all_activities = 0;
    let total_traces = normalized_split_traces.len();
    
    for trace in normalized_split_traces {
        // Create a set of activity types in this trace
        let mut trace_activities: HashSet<String> = HashSet::new();
        for event in trace {
            trace_activities.insert(event.1.clone()); // event.1 is the activity type
        }
        
        // Check if all required activities are present in this trace
        let all_activities_present = activity_names.iter()
            .all(|activity| trace_activities.contains(activity));
        
        if all_activities_present {
            traces_containing_all_activities += 1;
        }
    }
    
    // Return the probability as a fraction of traces containing all activities
    traces_containing_all_activities as f64 / total_traces as f64
}

/// Function to fit a curve to given points and return the formula
/// 
/// # Arguments
/// * `x_coords` - Vector of x coordinates
/// * `y_coords` - Vector of y coordinates
/// 
/// # Returns
/// * String containing the linear curve formula
/// 
/// This function performs polynomial curve fitting and returns the curve formula
pub fn get_curve(x_coords: &Vec<f64>, y_coords: &Vec<f64>) -> String {
    if x_coords.len() != y_coords.len() || x_coords.is_empty() {
        return String::new();
    }
    
    let n = x_coords.len();
    
    // Simple linear regression (y = mx + b)
    let sum_x: f64 = x_coords.iter().sum();
    let sum_y: f64 = y_coords.iter().sum();
    let sum_xy: f64 = x_coords.iter().zip(y_coords.iter()).map(|(x, y)| x * y).sum();
    let sum_x_squared: f64 = x_coords.iter().map(|x| x * x).sum();
    
    let n_f64 = n as f64;
    let slope = (n_f64 * sum_xy - sum_x * sum_y) / (n_f64 * sum_x_squared - sum_x * sum_x);
    let intercept = (sum_y - slope * sum_x) / n_f64;
    
    format!("Linear Curve Fit: y = {:.4}x + {:.4}", slope, intercept)
}

/// Function to fit a logarithmic curve to given points and return the formula
/// 
/// # Arguments
/// * `x_coords` - Vector of x coordinates (must be positive for logarithmic fitting)
/// * `y_coords` - Vector of y coordinates
/// 
/// # Returns
/// * String containing the logarithmic curve formula
/// 
/// This function performs logarithmic curve fitting (y = a * ln(x) + b) and returns the curve formula
pub fn get_logarithmic_curve(x_coords: &Vec<f64>, y_coords: &Vec<f64>) -> String {
    if x_coords.len() != y_coords.len() || x_coords.is_empty() {
        return String::new();
    }
    
    // Check if all x values are positive (required for logarithm)
    if x_coords.iter().any(|&x| x <= 0.0) {
        return String::new();
    }
    
    let n = x_coords.len();
    
    // Logarithmic regression: y = a * ln(x) + b
    // Transform x values to ln(x)
    let ln_x: Vec<f64> = x_coords.iter().map(|&x| x.ln()).collect();
    
    let sum_ln_x: f64 = ln_x.iter().sum();
    let sum_y: f64 = y_coords.iter().sum();
    let sum_ln_x_y: f64 = ln_x.iter().zip(y_coords.iter()).map(|(ln_x, y)| ln_x * y).sum();
    let sum_ln_x_squared: f64 = ln_x.iter().map(|ln_x| ln_x * ln_x).sum();
    
    let n_f64 = n as f64;
    let a = (n_f64 * sum_ln_x_y - sum_ln_x * sum_y) / (n_f64 * sum_ln_x_squared - sum_ln_x * sum_ln_x);
    let b = (sum_y - a * sum_ln_x) / n_f64;
    
    format!("Logarithmic Curve Fit: y = {:.4} * ln(x) + {:.4}", a, b)
}

/// Function to plot curve data and save to graphs folder
/// 
/// # Arguments
/// * `formula` - The curve formula as a string
/// * `x_coords` - Vector of x coordinates (data points)
/// * `y_coords` - Vector of y coordinates (data points)  
/// * `filename` - Name for the output file (without extension)
/// * `curve_type` - "linear" or "logarithmic" to determine curve equation
/// 
/// This function creates a plot with data points and the fitted curve, saving it as a PNG file
pub fn plot_curve_and_save(
    formula: &str,
    x_coords: &Vec<f64>,
    y_coords: &Vec<f64>,
    filename: &str,
    curve_type: &str
) -> Result<(), Box<dyn std::error::Error>> {
    if x_coords.is_empty() || y_coords.is_empty() || formula.is_empty() {
        return Ok(());
    }

    let filepath = format!("graphs/{}.png", filename);
    let root = BitMapBackend::new(&filepath, (800, 600)).into_drawing_area();
    root.fill(&WHITE)?;

    // Find data ranges
    let x_min = x_coords.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let x_max = x_coords.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
    let y_min = y_coords.iter().fold(f64::INFINITY, |a, &b| a.min(b));
    let y_max = y_coords.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));

    // Add some padding to the ranges
    let x_padding = (x_max - x_min) * 0.1;
    let y_padding = (y_max - y_min) * 0.1;
    let x_range = (x_min - x_padding)..(x_max + x_padding);
    let y_range = (y_min - y_padding)..(y_max + y_padding);

    let mut chart = ChartBuilder::on(&root)
        .caption(formula, ("sans-serif", 30))
        .margin(10)
        .x_label_area_size(40)
        .y_label_area_size(50)
        .build_cartesian_2d(x_range.clone(), y_range.clone())?;

    chart.configure_mesh().draw()?;

    // Plot data points
    chart
        .draw_series(PointSeries::of_element(
            x_coords.iter().zip(y_coords.iter()).map(|(&x, &y)| (x, y)),
            5,
            &RED,
            &|c, s, st| {
                return EmptyElement::at(c) + Circle::new((0, 0), s, st.filled());
            },
        ))?
        .label("Data Points")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 10, y)], &RED));

    // Extract coefficients from formula and plot curve
    if curve_type == "linear" {
        // Parse linear formula: "Linear Curve Fit: y = {slope}x + {intercept}"
        if let Some(equation_part) = formula.split("y = ").nth(1) {
            let parts: Vec<&str> = equation_part.split("x + ").collect();
            if parts.len() == 2 {
                if let (Ok(slope), Ok(intercept)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
                    // Generate points for the line
                    let curve_points: Vec<(f64, f64)> = (0..100)
                        .map(|i| {
                            let x = x_range.start + (x_range.end - x_range.start) * i as f64 / 99.0;
                            let y = slope * x + intercept;
                            (x, y)
                        })
                        .collect();

                    chart
                        .draw_series(LineSeries::new(curve_points, &BLUE))?
                        .label("Linear Fit")
                        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 10, y)], &BLUE));
                }
            }
        }
    } else if curve_type == "logarithmic" {
        // Parse logarithmic formula: "Logarithmic Curve Fit: y = {a} * ln(x) + {b}"
        if let Some(equation_part) = formula.split("y = ").nth(1) {
            if let Some(ln_part) = equation_part.split(" * ln(x) + ").nth(0) {
                if let Some(b_part) = equation_part.split(" * ln(x) + ").nth(1) {
                    if let (Ok(a), Ok(b)) = (ln_part.parse::<f64>(), b_part.parse::<f64>()) {
                        // Generate points for the logarithmic curve (only for positive x values)
                        let curve_points: Vec<(f64, f64)> = (1..100)
                            .filter_map(|i| {
                                let x = x_range.start.max(0.01) + (x_range.end - x_range.start.max(0.01)) * i as f64 / 99.0;
                                if x > 0.0 {
                                    let y = a * x.ln() + b;
                                    Some((x, y))
                                } else {
                                    None
                                }
                            })
                            .collect();

                        chart
                            .draw_series(LineSeries::new(curve_points, &GREEN))?
                            .label("Logarithmic Fit")
                            .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 10, y)], &GREEN));
                    }
                }
            }
        }
    }

    chart.configure_series_labels().draw()?;
    root.present()?;

    println!("Plot saved to: {}", filepath);
    Ok(())
}



// old slow code-
// for (a, b) in missing_edge_dfg.keys() {
//         let empty_vec: Vec<(String, String, String, String, String)> = Vec::new();
//         let a_events = events_per_activity.get(a).unwrap_or(&empty_vec);
//         let b_events = events_per_activity.get(b).unwrap_or(&empty_vec);

//         let a_events_len = a_events.len();
//         let b_events_len = b_events.len();
//         let mut sum = 0.0;

//         // Skip calculation if either event type has no occurrences
//         if a_events_len > 0 && b_events_len > 0 {
//             let mut an = 0;

//             while an < a_events_len {
//                 // Probability that 'a' happens at this specific time
//                 let p1 = 1.0 / a_events_len as f64;

//                 // Get the timestamp of this 'a' event
//                 let a_timestamp: f64 = a_events[an].2.parse().unwrap_or(0.0);

//                 // Find the first 'b' event that occurs after this 'a' event
//                 let mut bn = 0;
//                 while bn < b_events_len {
//                     let b_timestamp: f64 = b_events[bn].2.parse().unwrap_or(0.0);
//                     if b_timestamp > a_timestamp {
//                         break;
//                     }
//                     bn += 1;
//                 }

//                 // Probability that 'b' happens after this 'a' event
//                 let p2 = (b_events_len - bn) as f64 / b_events_len as f64;

//                 // Add to the total probability sum
//                 sum += p1 * p2;
//                 an += 1;
//             }
//         }

//         println!("Missing edge {} -> {}, score: {}", a, b, sum);

//         // Update missing_edge_dfg with the calculated score
//         if sum > 0.0 {
//             // Create range from 1 to avg_dfg_cost * 2
//             let max_cost = avg_dfg_cost * 2.0;
//             let min_cost = 1.0;
            
//             // If sum is above 1, clamp it to 1
//             let clamped_sum = if sum > 1.0 { 1.0 } else { sum };
            
//             // Convert sum to new value using inverse range
//             // sum = 0 -> new_value = max_cost (avg_dfg_cost * 2)
//             // sum = 1 -> new_value = min_cost (1)
//             // Formula: new_value = max_cost - (clamped_sum * (max_cost - min_cost))
//             let new_value = max_cost - (clamped_sum * (max_cost - min_cost));
            
//             // Store the update to apply later
//             edge_updates.push(((a.clone(), b.clone()), new_value));
//         }
//     }




// considering a, b and c activities-
// for (a, b) in missing_edge_dfg.keys() {
    //     let empty_vec: Vec<(String, String, String, String, String)> = Vec::new();
    //     let a_list = events_per_activity.get(a).unwrap_or(&empty_vec);
    //     let b_list = events_per_activity.get(b).unwrap_or(&empty_vec);
        
    //     let an = a_list.len();
    //     let bn = b_list.len();
    //     let mut result = 0.0;
        
    //     // Skip calculation if either event type has no occurrences
    //     if an > 0 && bn > 0 {
    //         for ai in 0..an {
    //             // Get timestamp of a_list[ai] (third element of tuple)
    //             let a_timestamp: f64 = a_list[ai].2.parse().unwrap_or(0.0);
                
    //             let y = binary_search(a_timestamp, b_list, "upper");
                
    //             if y != -1 {
    //                 let y_usize = y as usize;
    //                 for bi in y_usize..bn {
    //                     let mut prob_c_list: Vec<f64> = Vec::new();
                        
    //                     // Get timestamp of b_list[bi] (third element of tuple)
    //                     let b_timestamp: f64 = b_list[bi].2.parse().unwrap_or(0.0);
                        
    //                     // For every activity except 'a' or 'b'
    //                     for (activity, act_list) in &events_per_activity {
    //                         if activity != a && activity != b {
    //                             let c_start = binary_search(a_timestamp, act_list, "upper");
    //                             let c_end = binary_search(b_timestamp, act_list, "lower");
                                
    //                             let prob_c = if c_start < 0 || c_end < 0 {
    //                                 0.0
    //                             } else {
    //                                 let c_start_usize = c_start as usize;
    //                                 let c_end_usize = c_end as usize;
    //                                 if c_end_usize >= c_start_usize {
    //                                     (c_end_usize - c_start_usize + 1) as f64 / act_list.len() as f64
    //                                 } else {
    //                                     0.0
    //                                 }
    //                             };
                                
    //                             prob_c_list.push(prob_c);
    //                         }
    //                     }
                        
    //                     // Take the highest value in prob_c_list
    //                     let max_prob_c = if prob_c_list.is_empty() {
    //                                                 0.0f64
    //                                             } else {
    //                                                 prob_c_list.iter().fold(0.0f64, |a, &b| a.max(b))
    //                                             };
                        
    //                     let prob_a = 1.0 / an as f64;
    //                     let prob_b = 1.0 / bn as f64;
                        
    //                     result += prob_a * prob_b * (1.0 - max_prob_c);
    //                 }
    //             }
    //             // If y == -1, don't do anything (skip this iteration)
    //         }
    //     }

    //     println!("Missing edge {} -> {}, score: {}", a, b, result);

    //     // Update missing_edge_dfg with the calculated score
    //     if result > 0.0 {
    //         // If result is above 1, clamp it to 1
    //         let clamped_result = result.min(1.0);
            
    //         // Convert result to new value using inverse range
    //         // result = 0 -> new_value = max_cost (avg_dfg_cost * 2)
    //         // result = 1 -> new_value = min_cost (1)
    //         // Formula: new_value = max_cost - (clamped_result * (max_cost - min_cost))
    //         let new_value = max_cost - (clamped_result * cost_range);
            
    //         // Store the update to apply later (avoid cloning strings)
    //         edge_updates.push(((a.clone(), b.clone()), new_value));
    //     }
    // }





    // // test curve formulas and graphs
    // for (a, b) in missing_edge_dfg.keys() {
    //     let empty_vec: Vec<(String, String, String, String, String)> = Vec::new();
    //     let a_events = events_per_activity.get(a).unwrap_or(&empty_vec);
    //     let b_events = events_per_activity.get(b).unwrap_or(&empty_vec);

    //     let a_events_len = a_events.len();
    //     let b_events_len = b_events.len();

    //     // Add code here
    //     // Collect x and y coordinates for a_events
    //     if !a_events.is_empty() {
    //         // Count occurrences of each timestamp in a_events
    //         let mut a_timestamp_counts: HashMap<String, usize> = HashMap::new();
    //         for event in a_events {
    //             let timestamp = &event.2; // 3rd item in tuple
    //             *a_timestamp_counts.entry(timestamp.clone()).or_insert(0) += 1;
    //         }
            
    //         // Create x and y coordinates for a_events
    //         let mut a_x_coords: Vec<f64> = Vec::new();
    //         let mut a_y_coords: Vec<f64> = Vec::new();
            
    //         for (timestamp_str, count) in &a_timestamp_counts {
    //             if let Ok(x_coord) = timestamp_str.parse::<f64>() {
    //                 let y_coord = *count as f64 / a_events_len as f64;
    //                 a_x_coords.push(x_coord);
    //                 a_y_coords.push(y_coord);
    //             }
    //         }
            
    //         // Sort coordinates by x values for proper curve fitting
    //         let mut a_coords: Vec<(f64, f64)> = a_x_coords.iter().zip(a_y_coords.iter()).map(|(&x, &y)| (x, y)).collect();
    //         a_coords.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    //         a_x_coords = a_coords.iter().map(|(x, _)| *x).collect();
    //         a_y_coords = a_coords.iter().map(|(_, y)| *y).collect();
            
    //         if a_x_coords.len() >= 2 {
    //             let linear_formula = get_curve(&a_x_coords, &a_y_coords);
    //             if !linear_formula.is_empty() {
    //                 println!("{}", linear_formula);
    //                 let filename = format!("{}_{}_linear", a, b);
    //                 let _ = plot_curve_and_save(&linear_formula, &a_x_coords, &a_y_coords, &filename, "linear");
    //             }
                
    //             // Only do logarithmic fitting if all x coordinates are positive
    //             if a_x_coords.iter().all(|&x| x > 0.0) {
    //                 let log_formula = get_logarithmic_curve(&a_x_coords, &a_y_coords);
    //                 if !log_formula.is_empty() {
    //                     println!("{}", log_formula);
    //                     let filename = format!("{}_{}_logarithmic", a, b);
    //                     let _ = plot_curve_and_save(&log_formula, &a_x_coords, &a_y_coords, &filename, "logarithmic");
    //                 }
    //             }
    //         }
    //     }

    //     // Collect x and y coordinates for b_events
    //     if !b_events.is_empty() {
    //         // Count occurrences of each timestamp in b_events
    //         let mut b_timestamp_counts: HashMap<String, usize> = HashMap::new();
    //         for event in b_events {
    //             let timestamp = &event.2; // 3rd item in tuple
    //             *b_timestamp_counts.entry(timestamp.clone()).or_insert(0) += 1;
    //         }
            
    //         // Create x and y coordinates for b_events
    //         let mut b_x_coords: Vec<f64> = Vec::new();
    //         let mut b_y_coords: Vec<f64> = Vec::new();
            
    //         for (timestamp_str, count) in &b_timestamp_counts {
    //             if let Ok(x_coord) = timestamp_str.parse::<f64>() {
    //                 let y_coord = *count as f64 / b_events_len as f64;
    //                 b_x_coords.push(x_coord);
    //                 b_y_coords.push(y_coord);
    //             }
    //         }
            
    //         // Sort coordinates by x values for proper curve fitting
    //         let mut b_coords: Vec<(f64, f64)> = b_x_coords.iter().zip(b_y_coords.iter()).map(|(&x, &y)| (x, y)).collect();
    //         b_coords.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    //         b_x_coords = b_coords.iter().map(|(x, _)| *x).collect();
    //         b_y_coords = b_coords.iter().map(|(_, y)| *y).collect();
            
    //         if b_x_coords.len() >= 2 {
    //             let linear_formula = get_curve(&b_x_coords, &b_y_coords);
    //             if !linear_formula.is_empty() {
    //                 println!("{}", linear_formula);
    //                 let filename = format!("{}_{}_linear", a, b);
    //                 let _ = plot_curve_and_save(&linear_formula, &b_x_coords, &b_y_coords, &filename, "linear");
    //             }
                
    //             // Only do logarithmic fitting if all x coordinates are positive
    //             if b_x_coords.iter().all(|&x| x > 0.0) {
    //                 let log_formula = get_logarithmic_curve(&b_x_coords, &b_y_coords);
    //                 if !log_formula.is_empty() {
    //                     println!("{}", log_formula);
    //                     let filename = format!("{}_{}_logarithmic", a, b);
    //                     let _ = plot_curve_and_save(&log_formula, &b_x_coords, &b_y_coords, &filename, "logarithmic");
    //                 }
    //             }
    //         }
    //     }
    // }
