use serde::Serialize;

use crate::markdown::parse::Span;
use crate::markdown::parse::Token;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum Line {
    Empty(Span),
    EmptyParagraph(Span),
    AtxHeader(Span, usize, Vec<Token>),
    SetextHeader(Span, usize, Vec<Token>, Token),
    Paragraph(Span, Vec<Token>),
}
