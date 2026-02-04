import type { OntologyData, Node, Edge } from '../types';
import { MarkerType } from 'reactflow';

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
        const ontology = await loadJSON('ontology.json');

        return {
            nodes: processOntologyNodes(ontology),
            edges: processOntologyEdges(ontology),
            raw: { ontology }
        };
    } catch (error) {
        console.error('Failed to load ontology:', error);
        return {
            nodes: [],
            edges: [],
            raw: {}
        };
    }
}

function processOntologyNodes(ontology: any[]): Node[] {
    const nodeSet = new Set<string>();

    ontology.forEach((rel: any) => {
        if (rel.source?.name) nodeSet.add(rel.source.name);
        if (rel.target?.name) nodeSet.add(rel.target.name);
    });

    const AGENT_KINDS = new Set([
        'Agent', 'Architect', 'Engineer', 'Manager', 'ProductManager', 'QA',
        'ProductOwner', 'Developer', 'DevOps', 'ProjectManager', 'BusinessAnalyst', 'Tester'
    ]);

    const ARTIFACT_KINDS = new Set([
        'Artifact',
        'SourceFile', 'DesignSpec', 'Plan', 'TestCase', 'TestResult', 'Standard',
        'Observation', 'Persona', 'TechnologyStack', 'Tool', 'ArchitecturePattern',
        'ProjectStructure', 'ArchitectureComponent', 'Command', 'DataModel',
        'UserStory', 'AcceptanceCriteria', 'UIDesign', 'SoftwareArchitecture',
        'LogicalDataModel', 'PhysicalDataModel', 'OpenApiSpec', 'DomainEventSpec',
        'ImplementationPlan', 'UserStoryImplementationPlan', 'Code', 'UnitTest',
        'Feature', 'SuccessfulTestResult', 'QualityMetric', 'ArchitectureStyle'
    ]);

    return Array.from(nodeSet).map(name => {
        let kind = 'Entity';
        if (name === 'SoftwareApplication') kind = 'SoftwareApplication';
        else if (AGENT_KINDS.has(name)) kind = 'Agent';
        else if (ARTIFACT_KINDS.has(name)) kind = 'Artifact';
        // Add a catch-all for practices/styles to look like artifacts
        else if (['TDD', 'DDD', 'BDD', 'CodingStyle', 'CodingPractice', 'KISS', 'DRY', 'CyclomaticLowComplexity', 'GoogleCodingStyle', 'Microservices'].includes(name)) kind = 'Artifact';

        return {
            id: name,
            type: 'custom', // Use custom node type for specialized rendering
            position: { x: 0, y: 0 },
            data: {
                label: name,
                kind: kind
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
