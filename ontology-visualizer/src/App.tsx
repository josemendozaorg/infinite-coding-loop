import React, { useEffect, useState, useCallback } from 'react';
import ReactFlow, {
  Background,
  Controls,
  MiniMap,
  useNodesState,
  useEdgesState,
  addEdge,
  Panel
} from 'reactflow';
import type { Connection, Edge, Node } from 'reactflow';
import 'reactflow/dist/style.css';
import { Save, FileJson, Share2, AlignCenter, Box, Bot, FileText } from 'lucide-react';
import { loadOntology } from './services/schemaService';


const nodeWidth = 180;

// Custom Concentric Layout for compact visualization
// Custom Left-to-Right Layered Layout
const getLeftToRightLayout = (nodes: Node[], edges: Edge[], rootId = 'SoftwareApplication') => {
  // ... (keep existing layout logic)
  // 1. Build Adjacency List & Calculate Degrees
  const adj: Record<string, string[]> = {};
  const degrees: Record<string, number> = {};

  nodes.forEach(n => {
    adj[n.id] = [];
    degrees[n.id] = 0;
  });

  edges.forEach(e => {
    if (adj[e.source]) adj[e.source].push(e.target);
    if (adj[e.target]) adj[e.target].push(e.source);
    degrees[e.source] = (degrees[e.source] || 0) + 1;
    degrees[e.target] = (degrees[e.target] || 0) + 1;
  });

  // 2. BFS for Leveling
  const levels: Record<number, string[]> = {};
  const visited = new Set<string>();
  const queue: { id: string, level: number }[] = [];

  // Start with Root
  if (nodes.find(n => n.id === rootId)) {
    queue.push({ id: rootId, level: 0 });
    visited.add(rootId);
  } else if (nodes.length > 0) {
    // Fallback
    const maxDegreeNode = nodes.reduce((a, b) => (degrees[a.id] > degrees[b.id] ? a : b));
    queue.push({ id: maxDegreeNode.id, level: 0 });
    visited.add(maxDegreeNode.id);
  }

  while (queue.length > 0) {
    const sortedQueue = queue.sort((a, b) => a.level - b.level);
    const { id, level } = sortedQueue.shift()!;

    if (!levels[level]) levels[level] = [];
    levels[level].push(id);

    const neighbors = adj[id] || [];
    neighbors.forEach(nid => {
      if (!visited.has(nid) && nodes.find(n => n.id === nid)) {
        visited.add(nid);
        queue.push({ id: nid, level: level + 1 });
      }
    });
  }

  // Handle orphans
  nodes.forEach(n => {
    if (!visited.has(n.id)) {
      const lastLevel = Math.max(...Object.keys(levels).map(Number), 0);
      if (!levels[lastLevel + 1]) levels[lastLevel + 1] = [];
      levels[lastLevel + 1].push(n.id);
      visited.add(n.id);
    }
  });

  // 3. Assign Positions (Left-to-Right)
  const newNodes = [...nodes];
  const xSpacing = 350; // Horizontal distance between levels
  const ySpacing = 150; // Vertical distance between nodes

  Object.entries(levels).forEach(([lvlStr, nodeIds]) => {
    const level = parseInt(lvlStr);
    const x = level * xSpacing;

    // Center vertically based on number of nodes in this level
    const totalHeight = nodeIds.length * ySpacing;
    const startY = -(totalHeight / 2);

    nodeIds.forEach((nid, index) => {
      const node = newNodes.find(n => n.id === nid);
      if (node) {
        node.position = {
          x: x + (Math.random() * 20),
          y: startY + (index * ySpacing) + (Math.random() * 20)
        };
      }
    });
  });

  return { nodes: newNodes, edges };
};



const App: React.FC = () => {
  const [nodes, setNodes, onNodesChange] = useNodesState([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState([]);
  const [selectedNode, setSelectedNode] = useState<Node | null>(null);
  const [isPanelOpen, setIsPanelOpen] = useState(false);
  const [showOrphans, setShowOrphans] = useState(true);
  const [initialLayout, setInitialLayout] = useState<{ nodes: Node[], edges: Edge[] } | null>(null);

  // Exploration Mode State
  const [searchMode, setSearchMode] = useState(false); // If true, progressive disclosure
  const [visibleNodeIds, setVisibleNodeIds] = useState<Set<string>>(new Set(['SoftwareApplication']));
  const [rfInstance, setRfInstance] = useState<any>(null); // Store ReactFlow instance

  useEffect(() => {
    async function init() {
      try {
        const data = await loadOntology();

        // Calculate degrees
        const degrees: Record<string, number> = {};
        data.rules.forEach(r => {
          degrees[r.source] = (degrees[r.source] || 0) + 1;
          degrees[r.target] = (degrees[r.target] || 0) + 1;
        });

        const AGENT_KINDS = new Set([
          'Agent', 'Architect', 'Engineer', 'Manager', 'ProductManager', 'QA',
          'ProductOwner', 'Developer', 'DevOps', 'ProjectManager', 'BusinessAnalyst'
        ]);

        const ARTIFACT_KINDS = new Set([
          'Artifact',
          'SourceFile', 'DesignSpec', 'Plan', 'TestCase', 'TestResult', 'Standard',
          'Observation', 'Persona', 'TechnologyStack', 'Tool', 'ArchitecturePattern',
          'ProjectStructure', 'ArchitectureComponent', 'Command', 'DataModel',
          'UserStory', 'AcceptanceCriteria', 'UIDesign', 'SoftwareArchitecture',
          'LogicalDataModel', 'PhysicalDataModel', 'OpenApiSpec', 'DomainEventSpec',
          'ImplementationPlan', 'UserStoryImplementationPlan', 'Code', 'UnitTest'
        ]);

        // ... existing initialization of nodes/edges ...
        const initialNodes: Node[] = data.entities.map((entity) => {
          const isRoot = entity.kind === 'SoftwareApplication';
          const isAgent = AGENT_KINDS.has(entity.kind);
          const isArtifact = ARTIFACT_KINDS.has(entity.kind);

          return {
            id: entity.kind,
            data: {
              label: (
                <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '8px' }}>
                  {isRoot && <Box size={16} color="#58a6ff" />}
                  {isAgent && <Bot size={16} color="#7ee787" />}
                  {isArtifact && <FileText size={16} color="#79c0ff" />}
                  <span>{entity.kind}</span>
                </div>
              ),
              description: entity.description
            },
            position: { x: 0, y: 0 },
            sourcePosition: 'right' as any,
            targetPosition: 'left' as any,
            className: 'glass-panel',
            style: {
              background: 'rgba(22, 27, 34, 0.8)',
              color: '#c9d1d9',
              border: '1px solid rgba(255, 255, 255, 0.1)',
              borderRadius: '8px',
              padding: '10px',
              width: nodeWidth,
              textAlign: 'center' as const
            }
          };
        });

        const edgeMap: Record<string, { source: string; target: string; relations: string[] }> = {};
        data.rules.forEach((rule) => {
          const key = `${rule.source}->${rule.target}`;
          if (!edgeMap[key]) {
            edgeMap[key] = { source: rule.source, target: rule.target, relations: [] };
          }
          if (!edgeMap[key].relations.includes(rule.relation)) {
            edgeMap[key].relations.push(rule.relation);
          }
        });

        const initialEdges: Edge[] = Object.values(edgeMap).map((value) => ({
          id: `e-${value.source}-${value.target}`,
          source: value.source,
          target: value.target,
          label: value.relations.sort().join(', '),
          animated: true,
          style: { stroke: '#58a6ff' },
          labelStyle: { fill: '#8b949e', fontSize: 10, fontWeight: 700 },
          labelBgStyle: { fill: '#161b22', fillOpacity: 0.7 }
        }));

        // Calculate full layout once
        // Calculate full layout once
        const layouted = getLeftToRightLayout(initialNodes, initialEdges);
        setInitialLayout(layouted);

        // Initial render: Full graph
        setNodes(layouted.nodes);
        setEdges(layouted.edges);
      } catch (error) {
        console.error('Failed to initialize ontology:', error);
      }
    }

    init();
  }, [setNodes, setEdges]);

  // Handle Filtering & Exploration Updates
  useEffect(() => {
    if (!initialLayout) return;

    let activeNodes = initialLayout.nodes;
    let activeEdges = initialLayout.edges;

    // 1. Apply Exploration Mode
    if (searchMode) {
      activeNodes = activeNodes.filter(n => visibleNodeIds.has(n.id)).map(node => {
        // Check if has hidden neighbors
        const allNeighbors = initialLayout.edges
          .filter(e => e.source === node.id || e.target === node.id)
          .map(e => e.source === node.id ? e.target : e.source);

        const hasHidden = allNeighbors.some(nid => !visibleNodeIds.has(nid));

        // Clone node to avoid mutating initialLayout directly
        return {
          ...node,
          data: {
            ...node.data,
            label: (
              <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '8px' }}>
                {node.data.label}
                {hasHidden && <span style={{ fontSize: '10px', marginLeft: '4px' }}>➕</span>}
              </div>
            )
          }
        };
      });
      activeEdges = activeEdges.filter(e => visibleNodeIds.has(e.source) && visibleNodeIds.has(e.target));
      activeEdges = activeEdges.filter(e => visibleNodeIds.has(e.source) && visibleNodeIds.has(e.target));
    }

    // 2. Apply Orphan Filter (Only if NOT in search mode, as search mode naturally handles connectivity)
    else if (!showOrphans) {
      const connectedNodeIds = new Set<string>();
      initialLayout.edges.forEach(edge => {
        connectedNodeIds.add(edge.source);
        connectedNodeIds.add(edge.target);
      });
      activeNodes = activeNodes.filter(node => connectedNodeIds.has(node.id));
    }

    // 3. Re-Layout with Concentric
    // We strictly use Concentric now as requested for compactness
    const layouted = getLeftToRightLayout(activeNodes, activeEdges);
    setNodes(layouted.nodes);
    setEdges(layouted.edges);

    // 4. Fit View if in Exploration Mode or orphans toggled
    if (rfInstance && (searchMode || !showOrphans)) {
      setTimeout(() => {
        rfInstance.fitView({ padding: 0.2, duration: 800 });
      }, 100); // Small delay to allow render
    }

  }, [showOrphans, searchMode, visibleNodeIds, initialLayout, setNodes, setEdges, rfInstance]);

  const onConnect = useCallback(
    (params: Connection) => setEdges((eds) => addEdge(params, eds)),
    [setEdges]
  );

  const onLayout = useCallback(() => {
    // Trigger re-render which triggers layout effect
    // For now, simple force update isn't needed as effect handles state, 
    // but we can re-set nodes to trigger animation if needed.
    if (initialLayout) {
      setNodes([...nodes]);
      // Force re-layout in effect? No, effect depends on state.
      // Actually, if we moved nodes manually, this resets them.
      const layouted = getLeftToRightLayout(nodes, edges);
      setNodes(layouted.nodes);
    }
  }, [nodes, edges, initialLayout]);

  const onNodeClick = (_: React.MouseEvent, node: Node) => {
    setSelectedNode(node);
    setIsPanelOpen(true);

    if (searchMode && initialLayout) {
      // Find OUTGOING neighbors (children) in full graph
      const childrenIds = new Set<string>();
      initialLayout.edges.forEach(e => {
        if (e.source === node.id) childrenIds.add(e.target);
      });

      if (childrenIds.size === 0) return; // Leaf node

      const newIds = new Set(visibleNodeIds);

      const allChildrenVisible = Array.from(childrenIds).every(id => visibleNodeIds.has(id));

      if (allChildrenVisible) {
        childrenIds.forEach(id => newIds.delete(id));
      } else {
        childrenIds.forEach(id => newIds.add(id));
      }

      setVisibleNodeIds(newIds);
    }
  };

  const onInit = (reactFlowInstance: any) => {
    setRfInstance(reactFlowInstance);
    setTimeout(() => {
      reactFlowInstance.fitView({ padding: 0.2 });
    }, 500);
  };

  return (
    <div className="app-container">
      <div className="floating-header glass-panel">
        <div style={{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
          <FileJson size={20} color="#58a6ff" />
          <h1 style={{ fontSize: '1rem', margin: 0 }}>Ontology Visualizer</h1>
        </div>
        <div style={{ display: 'flex', gap: '0.75rem', alignItems: 'center' }}>
          <label style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', fontSize: '0.8rem', cursor: 'pointer', color: '#8b949e' }}>
            <input
              type="checkbox"
              checked={searchMode}
              onChange={(e) => {
                setSearchMode(e.target.checked);
                setVisibleNodeIds(new Set(['SoftwareApplication'])); // Reset on toggle
              }}
            />
            Exploration Mode
          </label>
          {!searchMode && (
            <label style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', fontSize: '0.8rem', cursor: 'pointer', color: '#8b949e' }}>
              <input
                type="checkbox"
                checked={showOrphans}
                onChange={(e) => setShowOrphans(e.target.checked)}
              />
              Show All Nodes
            </label>
          )}
          <div style={{ width: '1px', height: '24px', background: 'var(--border-primary)' }}></div>
          <button className="btn btn-sm" onClick={onLayout} title="Auto Layout">
            <AlignCenter size={14} /> Compact
          </button>
          <button className="btn btn-sm">
            <Share2 size={14} /> Export
          </button>
          <button className="btn btn-primary btn-sm">
            <Save size={14} /> Save
          </button>
        </div>
      </div>

      <main style={{ display: 'flex', flex: 1, overflow: 'hidden', position: 'relative' }}>
        <div className="graph-container">
          <ReactFlow
            nodes={nodes}
            edges={edges}
            onNodesChange={onNodesChange}
            onEdgesChange={onEdgesChange}
            onConnect={onConnect}
            onNodeClick={onNodeClick}
            onInit={onInit}
            fitView
          >
            <Background color="#161b22" gap={20} />
            <Controls />
            <MiniMap
              nodeColor="#21262d"
              maskColor="rgba(13, 17, 23, 0.7)"
              style={{ background: '#0d1117' }}
            />
            <Panel position="top-right">
              <div className="glass-panel" style={{ padding: '8px 12px', fontSize: '0.8rem', color: '#8b949e' }}>
                {nodes.length} Classes | {edges.length} Relationships
              </div>
            </Panel>
          </ReactFlow>
        </div>

        {isPanelOpen && selectedNode && (
          <aside className="side-panel floating">
            <div className="panel-header">
              <h2 style={{ fontSize: '1rem', margin: 0 }}>Properties</h2>
              <button className="btn" style={{ padding: '4px' }} onClick={() => setIsPanelOpen(false)}>
                &times;
              </button>
            </div>
            <div className="panel-content">
              <div>
                <div className="card">
                  <h3 style={{ fontSize: '0.875rem', marginTop: 0, color: '#58a6ff' }}>Entity Kind</h3>
                  <p style={{ margin: 0, fontWeight: 600 }}>{selectedNode.data.label}</p>
                </div>
                <div className="card">
                  <h3 style={{ fontSize: '0.875rem', marginTop: 0, color: '#58a6ff' }}>Description</h3>
                  <textarea
                    style={{
                      width: '100%',
                      background: 'transparent',
                      border: 'none',
                      color: 'var(--text-primary)',
                      resize: 'vertical',
                      minHeight: '100px'
                    }}
                    defaultValue={selectedNode.data.description || 'No description provided.'}
                    onChange={() => {
                      // Logic to update local node data will go here
                    }}
                  />
                </div>
                <div className="card">
                  <h3 style={{ fontSize: '0.875rem', marginTop: 0, color: '#58a6ff' }}>Relationships</h3>
                  <ul style={{ listStyle: 'none', padding: 0, margin: 0, fontSize: '0.875rem' }}>
                    {edges.filter(e => e.source === selectedNode.id || e.target === selectedNode.id).map(edge => (
                      <li key={edge.id} style={{ marginBottom: '8px', padding: '4px 0', borderBottom: '1px solid var(--border-primary)' }}>
                        <span style={edge.source === selectedNode.id ? { fontWeight: 700 } : {}}>
                          {edge.source}
                        </span>
                        <span style={{ color: 'var(--text-secondary)', margin: '0 4px' }}>
                          {edge.label}
                        </span>
                        <span style={edge.target === selectedNode.id ? { fontWeight: 700 } : {}}>
                          {edge.target}
                        </span>
                      </li>
                    ))}
                  </ul>
                </div>
              </div>
            </div>
          </aside>
        )}
      </main>
    </div>
  );
};

export default App;
