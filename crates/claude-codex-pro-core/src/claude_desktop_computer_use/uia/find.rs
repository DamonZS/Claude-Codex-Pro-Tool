//! Element finding and filtering logic.

use super::types::*;

/// Search for elements in a UI tree that match the given parameters.
pub fn find_in_tree(root: &UiElement, params: &FindParams, max_results: usize) -> Vec<UiElement> {
    let mut results = Vec::new();
    find_recursive(root, params, &mut results, max_results);
    results
}

fn find_recursive(
    element: &UiElement,
    params: &FindParams,
    results: &mut Vec<UiElement>,
    max_results: usize,
) {
    if results.len() >= max_results {
        return;
    }

    // Check if this element matches
    if element_matches(element, params) {
        results.push(element.clone());
        if results.len() >= max_results {
            return;
        }
    }

    // Search children
    for child in &element.children {
        find_recursive(child, params, results, max_results);
        if results.len() >= max_results {
            return;
        }
    }
}

fn element_matches(element: &UiElement, params: &FindParams) -> bool {
    // Filter by element type
    if let Some(wanted_type) = params.element_type {
        if element.element_type != wanted_type {
            return false;
        }
    }

    // Filter by query (case-insensitive substring match on label or automation_id)
    if let Some(query) = &params.query {
        let query_lower = query.to_lowercase();
        let label_match = element.label.to_lowercase().contains(&query_lower);
        let aid_match = element
            .automation_id
            .as_ref()
            .map(|aid| aid.to_lowercase().contains(&query_lower))
            .unwrap_or(false);

        if !label_match && !aid_match {
            return false;
        }
    }

    // Filter interactive-only elements
    if params.interactive_only {
        // An element is interactive if it has actions or is a known interactive type
        let is_interactive = !element.actions.is_empty()
            || matches!(
                element.element_type,
                ElementType::Button
                    | ElementType::CheckBox
                    | ElementType::RadioButton
                    | ElementType::ComboBox
                    | ElementType::Edit
                    | ElementType::ListBox
                    | ElementType::ListItem
                    | ElementType::MenuItem
                    | ElementType::TabItem
                    | ElementType::Link
                    | ElementType::Slider
                    | ElementType::Spinner
            );

        if !is_interactive {
            return false;
        }
    }

    true
}
