use anyhow::Result;
use petgraph::graph::DiGraph;
use petgraph::graph::NodeIndex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod executor;
mod validation_test;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationCategory {
    Creation,     // creates, implements
    Verification, // verifies
    Refinement,   // refines, improves
    Dependency,   // requires
    Context,      // uses, isA, defines, contains
}

impl RelationCategory {
    pub fn from_verb_type(s: &str) -> Self {
        match s {
            "Creation" => Self::Creation,
            "Verification" => Self::Verification,
            "Refinement" => Self::Refinement,
            "Dependency" => Self::Dependency,
            _ => Self::Context,
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
    pub rel_type: MetaVerb,
    #[serde(rename = "loop")]
    pub loop_config: Option<LoopConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaVerb {
    pub name: String,
    #[serde(rename = "verbType")]
    pub verb_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopConfig {
    #[serde(rename = "maxRetries", default = "LoopConfig::default_max_retries")]
    pub max_retries: usize,
    #[serde(
        rename = "passThreshold",
        default = "LoopConfig::default_pass_threshold"
    )]
    pub pass_threshold: f64,
}

impl Default for LoopConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            pass_threshold: 1.0,
        }
    }
}

impl LoopConfig {
    fn default_max_retries() -> usize {
        3
    }
    fn default_pass_threshold() -> f64 {
        1.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaEntity {
    pub name: String,
    #[serde(rename = "type")]
    pub entity_type: Option<String>,
    #[serde(rename = "modelType")]
    pub model_type: Option<String>,
    pub model: Option<String>,
    #[serde(rename = "aiCli")]
    pub ai_cli: Option<String>,
}

// 2. The In-Memory Graph
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    pub graph: DiGraph<String, String>, // Node=Entity, Edge=Relation
    pub kind_map: HashMap<String, NodeIndex>,
    // Key: (Source, Relation, Target), Value: Template Content
    pub prompt_templates: HashMap<(String, String, String), String>,
    pub relationship_prompts: HashMap<String, String>, // Key: Relation, Value: Default Template
    pub schemas: HashMap<String, String>,              // Key: Entity, Value: Schema Content
    pub loaded_agents: HashMap<String, String>,        // Key: Role, Value: JSON Content
    pub agent_roles: std::collections::HashSet<String>, // Roles defined in the metamodel
    pub node_types: HashMap<String, String>, // Key: Entity Name, Value: Type (e.g. "Code", "Agent")
    // Data-driven verb categories (replaces hardcoded from_str matching)
    pub edge_categories: HashMap<(String, String, String), RelationCategory>,
    pub loop_configs: HashMap<(String, String, String), LoopConfig>,
    pub node_configs: HashMap<String, MetaEntity>, // Key: Entity Name, Value: MetaEntity
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
            node_types: HashMap::new(),
            edge_categories: HashMap::new(),
            loop_configs: HashMap::new(),
            node_configs: HashMap::new(),
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

        // Load Meta Schemas for validation
        let mut meta_files = Vec::new();
        Self::find_json_files(&engine_meta_root, &mut meta_files);
        for path in meta_files {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(id) = json.get("$id").and_then(|v| v.as_str()) {
                        dg.schemas.insert(id.to_string(), content.clone());
                    }
                    if let Some(title) = json.get("title").and_then(|v| v.as_str()) {
                        dg.schemas.insert(title.to_string(), content);
                    }
                }
            }
        }

        // ALIAS HACK: The domain ontology (e.g. Requirement.schema.json) refers to "../base.schema.json",
        // which resolves to "https://infinite-coding-loop.dass/schemas/base.schema.json".
        // But the actual file has ID "https://infinite-coding-loop.dass/schemas/meta/base.schema.json".
        // We register the alias so resolution works.
        if let Some(content) = dg
            .schemas
            .get("https://infinite-coding-loop.dass/schemas/meta/base.schema.json")
            .cloned()
        {
            dg.schemas.insert(
                "https://infinite-coding-loop.dass/schemas/base.schema.json".to_string(),
                content,
            );
        }

        // Validate metamodel against ontology.schema.json
        if let Some(schema_content) = dg
            .schemas
            .get("https://infinite-coding-loop.dass/schemas/meta/ontology.schema.json")
        {
            let schema_json: serde_json::Value = serde_json::from_str(schema_content)?;
            let mut options = jsonschema::JSONSchema::options();
            // Add base.schema.json to options for resolution
            if let Some(base_content) = dg
                .schemas
                .get("https://infinite-coding-loop.dass/schemas/meta/base.schema.json")
            {
                let base_json: serde_json::Value = serde_json::from_str(base_content)?;
                options.with_document(
                    "https://infinite-coding-loop.dass/schemas/meta/base.schema.json".to_string(),
                    base_json,
                );
            }

            let compiled = options
                .compile(&schema_json)
                .map_err(|e| anyhow::anyhow!("Failed to compile Meta-Ontology schema: {}", e))?;

            let instance: serde_json::Value = serde_json::from_str(json_content)?;
            if let Err(errors) = compiled.validate(&instance) {
                let error_msg = errors.map(|e| e.to_string()).collect::<Vec<_>>().join(", ");
                return Err(anyhow::anyhow!(
                    "Metamodel validation against Meta-Ontology failed: {}",
                    error_msg
                ));
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
                // Infer entity name from filename
                if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
                    // Typical pattern: "entity_name.schema.json" -> "EntityName" (if we could, but here we just store snake case key)
                    // The validate_artifact method tries both Original and snake_case, so snake_case key is good.
                    let entity_kind_snake = file_stem.trim_end_matches(".schema");

                    // Store strict key
                    dg.schemas
                        .insert(entity_kind_snake.to_string(), content.clone());

                    // Also store CamelCase if title depends on it?
                    // No, reliance is on `validate_artifact` converting Kind -> Snake.

                    // BUT, `validate_topology` checks keys directly against Node Name (CamelCase).
                    // So we must convert snake to Camel or store both?
                    // Let's store a normalized "CamelCase" version if possible?
                    // Hard to convert snake to Camel without logic.
                    // Easier to store the file_stem as key? "user_story"

                    // Let's rely on parsing the Title from JSON if available, or just the filename map.
                }

                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(id) = json.get("$id").and_then(|v| v.as_str()) {
                        dg.schemas.insert(id.to_string(), content.clone());
                    }
                    if let Some(title) = json.get("title").and_then(|v| v.as_str()) {
                        // Title is usually CamelCase (e.g. "UserStory")
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
            let verb_type = rel.rel_type.verb_type.clone();

            let s_idx = dg.get_or_create_node(&source_str);
            let t_idx = dg.get_or_create_node(&target_str);
            dg.graph.add_edge(s_idx, t_idx, relation_str.clone());

            // Store edge category from verbType (data-driven, no hardcoded matching)
            let edge_key = (source_str.clone(), relation_str.clone(), target_str.clone());
            let category = RelationCategory::from_verb_type(&verb_type);
            dg.edge_categories.insert(edge_key.clone(), category);

            // Store loop config if present
            if let Some(lc) = rel.loop_config {
                dg.loop_configs.insert(edge_key.clone(), lc);
            }

            // Capture Node Types
            if let Some(t) = rel.source.entity_type.clone() {
                dg.node_types.insert(source_str.clone(), t);
            }
            if let Some(t) = rel.target.entity_type.clone() {
                dg.node_types.insert(target_str.clone(), t);
            }

            // Capture Node Configs (containing model overrides)
            dg.node_configs
                .insert(source_str.clone(), rel.source.clone());
            dg.node_configs
                .insert(target_str.clone(), rel.target.clone());

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

        let agent_dir = root.join("agent/system_prompt");
        if let Ok(entries) = std::fs::read_dir(&agent_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("md") {
                    if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
                        // file_stem is usually "Role" (e.g. "Architect") or "Role_system_prompt"
                        // Current format seems to be just "Architect.md"?
                        // Let's assume filename is the Role.
                        let role = file_stem.to_string();
                        dg.agent_roles.insert(role.clone());
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            // Construct a JSON config compatible with GenericAgent
                            // GenericAgent expects a JSON with "system_prompt" field.
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
        // Ensure all nodes are known either as Agents or Artifacts (have a schema)
        for node_idx in self.graph.node_indices() {
            let name = &self.graph[node_idx];

            // Check implicit or explicit types
            let is_agent = self.is_agent(name);
            let has_schema = self.schemas.contains_key(name)
                || self.schemas.contains_key(&Self::to_snake_case(name))
                || self
                    .schemas
                    .keys()
                    .any(|k| k.to_lowercase() == name.to_lowercase());

            // Check if explicitly typed as "Other" or "Code" (which might not have schema yet? Code should have schema now)
            let node_type = self.node_types.get(name).map(|s| s.as_str());
            let is_other = node_type == Some("Other");

            // If it's not an agent, not an artifact (no schema), and NOT explicitly "Other", then warn.
            if !is_agent && !has_schema && !is_other && name != "SoftwareApplication" {
                println!(
                    "Warning: Entity '{}' is neither an Agent nor has a known Artifact schema (Type: {:?})",
                    name, node_type
                );
            }
        }

        for edge in self.graph.edge_indices() {
            let (source_idx, target_idx) = self.graph.edge_endpoints(edge).unwrap();
            let source = &self.graph[source_idx];
            let target = &self.graph[target_idx];
            let relation = self.graph.edge_weight(edge).unwrap();
            let edge_key = (source.to_string(), relation.to_string(), target.to_string());
            let category = self
                .edge_categories
                .get(&edge_key)
                .copied()
                .unwrap_or(RelationCategory::Context);

            // Rule: Agent -(Creation)-> Artifact
            if self.is_agent(source) && category == RelationCategory::Creation {
                if self.is_agent(target) {
                    return Err(anyhow::anyhow!(
                        "Agent '{}' cannot 'create' another Agent '{}'",
                        source,
                        target
                    ));
                }
            }

            // Rule: Agent -(Verification)-> Artifact
            if self.is_agent(source) && category == RelationCategory::Verification {
                if self.is_agent(target) {
                    return Err(anyhow::anyhow!(
                        "Agent '{}' cannot 'verify' another Agent '{}'",
                        source,
                        target
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn is_agent(&self, kind: &str) -> bool {
        self.agent_roles.contains(kind)
            || self.agent_roles.contains(&kind.to_lowercase())
            || self.agent_roles.contains(&Self::to_snake_case(kind))
    }

    pub fn validate_artifact(&self, kind: &str, data: &serde_json::Value) -> Result<()> {
        let snake_kind = Self::to_snake_case(kind);
        // Try both CamelCase and snake_case keys for the schema map
        let schema_content = self
            .schemas
            .get(kind)
            .or_else(|| self.schemas.get(&snake_kind));

        if schema_content.is_none() {
            let node_type = self.node_types.get(kind).map(|s| s.as_str());
            if node_type == Some("Other") || kind == "SoftwareApplication" {
                return Ok(());
            }
            return Err(anyhow::anyhow!(
                "No schema found for artifact kind: {}",
                kind
            ));
        }

        let schema_content = schema_content.unwrap();

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
            // Support for Array of Artifacts:
            // If the data is an Array, and the schema failed (presumably because it expects an Object),
            // try validating each item in the array against the schema.
            if let Some(arr) = data.as_array() {
                if !arr.is_empty() {
                    let mut all_valid = true;
                    let mut array_errors = Vec::new();

                    for (i, item) in arr.iter().enumerate() {
                        if let Err(item_errors) = compiled.validate(item) {
                            all_valid = false;
                            let msg = item_errors
                                .map(|e| e.to_string())
                                .collect::<Vec<_>>()
                                .join(", ");
                            array_errors.push(format!("Item {}: {}", i, msg));
                        }
                    }

                    if all_valid {
                        return Ok(());
                    } else {
                        return Err(anyhow::anyhow!(
                            "Artifact validation failed for Array of {}: {}",
                            kind,
                            array_errors.join("; ")
                        ));
                    }
                }
            }

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
        let template = if let Some(t) = self.prompt_templates.get(&(
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

        Some(template)
    }

    pub fn get_related_artifacts(&self, pk: &str) -> Vec<String> {
        let mut related = Vec::new();

        use petgraph::visit::EdgeRef;

        if let Some(&node_idx) = self.kind_map.get(pk) {
            // Check incoming edges (e.g. "Spec defines Architecture")
            for edge in self
                .graph
                .edges_directed(node_idx, petgraph::Direction::Incoming)
            {
                let source_idx = edge.source();
                let source_name = &self.graph[source_idx];
                // Only artifacts (nodes with schemas or explicitly not agents)
                if !self.is_agent(source_name) {
                    related.push(source_name.clone());
                }
            }

            // Check outgoing edges (e.g. "Feature requires Requirement")
            for edge in self
                .graph
                .edges_directed(node_idx, petgraph::Direction::Outgoing)
            {
                let target_idx = edge.target();
                let target_name = &self.graph[target_idx];
                if !self.is_agent(target_name) {
                    related.push(target_name.clone());
                }
            }
        }

        // Dedup
        related.sort();
        related.dedup();
        related
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_and_query_graph() {
        let json = r#"[
            { "source": { "name": "Agent" }, "target": { "name": "Feature" }, "type": { "name": "creates", "verbType": "Creation" } },
            { "source": { "name": "Feature" }, "target": { "name": "Requirement" }, "type": { "name": "contains", "verbType": "Context" } }
        ]"#;

        let mut graph =
            DependencyGraph::load_from_metamodel(json, None).expect("Failed to load graph");
        graph.agent_roles.insert("Agent".to_string());

        // Query: What is related to Feature?
        // Feature contains Requirement -> Requirement is related
        // Agent creates Feature -> Agent is NOT related (as per is_agent filter in get_related_artifacts)
        // Wait, current impl filters out Agents. So only Requirement should be returned.

        let related = graph.get_related_artifacts("Feature");
        assert_eq!(related, vec!["Requirement"]);

        // Query: What is related to Requirement?
        // Feature contains Requirement -> Feature is related (incoming edge)
        let related_req = graph.get_related_artifacts("Requirement");
        assert_eq!(related_req, vec!["Feature"]);
    }

    #[test]
    fn test_invalid_topology_multiple_roots() {
        let json = r#"[
            { "source": { "name": "A" }, "target": { "name": "B" }, "type": { "name": "rel", "verbType": "Context" } },
            { "source": { "name": "C" }, "target": { "name": "D" }, "type": { "name": "rel", "verbType": "Context" } }
        ]"#;

        let result = DependencyGraph::load_from_metamodel(json, None);
        // Multiple roots are now allowed
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_topology_unreachable() {
        let json = r#"[
            { "source": { "name": "A" }, "target": { "name": "B" }, "type": { "name": "rel", "verbType": "Context" } },
            { "source": { "name": "B" }, "target": { "name": "A" }, "type": { "name": "rel", "verbType": "Context" } },
            { "source": { "name": "C" }, "target": { "name": "D" }, "type": { "name": "rel", "verbType": "Context" } }
        ]"#;

        let result = DependencyGraph::load_from_metamodel(json, None);
        // Unreachable nodes/cycles are now allowed
        assert!(result.is_ok());
    }
}
