use std::cmp::Ordering;
use std::ops::Add;

use pomelo::pomelo;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Span {
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
}

impl Span {
    /// Create a new span that only spans a single line.
    pub fn single_line(line: usize, start_col: usize, end_col: usize) -> Span {
        Span {
            start_line: line,
            end_line: line,
            start_col,
            end_col,
        }
    }

    /// Create a new span that spans many lines.
    pub fn multi_line(
        start_line: usize,
        end_line: usize,
        start_col: usize,
        end_col: usize,
    ) -> Span {
        Span {
            start_line,
            end_line,
            start_col,
            end_col,
        }
    }
}

impl Add for Span {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        match self.start_line.cmp(&other.start_line) {
            Ordering::Less => Span::multi_line(
                self.start_line,
                other.end_line,
                self.start_col,
                other.end_col,
            ),
            Ordering::Greater => Span::multi_line(
                other.start_line,
                self.end_line,
                other.start_col,
                self.end_col,
            ),
            Ordering::Equal => {
                let start_col = std::cmp::min(self.start_col, other.start_col);
                let end_col = std::cmp::max(self.end_col, other.end_col);
                Span::multi_line(self.start_line, self.start_line, start_col, end_col)
            }
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
    %token #[derive(Debug, Clone, Serialize, PartialEq, Eq)] pub enum Token {};
    %extra_token Span;

    // These terminal types are extracted by the lexer
    %type NewLine String;
    %type Whitespace String;
    %type Atx (usize, String);
    %type PlainText String;

    // Each line gets parsed and identified by the lexer
    %type LinePlain Vec<Token>;
    %type LineContinuation Vec<Token>;
    %type LineHeader (usize, Vec<Token>);

    %type LineHrOrSetext String;
    %type LineHr String;
    %type LineSetext String;

    // These non-terminal types are built by the parser
    %type doc Vec<Line>;
    %type blocks Vec<Line>;
    %type block Line;
    %type chunk (Span, Vec<Token>);
    %type chunk_line (Span, Vec<Token>);
    %type header Line;
    %type paragraph Line;
    %type setext_1 Line;
    %type setext_2 Line;
    %type empty Line;
    %type empty_chunk Span;
    %type empty_chunk_line Span;
    %type empty_paragraph Line;

    // %fallback PlainText NewLine Whitespace LineHrOrSetext LineHr LineSetext;
    %fallback EmptyLineContinuation LineEmpty;
    %fallback LineContinuation LinePlain;

    doc ::= blocks(b) { b };

    blocks ::= block(b) { vec![b] };
    blocks ::= blocks(b) block(c) { [b, vec![c]].concat() };

    // Single line blocks
    block ::= LineHeader((span, (s, t))) { Line::AtxHeader(span, s, t) };

    // Multiline blocks
    block ::= paragraph(p) { p };
    block ::= setext_1(p) { p };
    block ::= setext_2(p) { p };
    block ::= empty(b) { b };
    block ::= empty_paragraph(b) { b };

    // A chunk is a grouping of lines _not_ separated by an empty line.
    // These lines merge into the first line (whatever type that is.)
    chunk ::= chunk_line(a) { a };
    chunk ::= chunk((a, b)) chunk_line((c, d)) { (a + c, [b, d].concat()) };
    // Any LinePlains after an initial LinePlain (not separated by another
    // token) will become a LineContinuation because nothing else can parse it,
    // and we have specified the %fallback for LineContinuation.
    chunk_line ::= LineContinuation((span, tokens)) { (span, tokens) };

    // Paragraphs
    paragraph ::= LinePlain((a, b)) chunk((c, d)) { Line::Paragraph(a + c, [b, d].concat()) };
    paragraph ::= LinePlain((a, b)) { Line::Paragraph(a, b) };

    setext_1 ::= paragraph(Line::Paragraph(a, b)) LineSetext((c, d)) {
        Line::SetextHeader(a + c.clone(), 1, b, Token::PlainText((c, d)))
    };
    setext_2 ::= paragraph(Line::Paragraph(a, b)) LineHrOrSetext((c, d)) {
        Line::SetextHeader(a + c.clone(), 2, b, Token::PlainText((c, d)))
    };

    // Empty lines
    empty_paragraph ::= LineEmpty(a) EmptyLineContinuation(b) { Line::EmptyParagraph(a + b) };
    empty ::= LineEmpty(span) { Line::Empty(span) };
}

pub use parser::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::ast::Line;
    use crate::markdown::token::Tokenizer;

    #[test]
    fn parse_empty_line() {
        let tokens = vec![Token::LineEmpty(Span::single_line(0, 0, 0))];

        let mut parser = Parser::new();
        for token in tokens {
            parser.parse(token).unwrap();
        }
        let result = parser.end_of_input().unwrap();

        assert_eq!(vec![Line::Empty(Span::single_line(0, 0, 0))], result);
    }

    #[test]
    fn parse_multi_line_paragraphs() {
        let mut parser = Parser::new();

        let tokenizer = Tokenizer::new();
        let tokens = tokenizer.tokenize(
            &format!(
                "{}\n{}\n{}\n\n{}\n{}\n",
                "First", "second", "third", "first", "second",
            )[..],
        );

        for token in tokens {
            parser.parse(token).unwrap();
        }
        let result = parser.end_of_input().unwrap();

        assert_eq!(
            vec![
                Line::Paragraph(
                    Span::multi_line(0, 2, 0, 5),
                    vec![
                        Token::PlainText((Span::single_line(0, 0, 5), "First".into())),
                        Token::PlainText((Span::single_line(1, 0, 6), "second".into())),
                        Token::PlainText((Span::single_line(2, 0, 5), "third".into())),
                    ],
                ),
                Line::Empty(Span::single_line(3, 0, 0)),
                Line::Paragraph(
                    Span::multi_line(4, 5, 0, 6),
                    vec![
                        Token::PlainText((Span::single_line(4, 0, 5), "first".into())),
                        Token::PlainText((Span::single_line(5, 0, 6), "second".into())),
                    ],
                ),
                Line::Empty(Span::single_line(6, 0, 0)),
            ],
            result
        );
    }
}
