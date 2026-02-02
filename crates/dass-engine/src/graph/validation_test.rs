#[cfg(test)]
mod tests {
    use crate::graph::DependencyGraph;
    use serde_json::json;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_artifact_validation() {
        let dir = tempdir().expect("Failed to create temp dir");
        let entities_dir = dir.path().join("schemas/entities");
        fs::create_dir_all(&entities_dir).expect("Failed to create entities dir");

        let schema = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "count": { "type": "number" }
            },
            "required": ["name"]
        });
        fs::write(
            entities_dir.join("test_artifact.schema.json"),
            schema.to_string(),
        )
        .expect("Failed to write schema");

        let metamodel = json!({
            "entities": [],
            "relationships": [],
            "$defs": {
                "GraphRules": { "rules": [] }
            }
        });

        let dg = DependencyGraph::load_from_metamodel(&metamodel.to_string(), Some(dir.path()))
            .expect("Failed to load graph");

        // Valid data
        let valid_data = json!({ "name": "test", "count": 1 });
        assert!(dg.validate_artifact("TestArtifact", &valid_data).is_ok());

        // Invalid data (missing required field)
        let invalid_data = json!({ "count": 1 });
        let result = dg.validate_artifact("TestArtifact", &invalid_data);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("is a required property")
        );

        // Invalid data (wrong type)
        let invalid_type = json!({ "name": 1 });
        let result = dg.validate_artifact("TestArtifact", &invalid_type);
        assert!(result.is_err());
    }
}
