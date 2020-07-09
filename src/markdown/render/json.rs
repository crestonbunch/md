use std::convert::Into;

use serde::Serialize;

use crate::markdown::{Kind, Node};

#[derive(Serialize, Copy, Clone)]
pub enum K {
    // Container block tokens
    Document,
    BlockQuote,
    Empty,
    UnorderedList,
    OrderedList,
    ListItem,
    // Leaf block tokens
    Heading1,
    Heading2,
    Heading3,
    Heading4,
    Heading5,
    Heading6,
    Paragraph,
    EmptyLine,
    // Inline tokens
    Plaintext,
    Whitespace,
}

impl Into<i64> for K {
    fn into(self) -> i64 {
        match self {
            K::Document => 1,
            K::BlockQuote => 2,
            K::Empty => 3,
            K::UnorderedList => 4,
            K::OrderedList => 5,
            K::ListItem => 6,
            K::Heading1 => 7,
            K::Heading2 => 8,
            K::Heading3 => 9,
            K::Heading4 => 10,
            K::Heading5 => 11,
            K::Heading6 => 12,
            K::Paragraph => 13,
            K::EmptyLine => 14,
            K::Plaintext => 15,
            K::Whitespace => 16,
        }
    }
}

#[derive(Serialize)]
pub struct N {
    pub kind: K,
    pub span: (usize, usize),
    pub merkle: i64,
    pub children: Option<Vec<N>>,
    pub text: Option<String>,
}

impl N {
    fn new(source: &str, node: Node) -> N {
        match node.kind {
            Kind::Document => render_container(K::Document, source, node),
            Kind::BlockQuote => render_container(K::BlockQuote, source, node),
            Kind::Empty => render_container(K::Empty, source, node),
            Kind::UnorderedList(..) => render_container(K::UnorderedList, source, node),
            Kind::OrderedList(..) => render_container(K::OrderedList, source, node),
            Kind::ListItem => render_container(K::ListItem, source, node),
            Kind::Heading(size) => render_heading(source, node, size),
            Kind::Paragraph => render_container(K::Paragraph, source, node),
            Kind::EmptyLine => render_inline(K::EmptyLine, source, node),
            Kind::Plaintext => render_inline(K::Plaintext, source, node),
            Kind::Whitespace => render_inline(K::Whitespace, source, node),
        }
    }
}

fn render_container(kind: K, source: &str, node: Node) -> N {
    let (start, end) = node.span;
    let children = render_children(source, node);
    N {
        kind,
        span: (start, end),
        merkle: hash_n(kind, (start, end), &children, &None),
        children,
        text: None,
    }
}

fn render_inline(kind: K, source: &str, node: Node) -> N {
    let (start, end) = node.span;
    let text = &source[start..end];
    N {
        kind,
        span: (start, end),
        merkle: hash_str(text),
        children: None,
        text: Some(text.into()),
    }
}

fn render_children(source: &str, node: Node) -> Option<Vec<N>> {
    match node.kind {
        Kind::EmptyLine | Kind::Plaintext | Kind::Whitespace => None,
        _ => Some(
            node.children
                .into_iter()
                .map(|n| N::new(source, n))
                .collect(),
        ),
    }
}

fn render_heading(source: &str, node: Node, size: usize) -> N {
    let kind = match size {
        1 => K::Heading1,
        2 => K::Heading2,
        3 => K::Heading3,
        4 => K::Heading4,
        5 => K::Heading5,
        _ => K::Heading6,
    };
    render_container(kind, source, node)
}

pub fn render(source: &str, node: Node) -> String {
    let n = N::new(source, node);
    serde_json::to_string(&n).unwrap()
}

fn hash_n(kind: K, span: (usize, usize), children: &Option<Vec<N>>, text: &Option<String>) -> i64 {
    let hash: i64 = match (children, text) {
        (Some(v), None) => hash_vec(v),
        (None, Some(s)) => hash_str(&s[..]),
        _ => 0,
    };
    let (start, end) = span;
    let start = start as i64;
    let end = end as i64;
    let kind: i64 = kind.into();
    start + 11 * end + 17 * hash + 31 * kind
}

fn hash_str(s: &str) -> i64 {
    s.chars().into_iter().fold(0, |hash, c| {
        let h = (hash << 5) - hash + (c as i64);
        h | 0
    })
}

fn hash_vec(v: &Vec<N>) -> i64 {
    v.iter().fold(0, |hash, n| {
        let h = (hash << 5) - hash + n.merkle;
        h | 0
    })
}
