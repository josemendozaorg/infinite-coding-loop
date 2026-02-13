## Ideas

[] Visualizer should have the possibility to create or edit an ontology selected from file disk and allow to download it to disk. It should use AI CLI to help build the ontology with prompts.

[] Every ontology node should use the AI CLI to clarify assumptions and make questions back to the user with some options to choose from. It should do this as many times as necessary to resolve all assumptions. After collecting the feedback from the user with the multiple questions, then it can come back to execute the node and pass the information collected with the answers. The questions with the answers should be stored in a document related with the ontology node being executed.

[][] - Every node in the graph should have the possibility to be executed by a different AI CLI (cursor, gemini, claude code, copilot, opencode) and a different model. Perhaps, we could define at a higher level configuration like “Creation” verb Relationships in the graph should be executed by a top tier intelligent models, but Verification and Refining could be done by other less intelligent models. Of we could yet add an additional attribute to the Relationship like “LLM type” that would say like “High Reasoning and Intelligence”, “Fast Execution”, “Daily Driver”. 

[][] - The Visualizer should guide the creation of the Ontology.

[][]- quality verification and Iteration of the same node or bunch of nodes to achieve a quality threshold must be built it and not optional to define in the ontology graph by the user. But this means we should attach dynamically at runtime this verification and refinement nodes.

[]- all documents must be in a folder called spec. In the .infinitecodingloop, we would only have metadata of what has been generated successfully on every “Iteration” or “execution of the infinite coding loop tool. And we do this by instructing the AI CLI in the prompt. The folder name can be a parameter to choose from in the beginning of the loop too, as part of the Application configuration setup. And all This configuration is stored in .infinitecodingloop/config.json. The folder name can be changed by the user. The default folder name is spec. The folder name is relative to the root of the project.

[][] - Some documents are in the scope of the Application, some others are in the scope of features. We can scope this my declaring relationships, but the location in the file structure matters and avoiding to regenerate Application level documents every time for a new feature. So perhaps we need separate ontologies for this. We want to allow executing ontologies separately and to be chosen at the beginning. 

[][]- And we also want the possibility to execute a group of connected ontologies. So there should be a new Entity type called “ontology” that allows to reference other existing ontologies. This way we can compose ontologies and be able to reason better about them. But also execute them separately. 

[][]- A new entity type can be an “Ontology” itself, which reference a whole complete ontology to load with Agents and Entities, etc. This can make this really powerful. There could be made an engine thst would discover ontologies from a Marketplace amd register them like MCPs. This is brilliant idea.

[][] - Visualize the Graph in the vidualizator like it is now in the ontology tools. The Graph is presented like it will execute with the contexts needed.

[][] - Every “run” of the infinite loop could be just a feature in the application. Then the multiple features could be executed in parallel from the outside. This is just a possibility. We need to assess if this should be from the outside or from the inside. Perhaps we can do it from the inside but yet create a higher level abstraction. Like Ontogology of Ontologies, like one main graph run subgraphs, and the current subgraph is a feature subgraph, and there is a initialization graph to prepare the first document and the project structure, etc, but then that is not repeated, but reused instead, but the Document can be used. When started, it checks if all the “static" Documents (like DDD, TDD) in the graph are available in folder/database.

[][] - Create a public library of the “static” Documents that can be reference from new ontologies, so that they do not have to be recreated every time.

[][] - The ontogoly visualizer could be reused to follow in real-time the execution of the graph and see the Produced document.

[][] - Every Node should execute a Git Commit. Perhaps this should also be a node in the graph, so that the User can locate when and where it wants to create commits. “Engineer executes Commit”.

[][] - Create a “Unix Command” as a new type of Entity. Every Command should also have a prompt that will instruct the agent execute a unix command. “ontology/Command/prompt/Commit.md”.  “Command uses Feature“ to collect Context.

[][] - It should be stored the full history traceability of every Node execution. The graph itself should be the tool used for the traceability. Every Graph Iteration should be able to be seen what happened in the UI or the console tool.

[][] - Every feature should create a feature branch. Perhaps this should also be a node in the graph, to give the posibility to the User to decide when to create feature branches and when not.

[][]- infinite loop- the orchestrator should be a state machine. Each node id a agent working on a primitive (use Temporal?) Each Agent Asks Questions to human with a template for each they have to generate to resolve assumptions. The git agent must be present after each agent finishes its work. Also to open a branch PR for each feature. Also for initializing a repo in a Git organisation with gh cli.

[X] The visualizater should have an option button to visualize the graph in the same way it will be executed by the engine, in which sequence from first to last and on each step see what context is needed. This is how the ontology-tools display it now.

[] We should be able to choose the ontology to execute for every iteration of the infinite coding loop.

[X] We should be able to define loops in the graph to improve a document till it passes a quality metric threshold. Like improving the code until a test passes, or improving the code and test until it meets the full feature/story acceptance criteria, or until it has 90% coverage, or until it passes the cyclomatic complexity threshold, etc.

[] The execution graph is not printed always at the beginning when starting the project or a new iteration, it only appears when resuming an iteration. It should always be visible somewhere during the execution and be refreshed as we go, just to be able to see the progress.
  ITERATION STATUS: Initial Implementation
Completed Tasks:
  ✔ ArchitectureStyle
  ✔ Microservices
  ✔ DDD
  ✔ Methodology
  ✔ QualityMetric
  ✔ TDD
  ✔ CodingStyle
  ✔ GoogleCodingStyle
  ✔ CodingPractice
  ✔ DRY
  ✔ KISS
  ✔ Requirement
  ✔ Feature
Pending Tasks:
  ➜ UserStory
  ➜ AcceptanceCriteria
  ➜ DesignSpec
  ➜ ImplementationPlan
  ➜ UnitTest
  ➜ Code

[X] When providing a path, the CLI should search for the existing projects in that folder and allow me to select one to continue.

[] The model for the AI CLI for every graph node should be possible to be configured in the ontology graph json. It means we need to extend the base schema MetaRelationship to include the model per every graph node. It should optional, and when not defined, it should use the default model selected at the beginning of the application execution.

[X] When running the loop, few things happened:
  1. The Log is repeated, not sure why, perhaps it is calling the node twice:
    Iteration 3 - Evaluating Graph State...
    2026-02-12T22:30:53.386521Z  INFO Dispatched Action: ProductManager creates UserStory
    2026-02-12T22:30:53.386553Z  INFO Dispatched Action: ProductManager creates UserStory
  2. Execution stopped at ProductManager creates UserStory. Is it because model exhaustion? Attempt 1 failed: You have exhausted your capacity on this model. Your quota will reset after 1s.. Retrying after 1834.113302ms...
Attempt 1 failed: You have exhausted your capacity on this model. Your quota will reset after 0s.. Retrying after 255.492243ms...
Attempt 1 failed: You have exhausted your capacity on this model. Your quota will reset after 1s.. Retrying after 1912.8276259999998ms...

  3. It is creating repeated files for every Document. One inside "{App Name}/.infinitecodingloop/iterations/{iteration_id}/documents/" and another inside the root "{App Name}/".
  4. The {iteration_id} should be a sequential number with a date.. e.g. 20260213_0001. The sequence restarts per date.
  5. The first graph nodes were executed without asking for the approval to move on to the next node.
     The nodes Architect defines ArchitectureStyle,
     Architect defines Microservices and all others till ProductManager creates Requirement.
  6. It should be possible to select the model at the beginning.


[X] Code must not have schema validation either. 
    Dispatched Action: Engineer implements Code
    Thinking... [Agent: Engineer] executing Task: implements Code
    YOLO mode is enabled. All tool calls will be automatically approved.
    Loaded cached credentials.
    AI CLI SUCCESS
    2026-02-10T00:11:41.065331Z  WARN Artifact validation failed for Code: Artifact validation failed for Code:

[] Regardless if there is a relationship/prompt or not, the dass-engine must load the schema for the document to create if it exists any in the folder /schema.

[] We need to have a full execution log to be able to capture and report bugs in the middle of an iteration. So that we can provide the log to the AI to help fix it.

[] Show the graph execution progress in the UI as we progress. Also show the context that is loaded for each of the relationships in the graph.

[] Some documents like TDD, DDD, CodingStyle, CodingStandards, Microservices, ArchitectureStyle, etc, do not need to be generated every time. Perhaps they just need to be generated once and be available to be loaded by the dass-engine. We could create a library of those type of documents to be reused and loaded by the dass-engine. And then we could create a entity type like: Library Document, which would allow to select a document from the library to be loaded.

[X] We need to implement a way to do loops for improvement of Code, improvement of a document that does not satisfy a quality metric or to iterate code that does not pass an Unit Test. We need to implement a way to loop in the graph execution in the dass-engine.

[X] Make sure the cli creates a new folder if it does not exist for the application name.

[X] Create/Verify/Refine these verbs must not be hardcoded either. We should rather define a new field in the MetaVerb called "verbType" that can have a defined set of values like "Creation", "Verification", "Refinement", "Context". Then when defining the ontology, one can use any verb, but then the verbType would be used to determine if it is use to create, verify, refine or provide context. This would allow us to have a more flexible ontology and also to have a more flexible execution engine.

[] The ontology-tools must be re-used in the dass-engine to display the ontology graph execution prediction at the beginning when starting and also to load the ontology and the graph and the context nodes in the same way the ontology tools does it.

[X] We need to see what the AI CLI is doing. Because we do not know when it gets stuck.

[X] It is creating files now. But it is getting stuck in the #3 iteration.
   Also, it is not asking every time for approval. It seems that it implemented one YOLO for our tool and one YOLO for the AI CLI.

[X] THIS IS A BIG PROBLEM: Show the prompt for every agent. The AI CLI is not create the Documents still.

[X] I am not following. Wait a second. Why are we actually parsing the JSON/YAML output whatsover? The application is constantly failing at this. 
  "The AI is outputting multiple JSON code blocks (likely one for the "preview" and one for the "persistence" action), "
  Why is the AI outputing multiple JSON code blocks instead of persisting the files?
  The files are not being persisted.
 
[X] The Documents and Code generated should always be persisted.
[X] We should be able to resume where we left off once we start a project.

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
