use pulpo_engine::graph::DependencyGraph;

#[test]
fn test_schema_injection_integration() {
    // 1. Create a minimal mock metamodel JSON
    // We reference a real schema from our spec to verify loading.
    // Let's use "Feature" as the target since it exists: "ontology/schemas/entities/feature.schema.json"
    // And we need a real prompt template file to test the substitution
    // But since the template loading relies on file existence, we might need a temporary file or reuse an existing one.
    // However, existing ones have been modified to contain {{schema}}.

    let metamodel_json = r#"[
        {
            "source": { "name": "Architect" },
            "type": { "name": "creates", "verbType": "Creation" },
            "target": { "name": "DesignSpec" }
        }
    ]"#;

    // 2. Load the Graph
    // Build path relative to workspace root
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let workspace_root = std::path::Path::new(manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let ontology_path = workspace_root.join("pulpo-ontologies/software-engineering");

    let graph = DependencyGraph::load_from_metamodel(metamodel_json, Some(&ontology_path))
        .expect("Failed to load graph");

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
    // Note: Schema injection now happens in Orchestrator::enhance_prompt,
    // so the template returned by DependencyGraph no longer contains the schema.
    assert!(
        !template.contains("{{schema}}"),
        "The {{schema}} placeholder should not be present if it wasn't in the template"
    );
    assert!(
        !template.contains("\"title\": \"DesignSpec\""),
        "Template should NOT contain the DesignSpec schema definition here (it's injected in Orchestrator)"
    );
}

#[test]
fn test_source_differentiation() {
    let metamodel_json = r#"[
        {
            "source": { "name": "ProductManager" },
            "type": { "name": "creates", "verbType": "Creation" },
            "target": { "name": "Requirement" }
        },
        {
            "source": { "name": "ProductManager" },
            "type": { "name": "refines", "verbType": "Refinement" },
            "target": { "name": "Requirement" }
        }
    ]"#;

    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let workspace_root = std::path::Path::new(manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let ontology_path = workspace_root.join("pulpo-ontologies/software-engineering");

    // Load graph
    let graph = DependencyGraph::load_from_metamodel(metamodel_json, Some(&ontology_path))
        .expect("Failed to load graph");

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
    let metamodel_json = "[]";

    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let workspace_root = std::path::Path::new(manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let ontology_path = workspace_root.join("pulpo-ontologies/software-engineering");

    let graph = DependencyGraph::load_from_metamodel(metamodel_json, Some(&ontology_path))
        .expect("Failed to load graph");

    assert!(
        graph.loaded_agents.contains_key("ProductManager"),
        "ProductManager agent should be loaded"
    );
    let config = graph.loaded_agents.get("ProductManager").unwrap();
    assert!(
        config.contains("Product Manager"),
        "Config content should be loaded"
    );
}
