

use std::collections::HashSet;
use crate::types::{ProcessForest, TreeNode};
use uuid::Uuid;

// Given an Id of a node, this fn return a list of Ids of that node and all its descendants
pub fn find_ids_of_descendants(forest: &ProcessForest, target_id: &str) -> Vec<String> {
    let mut ids = Vec::new();
    
    for node in forest {
        if find_and_collect(node, target_id, &mut ids) {
            break;
        }
    }
    
    ids
}
fn find_and_collect(node: &TreeNode, target_id: &str, ids: &mut Vec<String>) -> bool {
    if node.id == target_id {
        collect_ids(node, ids);
        return true;
    }
    
    for child in &node.children {
        if find_and_collect(child, target_id, ids) {
            return true;
        }
    }
    
    false
}
fn collect_ids(node: &TreeNode, ids: &mut Vec<String>) {
    ids.push(node.id.clone());
    for child in &node.children {
        collect_ids(child, ids);
    }
}

// Given an ID of a node and ocpt, remove that node and all its descendants from the ocpt. return the new ocpt as well as a list of all labels/activity names of the removed nodes
pub fn replace_node_and_descendants(forest: ProcessForest, target_id: &str) -> (ProcessForest, HashSet<String>) {
    let mut removed_labels = HashSet::new();
    let new_forest = replace_with_flower_recursive(forest, target_id, &mut removed_labels);

    // Filter out structural nodes from removed_labels
    removed_labels.retain(|label| {
        !matches!(label.as_str(), "sequence" | "parallel" | "exclusive" | "redo" | "tau" | "flower")
    });

    (new_forest, removed_labels)
}

fn replace_with_flower_recursive(nodes: Vec<TreeNode>, target_id: &str, removed_labels: &mut HashSet<String>) -> Vec<TreeNode> {
    let mut new_nodes = Vec::new();
    
    for mut node in nodes {
        if node.id == target_id {
            collect_labels(&node, removed_labels);
            
            let mut children = Vec::new();
            for label in removed_labels.iter() {
                if !matches!(label.as_str(), "sequence" | "parallel" | "exclusive" | "redo" | "tau" | "flower") {
                    children.push(TreeNode {
                        id: Uuid::new_v4().to_string(),
                        label: label.clone(),
                        children: Vec::new(),
                    });
                }
            }

            let flower_node = TreeNode {
                id: Uuid::new_v4().to_string(),
                label: "flower".to_string(),
                children,
            };
            new_nodes.push(flower_node);
        } else {
            node.children = replace_with_flower_recursive(node.children, target_id, removed_labels);
            new_nodes.push(node);
        }
    }
    
    new_nodes
}

fn collect_labels(node: &TreeNode, labels: &mut HashSet<String>) {
    labels.insert(node.label.clone());
    for child in &node.children {
        collect_labels(child, labels);
    }
} 