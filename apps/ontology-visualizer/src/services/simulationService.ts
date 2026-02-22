
import type { Edge, Node } from 'reactflow';

export interface SimulationStep {
    id: number;
    agent: string;
    verb: string;
    verbType: string;
    target: string;
    context: string[];
    icon: string;
}


export const simulateExecution = (nodes: Node[], edges: Edge[]): SimulationStep[] => {
    const produced = new Set<string>(['SoftwareApplication']);
    const verified = new Set<string>();
    const steps: SimulationStep[] = [];
    let stepCount = 0;
    const maxSteps = 100;

    // Helper to check if a node is an Agent
    const isAgent = (nodeId: string) => {
        const node = nodes.find((n) => n.id === nodeId);
        return node?.data?.kind === 'Agent'; // Use 'kind' from processOntologyNodes, not 'type' (which is 'custom')
    };

    while (stepCount < maxSteps) {
        let actionFound = false;

        // Find first actionable edge
        // We iterate edges in order (mimicking engine's simple strategy)
        for (const edge of edges) {
            // Logic requires: Source is Agent
            if (!isAgent(edge.source)) {
                continue;
            }

            const verbType = edge.data?.verbType; // Assumes verbType is in edge data (populated by schemaService)
            const target = edge.target;

            let isActionable = false;

            if (verbType === 'Creation') {
                // Can create if not already produced
                if (!produced.has(target)) {
                    isActionable = true;
                }
            } else if (verbType === 'Verification') {
                // Can verify if produced but not verified
                if (produced.has(target) && !verified.has(target)) {
                    isActionable = true;
                }
            }

            if (isActionable) {
                actionFound = true;
                stepCount++;

                // Calculate Context
                // All neighbors (in/out) of target that are known (produced) and NOT agents
                const context = new Set<string>();

                // Incoming edges to target
                edges.forEach(e => {
                    if (e.target === target && !isAgent(e.source)) {
                        if (produced.has(e.source)) context.add(e.source);
                    }
                });

                // Outgoing edges from target
                edges.forEach(e => {
                    if (e.source === target && !isAgent(e.target)) {
                        if (produced.has(e.target)) context.add(e.target);
                    }
                });

                // Add self if verification
                if (verbType === 'Verification') {
                    context.add(target);
                }

                // Always add seed
                context.add('SoftwareApplication');

                steps.push({
                    id: stepCount,
                    agent: edge.source,
                    verb: edge.label as string || 'acts',
                    verbType: verbType || 'Unknown',
                    target: target,
                    context: Array.from(context).sort(),
                    icon: verbType === 'Creation' ? '🪄' : (verbType === 'Verification' ? '✅' : '➜')
                });

                // Execute action
                if (verbType === 'Creation') {
                    produced.add(target);
                } else if (verbType === 'Verification') {
                    verified.add(target);
                }

                break; // Only one action per step
            }
        }

        if (!actionFound) {
            break;
        }
    }

    return steps;
};
