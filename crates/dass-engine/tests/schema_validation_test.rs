use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

// Integration tests often run in a separate crate context.
// Ensure jsonschema is available.

#[test]
fn validate_all_schemas_and_configs() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    // Navigate to workspace root from crates/dass-engine
    let workspace_root = Path::new(manifest_dir).parent().unwrap().parent().unwrap();
    let schemas_dir = workspace_root.join("ontology/schemas");
    let agents_dir = workspace_root.join("ontology/agents");

    // 1. Identify all schemas
    let mut schema_files = Vec::new();
    find_files(&schemas_dir, "schema.json", &mut schema_files);

    assert!(
        !schema_files.is_empty(),
        "No schemas found in {:?}",
        schemas_dir
    );

    // A. Validate Schema Integrity
    for schema_path in &schema_files {
        let content = fs::read_to_string(schema_path)
            .unwrap_or_else(|_| panic!("Failed to read {:?}", schema_path));
        let schema_json: Value = serde_json::from_str(&content)
            .unwrap_or_else(|_| panic!("Failed to parse JSON in {:?}", schema_path));

        // Use fully qualified path check
        // If this still fails, verify dependency.
        if let Err(e) = jsonschema::JSONSchema::compile(&schema_json) {
            panic!("Schema invalid {:?}: {}", schema_path, e);
        }
    }

    // B. Validate Agent Configs
    let mut options = jsonschema::JSONSchema::options();

    // Pre-load base and taxonomy
    let base_path = schemas_dir.join("base.schema.json");
    let taxonomy_path = schemas_dir.join("taxonomy.schema.json");

    let base_json: Value = serde_json::from_str(&fs::read_to_string(base_path).unwrap()).unwrap();
    let taxonomy_json: Value =
        serde_json::from_str(&fs::read_to_string(taxonomy_path).unwrap()).unwrap();

    options.with_document(
        "https://infinite-coding-loop.dass/schemas/base.schema.json".to_string(),
        base_json,
    );
    options.with_document(
        "https://infinite-coding-loop.dass/schemas/taxonomy.schema.json".to_string(),
        taxonomy_json,
    );

    let config_schema_path = schemas_dir.join("agent_config.schema.json");
    let config_schema_json: Value =
        serde_json::from_str(&fs::read_to_string(config_schema_path).unwrap()).unwrap();

    let agent_validator = options
        .compile(&config_schema_json)
        .expect("Failed to compile agent_config schema");

    let mut agent_files = Vec::new();
    find_files(&agents_dir, ".json", &mut agent_files);

    assert!(
        !agent_files.is_empty(),
        "No agent configs found in {:?}",
        agents_dir
    );

    for agent_path in agent_files {
        let content = fs::read_to_string(&agent_path).unwrap();
        let instance: Value = serde_json::from_str(&content)
            .unwrap_or_else(|_| panic!("Failed to parse agent JSON {:?}", agent_path));

        if let Err(errors) = agent_validator.validate(&instance) {
            // Explicitly collect errors to string to satisfy type inference
            let mut err_msgs = Vec::new();
            for e in errors {
                err_msgs.push(e.to_string());
            }
            panic!(
                "Agent config invalid {:?}:\n{}",
                agent_path,
                err_msgs.join("\n")
            );
        }
    }
}

fn find_files(dir: &Path, suffix: &str, results: &mut Vec<PathBuf>) {
    if dir.is_dir() {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries {
                if let Ok(entry) = entry {
                    let path = entry.path();
                    if path.is_dir() {
                        find_files(&path, suffix, results);
                    } else if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if name.ends_with(suffix) {
                            results.push(path);
                        }
                    }
                }
            }
        }
    }
}
