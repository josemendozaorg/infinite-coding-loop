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
        #[arg(short, long, default_value = "ontology/schemas/metamodel.schema.json")]
        input: PathBuf,
        #[arg(short, long, default_value = "ontology/ontology.ttl")]
        output: PathBuf,
    },
    /// Verify a Turtle file syntax
    Verify {
        #[arg(short, long, default_value = "ontology/ontology.ttl")]
        input: PathBuf,
    },
    /// Validate the graph topology of the metamodel
    Validate {
        #[arg(short, long, default_value = "ontology/schemas/metamodel.schema.json")]
        input: PathBuf,
    },
}

#[derive(Deserialize, Debug)]
struct Metamodel {
    #[serde(rename = "$defs")]
    defs: Defs,
}

#[derive(Deserialize, Debug)]
struct Defs {
    #[serde(rename = "GraphRules")]
    graph_rules: GraphRulesWrapper,
    #[serde(rename = "AgentDefinitions")]
    agent_definitions: Option<AgentDefinitionsWrapper>,
}

#[derive(Deserialize, Debug)]
struct GraphRulesWrapper {
    rules: Vec<Rule>,
}

#[derive(Deserialize, Debug)]
struct AgentDefinitionsWrapper {
    agents: Vec<Agent>,
}

#[derive(Deserialize, Debug)]
struct Rule {
    source: String,
    target: String,
    relation: String,
}

#[derive(Deserialize, Debug)]
struct Agent {
    role: String,
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
    let metamodel: Metamodel = serde_json::from_reader(reader)?;

    println!("Writing ontology to {:?}", output_path);
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let outfile = File::create(output_path)?;
    let writer = BufWriter::new(outfile);
    let mut formatter = TurtleFormatter::new(writer);

    // Track declared classes/properties to avoid duplicates logic if needed,
    // but TurtleFormatter handles writing stream. We just emit triples.

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

    // Process Rules
    for rule in metamodel.defs.graph_rules.rules {
        let s_iri = format!("{}{}", BASE_IRI, rule.source);
        let t_iri = format!("{}{}", BASE_IRI, rule.target);
        let r_iri = format!("{}{}", BASE_IRI, rule.relation);

        // Declare Source Class
        if classes.insert(rule.source.clone()) {
            formatter.format(&Triple {
                subject: Subject::NamedNode(NamedNode { iri: &s_iri }),
                predicate: rdf_type,
                object: Term::NamedNode(owl_class),
            })?;
        }

        // Declare Target Class
        if classes.insert(rule.target.clone()) {
            formatter.format(&Triple {
                subject: Subject::NamedNode(NamedNode { iri: &t_iri }),
                predicate: rdf_type,
                object: Term::NamedNode(owl_class),
            })?;
        }

        // Declare Property
        if properties.insert(rule.relation.clone()) {
            formatter.format(&Triple {
                subject: Subject::NamedNode(NamedNode { iri: &r_iri }),
                predicate: rdf_type,
                object: Term::NamedNode(owl_obj_prop),
            })?;
        }

        // Domain and Range (Loose enforcement: just stating it applies)
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

    // Process Agents
    if let Some(agent_defs) = metamodel.defs.agent_definitions {
        for agent in agent_defs.agents {
            let a_iri = format!("{}{}", BASE_IRI, agent.role);

            // Agent is likely a Class in our metamodel (e.g. Architect creates Design),
            // but also can be viewed as an instance depending on interpretation.
            // For now, consistent with schema, they are source nodes, so Classes.
            // If we want actual agents as instances, we would differentiate.
            // Based on schema: "source": "Architect". So Architect is a Class of Nodes.

            if classes.insert(agent.role.clone()) {
                formatter.format(&Triple {
                    subject: Subject::NamedNode(NamedNode { iri: &a_iri }),
                    predicate: rdf_type,
                    object: Term::NamedNode(owl_class),
                })?;
            }
        }
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
    match DependencyGraph::load_from_metamodel(&content, None) {
        Ok(_) => {
            println!("✅ Graph topology is VALID (Single root, all reachable).");
            Ok(())
        }
        Err(e) => {
            eprintln!("❌ Graph validation FAILED: {}", e);
            std::process::exit(1);
        }
    }
}
