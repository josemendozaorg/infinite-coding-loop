use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Represents a Feature Specification (The Contract).
/// defined in the "Design" phase of the DASS process.
///
/// A Feature Spec must cover all linked Requirements and provide
/// specific technical details for implementation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FeatureSpec {
    /// Unique Identifier for the spec (usually matches the Feature Name/ID).
    pub id: String,
    /// Links to the Atomic Requirements this spec satisfies.
    pub requirement_ids: Vec<String>,
    /// UI Specification (Markdown content).
    pub ui_spec: String,
    /// Business Logic Specification (Markdown content).
    pub logic_spec: String,
    /// Data Model Specification (Markdown content).
    pub data_spec: String,
    /// Verification Plan (Markdown content with PBT definitions).
    pub verification_plan: String,
}

impl FeatureSpec {
    pub fn new(id: impl Into<String>, req_ids: Vec<String>) -> Self {
        Self {
            id: id.into(),
            requirement_ids: req_ids,
            ui_spec: String::new(),
            logic_spec: String::new(),
            data_spec: String::new(),
            verification_plan: String::new(),
        }
    }

    /// Checks if the spec is "complete" (no empty sections).
    /// This is a basic Gate check.
    pub fn is_complete(&self) -> bool {
        !self.ui_spec.is_empty()
            && !self.logic_spec.is_empty()
            && !self.data_spec.is_empty()
            && !self.verification_plan.is_empty()
    }
}
