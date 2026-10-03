//! Element finding and waiting operations.

use super::types::{FindParams, UiElement};

/// Find elements in a pre-built UI tree (in-memory search).
pub fn find_in_tree(tree: &UiElement, params: &FindParams, limit: usize) -> Vec<UiElement> {
    let mut results = Vec::new();
    find_in_tree_recursive(tree, params, &mut results, limit);
    results
}

fn find_in_tree_recursive(
    element: &UiElement,
    params: &FindParams,
    results: &mut Vec<UiElement>,
    limit: usize,
) {
    if results.len() >= limit {
        return;
    }

    // Check if this element matches
    if element_matches_params(element, params) {
        results.push(element.clone());
        if results.len() >= limit {
            return;
        }
    }

    // Recursively check children
    for child in &element.children {
        find_in_tree_recursive(child, params, results, limit);
        if results.len() >= limit {
            return;
        }
    }
}

fn element_matches_params(element: &UiElement, params: &FindParams) -> bool {
    // Check element type filter
    if let Some(wanted_type) = params.element_type {
        if element.element_type != wanted_type {
            return false;
        }
    }

    // Check query string (matches name or automation ID)
    if let Some(query) = &params.query {
        let query_lower = query.to_lowercase();
        let mut matched = false;

        // Try label
        if element.label.to_lowercase().contains(&query_lower) {
            matched = true;
        }

        // Try automation ID
        if !matched {
            if let Some(auto_id) = &element.automation_id {
                if auto_id.to_lowercase().contains(&query_lower) {
                    matched = true;
                }
            }
        }

        if !matched {
            return false;
        }
    }

    // Check interactive_only filter
    if params.interactive_only {
        if !element.enabled || !element.is_keyboard_focusable {
            return false;
        }
    }

    true
}

