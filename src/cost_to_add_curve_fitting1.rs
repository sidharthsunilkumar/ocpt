use std::collections::{HashMap, HashSet};
use plotters::prelude::*;
  

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
    
    // Ensure graphs directory exists
    let _ = std::fs::create_dir_all("graphs");

    for (a, b) in missing_edge_dfg.keys() {
        let empty_vec_f64: Vec<f64> = Vec::new();
        let a_timestamps = events_with_timestamps.get(a).unwrap_or(&empty_vec_f64);
        let b_timestamps = events_with_timestamps.get(b).unwrap_or(&empty_vec_f64);

        let a_len = a_timestamps.len();
        let b_len = b_timestamps.len();
        let mut sum = 0.0;
        let mut area_blue = 0.0;
        let mut area_red = 0.0;

        // Skip calculation if either event type has no occurrences
        if a_len > 0 && b_len > 0 {
            // Helper: Calculate bandwidth using Silverman's rule
            fn calc_bandwidth(data: &Vec<f64>) -> f64 {
                if data.len() < 2 { return 1.0; }
                let mean = data.iter().sum::<f64>() / data.len() as f64;
                let variance = data.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / (data.len() - 1) as f64;
                let std_dev = variance.sqrt();
                if std_dev == 0.0 { return 1.0; }
                1.06 * std_dev * (data.len() as f64).powf(-0.2)
            }

            let bw_a = calc_bandwidth(a_timestamps);
            let bw_b = calc_bandwidth(b_timestamps);

            // Define plotting range
            let min_t = (*a_timestamps.first().unwrap()).min(*b_timestamps.first().unwrap());
            let max_t = (*a_timestamps.last().unwrap()).max(*b_timestamps.last().unwrap());
            let margin = (max_t - min_t).abs() * 0.1 + 1.0; // +1.0 covers single point case
            let start_x = min_t - margin;
            let end_x = max_t + margin;
            
            // Generate points
            let number_of_buckets = 100;
            let step = (end_x - start_x) / number_of_buckets as f64;
            
            let mut plot_data_a = Vec::new();
            let mut plot_data_b = Vec::new();

            // Helper: Normal CDF approx (Sigmoid/Logistic for smooth curve)
            fn phi(x: f64) -> f64 {
                1.0 / (1.0 + (-1.702 * x).exp())
            }

            // Calculate Bucket Probabilities and integral
            for i in 0..number_of_buckets {
                let r = start_x + (i as f64) * step;
                let t = start_x + ((i + 1) as f64) * step;
                
                // Blue Y: (number of activity b that happens between timestamp r and t)/ total number of activity b occurance
                // Note: user says "activity b" to refer to the source activity 'a' in this context
                let count_in_bucket = a_timestamps.iter()
                    .filter(|&&val| val >= r && val <= t)
                    .count();
                
                let prob_mass_a = if a_len > 0 {
                    count_in_bucket as f64 / a_len as f64
                } else { 0.0 };
                
                // Smoothed P(c happens after x) = 1 - CDF_B(x) evaluated at t
                let mut cdf_b = 0.0;
                for &val in b_timestamps {
                    // evaluated at t (the end of the bucket, which is the x-axis point)
                    cdf_b += phi((t - val) / bw_b);
                }
                cdf_b /= b_len as f64;
                let prob_b_after = 1.0 - cdf_b;
                
                // Contribution to integral (Sum of area under product) - EDITED: Accumulate areas instead
                // sum += prob_mass_a * prob_b_after;

                area_blue += prob_mass_a * step;
                area_red += prob_b_after * step;
                
                plot_data_a.push((t, prob_mass_a)); // Blue curve points
                plot_data_b.push((t, prob_b_after)); // Red curve points
            }

            // Plotting
            let safe_a = a.chars().map(|c| if c.is_alphanumeric() { c } else { '_' }).collect::<String>();
            let safe_b = b.chars().map(|c| if c.is_alphanumeric() { c } else { '_' }).collect::<String>();
            let filename = format!("graphs/{}-{}.png", safe_a, safe_b);
            
            // Create backend and drawing area
            let root = BitMapBackend::new(&filename, (1024, 768)).into_drawing_area();
            let _ = root.fill(&WHITE);
            
            // Using 0..1 as requested for Y axis
            let chart = ChartBuilder::on(&root)
                .caption(format!("Missing Edge {} -> {}", a, b), ("sans-serif", 30))
                .margin(20)
                .x_label_area_size(40)
                .y_label_area_size(40)
                .build_cartesian_2d(start_x..end_x, 0.0..1.0);

            if let Ok(mut chart) = chart {
                let _ = chart.configure_mesh().draw();
                
                // Blue Curve: PDF of Source
                let _ = chart.draw_series(LineSeries::new(
                    plot_data_a.clone(),
                    BLUE.stroke_width(2),
                ));
                let _ = chart.draw_series(PointSeries::of_element(
                    plot_data_a,
                    5,
                    &BLUE,
                    &|c, s, st| {
                        return EmptyElement::at(c) + Circle::new((0,0),s,st.filled());
                    },
                ));
                
                // Red Curve: Prob Target Happens After
                let _ = chart.draw_series(LineSeries::new(
                    plot_data_b.clone(),
                    RED.stroke_width(2),
                ));
                let _ = chart.draw_series(PointSeries::of_element(
                    plot_data_b,
                    5,
                    &RED,
                    &|c, s, st| {
                        return EmptyElement::at(c) + Circle::new((0,0),s,st.filled());
                    },
                ));
            }
            let _ = root.present();
        }

        // Calculate probability using individual activity probabilities
        let probability_of_occurance_of_a = probability_of_occurance_of_activity.get(a).unwrap_or(&0.0);
        let probability_of_occurance_of_b = probability_of_occurance_of_activity.get(b).unwrap_or(&0.0);
        
        let mut area_value = area_red - area_blue;
        area_value = area_value * probability_of_occurance_of_a * probability_of_occurance_of_b;

        println!("Missing edge {} -> {}, score: {:e}", a, b, area_value);

        // Update missing_edge_dfg with the calculated score

        edge_updates.push(((a.clone(), b.clone()), area_value));

    }


    // Apply all the updates to missing_edge_dfg
    for ((a, b), new_cost) in edge_updates {
        if let Some(edge_cost) = missing_edge_dfg.get_mut(&(a, b)) {
            *edge_cost = new_cost;
        }
    }    

    // Normalise the cost of missing edges
    let mut max_abs_score = 0.0;
    for cost in missing_edge_dfg.values() {
        if *cost != 99999.0 && cost.abs() > max_abs_score {
            max_abs_score = cost.abs();
        }
    }

    if max_abs_score > 0.0 {
        for cost in missing_edge_dfg.values_mut() {
            if *cost != 99999.0 {
                let normalized_score = *cost / max_abs_score;
                *cost = (1.0 - normalized_score) * 100.0;
            }
        }
    }

    // Print missing edges with their final costs
    println!("\n=== FINAL MISSING EDGE COSTS WITH CURVE FITTING ===");
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
