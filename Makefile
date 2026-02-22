.PHONY: \
	help \
	install \
	build build-rust build-cli build-tui build-visualizer \
	test test-rust test-e2e \
	lint lint-rust lint-visualizer format \
	run-cli run-tui dev-visualizer \
	clean ontology

help:
	@echo "Available commands:"
	@echo "  make install             - Install NPM dependencies for workspaces"
	@echo "  make build               - Build everything (Rust crates & Visualizer)"
	@echo "  make test                - Run all tests (Rust & Visualizer)"
	@echo "  make test-e2e            - Run fast End-to-End integration tests"
	@echo "  make lint                - Run all linters (clippy & eslint)"
	@echo "  make format              - Auto-format Rust code"
	@echo "  make run-cli             - Run dass-cli locally"
	@echo "  make run-tui             - Run dass-tui locally"
	@echo "  make dev-visualizer      - Run ontology-visualizer in dev mode"
	@echo "  make clean               - Clean Cargo targets and NPM node_modules"
	@echo "  make ontology            - Generate and verify Ontology from JSON Schema"
	@echo ""
	@echo "See Makefile for more granular build, test, and run commands."

install:
	npm install

build: build-rust build-visualizer

build-rust:
	cargo build --release

build-cli:
	cargo build -p dass-cli --release

build-tui:
	cargo build -p dass-tui --release

build-visualizer:
	npm run build --workspace=ontology-visualizer

test: test-rust

test-rust:
	cargo test

test-e2e:
	@echo "Running slow End-to-End LLM integration tests..."
	cargo test -p dass-e2e --features e2e

lint: lint-rust lint-visualizer

lint-rust:
	cargo clippy --all-targets --all-features -- -D warnings

lint-visualizer:
	npm run lint --workspace=ontology-visualizer

format:
	cargo fmt --all

run-cli:
	cargo run -p dass-cli

run-tui:
	cargo run -p dass-tui

dev-visualizer:
	npm run dev --workspace=ontology-visualizer

clean:
	cargo clean
	rm -rf node_modules apps/*/node_modules packages/*/node_modules ontologies/*/node_modules

ontology:
	@echo "Generating Ontology TTL from JSON Schema..."
	cargo run -p ontology-tools -- convert
	@echo "Verifying generated Ontology..."
	cargo run -p ontology-tools -- verify
