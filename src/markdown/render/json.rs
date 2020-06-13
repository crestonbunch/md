use std::rc::Rc;

use serde::Serialize;
use wasm_bindgen::JsValue;

use crate::markdown::{Kind, Node};

#[derive(Serialize)]
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

#[derive(Serialize)]
pub struct N {
    pub kind: K,
    pub span: (usize, usize),
    pub children: Option<Vec<N>>,
    pub text: Option<String>,
}

impl N {
    fn new(source: &str, node: Node) -> N {
        match node.kind {
            Kind::Document => N {
                kind: K::Document,
                span: (node.start, node.end.unwrap()),
                children: render_children(source, node),
                text: None,
            },
            Kind::BlockQuote => N {
                kind: K::BlockQuote,
                span: (node.start, node.end.unwrap()),
                children: render_children(source, node),
                text: None,
            },
            Kind::Empty => N {
                kind: K::Empty,
                span: (node.start, node.end.unwrap()),
                children: render_children(source, node),
                text: None,
            },
            Kind::UnorderedList(..) => N {
                kind: K::UnorderedList,
                span: (node.start, node.end.unwrap()),
                children: render_children(source, node),
                text: None,
            },
            Kind::OrderedList(..) => N {
                kind: K::OrderedList,
                span: (node.start, node.end.unwrap()),
                children: render_children(source, node),
                text: None,
            },
            Kind::ListItem(..) => N {
                kind: K::ListItem,
                span: (node.start, node.end.unwrap()),
                children: render_children(source, node),
                text: None,
            },
            Kind::Heading(size) => render_heading(source, node, size),
            Kind::Paragraph => N {
                kind: K::Paragraph,
                span: (node.start, node.end.unwrap()),
                children: render_children(source, node),
                text: None,
            },
            Kind::EmptyLine => N {
                kind: K::EmptyLine,
                span: (node.start, node.end.unwrap()),
                children: None,
                text: Some((&source[node.start..node.end.unwrap()]).into()),
            },
            Kind::Plaintext => N {
                kind: K::Plaintext,
                span: (node.start, node.end.unwrap()),
                children: None,
                text: Some((&source[node.start..node.end.unwrap()]).into()),
            },
            Kind::Whitespace => N {
                kind: K::Whitespace,
                span: (node.start, node.end.unwrap()),
                children: None,
                text: Some((&source[node.start..node.end.unwrap()]).into()),
            },
        }
    }
}

fn render_children(source: &str, node: Node) -> Option<Vec<N>> {
    Some(
        node.children
            .into_iter()
            .map(|n| N::new(source, Rc::try_unwrap(n).unwrap().into_inner()))
            .collect(),
    )
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
    N {
        kind,
        span: (node.start, node.end.unwrap()),
        children: render_children(source, node),
        text: None,
    }
}

pub fn render(source: &str, node: Node) -> String {
    let n = N::new(source, node);
    serde_json::to_string(&n).unwrap()
}
