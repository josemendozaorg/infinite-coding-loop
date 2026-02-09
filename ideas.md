## Ideas

[] THIS IS A BIG PROBLEM: Show the prompt for every agent. The AI CLI is not create the Documents still.

[] I am not following. Wait a second. Why are we actually parsing the JSON/YAML output whatsover? The application is constantly failing at this. 
  "The AI is outputting multiple JSON code blocks (likely one for the "preview" and one for the "persistence" action), "
  Why is the AI outputing multiple JSON code blocks instead of persisting the files?
  The files are not being persisted.
 
[] The Documents and Code generated should always be persisted.
[] We should be able to resume where we left off once we start a project.

[] Allow ontology visualizer to load an ontology from a file. We need to see what we leave for the playwright tests.

[] Ontology visualizer should be able to edit and save an ontology and allow to download it or copy the raw json to clipboard.

[] We should be able to run an ontology graph from the ontology visualizer and follow the execution and the see prompts of each step and what the agent did because every node should have a separate commit.

[] We need now just to define a JSON that complies with the ontology.schema.json. We do not need more schemas, isn't it? And then, the dass-engine and antology-tools would just load the JSON instanc, validate it against the ontology.schema.json. The JSON instance of the ontology.schema.json would be Graph execution itself.

[] Perhaps it could still be a taxonomy and a metamodel schemas with restrictions for Software Engineering ontology to guide the definition of the final JSON ontology instance to input to dass-engine, so that the antology-visualizar is not fully free-style definiing entities and relationships, but rather guided.

[X] Validate the taxonomy and the metamodel with the base meta schemas in dass-engine. The ontology-tools crate should be able to validate the taxonomy and the metamodel with the base meta schemas in dass-engine. The dass-engine crate should validate at the beginning the taxonomy and the metamodel with the base meta schemas in dass-engine.

[] Use OpenCode SDK instead of using the "opencode --prompt" inside the AI CLI agent. 

[ ] We need to increase the metamodel with more entities, more relationships, more rules and constraints.

[ ] Do We need to define the execution graph? Because what we are building now in taxonomy and metamodel is the ground truth, the constitution, the guardrails, the rules and constraints, but also the logical path to build software that the agent would follow. Based on the taxonomy and the metamodel, infinite loop will ask questions to the User to decide the path to follow in the Graph defined in the metamodel. For example, the first graph root node is "SoftwareApplication". Then there are several paths to follow. The user is prompted to choose what kind of software application. Also the system suggests a recommended option with some arguments. The user can suggest a fully autonomous YOLO approach that the agent just decides himself on the way and just reports the choices made, and then the User could adjust the choises. The type of Software Application, the architecture Style, the technology stack, the database type, and then based on the taxonomy and the metamodel, the system would follow the execution graph, and this could be follow in a UI, this could be nice. If the SoftwareApplication already exists and the user just wants to add a feature, or fix a Bug, or increase test coverage, this can be offered upfront to the user, and then the Graph would start from there, but we would need to fullfil first all what a feature requires according to the metamodel.

[] We need to define the taxonomy and metamodel in a way that is easy to understand and use. The taxonomy in JSON schema seems to complicated. Let's try to use YAML instead. It is more human readable and it is supported by most of the tools.

[X] The ai clie client should change to the workdir before executed the gemini command. 

[X] DISCARDED Use OWL 2 for the taxonomy and metamodel. It is a standard for ontologies and it is supported by most of the tools.

[X] The prompts should not have the section of output json. That is boilerplate. Can be injected in runtime along with the schema itself. The same about the section of the context. The prompt should be more about what the Agent (architect) should do according to the relationship assigned with the entity.

[X] Do we really need the schemas per each entity type in the taxonomy?
    We need a template that can be enforced for each of the entities that are generated. So maybe yes we need a schema, but they share some common attributes coming from entity metadata.

[X] Shall we call "Entity" rather an Software Artifact? Real names in the real world. Entity means nothing.
   - Entity is the base abstraction for nouns. Relationships are verbs.   

[] What is really an Agent? It is rather a team member in the Software team. So there should be a TeamMemberMetadata instead. These do not need to be entities because these do not need IDs in runtime. But Software Artifact, which are produced multiple times, they do need IDs in runtime to be traceable.

[] What is a Relationship? There should be a RelationshipMetadata too. These do not need to be entities because these do not need IDs in runtime.

[] What about Rules/Constraints? These are not entities, but they are constraints for the entities. So there should be a RuleMetadata too. These do not need to be entities because these do not need IDs in runtime.

[] Can the Rules/Constraints be defines in isolation from the Entities? How do we model them along to the Entites if they cannot be model separately?

[] What is the difference between a Rule and a Constraint? Do they need to have separate metadata?

[] We need to create explicit rules and constraints and add them to the taxonomy, so we can use them in the Graph. Examples: Rule_MUST_EXIST(Design_Spec, Source_File) means a Design_Spec must exist before a Source_File can be created. And Rule_MUST_EXIST(Test_Case, Source_File) means a Test_Case must exist before a Source_File can be created. Do we really need these kind of rules? Or can we infer them from the relationships?

[] We need to define what are the quality requirements for every Software Artifact and a quality score from 0 to 100 for each of them. We could do it by convention as well. Example: 
quality_metrics/Feature.json
{
    "metrics": [
        {
            "metric": "It should be testable",
            "score": 90
        },  
        {
            "metric": "It should be feasible",
            "score": 90
        },
        {
            "metric": "It should be complete",
            "score": 90
        },
        {
            "metric": "It should be unambiguous",
            "score": 90
        }
    ]
}

quality_metrics/Requirement.json
{
    "metrics": [
        {
            "metric": "It should be testable",
            "score": 90
        },
        {
            "metric": "It should be feasible",
            "score": 90
        },
        {
            "metric": "It should be complete",
            "score": 90
        },
        {
            "metric": "It should be unambiguous",
            "score": 90
        }
    ]
}

quality_metrics/DesignSpec.json
{
    "metrics": [
        {
            "metric": "It should align with the Architecture ",
            "score": 90
        }
    ]
}

quality_metrics/SourceFile.json
{
    "metrics": [
        {
            "metric": "It should align with the Code Standards",
            "score": 90
        },
        {
            "metric": "It should align with the Architecture",
            "score": 90
        },
        {
            "metric": "It should pass the linting",
            "score": 90
        },
        {
            "metric": "It should pass the unit tests",
            "score": 90
        },
        {
            "metric": "It should pass the integration tests",
            "score": 90
        },
        {
            "metric": "It should pass the end-to-end tests",
            "score": 90
        },
        {
            "metric": "It should pass the Command COMPILE",
            "score": 90
        },
        {
            "metric": "It should pass the Command TEST",
            "score": 90
        },
        {
            "metric": "It should pass the Command LINT",
            "score": 90
        }
    ]
}

[] The quality metrics do not need to be part of the taxonomy nor the metamodel. They will just be loaded by convention and be used by the agents to evaluate the quality of the entities and iterate when below the score threshold.

[] The prompt for the agents should be generated by the system, not by the user. The user should only provide the initial requirements and the system should generate the prompts for the agents based on the taxonomy and the quality metrics, rules and constraints.

[] The quality metrics, rules and constraints should be defined per Software artifact by convention.
/quality_metrics/Feature.json
/quality_metrics/Requirement.json
/rules/Feature.json
/rules/Requirement.json
/constraints/Feature.json
/constraints/Requirement.json

[] System prompt per Team Member should be defined by convention as well.
/team_members/Architect.json
/team_members/Engineer.json
/team_members/Manager.json
/team_members/ProductManager.json
/team_members/QA.json

[] The context to inject because of a Software Artifact should be defined by convention as well.
/software_artifacts/Feature.json
/software_artifacts/Requirement.json
/software_artifacts/DesignSpec.json
/software_artifacts/SourceFile.json
