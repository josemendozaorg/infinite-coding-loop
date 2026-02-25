import type { Node, Edge } from 'reactflow';

export type { Node, Edge };

export interface EntityKind {
    kind: string;
    description?: string;
}

export interface RelationshipRule {
    source: string;
    target: string;
    relation: string;
    description?: string;
}

export interface OntologyData {
    nodes: Node[];
    edges: Edge[];
    raw?: unknown;
}

export interface NodeData {
    label: string;
    kind: string;
    description?: string;
    hasHidden?: boolean;
}

export interface EdgeData {
    relation: string;
}
