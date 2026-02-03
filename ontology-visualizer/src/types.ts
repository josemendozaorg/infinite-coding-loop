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
    entities: EntityKind[];
    rules: RelationshipRule[];
}

export interface NodeData {
    label: string;
    kind: string;
    description?: string;
}

export interface EdgeData {
    relation: string;
}
