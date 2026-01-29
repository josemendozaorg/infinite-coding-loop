// use crate::domain::types::EntityMetadata; // Using generated types
use anyhow::Result;
use petgraph::graph::DiGraph;
use petgraph::graph::NodeIndex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod executor;

// 1. Serialization Structs (mirroring metamodel.schema.json)
#[derive(Debug, Serialize, Deserialize)]
pub struct GraphDefinition {
    pub entities: Option<Vec<serde_json::Value>>,
    pub relationships: Option<Vec<RelationshipDef>>,
    pub definitions: Option<Definitions>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Definitions {
    #[serde(rename = "GraphRules")]
    pub graph_rules: GraphRules,
    #[serde(rename = "PromptTemplates")]
    pub prompt_templates: Option<PromptTemplates>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GraphRules {
    pub rules: Vec<Rule>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PromptTemplates {
    pub templates: Vec<PromptTemplate>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PromptTemplate {
    pub relation: String,
    pub target: String,
    pub template_ref: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Rule {
    pub source: String, // Entity Kind
    pub target: String, // Entity Kind
    pub relation: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RelationshipDef {
    pub source_id: String,
    pub target_id: String,
    #[serde(rename = "type")]
    pub rel_type: String,
}

// 2. The In-Memory Graph
pub struct DependencyGraph {
    pub graph: DiGraph<String, String>, // Node=EntityKind, Edge=RelationType
    pub kind_map: HashMap<String, NodeIndex>,
    // Key: (Relation, Target), Value: Template Content
    pub prompt_templates: HashMap<(String, String), String>,
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            kind_map: HashMap::new(),
            prompt_templates: HashMap::new(),
        }
    }

    pub fn load_from_metamodel(json_content: &str) -> Result<Self> {
        let def: GraphDefinition = serde_json::from_str(json_content)?;
        let mut dg = Self::new();

        if let Some(defs) = def.definitions {
            for rule in defs.graph_rules.rules {
                let s_idx = dg.get_or_create_node(&rule.source);
                let t_idx = dg.get_or_create_node(&rule.target);
                dg.graph.add_edge(s_idx, t_idx, rule.relation);
            }
            // Load Prompt Templates
            if let Some(templates) = defs.prompt_templates {
                for tmpl in templates.templates {
                    // Key: (Relation, Target) - simplified for now
                    let key = (tmpl.relation.clone(), tmpl.target.clone());

                    // Attempt to resolve path: try CWD, then ../.. (for crate tests)
                    let p = std::path::Path::new(&tmpl.template_ref);
                    let paths_to_try = vec![
                        p.to_path_buf(),
                        std::path::Path::new("../..").join(p),
                        // Also try assuming we are in a subdirectory of workspace
                        std::path::Path::new("..").join(p),
                    ];

                    let mut content = String::new();
                    let mut found = false;
                    for path in paths_to_try {
                        if let Ok(c) = std::fs::read_to_string(&path) {
                            content = c;
                            found = true;
                            break;
                        }
                    }

                    if !found {
                        eprintln!(
                            "Warning: Failed to load prompt template {}: No such file in tried paths",
                            tmpl.template_ref
                        );
                        content = format!("Error loading template: {}", tmpl.template_ref);
                    }
                    dg.prompt_templates.insert(key, content);
                }
            }
        }

        Ok(dg)
    }

    fn get_or_create_node(&mut self, kind: &str) -> NodeIndex {
        if let Some(&idx) = self.kind_map.get(kind) {
            idx
        } else {
            let idx = self.graph.add_node(kind.to_string());
            self.kind_map.insert(kind.to_string(), idx);
            idx
        }
    }

    pub fn get_prompt_template(&self, relation: &str, target: &str) -> Option<String> {
        // Look up by (Relation, Target)
        self.prompt_templates
            .get(&(relation.to_string(), target.to_string()))
            .cloned()
    }

    pub fn get_dependencies(&self, kind: &str, relation: &str) -> Vec<String> {
        let mut deps = Vec::new();
        if let Some(&start_idx) = self.kind_map.get(kind) {
            // Find inputs: relations pointing TO this node (if dependency means "required by")
            // Or typically in a dependency graph: Target depends on Source.
            // Our metamodel says "Agent creates Feature".
            // So if I want "Feature", I look for incoming edge "creates".

            for index in self.graph.node_indices() {
                let mut edges = self.graph.edges_connecting(index, start_idx);
                if edges.any(|e| e.weight() == relation) {
                    deps.push(self.graph[index].clone());
                }
            }
        }
        deps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_and_query_graph() {
        let json = r#"{
            "entities": [],
            "relationships": [],
            "definitions": {
                "GraphRules": {
                    "rules": [
                        { "source": "Agent", "target": "Feature", "relation": "creates" },
                        { "source": "Feature", "target": "Requirement", "relation": "contains" }
                    ]
                }
            }
        }"#;

        let graph = DependencyGraph::load_from_metamodel(json).expect("Failed to load graph");

        // Query: Who creates a Feature?
        // Method: get_dependencies("Feature", "creates") checks for incoming "creates" edges
        let creators = graph.get_dependencies("Feature", "creates");
        assert_eq!(creators, vec!["Agent"]);

        // Query: What verifies a Feature? (None in this graph)
        let verifiers = graph.get_dependencies("Feature", "verifies");
        assert!(verifiers.is_empty());
    }
}
