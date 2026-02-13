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
import {
  Save,
  FileJson,
  Share2,
  AlignCenter,
  Box,
  Bot,
  FileText,
  PlayCircle
} from 'lucide-react';
import { loadOntology } from './services/schemaService';
import { simulateExecution } from './services/simulationService';
import type { SimulationStep } from './services/simulationService';
import SimulationPanel from './components/SimulationPanel';
import { Handle, Position } from 'reactflow';

const nodeWidth = 180;

const CustomNode = ({ data }: { data: any }) => {
  const isRoot = data.kind === 'SoftwareApplication';
  const isAgent = data.kind === 'Agent';
  const isArtifact = data.kind === 'Artifact';

  return (
    <div className="glass-panel" style={{
      background: 'rgba(22, 27, 34, 0.8)',
      color: '#c9d1d9',
      border: '1px solid rgba(255, 255, 255, 0.1)',
      borderRadius: '8px',
      padding: '10px',
      width: nodeWidth,
      textAlign: 'center',
      position: 'relative'
    }}>
      <Handle type="target" position={Position.Left} style={{ background: '#555' }} />

      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '8px' }}>
        {isRoot && <Box size={16} color="#58a6ff" />}
        {isAgent && <Bot size={16} color="#7ee787" />}
        {isArtifact && <FileText size={16} color="#79c0ff" />}
        <span style={{ fontWeight: 500, fontSize: '0.9rem' }}>{data.label}</span>
        {data.hasHidden && <span style={{ fontSize: '10px', marginLeft: '4px' }}>➕</span>}
      </div>

      <Handle type="source" position={Position.Right} style={{ background: '#555' }} />
    </div>
  );
};

const nodeTypes = {
  custom: CustomNode,
};

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

  // Simulation State
  const [showSimulation, setShowSimulation] = useState(false);
  const [simulationSteps, setSimulationSteps] = useState<SimulationStep[]>([]);
  const [currentStepIndex, setCurrentStepIndex] = useState(-1);
  const [simulationLayoutMode, setSimulationLayoutMode] = useState<'ontology' | 'path'>('ontology');

  // Exploration Mode State
  const [searchMode, setSearchMode] = useState(false); // If true, progressive disclosure
  const [visibleNodeIds, setVisibleNodeIds] = useState<Set<string>>(new Set(['SoftwareApplication']));
  const [rfInstance, setRfInstance] = useState<any>(null); // Store ReactFlow instance

  useEffect(() => {
    async function init() {
      try {
        const data = await loadOntology();

        // Calculate degrees for default view if needed, or rely on layout
        // For now, we trust the nodes/edges from service are correct.

        const initialNodes = data.nodes;
        const initialEdges = data.edges;

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
        // Check if has hidden OUTGOING neighbors
        const outgoingNeighbors = initialLayout.edges
          .filter(e => e.source === node.id)
          .map(e => e.target);

        const hasHidden = outgoingNeighbors.some(nid => !visibleNodeIds.has(nid));

        // Clone node and update data for CustomNode
        return {
          ...node,
          data: {
            ...node.data,
            hasHidden: hasHidden
          }
        };
      });
      activeEdges = activeEdges.filter(e => visibleNodeIds.has(e.source) && visibleNodeIds.has(e.target));
    }

    // 2. Apply Simulation Progressive Visibility
    else if (showSimulation && simulationSteps.length > 0) {
      const producedNodeIds = new Set<string>(['SoftwareApplication']);

      // All nodes produced up to the currentStepIndex
      for (let i = 0; i <= currentStepIndex; i++) {
        const step = simulationSteps[i];
        if (step.verbType === 'Creation') {
          producedNodeIds.add(step.target);
        }
      }

      // Filter nodes: Show produced artifacts AND all Agents (actors)
      activeNodes = activeNodes.filter(node =>
        producedNodeIds.has(node.id) ||
        node.data.kind === 'Agent' ||
        node.id === 'SoftwareApplication'
      );

      // Filter edges: Only show edges between currently visible nodes
      const visibleNodeIdsSet = new Set(activeNodes.map(n => n.id));
      activeEdges = activeEdges.filter(edge =>
        visibleNodeIdsSet.has(edge.source) && visibleNodeIdsSet.has(edge.target)
      );
    }

    // 3. Apply Orphan Filter (Only if NOT in simulation or search mode)
    else if (!showOrphans) {
      const connectedNodeIds = new Set<string>();
      initialLayout.edges.forEach(edge => {
        connectedNodeIds.add(edge.source);
        connectedNodeIds.add(edge.target);
      });
      activeNodes = activeNodes.filter(node => connectedNodeIds.has(node.id));
    }

    // 4. Apply Simulation Highlighting
    if (showSimulation && simulationSteps.length > 0) {
      const activeStep = currentStepIndex >= 0 ? simulationSteps[currentStepIndex] : null;

      activeNodes = activeNodes.map(node => {
        let className = node.className || '';
        // Check if it was produced in THIS specific simulation run
        const producedInSteps = simulationSteps.slice(0, currentStepIndex + 1).some(s => s.target === node.id && s.verbType === 'Creation');

        if (producedInSteps || node.id === 'SoftwareApplication') {
          className += ' node-produced';
        }
        if (activeStep?.target === node.id) {
          className += ' node-highlighted';
        }
        if (activeStep?.context?.includes(node.id)) {
          className += ' node-context';
        }

        return {
          ...node,
          className: className.trim()
        };
      });

      activeEdges = activeEdges.map(edge => {
        let className = edge.className || '';
        if (activeStep && edge.source === activeStep.agent && edge.target === activeStep.target) {
          className += ' edge-active';
        }
        return {
          ...edge,
          className: className.trim()
        };
      });
    }

    let layouted = getLeftToRightLayout(activeNodes, activeEdges);

    // 6. Handle Layout Modes
    if (showSimulation && simulationSteps.length > 0) {
      if (simulationLayoutMode === 'path') {
        // Arrange nodes in a horizontal line based on execution order
        // Find FIRST appearance of each node in simulation
        const nodeFirstAppearance = new Map<string, number>();
        nodeFirstAppearance.set('SoftwareApplication', -1);

        simulationSteps.forEach((step, idx) => {
          if (!nodeFirstAppearance.has(step.agent)) nodeFirstAppearance.set(step.agent, idx);
          if (!nodeFirstAppearance.has(step.target)) nodeFirstAppearance.set(step.target, idx);
        });

        const sortedNodeIds = Array.from(nodeFirstAppearance.entries())
          .sort((a, b) => a[1] - b[1])
          .map(e => e[0]);

        const pathNodes = activeNodes.map(node => {
          const idx = sortedNodeIds.indexOf(node.id);
          return {
            ...node,
            position: { x: idx * 250, y: 300 + (idx % 2 === 0 ? 0 : 100) } // Zig-zag for better vertical space
          };
        });
        setNodes(pathNodes);
        setEdges(activeEdges);
      } else if (initialLayout) {
        // ONTOLOGY MODE - Stable Positions
        const stableNodes = activeNodes.map(node => {
          const initialNode = initialLayout.nodes.find(n => n.id === node.id);
          return {
            ...node,
            position: initialNode?.position || node.position
          };
        });
        setNodes(stableNodes);
        setEdges(layouted.edges);
      }
    } else {
      setNodes(layouted.nodes);
      setEdges(layouted.edges);
    }

    // 7. Fit View if in Exploration Mode, Simulation, or orphans toggled
    if (rfInstance && (searchMode || showSimulation || !showOrphans)) {
      setTimeout(() => {
        rfInstance.fitView({ padding: 0.2, duration: 800 });
      }, 100); // Small delay to allow render
    }
  }, [showOrphans, searchMode, visibleNodeIds, initialLayout, setNodes, setEdges, rfInstance, showSimulation, simulationSteps, currentStepIndex, simulationLayoutMode]);

  const onConnect = useCallback(
    (params: Connection) => setEdges((eds) => addEdge(params, eds)),
    [setEdges]
  );

  const onLayout = useCallback(() => {
    // Trigger re-render which triggers layout effect
    if (initialLayout) {
      const layouted = getLeftToRightLayout(nodes, edges);
      setNodes(layouted.nodes);
    }
  }, [nodes, edges, initialLayout, setNodes]);

  const runSimulation = useCallback(() => {
    if (!initialLayout) return;
    const steps = simulateExecution(initialLayout.nodes, initialLayout.edges);
    setSimulationSteps(steps);
    setCurrentStepIndex(-1); // Reset to start
    setShowSimulation(true);
    setIsPanelOpen(false); // Close properties panel to avoid clutter
    setSearchMode(false); // Likely want full graph visible or at least default view
  }, [initialLayout]);

  const onNodeClick = (_: React.MouseEvent, node: Node) => {
    setSelectedNode(node);
    setIsPanelOpen(true);

    if (searchMode && initialLayout) {
      const getDescendants = (id: string, visited = new Set<string>()): string[] => {
        const descendants: string[] = [];
        initialLayout.edges.forEach(e => {
          if (e.source === id && !visited.has(e.target)) {
            visited.add(e.target);
            descendants.push(e.target, ...getDescendants(e.target, visited));
          }
        });
        return descendants;
      };

      // Find OUTGOING neighbors (children) in full graph
      const childrenIds = new Set<string>();
      initialLayout.edges.forEach(e => {
        if (e.source === node.id) childrenIds.add(e.target);
      });

      if (childrenIds.size === 0) return; // Leaf node

      const newIds = new Set(visibleNodeIds);
      const allChildrenVisible = Array.from(childrenIds).every(id => visibleNodeIds.has(id));

      if (allChildrenVisible) {
        // RECURSIVE DELETE
        const toDelete = new Set([node.id, ...getDescendants(node.id)]);
        toDelete.delete(node.id); // Keep the clicked node itself visible
        toDelete.forEach(id => newIds.delete(id));
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
          <button
            className={`btn btn-sm ${showSimulation ? 'active' : ''}`}
            onClick={() => setShowSimulation(!showSimulation)}
            style={showSimulation ? { background: 'rgba(56, 139, 253, 0.15)', color: '#58a6ff', borderColor: '#58a6ff' } : {}}
          >
            <PlayCircle size={14} /> Simulate
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
            nodeTypes={nodeTypes}
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

        <SimulationPanel
          isOpen={showSimulation}
          onClose={() => setShowSimulation(false)}
          steps={simulationSteps}
          currentStepIndex={currentStepIndex}
          runSimulation={runSimulation}
          onStepClick={(step) => {
            if (rfInstance) {
              const node = nodes.find(n => n.id === step.target);
              if (node) {
                rfInstance.setCenter(node.position.x + 90, node.position.y, { zoom: 1.5, duration: 800 });
              }
            }
          }}
          onSetStepIndex={setCurrentStepIndex}
          layoutMode={simulationLayoutMode}
          onSetLayoutMode={setSimulationLayoutMode}
        />

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
