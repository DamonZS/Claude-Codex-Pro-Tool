//! Element finding and filtering logic for UI Automation.

use anyhow::{anyhow, Result};

#[cfg(target_os = "windows")]
use windows::Win32::UI::Accessibility::{
    IUIAutomation, IUIAutomationCondition,
    UIA_ControlTypePropertyId, UIA_IsEnabledPropertyId, UIA_NamePropertyId,
};
#[cfg(target_os = "windows")]
use windows::core::VARIANT;

use super::types::*;

/// Element matcher for filtering search results
pub struct ElementMatcher {
    query: Option<String>,
    element_type: Option<ElementType>,
    interactive_only: bool,
}

impl ElementMatcher {
    pub fn new(params: &FindParams) -> Self {
        Self {
            query: params.query.clone(),
            element_type: params.element_type,
            interactive_only: params.interactive_only,
        }
    }

    /// Check if an element matches the criteria
    pub fn matches(&self, element: &UiElement) -> bool {
        // Type filter
        if let Some(wanted_type) = self.element_type {
            if element.element_type != wanted_type {
                return false;
            }
        }

        // Interactive filter
        if self.interactive_only {
            if !element.enabled || !self.is_interactive_type(element.element_type) {
                return false;
            }
        }

        // Text query filter
        if let Some(ref query) = self.query {
            if !self.text_matches(element, query) {
                return false;
            }
        }

        true
    }

    fn text_matches(&self, element: &UiElement, query: &str) -> bool {
        let query_lower = query.to_lowercase();

        // Check label
        if element.label.to_lowercase().contains(&query_lower) {
            return true;
        }

        // Check value
        if let Some(ref value) = element.value {
            if value.to_lowercase().contains(&query_lower) {
                return true;
            }
        }

        // Check automation_id
        if let Some(ref auto_id) = element.automation_id {
            if auto_id.to_lowercase().contains(&query_lower) {
                return true;
            }
        }

        // Check class_name
        if let Some(ref class) = element.class_name {
            if class.to_lowercase().contains(&query_lower) {
                return true;
            }
        }

        false
    }

    fn is_interactive_type(&self, element_type: ElementType) -> bool {
        matches!(
            element_type,
            ElementType::Button
                | ElementType::SplitButton
                | ElementType::Edit
                | ElementType::CheckBox
                | ElementType::RadioButton
                | ElementType::ComboBox
                | ElementType::ListBox
                | ElementType::ListItem
                | ElementType::TreeItem
                | ElementType::MenuItem
                | ElementType::TabItem
                | ElementType::Link
                | ElementType::Slider
                | ElementType::Spinner
        )
    }
}

/// Find elements in a tree by walking and filtering
pub fn find_in_tree(root: &UiElement, params: &FindParams) -> Vec<UiElement> {
    let matcher = ElementMatcher::new(params);
    let mut results = Vec::new();
    collect_matches(root, &matcher, &mut results);
    results
}

fn collect_matches(element: &UiElement, matcher: &ElementMatcher, results: &mut Vec<UiElement>) {
    if matcher.matches(element) {
        results.push(element.clone());
    }

    for child in &element.children {
        collect_matches(child, matcher, results);
    }
}

/// Build a UIA condition for searching
#[cfg(target_os = "windows")]
pub unsafe fn build_condition(
    automation: &IUIAutomation,
    params: &FindParams,
) -> Result<IUIAutomationCondition> {
    unsafe {
        let mut conditions = Vec::new();

        // Type condition
        if let Some(element_type) = params.element_type {
            let ctrl_type = element_type_to_control_type(element_type);
            let variant = VARIANT::from(ctrl_type);
            if let Ok(cond) = automation.CreatePropertyCondition(UIA_ControlTypePropertyId, &variant) {
                conditions.push(cond);
            }
        }

        // Interactive condition (enabled + focusable)
        if params.interactive_only {
            let enabled_variant = VARIANT::from(true);
            if let Ok(cond) = automation.CreatePropertyCondition(UIA_IsEnabledPropertyId, &enabled_variant) {
                conditions.push(cond);
            }
        }

        // Text query condition (search in Name)
        if let Some(ref query) = params.query {
            let bstr = windows::core::BSTR::from(query.as_str());
            let variant = VARIANT::from(bstr);

            // Simple substring match via name property
            if let Ok(name_cond) = automation.CreatePropertyCondition(UIA_NamePropertyId, &variant) {
                conditions.push(name_cond);
            }
        }

        // Combine conditions with AND
        if conditions.is_empty() {
            automation.CreateTrueCondition()
                .map_err(|e| anyhow!("Failed to create true condition: {}", e))
        } else if conditions.len() == 1 {
            Ok(conditions.into_iter().next().unwrap())
        } else {
            // Combine with AND - create pairs iteratively
            let mut combined = conditions[0].clone();
            for cond in conditions.iter().skip(1) {
                combined = automation.CreateAndCondition(&combined, cond)
                    .map_err(|e| anyhow!("Failed to create AND condition: {}", e))?;
            }
            Ok(combined)
        }
    }
}

#[cfg(target_os = "windows")]
fn element_type_to_control_type(element_type: ElementType) -> i32 {
    match element_type {
        ElementType::Button => 50000,
        ElementType::Calendar => 50001,
        ElementType::CheckBox => 50002,
        ElementType::ComboBox => 50003,
        ElementType::Edit => 50004,
        ElementType::Link => 50005,
        ElementType::Image => 50006,
        ElementType::ListItem => 50007,
        ElementType::ListBox => 50008,
        ElementType::Menu => 50009,
        ElementType::MenuBar => 50010,
        ElementType::MenuItem => 50011,
        ElementType::ProgressBar => 50012,
        ElementType::RadioButton => 50013,
        ElementType::ScrollBar => 50014,
        ElementType::Slider => 50015,
        ElementType::Spinner => 50016,
        ElementType::StatusBar => 50017,
        ElementType::TabControl => 50018,
        ElementType::TabItem => 50019,
        ElementType::Text => 50020,
        ElementType::ToolBar => 50021,
        ElementType::ToolTip => 50022,
        ElementType::TreeView => 50023,
        ElementType::TreeItem => 50024,
        ElementType::Custom => 50025,
        ElementType::Group => 50026,
        ElementType::Thumb => 50027,
        ElementType::DataGrid => 50028,
        ElementType::DataItem => 50029,
        ElementType::Document => 50030,
        ElementType::SplitButton => 50031,
        ElementType::Window => 50032,
        ElementType::Pane => 50033,
        ElementType::Header => 50034,
        ElementType::HeaderItem => 50035,
        ElementType::Table => 50036,
        ElementType::TitleBar => 50037,
        ElementType::Separator => 50038,
        ElementType::Dialog => 50032, // Dialog uses Window control type
        ElementType::Unknown => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matcher_type_filter() {
        let params = FindParams {
            query: None,
            element_type: Some(ElementType::Button),
            interactive_only: false,
        };
        let matcher = ElementMatcher::new(&params);

        let button = UiElement::new("1".to_string(), ElementType::Button);
        let edit = UiElement::new("2".to_string(), ElementType::Edit);

        assert!(matcher.matches(&button));
        assert!(!matcher.matches(&edit));
    }

    #[test]
    fn test_matcher_query_filter() {
        let params = FindParams {
            query: Some("submit".to_string()),
            element_type: None,
            interactive_only: false,
        };
        let matcher = ElementMatcher::new(&params);

        let mut button = UiElement::new("1".to_string(), ElementType::Button);
        button.label = "Submit Button".to_string();

        let mut other = UiElement::new("2".to_string(), ElementType::Button);
        other.label = "Cancel".to_string();

        assert!(matcher.matches(&button));
        assert!(!matcher.matches(&other));
    }

    #[test]
    fn test_matcher_interactive_filter() {
        let params = FindParams {
            query: None,
            element_type: None,
            interactive_only: true,
        };
        let matcher = ElementMatcher::new(&params);

        let button = UiElement::new("1".to_string(), ElementType::Button);
        let text = UiElement::new("2".to_string(), ElementType::Text);
        let mut disabled_button = UiElement::new("3".to_string(), ElementType::Button);
        disabled_button.enabled = false;

        assert!(matcher.matches(&button));
        assert!(!matcher.matches(&text));
        assert!(!matcher.matches(&disabled_button));
    }

    #[test]
    fn test_find_in_tree() {
        let mut root = UiElement::new("root".to_string(), ElementType::Window);

        let mut button1 = UiElement::new("btn1".to_string(), ElementType::Button);
        button1.label = "OK".to_string();

        let mut button2 = UiElement::new("btn2".to_string(), ElementType::Button);
        button2.label = "Cancel".to_string();

        let text = UiElement::new("txt".to_string(), ElementType::Text);

        root.children.push(button1);
        root.children.push(button2);
        root.children.push(text);

        let params = FindParams {
            query: None,
            element_type: Some(ElementType::Button),
            interactive_only: false,
        };

        let results = find_in_tree(&root, &params);
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|e| e.element_type == ElementType::Button));
    }
}
