use super::vec::{GenIndex, GenOptVec};

#[derive(Debug)]
pub struct Tree<V> {
    pub(crate) nodes: GenOptVec<Node<V>>,
}

impl Tree<()> {
    pub const ROOT: NodeIndex = NodeIndex(GenIndex::ignore_gen(0));
}

impl<V: Default> Tree<V> {
    pub fn with_default() -> Self {
        Self::new(V::default())
    }
}

impl<V> Tree<V> {
    pub fn new(root: V) -> Self {
        // Empty root node.
        let root_node = Node {
            value: root,
            parent: Tree::ROOT.0,
            children: Vec::new(),
        };

        let mut nodes = GenOptVec::new();
        let i = nodes.add(root_node);
        debug_assert_eq!(i.into_inner(), Tree::ROOT.0.into_inner());

        Self { nodes }
    }

    pub fn clear(&mut self) {
        // Removes all values except the root node.
        self.nodes.truncate(1);
    }

    pub fn is_leaf(&self, index: NodeIndex) -> bool {
        if let Some(node) = self.nodes.get(index.0) {
            node.children.is_empty()
        } else {
            false
        }
    }

    pub fn is_non_leaf(&self, index: NodeIndex) -> bool {
        if let Some(node) = self.nodes.get(index.0) {
            !node.children.is_empty()
        } else {
            false
        }
    }

    pub fn parent(&self, index: NodeIndex) -> Option<NodeIndex> {
        self.nodes.get(index.0).map(|node| NodeIndex(node.parent))
    }

    pub fn get(&self, index: NodeIndex) -> Option<&V> {
        self.nodes.get(index.0).map(|node| &node.value)
    }

    pub fn get_mut(&mut self, index: NodeIndex) -> Option<&mut V> {
        self.nodes.get_mut(index.0).map(|node| &mut node.value)
    }

    /// Traverses sub-tree starting with the `from` node in a DFS way.
    ///
    /// # Panics
    ///
    /// Panics if the given node index is out of bounds.
    pub fn traverse_from<F, R>(&self, from: NodeIndex, mut f: F) -> Option<R>
    where
        F: FnMut(&V) -> Option<R>,
    {
        fn traverse<V, F, R>(this: &Tree<V>, from: NodeIndex, f: &mut F) -> Option<R>
        where
            F: FnMut(&V) -> Option<R>,
        {
            let node = this.nodes.get(from.0)?;
            let ret = f(&node.value);
            if ret.is_some() {
                return ret;
            }

            for child in &node.children {
                let ret = traverse(this, NodeIndex(*child), f);
                if ret.is_some() {
                    return ret;
                }
            }

            None
        }

        traverse(self, from, &mut f)
    }

    pub fn next_index(&self) -> NodeIndex {
        NodeIndex(self.nodes.next_index())
    }

    /// Inserts a node with the given value under the parent node, then returns true if the
    /// operation was successful.
    pub fn insert(&mut self, parent: NodeIndex, value: V) -> Option<NodeIndex> {
        self.nodes.get(parent.0)?;

        let index = self.nodes.add(Node {
            value,
            parent: parent.0,
            children: Vec::new(),
        });
        let parent_node = self.nodes.get_mut(parent.0).unwrap();
        parent_node.children.push(index);
        Some(NodeIndex(index))
    }

    /// Takes out the value at the given index.
    ///
    /// Other indices are not affected by this operation, but the whole sub-tree starting with the
    /// node will be removed.
    pub fn take(&mut self, index: NodeIndex) -> Option<V> {
        let node = self.nodes.take(index.0)?;

        // Destroys all descendants of the node.
        for index in node.children {
            self.take(NodeIndex(index));
        }

        Some(node.value)
    }
}

#[derive(Debug)]
pub struct Node<V> {
    value: V,
    parent: GenIndex,
    children: Vec<GenIndex>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct NodeIndex(GenIndex);
