.PHONY: ontology test

ontology:
	@echo "Generating Ontology TTL from JSON Schema..."
	cargo run -p ontology-tools -- convert
	@echo "Verifying generated Ontology..."
	cargo run -p ontology-tools -- verify

test:
	cargo test
