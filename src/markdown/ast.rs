use serde::Serialize;

use crate::markdown::parse::Span;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum Block {
    Container(ContainerBlock),
    Leaf(LeafBlock),
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum ContainerBlock {
    // TODO:
    Blockquote(Span, Vec<Block>),
    UnorderedList(Span, Vec<Block>),
    OrderedList(Span, Vec<Block>),
    ListItem(Span, Vec<Block>),
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum LeafBlock {
    Empty(Span),
    Paragraph(Span, Vec<Line>),
    Header(Span, usize, Line),
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum Line {
    Plaintext(Span, String),
}
