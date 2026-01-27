use crate::plan::action::ImplementationPlan;
use crate::product::requirement::Requirement;
use crate::spec::feature_spec::FeatureSpec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum Primitive {
    Requirement(Requirement),
    Specification(FeatureSpec),
    Plan(ImplementationPlan),
    Code {
        path: String,
        content: String,
    },
    Verification {
        test_command: String,
        success: bool,
        output: String,
    },
}

impl Primitive {
    pub fn id(&self) -> String {
        match self {
            Primitive::Requirement(r) => r.id.clone(),
            Primitive::Specification(s) => s.id.clone(),
            Primitive::Plan(p) => p.feature_id.clone(),
            Primitive::Code { path, .. } => path.clone(),
            Primitive::Verification { test_command, .. } => test_command.clone(),
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Primitive::Requirement(_) => "REQ",
            Primitive::Specification(_) => "SPEC",
            Primitive::Plan(_) => "PLAN",
            Primitive::Code { .. } => "CODE",
            Primitive::Verification { .. } => "VERIFY",
        }
    }
}
