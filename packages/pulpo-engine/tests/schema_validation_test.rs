use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

// Integration tests often run in a separate crate context.
// Ensure jsonschema is available.

#[test]
fn validate_all_schemas_and_configs() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    // Navigate to workspace root from crates/pulpo-engine
    let workspace_root = Path::new(manifest_dir).parent().unwrap().parent().unwrap();
    let domain_ontology_dir = workspace_root.join("pulpo-ontologies/software-engineering");
    let artifact_schema_dir = domain_ontology_dir.join("artifact/schema");
    let meta_dir = workspace_root.join("packages/pulpo-schema/meta");

    // 1. Identify all schemas
    let mut schema_files = Vec::new();
    find_files(&artifact_schema_dir, "schema.json", &mut schema_files);

    assert!(
        !schema_files.is_empty(),
        "No schemas found in {:?} or its subdirectories",
        artifact_schema_dir
    );

    // 2. Pre-load resources into options
    let mut options = jsonschema::options();

    // Load Taxonomy
    let taxonomy_path = artifact_schema_dir.join("taxonomy.schema.json");
    if taxonomy_path.exists() {
        let taxonomy_json: Value =
            serde_json::from_str(&fs::read_to_string(taxonomy_path).unwrap()).unwrap();
        options.with_resource(
            "https://pulpo.dev/schemas/taxonomy.schema.json",
            jsonschema::Resource::from_contents(taxonomy_json).unwrap(),
        );
    }

    // Load Meta Schemas
    let meta_configs = [
        (
            "base.schema.json",
            "https://pulpo.dev/schemas/meta/base.schema.json",
        ),
        (
            "ontology.schema.json",
            "https://pulpo.dev/schemas/meta/ontology.schema.json",
        ),
        (
            "agent.schema.json",
            "https://pulpo.dev/schemas/meta/agent.schema.json",
        ),
    ];

    for (filename, id) in meta_configs {
        let path = meta_dir.join(filename);
        if path.exists() {
            let json: Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
            options.with_resource(
                id,
                jsonschema::Resource::from_contents(json.clone()).unwrap(),
            );
            // ALIAS HACK: The domain ontology (e.g. Requirement.schema.json) refers to "../base.schema.json",
            // which resolves to "https://pulpo.dev/schemas/base.schema.json".
            if filename == "base.schema.json" {
                options.with_resource(
                    "https://pulpo.dev/schemas/base.schema.json",
                    jsonschema::Resource::from_contents(json).unwrap(),
                );
            }
        }
    }

    // 3. Validate Schema Integrity and Cross-References
    for schema_path in &schema_files {
        let content = fs::read_to_string(schema_path)
            .unwrap_or_else(|_| panic!("Failed to read {:?}", schema_path));
        let schema_json: Value = serde_json::from_str(&content)
            .unwrap_or_else(|_| panic!("Failed to parse JSON in {:?}", schema_path));

        if let Err(e) = options.build(&schema_json) {
            panic!("Schema invalid {:?}: {}", schema_path, e);
        }
    }
}

fn find_files(dir: &Path, suffix: &str, results: &mut Vec<PathBuf>) {
    if dir.is_dir()
        && let Ok(entries) = fs::read_dir(dir)
    {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                find_files(&path, suffix, results);
            } else if let Some(name) = path.file_name().and_then(|n| n.to_str())
                && name.ends_with(suffix)
            {
                results.push(path);
            }
        }
    }
}
