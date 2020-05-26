use std::cmp::Ordering;
use std::ops::Add;

use pomelo::pomelo;
use serde::Serialize;

#[derive(Debug, Copy, Clone, Serialize, PartialEq, Eq)]
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

pomelo! {
    %include {
        use serde::Serialize;

        use crate::markdown::ast;
        use crate::markdown::parse::Span;
    }
    %token #[derive(Debug, Clone, Serialize, PartialEq, Eq)] pub enum Token {};
    %error String;
    %extra_token Span;

    %fallback Eof UnorderedListEnd OrderedListEnd ListItemEnd BlockquoteEnd ParagraphEnd;

    %type UnorderedListStart usize;
    // %type UnorderedListEnd ();
    %type OrderedListStart usize;
    // %type OrderedListEnd ();
    %type ListItemStart usize;
    // %type ListItemEnd ();
    %type BlockquoteStart ();
    // %type BlockquoteEnd ();
    %type ParagraphStart ();
    // %type ParagraphEnd ();
    %type Header usize;
    %type Plaintext String;
    // %type Eof ();

    %type doc Vec<ast::Block>;

    doc ::= Plaintext { vec![] };
}

pub use parser::*;
