use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use crate::types::{Event, Object};



pub fn build_relations(
    events: &HashMap<String, Event>,
    objects: &HashMap<String, Object>,
) -> Vec<(String, String, String, String, String)> {
    let mut relations = Vec::new();

    for (event_id, event) in events {
        for object_id in &event.omap {
            if let Some(object) = objects.get(object_id) {
                relations.push((
                    event_id.clone(),
                    event.activity.clone(),
                    event.timestamp.clone(),
                    object_id.clone(),
                    object.object_type.clone(),
                ));
            }
        }
    }

    relations
}
