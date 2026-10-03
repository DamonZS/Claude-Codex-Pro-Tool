//! Element registry for stable ID management.
//!
//! This module provides element ID generation and lookup using RuntimeId.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};

#[cfg(target_os = "windows")]
use windows::Win32::UI::Accessibility::IUIAutomationElement;

/// Thread-safe element registry
pub struct ElementRegistry {
    #[cfg(target_os = "windows")]
    elements: Arc<Mutex<HashMap<String, IUIAutomationElement>>>,
}

impl ElementRegistry {
    pub fn new() -> Self {
        Self {
            #[cfg(target_os = "windows")]
            elements: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register an element and return its ID
    #[cfg(target_os = "windows")]
    pub fn register(&self, _element: &IUIAutomationElement) -> Result<String> {
        // TODO: Extract RuntimeId from element
        // TODO: Format as "pid.runtime_id"
        // TODO: Store in registry
        Ok(String::from("placeholder-id"))
    }

    /// Lookup element by ID
    #[cfg(target_os = "windows")]
    pub fn lookup(&self, id: &str) -> Result<IUIAutomationElement> {
        let elements = self.elements.lock().unwrap();
        elements
            .get(id)
            .cloned()
            .ok_or_else(|| anyhow!("Element not found: {}", id))
    }

    /// Remove element from registry
    pub fn unregister(&self, _id: &str) -> Result<()> {
        // TODO: Implement
        Ok(())
    }

    /// Clear all registered elements
    pub fn clear(&self) {
        #[cfg(target_os = "windows")]
        {
            let mut elements = self.elements.lock().unwrap();
            elements.clear();
        }
    }
}

impl Default for ElementRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = ElementRegistry::new();
        registry.clear();
        assert!(true);
    }
}
