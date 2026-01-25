use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub enum NodeType {
    Mission(String),
    Task(String),
    Worker(String),
}

pub struct MentalMap {
    pub graph: DiGraph<NodeType, ()>,
    nodes: HashMap<String, NodeIndex>,
}

impl MentalMap {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            nodes: HashMap::new(),
        }
    }

    pub fn add_mission(&mut self, id: Uuid, name: &str) {
        let node = self.graph.add_node(NodeType::Mission(name.to_string()));
        self.nodes.insert(id.to_string(), node);
    }

    pub fn add_task(&mut self, mission_id: Uuid, task_id: Uuid, name: &str) {
        let node = self.graph.add_node(NodeType::Task(name.to_string()));
        self.nodes.insert(task_id.to_string(), node);

        if let Some(&m_node) = self.nodes.get(&mission_id.to_string()) {
            self.graph.add_edge(m_node, node, ());
        }
    }

    pub fn assign_worker(&mut self, task_id: Uuid, worker_name: &str) {
        let w_id = format!("worker:{}", worker_name);
        let w_node = if let Some(&node) = self.nodes.get(&w_id) {
            node
        } else {
            let node = self
                .graph
                .add_node(NodeType::Worker(worker_name.to_string()));
            self.nodes.insert(w_id, node);
            node
        };

        if let Some(&t_node) = self.nodes.get(&task_id.to_string()) {
            self.graph.add_edge(t_node, w_node, ());
        }
    }
}

impl Default for MentalMap {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mental_map_construction() {
        let mut map = MentalMap::new();
        let m_id = Uuid::new_v4();
        let t_id = Uuid::new_v4();

        map.add_mission(m_id, "Test Mission");
        map.add_task(m_id, t_id, "Test Task");
        map.assign_worker(t_id, "Architect");

        assert_eq!(map.graph.node_count(), 3);
        assert_eq!(map.graph.edge_count(), 2);
    }
}
