#![warn(rust_2018_idioms)]

use source::FragmentedSourceRange;

pub use builder::TreeBuilder;
pub use kind::*;

mod builder;
mod kind;

pub type Token = lex::Token<TokenKind>;

#[derive(Debug)]
pub struct Node {
    kind: NodeKind,
    range: FragmentedSourceRange,
    children: Vec<Element>,
}

impl Node {
    pub fn new(kind: NodeKind, children: Vec<Element>) -> Self {
        let first = children.first().expect("passed empty children");
        let last = children.last().unwrap();

        let range = FragmentedSourceRange::new(first.range().start, last.range().end);

        Self {
            kind,
            range,
            children,
        }
    }

    #[inline]
    pub fn kind(&self) -> NodeKind {
        self.kind
    }

    #[inline]
    pub fn range(&self) -> FragmentedSourceRange {
        self.range
    }

    pub fn children(&self) -> impl Iterator<Item = &'_ Element> {
        self.children.iter()
    }

    pub fn child_nodes(&self) -> impl Iterator<Item = &'_ Node> {
        self.children().filter_map(Element::as_node)
    }

    pub fn child_tokens(&self) -> impl Iterator<Item = &'_ Token> {
        self.children().filter_map(Element::as_token)
    }
}

#[derive(Debug)]
pub enum Element {
    Node(Node),
    Token(Token),
}

impl Element {
    pub fn as_node(&self) -> Option<&Node> {
        match self {
            Element::Node(node) => Some(node),
            _ => None,
        }
    }

    pub fn as_token(&self) -> Option<&Token> {
        match self {
            Element::Token(tok) => Some(tok),
            _ => None,
        }
    }

    pub fn range(&self) -> FragmentedSourceRange {
        match self {
            Element::Node(node) => node.range(),
            Element::Token(tok) => tok.range.into(),
        }
    }
}

impl From<Token> for Element {
    #[inline]
    fn from(v: Token) -> Self {
        Element::Token(v)
    }
}

impl From<Node> for Element {
    #[inline]
    fn from(v: Node) -> Self {
        Element::Node(v)
    }
}
