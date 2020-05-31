use std::cmp::Ordering;
use std::ops::Add;

use pomelo::pomelo;
use serde::Serialize;

#[derive(Debug, Copy, Clone, Serialize, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Span {
        Span { start, end }
    }
}

impl Add for Span {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        let start = match self.start.cmp(&other.start) {
            Ordering::Less => self.start,
            Ordering::Greater => other.start,
            Ordering::Equal => self.start,
        };
        let end = match self.end.cmp(&other.end) {
            Ordering::Less => other.end,
            Ordering::Greater => self.end,
            Ordering::Equal => self.end,
        };
        Span::new(start, end)
    }
}

impl std::convert::Into<Span> for &std::ops::Range<usize> {
    fn into(self) -> Span {
        Span::new(self.start, self.end)
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

    %type UnorderedListStart usize;
    %type OrderedListStart usize;
    %type ListItemStart usize;
    %type Header usize;
    %type Plaintext String;


    %type doc Vec<ast::Block>;
    %type block (Span, ast::Block);
    %type blocks (Span, Vec<ast::Block>);
    %type plaintext (Span, ast::Inline);
    %type inline (Span, Vec<ast::Inline>);
    %type header (Span, ast::LeafBlock);
    %type paragraph (Span, ast::LeafBlock);
    %type empty_lines (Span, Vec<ast::Inline>);
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

    plaintext ::= Plaintext((sa, a)) { (sa, ast::Inline::Plaintext(sa, a)) };
    inline ::= inline((sa, a)) plaintext((sb, b)) { (sa + sb, [a, vec![b]].concat()) };
    inline ::= plaintext((sa, a)) { (sa, vec![a]) };
    inline ::= DoubleAsterisk inline(x) DoubleAsterisk { x }; // TODO: strong

    header ::= Header((sa, size)) inline((sb, b)) {
        (sa + sb, ast::LeafBlock::Header(sa + sb, size, b))
    }
    header ::= Header((sa, size)) {
        (sa, ast::LeafBlock::Header(sa, size, vec![]))
    }

    paragraph ::= ParagraphStart(sa) inline((sb, b)) ParagraphEnd {
       (sa + sb, ast::LeafBlock::Paragraph(sa + sb, b))
    }

    empty_lines ::= empty_lines((sa, a)) Empty(sb) {
        (sa + sb, [a, vec![ast::Inline::Empty(sb)]].concat())
    }
    empty_lines ::= Empty(sa) { (sa, vec![ast::Inline::Empty(sa)] )}
    empty ::= EmptyStart(sa) EmptyEnd { (sa, ast::LeafBlock::Empty(sa, vec![])) }
    empty ::= EmptyStart(sa) empty_lines((sb, b)) EmptyEnd {
        (sa + sb, ast::LeafBlock::Empty(sa + sb, b))
    }

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

    #[test]
    fn test_empty_source() {
        let source = "";
        let mut tokenizer = Tokenizer::new(source);
        let tokens = tokenizer.tokenize();

        let mut parser = Parser::new();
        for token in tokens {
            parser.parse(token).unwrap();
        }

        let result = parser.end_of_input().unwrap();

        assert_eq!(Vec::<ast::Block>::new(), result);
    }

    #[test]
    fn test_empty_paragraph() {
        let source = "Hello\n\n\n\nWorld";
        let mut tokenizer = Tokenizer::new(source);
        let tokens = tokenizer.tokenize();

        let mut parser = Parser::new();
        for token in tokens {
            parser.parse(token).unwrap();
        }

        let result = parser.end_of_input().unwrap();
        assert_eq!(
            vec![
                ast::Block::Leaf(ast::LeafBlock::Paragraph(
                    Span::new(0, 5),
                    vec![ast::Inline::Plaintext(Span::new(0, 5), "Hello".into())]
                )),
                ast::Block::Leaf(ast::LeafBlock::Empty(
                    Span::new(6, 8),
                    vec![
                        ast::Inline::Empty(Span::new(6, 6)),
                        ast::Inline::Empty(Span::new(7, 7)),
                        ast::Inline::Empty(Span::new(8, 8)),
                    ]
                )),
                ast::Block::Leaf(ast::LeafBlock::Paragraph(
                    Span::new(9, 14),
                    vec![ast::Inline::Plaintext(Span::new(9, 14), "World".into())]
                )),
            ],
            result
        );
    }

    #[test]
    fn test_header_paragraph() {
        let source = ["# Title", "Hello,", "World!"].join("\n");
        let mut tokenizer = Tokenizer::new(&source[..]);
        let tokens = tokenizer.tokenize();

        let mut parser = Parser::new();
        for token in tokens {
            parser.parse(token).unwrap();
        }

        let result = parser.end_of_input().unwrap();

        assert_eq!(
            vec![
                ast::Block::Leaf(ast::LeafBlock::Header(
                    Span::new(0, 7),
                    1,
                    vec![ast::Inline::Plaintext(Span::new(2, 7), "Title".into())]
                )),
                ast::Block::Leaf(ast::LeafBlock::Paragraph(
                    Span::new(8, 21),
                    vec![
                        ast::Inline::Plaintext(Span::new(8, 14), "Hello,".into()),
                        ast::Inline::Plaintext(Span::new(15, 21), "World!".into()),
                    ]
                )),
            ],
            result
        );
    }
}
