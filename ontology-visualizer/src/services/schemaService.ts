import type { OntologyData, EntityKind, RelationshipRule } from '../types';

// In a real app, these would be fetched from an API or read via File System API
// For this demo, we'll try to use the raw files if possible or mock them if fetch fails
const BASE_PATH = '/@fs/home/dev/repos/infinite-coding-loop/ontology/schemas';

async function loadJSON(path: string) {
    const response = await fetch(`${BASE_PATH}/${path}`);
    if (!response.ok) {
        throw new Error(`Failed to load ${path}: ${response.statusText}`);
    }
    return response.json();
}

export async function loadOntology(): Promise<OntologyData> {
    try {
        const taxonomy = await loadJSON('taxonomy.schema.json');
        const metamodel = await loadJSON('metamodel.schema.json');

        const entities: EntityKind[] = [];

        // Extract EntityKinds from taxonomy
        // Looking at the schema, they are in $defs
        if (taxonomy.$defs) {
            Object.entries(taxonomy.$defs).forEach(([key, value]: [string, any]) => {
                if (key.startsWith('Kind_')) {
                    entities.push({
                        kind: value.const || key.replace('Kind_', ''),
                        description: value.description
                    });
                }
            });
        }

        const rules: RelationshipRule[] = [];
        // Extract rules from metamodel
        if (metamodel.$defs && metamodel.$defs.GraphRules && metamodel.$defs.GraphRules.rules) {
            metamodel.$defs.GraphRules.rules.forEach((rule: any) => {
                rules.push({
                    source: rule.source,
                    target: rule.target,
                    relation: rule.relation,
                    description: rule.description
                });
            });
        }

        return { entities, rules };
    } catch (error) {
        console.error('Error loading ontology:', error);
        // Fallback data for demonstration if fetch fails
        return {
            entities: [
                { kind: 'SoftwareApplication', description: 'The root application entity' },
                { kind: 'Feature', description: 'A functional capability' },
                { kind: 'Requirement', description: 'A specific requirement' }
            ],
            rules: [
                { source: 'SoftwareApplication', target: 'Feature', relation: 'contains' },
                { source: 'Feature', target: 'Requirement', relation: 'contains' }
            ]
        };
    }
}
