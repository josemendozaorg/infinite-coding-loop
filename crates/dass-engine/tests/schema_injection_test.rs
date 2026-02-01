use dass_engine::graph::DependencyGraph;

#[test]
fn test_schema_injection_integration() {
    // 1. Create a minimal mock metamodel JSON
    // We reference a real schema from our spec to verify loading.
    // Let's use "Feature" as the target since it exists: "ontology/schemas/entities/feature.schema.json"
    // And we need a real prompt template file to test the substitution
    // But since the template loading relies on file existence, we might need a temporary file or reuse an existing one.
    // However, existing ones have been modified to contain {{schema}}.

    let metamodel_json = r#"{
        "entities": [],
        "relationships": [],
        "$defs": {
            "GraphRules": {
                "rules": [
                    {
                        "source": "Architect",
                        "relation": "creates",
                        "target": "DesignSpec"
                    }
                ]
            },
            "AgentDefinitions": { "agents": [] }
        }
    }"#;

    // 2. Load the Graph
    // This assumes we are running from the workspace root where `spec/` is accessible.
    // If running via `cargo test`, we usually are.
    let graph =
        DependencyGraph::load_from_metamodel(metamodel_json, None).expect("Failed to load graph");

    // 3. Verify Schema Loaded
    assert!(
        graph.schemas.contains_key("DesignSpec"),
        "Schema for DesignSpec should be loaded"
    );

    // 4. Get Prompt Template and Verify Injection
    let template = graph
        .get_prompt_template("Architect", "creates", "DesignSpec")
        .expect("Prompt template not found");

    println!("Loaded Template: {}", template);

    // 5. Assertions
    assert!(
        !template.contains("{{schema}}"),
        "The {{schema}} placeholder should be replaced"
    );

    // Check if it contains some characteristic of the DesignSpec schema
    assert!(
        template.contains("\"title\": \"DesignSpec\""),
        "Template should contain the DesignSpec schema definition"
    );
}

#[test]
fn test_source_differentiation() {
    let metamodel_json = r#"{
        "entities": [],
        "relationships": [],
        "$defs": {
            "GraphRules": {
                "rules": [
                    {
                        "source": "ProductManager",
                        "relation": "creates",
                        "target": "Requirement"
                    },
                    {
                        "source": "ProductManager",
                        "relation": "refines",
                        "target": "Requirement"
                    }
                ]
            }
        }
    }"#;

    // Load graph
    let graph =
        DependencyGraph::load_from_metamodel(metamodel_json, None).expect("Failed to load graph");

    // Verify PM creates gets PM creates prompt
    let creates_template = graph
        .get_prompt_template("ProductManager", "creates", "Requirement")
        .unwrap();
    // Verify PM refines gets PM refines prompt
    let refines_template = graph
        .get_prompt_template("ProductManager", "refines", "Requirement")
        .unwrap();

    assert_ne!(
        creates_template, refines_template,
        "Prompts should differ for different relations"
    );
}

#[test]
fn test_agent_loading_integration() {
    let metamodel_json = r#"{
        "entities": [],
        "relationships": [],
        "$defs": {
            "GraphRules": { "rules": [] },
            "AgentDefinitions": {
                "agents": [
                    {
                        "role": "ProductManager",
                        "config_ref": "agents/product_manager.json"
                    }
                ]
            }
        }
    }"#;

    let graph =
        DependencyGraph::load_from_metamodel(metamodel_json, None).expect("Failed to load graph");

    assert!(
        graph.loaded_agents.contains_key("ProductManager"),
        "ProductManager agent should be loaded"
    );
    let config = graph.loaded_agents.get("ProductManager").unwrap();
    assert!(
        config.contains("ProductManager"),
        "Config content should be loaded"
    );
}
