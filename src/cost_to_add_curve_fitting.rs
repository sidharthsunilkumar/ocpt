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
    

    // Normalised timestamp traces
    let mut normalized_traces: Vec<Vec<(String, String, String, String, String)>> = Vec::new();
    
    for trace in &traces {
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
                    normalized_traces.push(normalized_trace);
                }
            } else {
                // If parsing fails, use original trace
                normalized_traces.push(trace.clone());
            }
        }
    }
    

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
                missing_edge_dfg.insert((a.clone(), b.clone()), 99999.0);
            }
        }
    }

    // Precompute: group all events by activity type and sort them by timestamp once
    let mut events_per_activity: HashMap<String, Vec<(String, String, String, String, String)>> = HashMap::new();

    for trace in &normalized_traces {
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

    // Probability of event occurring in a trace
    let mut probability_of_occurance_of_activity: HashMap<String, f64> = HashMap::new();
    
    // Number of traces that contain each activity
    let mut number_of_traces_with_activity: HashMap<String, usize> = HashMap::new();
    
    // Calculate probability that each activity occurs at least once in a trace
    for activity in &activities {
        let mut traces_containing_activity = 0;
        let total_traces = normalized_traces.len();
        
        // Count traces that contain this activity at least once
        for trace in &normalized_traces {
            let mut activity_found = false;
            for event in trace {
                if &event.1 == activity { // event.1 is the activity type
                    activity_found = true;
                    break;
                }
            }
            if activity_found {
                traces_containing_activity += 1;
            }
        }
        
        // Calculate probability as fraction of traces containing the activity
        let probability = if total_traces > 0 {
            traces_containing_activity as f64 / total_traces as f64
        } else {
            0.0
        };
        
        // Store both the count and the probability
        number_of_traces_with_activity.insert(activity.clone(), traces_containing_activity);
        probability_of_occurance_of_activity.insert(activity.clone(), probability);
        
    }

    // Process each missing edge to calculate probability scores
    let mut edge_updates: Vec<((String, String), f64)> = Vec::new(); // Store updates to apply later
    

    for (a, b) in missing_edge_dfg.keys() {
        let empty_vec_f64: Vec<f64> = Vec::new();
        let a_timestamps = events_with_timestamps.get(a).unwrap_or(&empty_vec_f64);
        let b_timestamps = events_with_timestamps.get(b).unwrap_or(&empty_vec_f64);

        let a_len = a_timestamps.len();
        let b_len = b_timestamps.len();
        let mut sum = 0.0;

        // Skip calculation if either event type has no occurrences
        if a_len > 0 && b_len > 0 {
            let mut b_idx = 0;
            let mut count_sum = 0.0;
            
            // Both timestamp vectors are sorted. Use two-pointer approach for O(N+M) complexity.
            for &a_t in a_timestamps {
                // Find first b_t > a_t
                // Since a_t are sorted ascending, b_idx can only move forward
                while b_idx < b_len && b_timestamps[b_idx] <= a_t {
                    b_idx += 1;
                }
                
                // All remaining b events are after a_t
                count_sum += (b_len - b_idx) as f64;
            }
            
            sum = count_sum / (a_len as f64 * b_len as f64);
        }

        // Calculate probability using individual activity probabilities
        let probability_of_occurance_of_a = probability_of_occurance_of_activity.get(a).unwrap_or(&0.0);
        let probability_of_occurance_of_b = probability_of_occurance_of_activity.get(b).unwrap_or(&0.0);
        sum = sum * probability_of_occurance_of_a * probability_of_occurance_of_b;

        println!("Missing edge {} -> {}, score: {:e}", a, b, sum);

        // Update missing_edge_dfg with the calculated score
        if sum > 0.0 {            
            // Store the update to apply later

            sum = 1.0 - sum;
            sum = sum * 100.0;

            edge_updates.push(((a.clone(), b.clone()), sum));
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

//         // Calculate probability using individual activity probabilities

//         let probability_of_occurance_of_a = probability_of_occurance_of_activity.get(a).unwrap_or(&0.0);
//         let probability_of_occurance_of_b = probability_of_occurance_of_activity.get(b).unwrap_or(&0.0);
//         sum = sum * probability_of_occurance_of_a * probability_of_occurance_of_b;

//         println!("Missing edge {} -> {}, score: {:e}", a, b, sum);

//         // Update missing_edge_dfg with the calculated score
//         if sum > 0.0 {            
//             // Store the update to apply later

//             sum = 1.0 - sum;
//             sum = sum * 100.0;

//             edge_updates.push(((a.clone(), b.clone()), sum));
//         }
//     }
