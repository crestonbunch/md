use serde::Serialize;

use crate::markdown::parse::Span;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum Block {
    Container(ContainerBlock),
    Leaf(LeafBlock),
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum ContainerBlock {
    Blockquote(Span, Vec<Block>),
    UnorderedList(Span, usize, Vec<Block>),
    OrderedList(Span, usize, Vec<Block>),
    ListItem(Span, usize, Vec<Block>),
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum LeafBlock {
    Empty(Span, Vec<Inline>),
    Paragraph(Span, Vec<Inline>),
    Header(Span, usize, Vec<Inline>),
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum Inline {
    Empty(Span),
    Plaintext(Span, String),
}
