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
    #[serde(rename = "AgentDefinitions")]
    pub agents: Option<AgentDefinitions>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GraphRules {
    pub rules: Vec<Rule>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentDefinitions {
    pub agents: Vec<AgentDefinition>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentDefinition {
    pub role: String,
    pub config_ref: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Rule {
    pub source: String, // Entity Kind or Agent Role
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
    // Key: (Source, Relation, Target), Value: Template Content
    pub prompt_templates: HashMap<(String, String, String), String>,
    pub schemas: HashMap<String, String>, // Key: Entity Kind, Value: Schema Content
    pub loaded_agents: HashMap<String, String>, // Key: Role, Value: JSON Content
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
            schemas: HashMap::new(),
            loaded_agents: HashMap::new(),
        }
    }

    pub fn load_from_metamodel(json_content: &str) -> Result<Self> {
        let def: GraphDefinition = serde_json::from_str(json_content)?;
        let mut dg = Self::new();

        if let Some(defs) = def.definitions {
            for rule in defs.graph_rules.rules {
                let s_idx = dg.get_or_create_node(&rule.source);
                let t_idx = dg.get_or_create_node(&rule.target);
                dg.graph.add_edge(s_idx, t_idx, rule.relation.clone()); // Clone relation string

                // Infer Prompt Path: ontology/prompts/{Source}_{Relation}_{Target}.md
                let prompt_path_str = format!(
                    "ontology/prompts/{}_{}_{}.md",
                    rule.source, rule.relation, rule.target
                );

                let p = std::path::Path::new(&prompt_path_str);
                let paths_to_try = vec![
                    p.to_path_buf(),
                    std::path::Path::new("../..").join(p),
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

                if found {
                    let key = (
                        rule.source.clone(),
                        rule.relation.clone(),
                        rule.target.clone(),
                    );
                    dg.prompt_templates.insert(key, content);
                }

                // Also Load Schema for the Target Entity (Target IS Entity)
                // Convention: ontology/schemas/entities/{snake_case_target}.schema.json
                let target_entity = &rule.target;
                if !dg.schemas.contains_key(target_entity) {
                    let snake_case_target = Self::to_snake_case(target_entity);
                    let schema_path_str = format!(
                        "ontology/schemas/entities/{}.schema.json",
                        snake_case_target
                    );
                    let schema_p = std::path::Path::new(&schema_path_str);

                    let schema_paths_to_try = vec![
                        schema_p.to_path_buf(),
                        std::path::Path::new("../..").join(schema_p),
                        std::path::Path::new("..").join(schema_p),
                    ];

                    for path in schema_paths_to_try {
                        if let Ok(c) = std::fs::read_to_string(&path) {
                            dg.schemas.insert(target_entity.clone(), c);
                            break;
                        }
                    }
                }
            }
            // Load Agent Definitions
            if let Some(agent_defs) = defs.agents {
                for agent_def in agent_defs.agents {
                    let role = agent_def.role.clone();
                    let p = std::path::Path::new(&agent_def.config_ref);
                    let paths_to_try = vec![
                        p.to_path_buf(),
                        std::path::Path::new("../..").join(p),
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
                    if found {
                        dg.loaded_agents.insert(role, content);
                    } else {
                        eprintln!(
                            "Warning: Failed to load agent config {}: No such file in tried paths",
                            agent_def.config_ref
                        );
                    }
                }
            }
        }

        Ok(dg)
    }

    fn to_snake_case(s: &str) -> String {
        let mut result = String::new();
        for (i, c) in s.char_indices() {
            if c.is_uppercase() {
                if i > 0 {
                    result.push('_');
                }
                result.push(c.to_ascii_lowercase());
            } else {
                result.push(c);
            }
        }
        result
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

    pub fn get_prompt_template(
        &self,
        source: &str,
        relation: &str,
        target: &str,
    ) -> Option<String> {
        // Look up by (Source, Relation, Target)
        let template = self
            .prompt_templates
            .get(&(source.to_string(), relation.to_string(), target.to_string()))
            .cloned()?;

        // Inject Schema if present
        if let Some(schema_content) = self.schemas.get(target) {
            return Some(template.replace("{{schema}}", schema_content));
        }

        Some(template)
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
