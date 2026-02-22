.PHONY: ontology test test-e2e

ontology:
	@echo "Generating Ontology TTL from JSON Schema..."
	cargo run -p ontology-tools -- convert
	@echo "Verifying generated Ontology..."
	cargo run -p ontology-tools -- verify

test:
	cargo test

test-e2e:
	@echo "Running slow End-to-End LLM integration tests..."
	cargo test -p dass-e2e --features e2e
