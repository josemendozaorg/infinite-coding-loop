use crate::domain::primitive::Primitive;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoftwareApplication {
    pub id: String,
    pub name: String,
    pub work_dir: Option<String>,
    pub primitives: HashMap<String, Primitive>,
}

impl SoftwareApplication {
    pub fn new(id: String, name: String) -> Self {
        Self {
            id,
            name,
            work_dir: None,
            primitives: HashMap::new(),
        }
    }

    pub fn add_primitive(&mut self, primitive: Primitive) {
        self.primitives.insert(primitive.id(), primitive);
    }
}
