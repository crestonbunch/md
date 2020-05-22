use serde::Serialize;

use crate::markdown::parse::Span;
use crate::markdown::parse::Token;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum Line {
    Empty(Span),
    Header(Span, usize, Vec<Token>),
    Paragraph(Span, Vec<Token>),
}
