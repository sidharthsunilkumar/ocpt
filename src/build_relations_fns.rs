use std::collections::HashMap;
use crate::types::{OCELEvent, OCELObject};



pub fn build_relations(
    events: &Vec<OCELEvent>,
    objects: &Vec<OCELObject>,
) -> Vec<(String, String, String, String, String)> {

    // Format (event_id, event_type, timestamp, object_id, object_type)

    let mut relations = Vec::new();

    // Create a HashMap for quick object lookup
    let object_map: HashMap<String, &OCELObject> = objects.iter()
        .map(|obj| (obj.id.clone(), obj))
        .collect();

    for event in events {
        for relationship in &event.relationships {
            if let Some(object) = object_map.get(&relationship.object_id) {
                relations.push((
                    event.id.clone(),
                    event.event_type.clone(),
                    event.time.to_rfc3339(),
                    relationship.object_id.clone(),
                    object.object_type.clone(),
                ));
            }
        }
    }

    // relations.sort(); 

    // First sorting by event id, then by timestamp
    relations.sort_by(|a, b| a.0.cmp(&b.0));
    relations.sort_by(|a, b| a.2.cmp(&b.2));

    relations
}
