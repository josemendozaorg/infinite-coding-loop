import type { OntologyData, Node, Edge } from '../types';
import { MarkerType } from 'reactflow';
import Ajv from 'ajv/dist/2020';

const ajv = new Ajv({ useDefaults: true });

// In a real app, these would be fetched from an API or read via File System API
// For this demo, we'll try to use the raw files if possible or mock them if fetch fails
const BASE_PATH = '/';

async function loadJSON(path: string) {
    const response = await fetch(`${BASE_PATH}${path}`);
    if (!response.ok) {
        throw new Error(`Failed to load ${path}: ${response.statusText}`);
    }
    return response.json();
}

export async function loadOntology(): Promise<OntologyData> {
    try {
        const [ontology, ontologySchema, baseSchema] = await Promise.all([
            loadJSON('ontology.json'),
            loadJSON('schemas/meta/ontology.schema.json'),
            loadJSON('schemas/meta/base.schema.json')
        ]);

        // Defensive schema registration
        if (!ajv.getSchema("base.schema.json")) {
            ajv.addSchema(baseSchema, "base.schema.json");
        }

        // Use the $id from the schema if available, otherwise compile directly
        const schemaId = ontologySchema.$id || "ontology.schema.json";
        let validate = ajv.getSchema(schemaId);
        if (!validate) {
            validate = ajv.compile(ontologySchema);
        }

        if (!validate(ontology)) {
            console.error('[loadOntology] Validation failed:', validate.errors);
            const errorMsg = validate.errors?.map(e => `${e.instancePath} ${e.message}`).join(', ');
            throw new Error(`Ontology validation failed: ${errorMsg}`);
        }

        const processedNodes = processOntologyNodes(ontology as any[]);
        const processedEdges = processOntologyEdges(ontology as any[]);

        return {
            nodes: processedNodes,
            edges: processedEdges,
            raw: { ontology }
        };
    } catch (error) {
        console.error('[loadOntology] Error:', error);
        return {
            nodes: [],
            edges: [],
            raw: {}
        };
    }
}

function processOntologyNodes(ontology: any[]): Node[] {
    const nodeSet = new Map<string, string>(); // Name -> Kind/Type

    ontology.forEach((rel: any) => {
        if (rel.source?.name) {
            // Use the type defined in the ontology instance
            // Assuming required by schema
            const type = rel.source.type || 'Other';
            if (!nodeSet.has(rel.source.name) || type !== 'Other') {
                nodeSet.set(rel.source.name, type);
            }
        }
        if (rel.target?.name) {
            const type = rel.target.type || 'Other';
            if (!nodeSet.has(rel.target.name) || type !== 'Other') {
                nodeSet.set(rel.target.name, type);
            }
        }
    });

    return Array.from(nodeSet.entries()).map(([name, kind]) => {
        let displayKind = 'Entity';

        if (name === 'SoftwareApplication') {
            displayKind = 'SoftwareApplication';
        } else if (kind === 'Agent') {
            displayKind = 'Agent';
        } else if (['Document', 'Code', 'Other'].includes(kind)) {
            // Map Document, Code, and Other (concepts/methodologies) to Artifact for visualization
            // 'Other' includes definitions like TDD, Methodology which are visualized as artifacts/concepts
            displayKind = 'Artifact';
        }

        return {
            id: name,
            type: 'custom', // Use custom node type for specialized rendering
            position: { x: 0, y: 0 },
            data: {
                label: name,
                kind: displayKind,
                originalType: kind
            }
        };
    });
}

function processOntologyEdges(ontology: any[]): Edge[] {
    const edgeMap: Record<string, { source: string; target: string; relations: string[] }> = {};

    ontology.forEach((rel: any) => {
        if (!rel.source?.name || !rel.target?.name || !rel.type?.name) return;

        const key = `${rel.source.name}->${rel.target.name}`;
        if (!edgeMap[key]) {
            edgeMap[key] = { source: rel.source.name, target: rel.target.name, relations: [] };
        }
        if (!edgeMap[key].relations.includes(rel.type.name)) {
            edgeMap[key].relations.push(rel.type.name);
        }
    });

    return Object.values(edgeMap).map((value) => ({
        id: `e-${value.source}-${value.target}`,
        source: value.source,
        target: value.target,
        label: value.relations.sort().join(', '),
        animated: true,
        style: { stroke: '#58a6ff' },
        labelStyle: { fill: '#8b949e', fontSize: 10, fontWeight: 700 },
        labelBgStyle: { fill: '#161b22', fillOpacity: 0.7 },
        markerEnd: {
            type: MarkerType.ArrowClosed,
            color: '#58a6ff'
        },
    }));
}
