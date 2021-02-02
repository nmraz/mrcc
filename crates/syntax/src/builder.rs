use crate::{Element, Node, NodeKind, Token};

#[derive(Default)]
pub struct TreeBuilder {
    pending_nodes: Vec<(NodeKind, usize)>,
    pending_children: Vec<Element>,
}

impl TreeBuilder {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start_node(&mut self, kind: NodeKind) {
        self.pending_nodes.push((kind, self.pending_children.len()));
    }

    pub fn finish_node(&mut self) {
        let (kind, first_child) = self
            .pending_nodes
            .pop()
            .expect("no pending nodes to finish");

        let children = self.pending_children.split_off(first_child);
        let node = Node::new(kind, children);
        self.pending_children.push(node.into());
    }

    pub fn token(&mut self, tok: Token) {
        self.pending_children.push(tok.into());
    }

    pub fn finish(mut self) -> Node {
        assert!(
            self.pending_nodes.is_empty(),
            "builder has unfinished nodes"
        );
        assert!(
            self.pending_children.len() == 1,
            "builder has disconnected children"
        );

        let root_elem = self.pending_children.remove(0);

        match root_elem {
            Element::Node(node) => node,
            _ => panic!("root of tree must be a node"),
        }
    }
}
