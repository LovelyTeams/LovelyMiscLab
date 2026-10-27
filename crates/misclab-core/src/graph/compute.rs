//! The petgraph-backed compute graph: builds from a [`SerializedGraph`], rejects
//! cycles, and yields a topological execution order.

use std::collections::{HashMap, HashSet};

use petgraph::algo::toposort;
use petgraph::graph::DiGraph;

use crate::error::CoreError;
use crate::graph::model::{Edge, NodeId, SerializedGraph};

pub struct ComputeGraph {
    graph: DiGraph<NodeId, ()>,
    /// The original edges, retained for input-gathering during execution.
    pub edges: Vec<Edge>,
}

impl ComputeGraph {
    pub fn from_serialized(g: &SerializedGraph) -> Result<Self, CoreError> {
        let mut graph = DiGraph::new();
        let mut index = HashMap::new();

        for n in &g.nodes {
            if n.id.is_empty() || index.contains_key(&n.id) {
                return Err(CoreError::Graph(format!(
                    "duplicate or empty node id '{}'",
                    n.id
                )));
            }
            let idx = graph.add_node(n.id.clone());
            index.insert(n.id.clone(), idx);
        }

        let mut occupied = HashSet::new();
        for e in &g.edges {
            if !occupied.insert((&e.to.node, &e.to.port)) {
                return Err(CoreError::Graph(format!(
                    "multiple edges feed input '{}.{}'",
                    e.to.node, e.to.port
                )));
            }
            let from = *index.get(&e.from.node).ok_or_else(|| {
                CoreError::Graph(format!("edge references unknown node '{}'", e.from.node))
            })?;
            let to = *index.get(&e.to.node).ok_or_else(|| {
                CoreError::Graph(format!("edge references unknown node '{}'", e.to.node))
            })?;
            graph.add_edge(from, to, ());
        }

        let cg = Self {
            graph,
            edges: g.edges.clone(),
        };
        // Validate acyclicity up front.
        cg.execution_order()?;
        Ok(cg)
    }

    /// Node ids in dependency (topological) order.
    pub fn execution_order(&self) -> Result<Vec<NodeId>, CoreError> {
        match toposort(&self.graph, None) {
            Ok(order) => Ok(order.into_iter().map(|i| self.graph[i].clone()).collect()),
            Err(_) => Err(CoreError::Graph("graph contains a cycle".into())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::model::{NodeInstance, PortRef};

    fn node(id: &str) -> NodeInstance {
        NodeInstance {
            id: id.into(),
            descriptor_id: "text_input".into(),
            params: serde_json::json!({}),
            position: (0.0, 0.0),
        }
    }

    #[test]
    fn rejects_duplicate_or_empty_ids() {
        for nodes in [vec![node("a"), node("a")], vec![node("")]] {
            assert!(ComputeGraph::from_serialized(&SerializedGraph {
                nodes,
                edges: vec![]
            })
            .is_err());
        }
    }

    #[test]
    fn rejects_ambiguous_input_but_accepts_output_fanout() {
        let edge = |from: &str, to: &str| Edge {
            from: PortRef {
                node: from.into(),
                port: "text".into(),
            },
            to: PortRef {
                node: to.into(),
                port: "text".into(),
            },
        };
        let mut graph = SerializedGraph {
            nodes: vec![node("a"), node("b"), node("c")],
            edges: vec![edge("a", "c"), edge("b", "c")],
        };
        assert!(ComputeGraph::from_serialized(&graph).is_err());
        graph.edges = vec![edge("a", "b"), edge("a", "c")];
        let order = ComputeGraph::from_serialized(&graph)
            .unwrap()
            .execution_order()
            .unwrap();
        assert_eq!(order.first().map(String::as_str), Some("a"));
    }
}
