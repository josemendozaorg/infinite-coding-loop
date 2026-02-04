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
            default_value = "ontology-software-engineering/ontology.json"
        )]
        input: PathBuf,
        #[arg(
            short,
            long,
            default_value = "ontology-software-engineering/ontology.ttl"
        )]
        output: PathBuf,
    },
    /// Verify a Turtle file syntax
    Verify {
        #[arg(
            short,
            long,
            default_value = "ontology-software-engineering/ontology.ttl"
        )]
        input: PathBuf,
    },
    /// Validate the graph topology of the metamodel
    Validate {
        #[arg(
            short,
            long,
            default_value = "ontology-software-engineering/ontology.json"
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

const BASE_IRI: &str = "https://infinite-coding-loop.dass/ontology/";

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
    use dass_engine::graph::DependencyGraph;
    println!("Validating graph topology of {:?}", input_path);

    let content = std::fs::read_to_string(input_path)?;
    // Derive base_path from input_path (ignoring "schemas" or "artifact/schema" intermediate dirs if possible)
    // The convention is that `input_path` is usually `ontology/artifact/schema/metamodel.schema.json`.
    // The base path should be `ontology/`.
    // Let's try to infer it. If input has parent, use it.
    let base_path = input_path.parent().and_then(|p| {
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
        Some(current)
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
