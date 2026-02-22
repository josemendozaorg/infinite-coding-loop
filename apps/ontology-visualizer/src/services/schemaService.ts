import type { OntologyData, Node, Edge } from '../types';
import { MarkerType } from 'reactflow';
import Ajv from 'ajv/dist/2020';

// Import local JSON schemas statically
import ontologySchema from '@icl/ontology-schema/meta/ontology.schema.json';
import baseSchema from '@icl/ontology-schema/meta/base.schema.json';
// Import the actual ontology data statically
// @ts-ignore
import ontology from '@icl/ontology-software-engineering/ontology.json';

const ajv = new Ajv({ useDefaults: true });

export async function loadOntology(): Promise<OntologyData> {
    try {
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
    const edgeMap: Record<string, { source: string; target: string; relations: string[]; verbTypes: Set<string>; loopInfo?: string }> = {};

    ontology.forEach((rel: any) => {
        if (!rel.source?.name || !rel.target?.name || !rel.type?.name) return;

        const key = `${rel.source.name}->${rel.target.name}`;
        if (!edgeMap[key]) {
            edgeMap[key] = {
                source: rel.source.name,
                target: rel.target.name,
                relations: [],
                verbTypes: new Set()
            };
        }
        if (!edgeMap[key].relations.includes(rel.type.name)) {
            edgeMap[key].relations.push(rel.type.name);
        }
        if (rel.type.verbType) {
            edgeMap[key].verbTypes.add(rel.type.verbType);
        }

        // Capture loop info for tooltip if present
        if (rel.loop) {
            const info = `Max Retries: ${rel.loop.maxRetries || 3}, Threshold: ${rel.loop.passThreshold || 1.0}`;
            edgeMap[key].loopInfo = info;
        }
    });

    return Object.values(edgeMap).map((value) => {
        // Determine color based on priority of verb types
        let color = '#58a6ff'; // Default Context (Blue)
        let styleType = 'solid';

        if (value.verbTypes.has('Refinement')) {
            color = '#d2a8ff'; // Purple
            styleType = 'dashed';
        } else if (value.verbTypes.has('Verification')) {
            color = '#f0883e'; // Orange
        } else if (value.verbTypes.has('Dependency')) {
            color = '#ff7b72'; // Red
        } else if (value.verbTypes.has('Creation')) {
            color = '#7ee787'; // Green
        }

        let label = value.relations.sort().join(', ');
        if (value.loopInfo) {
            // Shorten for label
            const shortInfo = value.loopInfo.replace('Max Retries', 'Retries').replace('Threshold', 'Thres');
            label += `\n[${shortInfo}]`;
        }

        return {
            id: `e-${value.source}-${value.target}`,
            source: value.source,
            target: value.target,
            label: label,
            animated: value.verbTypes.has('Refinement') || value.verbTypes.has('Creation'),
            style: {
                stroke: color,
                strokeDasharray: styleType === 'dashed' ? '5,5' : undefined
            },
            labelStyle: { fill: color, fontSize: 10, fontWeight: 700 },
            labelBgStyle: { fill: '#161b22', fillOpacity: 0.8 },
            markerEnd: {
                type: MarkerType.ArrowClosed,
                color: color
            },
            data: {
                loopInfo: value.loopInfo,
                verbType: value.verbTypes.has('Creation') ? 'Creation' :
                    value.verbTypes.has('Verification') ? 'Verification' :
                        value.verbTypes.has('Refinement') ? 'Refinement' :
                            value.verbTypes.has('Dependency') ? 'Dependency' : 'Context'
            }
        };
    });
}
