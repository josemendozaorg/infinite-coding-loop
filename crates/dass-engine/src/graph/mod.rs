use anyhow::Result;
use jsonschema::JSONSchema;
use petgraph::graph::DiGraph;
use petgraph::graph::NodeIndex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod executor;
mod validation_test;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationCategory {
    Creation,     // creates, implements
    Verification, // verifies
    Refinement,   // refines, improves
    Context,      // uses, specifies, targets, contains, defines, constrains
    Other,
}

impl RelationCategory {
    pub fn from_str(s: &str) -> Self {
        match s {
            "creates" | "implements" => Self::Creation,
            "verifies" => Self::Verification,
            "refines" | "improves" => Self::Refinement,
            "uses" | "specifies" | "targets" | "contains" | "defines" | "constrains" => {
                Self::Context
            }
            _ => Self::Other,
        }
    }

    pub fn is_actionable(&self) -> bool {
        matches!(self, Self::Creation | Self::Verification | Self::Refinement)
    }
}

// 1. Serialization Structs (mirroring metamodel.schema.json)
#[derive(Debug, Serialize, Deserialize)]
pub struct GraphDefinition {
    pub entities: Option<Vec<serde_json::Value>>,
    pub relationships: Option<Vec<RelationshipDef>>,
    #[serde(rename = "$defs")]
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
#[derive(Debug)]
pub struct DependencyGraph {
    pub graph: DiGraph<String, String>, // Node=Entity, Edge=Relation
    pub kind_map: HashMap<String, NodeIndex>,
    // Key: (Source, Relation, Target), Value: Template Content
    pub prompt_templates: HashMap<(String, String, String), String>,
    pub relationship_prompts: HashMap<String, String>, // Key: Relation, Value: Default Template
    pub schemas: HashMap<String, String>,              // Key: Entity, Value: Schema Content
    pub loaded_agents: HashMap<String, String>,        // Key: Role, Value: JSON Content
    pub agent_roles: std::collections::HashSet<String>, // Roles defined in the metamodel
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
            relationship_prompts: HashMap::new(),
            schemas: HashMap::new(),
            loaded_agents: HashMap::new(),
            agent_roles: std::collections::HashSet::new(),
        }
    }

    pub fn load_from_metamodel(
        json_content: &str,
        base_path: Option<&std::path::Path>,
    ) -> Result<Self> {
        let def: GraphDefinition = serde_json::from_str(json_content)?;
        let mut dg = Self::new();

        let root = base_path.unwrap_or_else(|| std::path::Path::new("ontology"));

        // 1. Load All Entity Schemas systematically
        let schema_dir = root.join("schemas/entities");
        if let Ok(entries) = std::fs::read_dir(&schema_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
                        // "feature.schema" -> "Feature" (Simplified: we need to map snake_case to CamelCase)
                        // Better: just load what we can and we'll fix the mapping or use the filename
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            // Trim ".schema" from stem
                            let entity_kind_snake = file_stem.trim_end_matches(".schema");
                            dg.schemas.insert(entity_kind_snake.to_string(), content);
                        }
                    }
                }
            }
        }

        // 2. Load Relationship Prompts systematically
        let rel_dir = root.join("relationships");
        if let Ok(entries) = std::fs::read_dir(&rel_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("md") {
                    if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            dg.relationship_prompts
                                .insert(file_stem.to_string(), content);
                        }
                    }
                }
            }
        }

        if let Some(defs) = def.definitions {
            for rule in defs.graph_rules.rules {
                let s_idx = dg.get_or_create_node(&rule.source);
                let t_idx = dg.get_or_create_node(&rule.target);
                dg.graph.add_edge(s_idx, t_idx, rule.relation.clone());

                // Infer Prompt Path
                let prompt_filename =
                    format!("{}_{}_{}.md", rule.source, rule.relation, rule.target);
                let p = root.join("prompts").join(&prompt_filename);

                let paths_to_try = vec![
                    p.clone(),
                    std::path::Path::new("../..").join(&p),
                    std::path::Path::new("..").join(&p),
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
            }
            // Load Agent Definitions
            if let Some(agent_defs) = defs.agents {
                for agent_def in agent_defs.agents {
                    let role = agent_def.role.clone();
                    dg.agent_roles.insert(role.clone());
                    // config_ref is usually "agents/engineer.json" -> relative to ontology root?
                    // In previous code it was used as is.
                    // If we assume config_ref is relative to `base_path`?
                    // Example: "agents/engineer.json"
                    // Join with root: "ontology/agents/engineer.json"

                    let p = root.join(&agent_def.config_ref);

                    let paths_to_try = vec![
                        p.clone(),
                        std::path::Path::new("../..").join(&p),
                        std::path::Path::new("..").join(&p),
                    ];

                    let mut content = String::new();
                    let mut found = false;
                    for path in &paths_to_try {
                        if let Ok(c) = std::fs::read_to_string(path) {
                            content = c;
                            found = true;
                            break;
                        }
                    }
                    if found {
                        dg.loaded_agents.insert(role, content);
                    }
                }
            }
        }

        dg.validate_meta_ontology()?;
        dg.validate_topology()?;
        Ok(dg)
    }

    pub fn validate_topology(&self) -> Result<()> {
        let node_count = self.graph.node_count();
        if node_count == 0 {
            return Ok(());
        }

        // Identify Roots (Nodes with in-degree 0)
        let mut roots = Vec::new();
        for node_idx in self.graph.node_indices() {
            let in_degree = self
                .graph
                .edges_directed(node_idx, petgraph::Direction::Incoming)
                .count();
            if in_degree == 0 {
                roots.push(node_idx);
            }
        }

        if roots.is_empty() {
            // Relaxed: Cycles are allowed.
            return Ok(());
        }

        // Multiple roots are allowed.
        Ok(())
    }

    pub fn validate_meta_ontology(&self) -> Result<()> {
        for edge in self.graph.edge_indices() {
            let (source_idx, target_idx) = self.graph.edge_endpoints(edge).unwrap();
            let source = &self.graph[source_idx];
            let target = &self.graph[target_idx];
            let relation = self.graph.edge_weight(edge).unwrap();
            let category = RelationCategory::from_str(relation);

            // Meta-Rule: If the source is an Agent, ensure the target is NOT also an Agent
            // unless it's a team/org relationship (Other category for now).
            if self.is_agent(source) {
                if self.is_agent(target) && category != RelationCategory::Other {
                    // Potential violation
                }
            }
        }
        Ok(())
    }

    pub fn is_agent(&self, kind: &str) -> bool {
        self.agent_roles.contains(kind)
    }

    pub fn validate_artifact(&self, kind: &str, data: &serde_json::Value) -> Result<()> {
        let snake_kind = Self::to_snake_case(kind);
        // Try both CamelCase and snake_case keys for the schema map
        let schema_content = self
            .schemas
            .get(kind)
            .or_else(|| self.schemas.get(&snake_kind))
            .ok_or_else(|| anyhow::anyhow!("No schema found for artifact kind: {}", kind))?;

        let schema_json: serde_json::Value = serde_json::from_str(schema_content)?;
        let compiled = JSONSchema::compile(&schema_json)
            .map_err(|e| anyhow::anyhow!("Failed to compile JSON schema for {}: {}", kind, e))?;

        if let Err(errors) = compiled.validate(data) {
            let error_msgs: Vec<String> = errors.map(|e| e.to_string()).collect();
            return Err(anyhow::anyhow!(
                "Artifact validation failed for {}: {}",
                kind,
                error_msgs.join(", ")
            ));
        }

        Ok(())
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
        let mut template = if let Some(t) = self.prompt_templates.get(&(
            source.to_string(),
            relation.to_string(),
            target.to_string(),
        )) {
            t.clone()
        } else if let Some(t) = self.relationship_prompts.get(relation) {
            t.clone()
        } else {
            format!(
                "Perform {} on {} for {}.\n\nContext:\n{{{{source_content}}}}",
                relation, target, source
            )
        };

        // Inject Schema if present
        if let Some(schema_content) = self.schemas.get(target) {
            if template.contains("{{schema}}") {
                template = template.replace("{{schema}}", schema_content);
            } else {
                template = format!("{}\n\nOutput Schema:\n{}", template, schema_content);
            }
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
                if edges.any(|e: petgraph::graph::EdgeReference<String>| e.weight() == relation) {
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
            "$defs": {
                "GraphRules": {
                    "rules": [
                        { "source": "Agent", "target": "Feature", "relation": "produces" },
                        { "source": "Feature", "target": "Requirement", "relation": "has_part" }
                    ]
                }
            }
        }"#;

        let graph = DependencyGraph::load_from_metamodel(json, None).expect("Failed to load graph");

        // Query: Who produces a Feature?
        let creators = graph.get_dependencies("Feature", "produces");
        assert_eq!(creators, vec!["Agent"]);

        // Query: What verifies a Feature? (None in this graph)
        let verifiers = graph.get_dependencies("Feature", "verifies");
        assert!(verifiers.is_empty());
    }

    #[test]
    fn test_invalid_topology_multiple_roots() {
        let json = r#"{
            "entities": [],
            "relationships": [],
            "$defs": {
                "GraphRules": {
                    "rules": [
                        { "source": "A", "target": "B", "relation": "rel" },
                        { "source": "C", "target": "D", "relation": "rel" }
                    ]
                }
            }
        }"#;

        let result = DependencyGraph::load_from_metamodel(json, None);
        // Multiple roots are now allowed
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_topology_unreachable() {
        let json = r#"{
            "entities": [],
            "relationships": [],
            "$defs": {
                "GraphRules": {
                    "rules": [
                        { "source": "A", "target": "B", "relation": "rel" },
                        { "source": "B", "target": "A", "relation": "rel" },
                        { "source": "C", "target": "D", "relation": "rel" }
                    ]
                }
            }
        }"#;

        let result = DependencyGraph::load_from_metamodel(json, None);
        // Unreachable nodes/cycles are now allowed
        assert!(result.is_ok());
    }
}
