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
            Ordering::Equal => match self.end_line.cmp(&other.end_line) {
                Ordering::Less => Span::multi_line(
                    self.start_line,
                    other.end_line,
                    std::cmp::min(self.start_col, other.start_col),
                    other.end_col,
                ),
                Ordering::Greater => Span::multi_line(
                    self.start_line,
                    self.end_line,
                    std::cmp::min(self.start_col, other.start_col),
                    self.end_col,
                ),
                Ordering::Equal => Span::multi_line(
                    self.start_line,
                    other.end_line,
                    std::cmp::min(self.start_col, other.start_col),
                    std::cmp::max(self.end_col, other.end_col),
                ),
            },
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

    %fallback Eof Empty;

    %type UnorderedListStart usize;
    %type OrderedListStart usize;
    %type ListItemStart usize;
    %type Header usize;
    %type Plaintext String;


    %type doc Vec<ast::Block>;
    %type block (Span, ast::Block);
    %type blocks (Span, Vec<ast::Block>);
    %type plaintext (Span, Vec<ast::Line>);
    %type header (Span, ast::LeafBlock);
    %type paragraph (Span, ast::LeafBlock);
    %type empty (Span, ast::LeafBlock);
    %type blockquote (Span, ast::ContainerBlock);
    %type ul (Span, ast::ContainerBlock);
    %type ol (Span, ast::ContainerBlock);
    %type list_items (Span, Vec<ast::Block>);
    %type list_item (Span, ast::Block);

    doc ::= blocks((_, b)) Eof { b };
    doc ::= Eof { vec![] };

    blocks ::= blocks((sa, a)) block((sb, b)) { (sa + sb, [a, vec![b]].concat()) };
    blocks ::= block((sb, b)) { (sb, vec![b]) };

    block ::= header((sh, h)) { (sh, ast::Block::Leaf(h)) };
    block ::= paragraph((sp, p)) { (sp, ast::Block::Leaf(p)) };
    block ::= empty((se, e)) { (se, ast::Block::Leaf(e)) };
    block ::= blockquote((sbq, bq)) { (sbq, ast::Block::Container(bq)) };
    block ::= ul((sl, l)) { (sl, ast::Block::Container(l)) };
    block ::= ol((sl, l)) { (sl, ast::Block::Container(l)) };

    plaintext ::= plaintext((sa, a)) Plaintext((sb, b)) {
       (sa + sb, [a, vec![ast::Line::Plaintext(sb, b)]].concat())
    };
    plaintext ::= Plaintext((sa, a)) { (sa, vec![ast::Line::Plaintext(sa, a)]) };

    header ::= Header((sa, size)) Plaintext((sb, b)) {
        let b = ast::Line::Plaintext(sb, b);
        (sa + sb, ast::LeafBlock::Header(sa + sb, size, b))
    }

    paragraph ::= ParagraphStart(sa) plaintext((sb, b)) ParagraphEnd {
       (sa + sb, ast::LeafBlock::Paragraph(sa + sb, b))
    }

    empty ::= Empty(sa) { (sa, ast::LeafBlock::Empty(sa) )}

    blockquote ::= BlockquoteStart(sa) blocks((sb, b)) BlockquoteEnd {
        (sa + sb, ast::ContainerBlock::Blockquote(sa + sb, b))
    };

    ul ::= UnorderedListStart((sa, _)) list_items((sb, b)) UnorderedListEnd {
        (sa + sb, ast::ContainerBlock::UnorderedList(sa + sb, b))
    };
    ol ::= OrderedListStart((sa, _)) list_items((sb, b)) OrderedListEnd {
        (sa + sb, ast::ContainerBlock::OrderedList(sa + sb, b))
    };

    list_items ::= list_items((sa, a)) list_item((sb, b)) { (sa + sb, [a, vec![b]].concat()) };
    list_items ::= list_item((sa, a)) { (sa, vec![a]) };
    list_item ::= ListItemStart((sa, _)) blocks((sb, b)) ListItemEnd {
        (sa + sb, ast::Block::Container(ast::ContainerBlock::ListItem(sa + sb, b)))
    };
}

pub use parser::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::ast;
    use crate::markdown::token::Tokenizer;

    fn s(line: usize, start: usize, end: usize) -> Span {
        Span::single_line(line, start, end)
    }

    #[test]
    fn test_empty_source() {
        let source = "";
        let tokens = Tokenizer::tokenize(source);

        let mut parser = Parser::new();
        for token in tokens {
            parser.parse(token).unwrap();
        }

        let result = parser.end_of_input().unwrap();

        assert_eq!(Vec::<ast::Block>::new(), result);
    }

    #[test]
    fn test_header_paragraph() {
        let source = ["# Title", "Hello,", "World!"].join("\n");
        let tokens = Tokenizer::tokenize(&source[..]);

        let mut parser = Parser::new();
        for token in tokens {
            parser.parse(token).unwrap();
        }

        let result = parser.end_of_input().unwrap();

        assert_eq!(
            vec![
                ast::Block::Leaf(ast::LeafBlock::Header(
                    s(0, 0, 7),
                    1,
                    ast::Line::Plaintext(s(0, 2, 7), "Title".into())
                )),
                ast::Block::Leaf(ast::LeafBlock::Paragraph(
                    Span::multi_line(1, 2, 0, 6),
                    vec![
                        ast::Line::Plaintext(s(1, 0, 6), "Hello,".into()),
                        ast::Line::Plaintext(s(2, 0, 6), "World!".into()),
                    ]
                )),
            ],
            result
        );
    }
}
