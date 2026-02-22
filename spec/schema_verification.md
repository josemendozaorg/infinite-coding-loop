# Schema Verification Guide

This document outlines the procedure for verifying the integrity of the ontology schemas and agent configurations within the `ontology/` directory.

## Prerequisite
Ensure you are in the project root.

## Verification Command
To validate all JSON schemas (entities, metamodel, taxonomy) and ensure all agent configurations conform to the `agent_config.schema.json`, run the following command:

```bash
cargo test -p pulpo-engine --test schema_validation_test
```

## What This Tests
1. **Schema Integrity**: Compiles every `.schema.json` file in `ontology/schemas` to ensure valid JSON Schema syntax.
2. **Agent Configuration**: Validates every `.json` file in `ontology/agents` against the `agent_config.schema.json`.

## Troubleshooting
If the test fails, check the output for specific validation errors. Common issues include:
- Missing required fields in agent configs.
- Invalid JSON syntax in schema files.
- References to non-existent schemas.
