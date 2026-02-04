use anyhow::Result;
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
            "improves" => Self::Refinement,
            "uses" | "contains" | "defines" | "constrains" | "requires" | "isA" | "applies" => {
                Self::Context
            }
            _ => Self::Other,
        }
    }

    pub fn is_actionable(&self) -> bool {
        matches!(self, Self::Creation | Self::Verification | Self::Refinement)
    }
}

// 1. Serialization Structs (mirroring ontology.json)
#[derive(Debug, Serialize, Deserialize)]
pub struct MetaRelationship {
    pub source: MetaEntity,
    pub target: MetaEntity,
    #[serde(rename = "type")]
    pub rel_type: MetaEntity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaEntity {
    pub name: String,
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
        // Try to parse as the new Array format
        let relationships: Vec<MetaRelationship> = serde_json::from_str(json_content)?;

        let mut dg = Self::new();

        // 0. Validate? (Optional, handled by serde type checking to some extent)
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let engine_meta_root = std::path::Path::new(manifest_dir).join("ontology/schemas/meta");

        // Load Schema defs for validation logic (Optional but kept for robustness)
        let mut meta_files = Vec::new();
        Self::find_json_files(&engine_meta_root, &mut meta_files);
        for path in meta_files {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(id) = json.get("$id").and_then(|v| v.as_str()) {
                        dg.schemas.insert(id.to_string(), content);
                    }
                }
            }
        }

        // 1. Load Schemas from artifact/schema (kept same)
        let root =
            base_path.unwrap_or_else(|| std::path::Path::new("ontology-software-engineering"));

        let schema_root = root.join("artifact/schema");
        let mut schema_files = Vec::new();
        Self::find_json_files(&schema_root, &mut schema_files);

        for path in schema_files {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Some(parent) = path.parent() {
                    if parent.ends_with("schema") {
                        if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
                            let entity_kind_snake = file_stem.trim_end_matches(".schema");
                            dg.schemas
                                .insert(entity_kind_snake.to_string(), content.clone());
                        }
                    }
                }
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(id) = json.get("$id").and_then(|v| v.as_str()) {
                        dg.schemas.insert(id.to_string(), content.clone());
                    }
                    if let Some(title) = json.get("title").and_then(|v| v.as_str()) {
                        dg.schemas.insert(title.to_string(), content.clone());
                    }
                }
            }
        }

        // 2. Load Relationship Prompts systematically (kept same)
        let rel_dir = root.join("relationship/prompt");
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

        // 3. Process Relationships (FROM ARRAY)
        for rel in relationships {
            let source_str = rel.source.name.clone();
            let target_str = rel.target.name.clone();
            let relation_str = rel.rel_type.name.clone();

            let s_idx = dg.get_or_create_node(&source_str);
            let t_idx = dg.get_or_create_node(&target_str);
            dg.graph.add_edge(s_idx, t_idx, relation_str.clone());

            // Infer Prompt Path
            let prompt_filename = format!("{}_{}_{}.md", source_str, relation_str, target_str);
            let p = root.join("relationship/prompt").join(&prompt_filename);

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
                let key = (source_str, relation_str, target_str);
                dg.prompt_templates.insert(key, content);
            }
        }

        // 4. Load Agents via Discovery (Since definitions are gone)
        // Ensure "Agent" role is known if used in graph?
        // We scan `agent/` folder for *.json files (or *.md based on migration?)
        // The user says "new JSON instance", but recent commits mentioned "agent/system_prompt (markdown)".
        // We should support both or check what's there.
        // Let's assume the new standard: Scan `agent/system_prompt/*.md` and use filename as Role.

        let agent_dir = root.join("agent/system_prompt"); // updated path based on history
        if let Ok(entries) = std::fs::read_dir(&agent_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("md") {
                    if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
                        let role = file_stem.to_string(); // e.g. "Architect"
                        dg.agent_roles.insert(role.clone());
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            // System prompt is the content
                            // We wrap it in JSON simply so the Executor generic agent can read it "as config"
                            // or we change GenericAgent to take raw string.
                            // Currently GenericAgent expects config with "system_prompt" field?
                            // Orchestrator::new_with_metamodel deserializes config as JSON to get system_prompt.
                            // So we should wrap it.
                            let config_wrapper = serde_json::json!({
                                "name": role,
                                "system_prompt": content
                            });
                            dg.loaded_agents.insert(role, config_wrapper.to_string());
                        }
                    }
                }
            }
        }

        // Fallback: Check old agent dir for JSONs just in case
        let agent_dir_legacy = root.join("agent");
        if let Ok(entries) = std::fs::read_dir(&agent_dir_legacy) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                            if let Some(name) = json.get("name").and_then(|v| v.as_str()) {
                                let role = name.to_string();
                                dg.agent_roles.insert(role.clone());
                                dg.loaded_agents.insert(role, content);
                            }
                        }
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

        // Build validator with all known schemas for resolution
        let mut options = jsonschema::JSONSchema::options();
        for (id, content) in &self.schemas {
            if id.starts_with("http") {
                if let Ok(sub_json) = serde_json::from_str::<serde_json::Value>(content) {
                    options.with_document(id.to_string(), sub_json);
                }
            }
        }

        let compiled = options
            .compile(&schema_json)
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

    fn find_json_files(dir: &std::path::Path, results: &mut Vec<std::path::PathBuf>) {
        if dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        Self::find_json_files(&path, results);
                    } else if path.extension().and_then(|s| s.to_str()) == Some("json") {
                        results.push(path);
                    }
                }
            }
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
                        { "source": { "name": "Agent" }, "target": { "name": "Feature" }, "relation": { "name": "creates" } },
                        { "source": { "name": "Feature" }, "target": { "name": "Requirement" }, "relation": { "name": "contains" } }
                    ]
                }
            }
        }"#;

        let graph = DependencyGraph::load_from_metamodel(json, None).expect("Failed to load graph");

        // Query: Who produces a Feature?
        let creators = graph.get_dependencies("Feature", "creates");
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
                        { "source": { "name": "A" }, "target": { "name": "B" }, "relation": { "name": "rel" } },
                        { "source": { "name": "C" }, "target": { "name": "D" }, "relation": { "name": "rel" } }
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
                        { "source": { "name": "A" }, "target": { "name": "B" }, "relation": { "name": "rel" } },
                        { "source": { "name": "B" }, "target": { "name": "A" }, "relation": { "name": "rel" } },
                        { "source": { "name": "C" }, "target": { "name": "D" }, "relation": { "name": "rel" } }
                    ]
                }
            }
        }"#;

        let result = DependencyGraph::load_from_metamodel(json, None);
        // Unreachable nodes/cycles are now allowed
        assert!(result.is_ok());
    }
}
