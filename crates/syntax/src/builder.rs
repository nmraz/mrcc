use crate::{Element, Node, NodeKind, Token};

pub struct Checkpoint(usize);

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

    #[inline]
    pub fn checkpoint(&self) -> Checkpoint {
        Checkpoint(self.pending_children.len())
    }

    pub fn start_node(&mut self, kind: NodeKind) {
        self.pending_nodes.push((kind, self.pending_children.len()));
    }

    pub fn start_node_at(&mut self, checkpoint: Checkpoint, kind: NodeKind) {
        let checkpoint = checkpoint.0;

        assert!(
            checkpoint <= self.pending_children.len(),
            "checkpoint points to nonexistent location"
        );

        if let Some(&(_, deepest_first_child)) = self.pending_nodes.last() {
            assert!(
                checkpoint >= deepest_first_child,
                "checkpoint intersects pending node"
            );
        }

        self.pending_nodes.push((kind, checkpoint));
    }

    pub fn finish_node(&mut self) -> Checkpoint {
        let (kind, first_child) = self
            .pending_nodes
            .pop()
            .expect("no pending nodes to finish");

        let children = self.pending_children.split_off(first_child);
        let node = Node::new(kind, children);

        let checkpoint = self.checkpoint();
        self.pending_children.push(node.into());
        checkpoint
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
