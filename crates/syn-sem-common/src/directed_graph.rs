//! Generic and interned directed graphs with forward and reverse adjacency.
//!
//! Each edge has one source and one target. Graphs retain both outgoing and incoming adjacency so
//! callers can traverse in either direction without changing the direction of the stored edge.
//! Node and edge insertion order is preserved, and cycles are permitted.

use fxhash::FxBuildHasher;
use indexmap::IndexSet;
use std::{hash::Hash, ops::Index};

/// Generic directed graph with outgoing and incoming adjacency.
///
/// Node identity comes exclusively from [`GraphNodeId`], so equal node values may be added as
/// distinct nodes. Repeated edges are ignored while preserving the insertion order of the first
/// edge. Cycles and self-edges are retained without special handling.
#[derive(Debug, Default)]
pub struct DirectedGraph<N> {
    nodes: Vec<N>,
    edges: DirectedEdges,
}

impl<N> DirectedGraph<N> {
    /// Creates an empty directed graph.
    pub const fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: DirectedEdges::new(),
        }
    }

    /// Adds a node and returns its new graph-local id.
    pub fn add_node(&mut self, node: N) -> GraphNodeId {
        let id = self.edges.add_node();
        self.nodes.push(node);
        id
    }

    /// Adds a directed edge from `source` to `target`.
    ///
    /// Returns whether the edge was newly inserted. Repeated edges preserve their original
    /// position and return `false`.
    ///
    /// # Panics
    ///
    /// Panics if either id is out of bounds for this graph.
    pub fn add_edge(&mut self, source: GraphNodeId, target: GraphNodeId) -> bool {
        self.edges.add_edge(source, target)
    }

    /// Returns the number of nodes in the graph.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Returns whether the graph contains no nodes.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Returns all nodes in insertion order.
    pub fn nodes(&self) -> &[N] {
        &self.nodes
    }

    /// Returns all node ids in insertion order.
    pub fn node_ids(&self) -> impl Iterator<Item = GraphNodeId> + '_ {
        (0..self.nodes.len()).map(GraphNodeId)
    }

    /// Returns the direct targets of edges leaving `source`, in edge insertion order.
    ///
    /// # Panics
    ///
    /// Panics if `source` is out of bounds for this graph.
    pub fn outgoing(&self, source: GraphNodeId) -> &[GraphNodeId] {
        self.edges.outgoing(source)
    }

    /// Returns the direct sources of edges entering `target`, in edge insertion order.
    ///
    /// # Panics
    ///
    /// Panics if `target` is out of bounds for this graph.
    pub fn incoming(&self, target: GraphNodeId) -> &[GraphNodeId] {
        self.edges.incoming(target)
    }
}

impl<N> Index<GraphNodeId> for DirectedGraph<N> {
    type Output = N;

    fn index(&self, id: GraphNodeId) -> &Self::Output {
        &self.nodes[id.index()]
    }
}

/// Directed graph that interns equal node values to one graph-local id.
///
/// Unlike [`DirectedGraph`], interning an equal value more than once returns the id assigned by the
/// first insertion. Nodes and edges preserve their first insertion order. Repeated edges are
/// ignored, and cycles and self-edges are retained.
#[derive(Debug)]
pub struct InternedDirectedGraph<N> {
    nodes: IndexSet<N, FxBuildHasher>,
    edges: DirectedEdges,
}

impl<N> InternedDirectedGraph<N>
where
    N: Eq + Hash,
{
    /// Creates an empty interned directed graph.
    pub fn new() -> Self {
        Self {
            nodes: IndexSet::with_hasher(FxBuildHasher::default()),
            edges: DirectedEdges::new(),
        }
    }

    /// Interns a node and returns its graph-local id.
    ///
    /// If an equal node is already present, returns its existing id without replacing its value.
    pub fn intern(&mut self, node: N) -> GraphNodeId {
        let (index, inserted) = self.nodes.insert_full(node);
        let id = GraphNodeId(index);

        if inserted {
            debug_assert_eq!(self.edges.add_node(), id);
        }
        id
    }

    /// Returns the graph-local id of a node equal to `node`.
    pub fn node_id(&self, node: &N) -> Option<GraphNodeId> {
        self.nodes.get_index_of(node).map(GraphNodeId)
    }

    /// Adds a directed edge from `source` to `target`.
    ///
    /// Returns whether the edge was newly inserted. Repeated edges preserve their original
    /// position and return `false`.
    ///
    /// # Panics
    ///
    /// Panics if either id is out of bounds for this graph.
    pub fn add_edge(&mut self, source: GraphNodeId, target: GraphNodeId) -> bool {
        self.edges.add_edge(source, target)
    }

    /// Returns the number of nodes in the graph.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Returns whether the graph contains no nodes.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Returns all nodes in insertion order.
    pub fn nodes(&self) -> impl Iterator<Item = &N> {
        self.nodes.iter()
    }

    /// Returns all node ids in insertion order.
    pub fn node_ids(&self) -> impl Iterator<Item = GraphNodeId> + '_ {
        (0..self.nodes.len()).map(GraphNodeId)
    }

    /// Returns the direct targets of edges leaving `source`, in edge insertion order.
    ///
    /// # Panics
    ///
    /// Panics if `source` is out of bounds for this graph.
    pub fn outgoing(&self, source: GraphNodeId) -> &[GraphNodeId] {
        self.edges.outgoing(source)
    }

    /// Returns the direct sources of edges entering `target`, in edge insertion order.
    ///
    /// # Panics
    ///
    /// Panics if `target` is out of bounds for this graph.
    pub fn incoming(&self, target: GraphNodeId) -> &[GraphNodeId] {
        self.edges.incoming(target)
    }
}

impl<N> Default for InternedDirectedGraph<N>
where
    N: Eq + Hash,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<N> Index<GraphNodeId> for InternedDirectedGraph<N> {
    type Output = N;

    fn index(&self, id: GraphNodeId) -> &Self::Output {
        &self.nodes[id.index()]
    }
}

#[derive(Debug, Default)]
struct DirectedEdges {
    outgoing: Vec<Vec<GraphNodeId>>,
    incoming: Vec<Vec<GraphNodeId>>,
}

impl DirectedEdges {
    const fn new() -> Self {
        Self {
            outgoing: Vec::new(),
            incoming: Vec::new(),
        }
    }

    fn add_node(&mut self) -> GraphNodeId {
        let id = GraphNodeId(self.outgoing.len());
        self.outgoing.push(Vec::new());
        self.incoming.push(Vec::new());
        id
    }

    fn add_edge(&mut self, source: GraphNodeId, target: GraphNodeId) -> bool {
        self.assert_node(source);
        self.assert_node(target);

        if self.outgoing[source.index()].contains(&target) {
            return false;
        }
        self.outgoing[source.index()].push(target);
        self.incoming[target.index()].push(source);
        true
    }

    #[inline]
    fn assert_node(&self, id: GraphNodeId) {
        assert!(
            id.index() < self.outgoing.len(),
            "graph node id must be in bounds"
        );
    }

    fn outgoing(&self, source: GraphNodeId) -> &[GraphNodeId] {
        &self.outgoing[source.index()]
    }

    fn incoming(&self, target: GraphNodeId) -> &[GraphNodeId] {
        &self.incoming[target.index()]
    }
}

/// Stable index of a node within one directed graph.
///
/// Node ids are local to the graph that created them and must not be mixed between graphs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GraphNodeId(usize);

impl GraphNodeId {
    /// Returns the dense zero-based index of this node.
    #[inline]
    pub const fn index(self) -> usize {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_nodes_in_insertion_order() {
        let mut graph = DirectedGraph::default();
        let first = graph.add_node("first");
        let second = graph.add_node("second");

        assert_eq!(graph.nodes(), &["first", "second"]);
        assert_eq!(graph[first], "first");
        assert_eq!(graph.node_ids().collect::<Vec<_>>(), &[first, second]);
        assert_eq!(graph.len(), 2);
        assert!(!graph.is_empty());
    }

    #[test]
    fn equal_values_remain_distinct_nodes() {
        let mut graph = DirectedGraph::default();
        let first = graph.add_node("same");
        let second = graph.add_node("same");

        assert_ne!(first, second);
        assert_eq!(graph.nodes(), &["same", "same"]);
    }

    #[test]
    fn stores_outgoing_and_incoming_edges_once() {
        let mut graph = DirectedGraph::default();
        let source = graph.add_node("source");
        let first = graph.add_node("first");
        let second = graph.add_node("second");

        assert!(graph.add_edge(source, first));
        assert!(graph.add_edge(source, second));
        assert!(!graph.add_edge(source, first));

        assert_eq!(graph.outgoing(source), &[first, second]);
        assert_eq!(graph.incoming(first), &[source]);
        assert_eq!(graph.incoming(second), &[source]);
        assert!(graph.outgoing(first).is_empty());
    }

    #[test]
    fn retains_cycles_and_self_edges() {
        let mut graph = DirectedGraph::default();
        let a = graph.add_node("a");
        let b = graph.add_node("b");

        assert!(graph.add_edge(a, b));
        assert!(graph.add_edge(b, a));
        assert!(graph.add_edge(a, a));

        assert_eq!(graph.outgoing(a), &[b, a]);
        assert_eq!(graph.outgoing(b), &[a]);
        assert_eq!(graph.incoming(a), &[b, a]);
        assert_eq!(graph.incoming(b), &[a]);
    }

    #[test]
    fn interns_equal_nodes_to_one_id() {
        let mut graph = InternedDirectedGraph::default();
        let first = graph.intern(String::from("same"));
        let second = graph.intern(String::from("same"));

        assert_eq!(first, second);
        assert_eq!(graph.node_id(&String::from("same")), Some(first));
        assert_eq!(graph.nodes().collect::<Vec<_>>(), &[&String::from("same")]);
        assert_eq!(graph.node_ids().collect::<Vec<_>>(), &[first]);
        assert_eq!(graph.len(), 1);
        assert!(!graph.is_empty());
    }

    #[test]
    fn interned_graph_stores_edges_in_both_directions() {
        let mut graph = InternedDirectedGraph::default();
        let source = graph.intern("source");
        let target = graph.intern("target");

        assert!(graph.add_edge(source, target));
        assert!(!graph.add_edge(source, target));
        assert!(graph.add_edge(target, source));
        assert!(graph.add_edge(source, source));

        assert_eq!(graph[source], "source");
        assert_eq!(graph.outgoing(source), &[target, source]);
        assert_eq!(graph.outgoing(target), &[source]);
        assert_eq!(graph.incoming(source), &[target, source]);
        assert_eq!(graph.incoming(target), &[source]);
    }
}
