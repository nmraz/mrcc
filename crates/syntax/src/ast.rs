use crate::Node;

pub trait AstNode<'a> {
    fn syntax(&self) -> &'a Node;

    fn cast(syntax: &'a Node) -> Option<Self>
    where
        Self: Sized;
}

pub fn children<'a, N: AstNode<'a> + 'a>(syntax: &'a Node) -> impl Iterator<Item = N> + 'a {
    syntax.child_nodes().filter_map(N::cast)
}
