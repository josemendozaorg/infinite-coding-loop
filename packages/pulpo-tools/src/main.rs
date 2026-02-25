use clap::{Parser, Subcommand};
use rio_api::formatter::TriplesFormatter;
use rio_api::model::{NamedNode, Subject, Term, Triple};
use rio_turtle::{TurtleFormatter, TurtleParser};
use serde::Deserialize;
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Convert JSON Schema metamodel to Turtle ontology
    Convert {
        #[arg(
            short,
            long,
            default_value = "pulpo-ontologies/software-engineering/ontology.json"
        )]
        input: PathBuf,
        #[arg(
            short,
            long,
            default_value = "pulpo-ontologies/software-engineering/ontology.ttl"
        )]
        output: PathBuf,
    },
    /// Verify a Turtle file syntax
    Verify {
        #[arg(
            short,
            long,
            default_value = "pulpo-ontologies/software-engineering/ontology.ttl"
        )]
        input: PathBuf,
    },
    /// Validate the graph topology of the metamodel
    Validate {
        #[arg(
            short,
            long,
            default_value = "pulpo-ontologies/software-engineering/ontology.json"
        )]
        input: PathBuf,
    },
    /// Predict and print the execution path from the ontology
    Plan {
        #[arg(
            short,
            long,
            default_value = "pulpo-ontologies/software-engineering/ontology.json"
        )]
        input: PathBuf,
    },
}

#[derive(Deserialize, Debug)]
struct MetaRelationship {
    source: MetaEntity,
    target: MetaEntity,
    #[serde(rename = "type")]
    rel_type: MetaEntity,
}

#[derive(Deserialize, Debug)]
struct MetaEntity {
    name: String,
}

const BASE_IRI: &str = "https://pulpo.dev/ontology/";

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Convert { input, output } => {
            convert(&input, &output)?;
        }
        Commands::Verify { input } => {
            verify(&input)?;
        }
        Commands::Validate { input } => {
            validate_graph(&input)?;
        }
        Commands::Plan { input } => {
            simulate_path(&input)?;
        }
    }

    Ok(())
}

fn convert(input_path: &PathBuf, output_path: &PathBuf) -> anyhow::Result<()> {
    println!("Reading schema from {:?}", input_path);
    let file = File::open(input_path)?;
    let reader = BufReader::new(file);
    let relationships: Vec<MetaRelationship> = serde_json::from_reader(reader)?;

    println!("Writing ontology to {:?}", output_path);
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let outfile = File::create(output_path)?;
    let writer = BufWriter::new(outfile);
    let mut formatter = TurtleFormatter::new(writer);

    // Common Namespaces
    let rdf_type = NamedNode {
        iri: "http://www.w3.org/1999/02/22-rdf-syntax-ns#type",
    };
    let owl_class = NamedNode {
        iri: "http://www.w3.org/2002/07/owl#Class",
    };
    let owl_obj_prop = NamedNode {
        iri: "http://www.w3.org/2002/07/owl#ObjectProperty",
    };
    let rdfs_domain = NamedNode {
        iri: "http://www.w3.org/2000/01/rdf-schema#domain",
    };
    let rdfs_range = NamedNode {
        iri: "http://www.w3.org/2000/01/rdf-schema#range",
    };

    let mut classes = HashSet::new();
    let mut properties = HashSet::new();

    for rel in relationships {
        let s_name = rel.source.name;
        let t_name = rel.target.name;
        let r_name = rel.rel_type.name;

        let s_iri = format!("{}{}", BASE_IRI, s_name);
        let t_iri = format!("{}{}", BASE_IRI, t_name);
        let r_iri = format!("{}{}", BASE_IRI, r_name);

        // Declare Source Class
        if classes.insert(s_name.clone()) {
            formatter.format(&Triple {
                subject: Subject::NamedNode(NamedNode { iri: &s_iri }),
                predicate: rdf_type,
                object: Term::NamedNode(owl_class),
            })?;
        }

        // Declare Target Class
        if classes.insert(t_name.clone()) {
            formatter.format(&Triple {
                subject: Subject::NamedNode(NamedNode { iri: &t_iri }),
                predicate: rdf_type,
                object: Term::NamedNode(owl_class),
            })?;
        }

        // Declare Property
        if properties.insert(r_name.clone()) {
            formatter.format(&Triple {
                subject: Subject::NamedNode(NamedNode { iri: &r_iri }),
                predicate: rdf_type,
                object: Term::NamedNode(owl_obj_prop),
            })?;
        }

        // Domain and Range
        formatter.format(&Triple {
            subject: Subject::NamedNode(NamedNode { iri: &r_iri }),
            predicate: rdfs_domain,
            object: Term::NamedNode(NamedNode { iri: &s_iri }),
        })?;

        formatter.format(&Triple {
            subject: Subject::NamedNode(NamedNode { iri: &r_iri }),
            predicate: rdfs_range,
            object: Term::NamedNode(NamedNode { iri: &t_iri }),
        })?;
    }

    formatter.finish()?;
    println!("Successfully wrote ontology.");
    Ok(())
}

fn verify(input_path: &PathBuf) -> anyhow::Result<()> {
    use rio_api::parser::TriplesParser;

    println!("Verifying syntax of {:?}", input_path);
    let file = File::open(input_path)?;
    let reader = BufReader::new(file);
    let mut parser = TurtleParser::new(reader, Some(BASE_IRI.parse()?));

    let mut count = 0;
    while !parser.is_end() {
        if let Err(e) = parser.parse_step(&mut |_| -> Result<(), rio_turtle::TurtleError> {
            count += 1;
            Ok(())
        }) {
            eprintln!("Verification FAILED: {}", e);
            std::process::exit(1);
        }
    }

    println!("Verification passed. Read {} triples.", count);
    Ok(())
}

fn validate_graph(input_path: &PathBuf) -> anyhow::Result<()> {
    use pulpo_engine::graph::DependencyGraph;
    println!("Validating graph topology of {:?}", input_path);

    let content = std::fs::read_to_string(input_path)?;
    // Derive base_path from input_path (ignoring "schemas" or "artifact/schema" intermediate dirs if possible)
    // The convention is that `input_path` is usually `ontology/artifact/schema/metamodel.schema.json`.
    // The base path should be `ontology/`.
    // Let's try to infer it. If input has parent, use it.
    let base_path = input_path.parent().map(|p| {
        // If parent is "schemas" or "artifact/schema", go up?
        // Since `load_from_metamodel` expects `base_path` to be the root containing `agent/`, `relationship/` etc.
        // If input is `.../ontology-software-engineering/artifact/schema/metamodel.schema.json`
        // Parent: `artifact/schema`
        // Parent: `artifact`
        // Parent: `ontology-software-engineering` (This is what we want)

        // Simple heuristic: walk up until we see "agent" or "artifact" folders?
        // Or just simpler: Pass the parent of the input file, and let the User specify the root?
        // Or if the user passes `.../metamodel.schema.json`, and we assume standard structure.

        // Let's try to assume standard simple structure first:
        // If file is in `artifact/schema`, we go up 2 levels.
        // If file is in `schemas` (legacy), we go up 1 level.
        // If indeterminate, use parent.

        let mut current = p;
        if current.ends_with("schema") {
            current = current.parent().unwrap_or(current);
        }
        if current.ends_with("artifact") {
            current = current.parent().unwrap_or(current);
        }
        if current.ends_with("schemas") {
            current = current.parent().unwrap_or(current);
        }
        current
    });

    println!("Using base path: {:?}", base_path);

    match DependencyGraph::load_from_metamodel(&content, base_path) {
        Ok(_) => {
            println!("✅ Graph topology and Schema are VALID.");
            Ok(())
        }
        Err(e) => {
            eprintln!("❌ Graph validation FAILED: {}", e);
            std::process::exit(1);
        }
    }
}

fn simulate_path(input_path: &PathBuf) -> anyhow::Result<()> {
    use console::style;
    use pulpo_engine::graph::{DependencyGraph, RelationCategory};
    use std::collections::HashSet;

    println!("Simulating execution path for {:?}", input_path);
    let content = std::fs::read_to_string(input_path)?;

    // Simple base path inference as in validate_graph
    let base_path = input_path.parent().map(|p| {
        let mut current = p;
        if current.ends_with("schema") {
            current = current.parent().unwrap_or(current);
        }
        if current.ends_with("artifact") {
            current = current.parent().unwrap_or(current);
        }
        if current.ends_with("schemas") {
            current = current.parent().unwrap_or(current);
        }
        current
    });

    let graph = DependencyGraph::load_from_metamodel(&content, base_path)?;

    let mut produced = HashSet::new();
    produced.insert("SoftwareApplication".to_string());

    let mut missing_creator = Vec::new();

    // Identify Gaps: Any non-Agent (except root) that isn't created by an Agent
    use petgraph::visit::EdgeRef;
    for node_idx in graph.graph.node_indices() {
        let node_name = &graph.graph[node_idx];
        if graph.is_agent(node_name) || node_name == "SoftwareApplication" {
            continue;
        }

        let is_created_by_agent = graph
            .graph
            .edges_directed(node_idx, petgraph::Direction::Incoming)
            .any(|edge| {
                let relation = edge.weight();
                let source_idx = edge.source();
                let source_name = &graph.graph[source_idx];
                let edge_key = (
                    source_name.to_string(),
                    relation.to_string(),
                    node_name.to_string(),
                );
                let category = graph
                    .edge_categories
                    .get(&edge_key)
                    .copied()
                    .unwrap_or(RelationCategory::Context);

                // Effective creation: Agent --(Creation)--> Node
                category == RelationCategory::Creation && graph.is_agent(source_name)
            });

        if !is_created_by_agent {
            missing_creator.push(node_name.clone());
            // We still add them to produced so the simulation can show them in context
            // but we will flag them as errors at the end.
            produced.insert(node_name.clone());
        }
    }

    let mut verified = HashSet::new();
    let mut steps = 0;
    let max_steps = 100;

    println!("\n{}", style("PREDICTED EXECUTION PATH:").bold().yellow());
    println!("{} SoftwareApplication (Initial Goal)", style("🏠").green());

    while steps < max_steps {
        let mut actions = Vec::new();

        for edge_idx in graph.graph.edge_indices() {
            let (source_idx, target_idx) = graph.graph.edge_endpoints(edge_idx).unwrap();
            let source_kind = &graph.graph[source_idx];
            let target_kind = &graph.graph[target_idx];
            let relation = &graph.graph[edge_idx];
            let edge_key = (
                source_kind.to_string(),
                relation.to_string(),
                target_kind.to_string(),
            );
            let category = graph
                .edge_categories
                .get(&edge_key)
                .copied()
                .unwrap_or(RelationCategory::Context);

            if graph.is_agent(source_kind) {
                match category {
                    RelationCategory::Creation => {
                        if !produced.contains(target_kind) {
                            actions.push((
                                source_kind.clone(),
                                relation.clone(),
                                target_kind.clone(),
                                "Creation",
                            ));
                        }
                    }
                    RelationCategory::Verification => {
                        if produced.contains(target_kind) && !verified.contains(target_kind) {
                            actions.push((
                                source_kind.clone(),
                                relation.clone(),
                                target_kind.clone(),
                                "Verification",
                            ));
                        }
                    }
                    _ => {}
                }
            }
        }

        if actions.is_empty() {
            println!(
                "\n{}",
                style("Simulation complete. No more actionable nodes found.")
                    .bold()
                    .dim()
            );
            break;
        }

        // The engine currently picks the first actionable action it finds in edge order.
        let (agent, relation, target, cat) = &actions[0];
        steps += 1;

        let icon = match *cat {
            "Creation" => style("🪄").cyan(),
            "Verification" => style("✅").green(),
            _ => style("➜").white(),
        };

        println!(
            "{:02}. {} {} {} {}",
            steps,
            icon,
            style(agent).bold().blue(),
            style(relation).dim(),
            style(target).bold().magenta()
        );

        // Show Context
        let mut context = graph.get_related_artifacts(target);
        if *cat == "Verification" || *cat == "Refines" {
            context.push(target.clone());
        }
        context.push("SoftwareApplication".to_string());

        // Filter to only show context that exists at this point in the simulation
        context.retain(|c| produced.contains(c));

        context.sort();
        context.dedup();

        if !context.is_empty() {
            let context_str = context
                .iter()
                .map(|c| style(c).dim().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            println!("    {} {}", style("Context:").dim(), context_str);
        }

        if *cat == "Creation" {
            produced.insert(target.clone());
        } else if *cat == "Verification" {
            verified.insert(target.clone());
        }
    }

    if steps >= max_steps {
        println!(
            "\n{}",
            style("Reached maximum simulation steps. Possible infinite loop in ontology?")
                .red()
                .bold()
        );
    }

    // Detect Unlinked Nodes
    let all_nodes: HashSet<String> = graph
        .graph
        .node_indices()
        .map(|i| graph.graph[i].clone())
        .collect();
    let visited_nodes = produced; // produced contains all artifacts/entry points reached.

    // Note: unlinked here means not reachable from the starting point "SoftwareApplication"
    // via the engine's execution logic.
    let mut unlinked: Vec<&String> = all_nodes
        .iter()
        .filter(|n| !visited_nodes.contains(*n) && !graph.is_agent(n))
        .collect();
    unlinked.sort();

    if !missing_creator.is_empty() {
        println!(
            "\n{}",
            style("ERROR: ARTIFACTS MISSING AGENT CREATOR:")
                .bold()
                .red()
        );
        let mut sorted = missing_creator.clone();
        sorted.sort();
        for node in sorted {
            println!("  {} {}", style("✗").red(), node);
        }
        println!(
            "    {}",
            style("Add 'creates', 'implements', or 'defines' relationships from an Agent.")
                .dim()
                .italic()
        );
    }

    if !unlinked.is_empty() {
        println!(
            "\n{}",
            style("UNLINKED / UNREACHABLE ARTIFACTS:").bold().red()
        );
        for node in unlinked {
            println!("  {} {}", style("✗").red(), node);
        }
    } else {
        println!(
            "\n{}",
            style("All artifacts in the graph are reachable.")
                .bold()
                .green()
        );
    }

    Ok(())
}
