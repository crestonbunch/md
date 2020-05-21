use pomelo::pomelo;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Span {
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
}

impl Span {
    /// Create a new span that only spans a single line.
    pub fn new(line: usize, start_col: usize, end_col: usize) -> Span {
        Span {
            start_line: line,
            end_line: line,
            start_col,
            end_col,
        }
    }
}

// Lemon language definition adapted from:
// https://github.com/fletcher/MultiMarkdown-6/blob/develop/Sources/libMultiMarkdown/parser.y

pomelo! {
    %include {
        use serde::Serialize;

        use crate::markdown::ast::Line;
        use crate::markdown::parse::Span;
    }
    %token #[derive(Debug, Clone, Serialize)] pub enum Token {};
    %extra_token Span;

    // These terminal types are extracted by the lexer
    %type NewLine String;
    %type Whitespace String;
    %type Hash1 String;
    %type PlainText String;

    // Each line gets parsed and identified by the lexer
    %type LinePlain Vec<Token>;
    %type LineHeader (usize, Vec<Token>);

    // These non-terminal types are built by the parser
    %type doc Vec<Line>;
    %type blocks Vec<Line>;
    %type block Line;
    %type header Line;
    %type paragraph Line;
    %type empty Line;

    %fallback PlainText NewLine Hash1 Whitespace;

    doc ::= blocks(b) { b };

    blocks ::= blocks(b) block(c) { [b, vec![c]].concat() };
    blocks ::= block(b) { vec![b] };

    // Single line blocks
    block ::= LineHeader((span, (s, t))) { Line::Header(span, s, t) };

    // Multiline blocks
    block ::= paragraph(p) { p };
    block ::= empty(b) { b };

    // Paragraphs
    paragraph ::= LinePlain((a, b)) { Line::Paragraph(a, b) };

    // Empty lines
    // empty ::= empty(b) LineEmpty(c) { Line::Empty };
    empty ::= LineEmpty(span) { Line::Empty(span) };
}

pub use parser::*;
