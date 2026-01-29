use std::env;
use std::fs;
use std::path::Path;
use typify::{TypeSpace, TypeSpaceSettings};
use walkdir::WalkDir;

fn main() {
    // 1. Setup Input/Output paths
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let schema_dir = Path::new(&manifest_dir).join("../../spec/schemas/entities");
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("domain_types.rs");

    // 2. Rerun if schemas change
    println!("cargo:rerun-if-changed={}", schema_dir.display());

    // 3. Initialize TypeSpace
    let mut settings = TypeSpaceSettings::default();
    settings.with_struct_builder(true);

    let mut type_space = TypeSpace::new(&settings);

    // 4. Pre-load key shared schemas (Taxonomy)
    let taxonomy_path = Path::new(&manifest_dir).join("../../spec/schemas/taxonomy.schema.json");
    if taxonomy_path.exists() {
        let content = fs::read_to_string(taxonomy_path).expect("Failed to read taxonomy schema");
        let mut schema_json: serde_json::Value =
            serde_json::from_str(&content).expect("Invalid Taxonomy JSON");

        // Fix: Remove 'oneOf' to avoid external reference errors in typify
        if let Some(obj) = schema_json.as_object_mut() {
            obj.remove("oneOf");
        }

        let root_schema: schemars::schema::RootSchema =
            serde_json::from_value(schema_json).expect("Invalid Taxonomy Schema");
        type_space
            .add_root_schema(root_schema)
            .expect("Failed to add Taxonomy to TypeSpace");
    }

    // 5. Iterate over entity files
    for entry in WalkDir::new(&schema_dir)
        .sort_by_file_name()
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            let content = fs::read_to_string(path).expect("Failed to read schema");
            let root_schema: schemars::schema::RootSchema =
                serde_json::from_str(&content).expect("Invalid JSON Schema");

            // Register schema
            type_space
                .add_root_schema(root_schema)
                .expect("Failed to add schema to TypeSpace");
        }
    }

    // 5. Generate Code
    let contents = type_space.to_stream().to_string();
    fs::write(&dest_path, contents).expect("Failed to write generated code");
}
