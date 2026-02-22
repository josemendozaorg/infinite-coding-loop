
import json
import os

ontology_path = "ontologies/software-engineering/ontology.json"

with open(ontology_path, 'r') as f:
    ontology = json.load(f)

agents = [
    "ProductManager", "Engineer", "Architect", "Tester", 
    "DevOps", "BusinessAnalyst", "ProjectManager"
]

documents = [
    "Requirement", "DesignSpec", "Plan", "UserStory", 
    "AcceptanceCriteria", "ImplementationPlan", "TestCase", 
    "UnitTest", "TestResult", "Feature", "Risk", "ChangeRequest",
    "ProjectStructure", "Standard", "TechnologyStack", "Persona",
    "Observation", "DataModel", "Command", "Environment"
]

# Map entity name to type
type_map = {}
for a in agents:
    type_map[a] = "Agent"
for d in documents:
    type_map[d] = "Document"
type_map["Code"] = "Code"
type_map["SourceFile"] = "Code"
type_map["SoftwareApplication"] = "Other"

# Default to "Other" for anything else (Methodologies, Concepts, etc)

for relation in ontology:
    # Update Source
    s_name = relation["source"]["name"]
    s_type = type_map.get(s_name, "Other")
    relation["source"]["type"] = s_type
    
    # Update Target
    t_name = relation["target"]["name"]
    t_type = type_map.get(t_name, "Other")
    relation["target"]["type"] = t_type

with open(ontology_path, 'w') as f:
    json.dump(ontology, f, indent=2)

print("Updated ontology.json successfully.")
