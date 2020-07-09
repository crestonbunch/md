mod token;

use peg;
use token::{Span, Token, Tokenizer};

#[derive(Debug)]
pub enum Kind {
    // Container block tokens
    Document,
    BlockQuote,
    Empty,
    UnorderedList(bool),
    OrderedList(bool),
    ListItem,
    // Leaf block tokens
    Heading(usize),
    Paragraph,
    EmptyLine,
    // Inline tokens
    Plaintext,
    Whitespace,
}

#[derive(Debug)]
pub struct Node {
    pub kind: Kind,
    pub span: (usize, usize),
    pub children: Vec<Node>,
}

impl Node {
    pub fn new(kind: Kind, start: usize, end: usize) -> Self {
        Node {
            kind,
            span: (start, end),
            children: vec![],
        }
    }

    pub fn new_block(kind: Kind, start: usize, end: usize, children: Vec<Node>) -> Self {
        Node {
            kind,
            span: (start, end),
            children,
        }
    }
}

peg::parser! {
    // Adapted from https://github.com/jgm/peg-markdown/blob/master/markdown_parser.leg
    pub grammar md_parser() for [Token] {
        pub rule doc() -> Node
            = a:(b:empty() { vec![b] } / b:block()* { b }) {
                let children = a.into_iter().flatten().collect::<Vec<_>>();
                let end = children.last().map(|n| n.span.1).unwrap_or(0);
                Node::new_block(Kind::Document, 0, end, children)
            }

        rule block() -> Vec<Node>
            = a:blank_lines_eof()? b:(
                c:heading() /
                c:block_quote() /
                c:unordered_list() /
                c:ordered_list() /
                c:paragraph()
                { c }
            ) {
                match a.flatten() {
                    Some(a) => {
                        let mut v = vec![a];
                        v.extend(b);
                        v
                    },
                    None => b,
                }
            }

        // Heading
        rule atx_inline() -> Node
            = !newline() x:inline() { x }
        rule atx_start() -> usize
            = a:$([Token::Hash((a, b)) if (b - a) <= 6]) {
                let (start, end) = a[0].span();
                end - start
            }
        rule atx_heading() -> Node
            = s:atx_start() sp() a:atx_inline()+ sp() newline() {
                let (start, _) = a.first().unwrap().span;
                let (_, end) = a.last().unwrap().span;
                Node::new_block(Kind::Heading(s), start, end, a)
            }
        rule heading() -> Vec<Node>
            = h:atx_heading()
            { vec![h] }

        // Block quote
        rule block_quote_start() -> Vec<Token>
            = x:$([Token::RightCaret(..)] [Token::Whitespace(..)]?) {
                Vec::from(x)
            }
        rule block_quote() -> Vec<Node>
            = x:block_quote_start() a:line()
              b:(
                [Token::RightCaret(..)] [Token::Whitespace(..)]? c:line() { c } /
                ![Token::RightCaret(..)] !blank_line() c:line() { c }
              )*
              c:blank_lines_eof()
              {
                let (start, _) = x[0].span();
                let s = [a, b.into_iter().flatten().collect()].concat();
                let (_, end) = (&c)
                    .as_ref()
                    .map(|n| n.span)
                    .unwrap_or(s.last().unwrap().span());
                let sub = md_parser::doc(&s).unwrap();
                let bq = Node::new_block(Kind::BlockQuote, start, end, sub.children);
                match c {
                    Some(c) => vec![bq, c],
                    None => vec![bq],
                }
              }

        // List
        rule bullet() -> usize
            = a:non_indent_space()
              [Token::Plus(..) | Token::Asterisk(..) | Token::Dash(..)]
              b:whitespace() {
                let (a, _) = a.unwrap_or((0, 0));
                let (_, b) = b.span;
                b - a
              }
        rule enumerator() -> usize
            = a:non_indent_space()
              [Token::NumDot(..) | Token::NumParen(..)]
              b:whitespace() {
                let (a, _) = a.unwrap_or((0, 0));
                let (_, b) = b.span;
                b - a
              }
        rule unordered_list() -> Vec<Node>
            = &bullet()
              a:(
                  b:list_tight() { (b, false) } /
                  b:list_loose() { (b, true) } /
                  b:list_empty() { (b, false) }
                ) {
                let (children, loose) = a;
                let (start, _) = children.first().unwrap().span;
                let (_, end) = children.last().unwrap().span;
                let n = Node::new_block(Kind::UnorderedList(loose), start, end, children);
                vec![n]
            }
        rule ordered_list() -> Vec<Node>
            = &enumerator()
              a:(
                  b:list_tight() { (b, false) } /
                  b:list_loose() { (b, true) } /
                  b:list_empty() { (b, false) }
                ) {
                let (children, loose) = a;
                let (start, _) = children.first().unwrap().span;
                let (_, end) = children.last().unwrap().span;
                let n = Node::new_block(Kind::OrderedList(loose), start, end, children);
                vec![n]
            }
        rule list_tight() -> Vec<Node>
            = a:list_item_tight()+
              blank_line()* !(bullet() / enumerator()) { a }
        rule list_loose() -> Vec<Node>
            = a:(b:list_item() blank_line()* { b })+
        rule list_empty() -> Vec<Node>
            = a:list_item_empty()+
              blank_line()* !(bullet() / enumerator()) { a }
        rule list_item() -> Node
            = width:(bullet() / enumerator())
              a:list_block()
              b:(list_continuation_block(width)*) {
                let s = [a, b.into_iter().flatten().collect()].concat();
                let (start, _) = s.first().unwrap().span();
                let (_, end) = s.last().unwrap().span();
                let sub = md_parser::doc(&s).unwrap();
                Node::new_block(Kind::ListItem, start, end, sub.children)
              }
        rule list_item_tight() -> Node
            = width:(bullet() / enumerator())
              a:list_block()
              b:(!blank_line() c:list_continuation_block(width) { c })*
              !list_continuation_block(width) {
                let s = [a, b.into_iter().flatten().collect()].concat();
                let (start, _) = s.first().unwrap().span();
                let (_, end) = s.last().unwrap().span();
                let sub = md_parser::doc(&s).unwrap();
                Node::new_block(Kind::ListItem, start, end, sub.children)
              }
        rule list_item_empty() -> Node
            = width:(bullet() / enumerator())
              a:blank_lines() {
                let (start, _) = a.span;
                let (_, end) = a.span;
                Node::new_block(Kind::ListItem, start, end, vec![a])
            }
        rule list_block() -> Vec<Token>
            = !blank_line() a:line() b:list_block_line()* {
                [a, b.into_iter().flatten().collect()].concat()
            }
        rule list_continuation_indent(width: usize) -> Option<Token>
            = a:$([Token::Whitespace((start, end)) if {(end - start) >= width} ]) {
                let (start, _) = a.first().unwrap().span();
                let (_, end) = a.last().unwrap().span();
                if (start + width == end) {
                    None
                } else {
                    // Chop off the indentation matching the current block
                    Some(Token::Whitespace((start + width, end)))
                }
            }
        rule list_continuation_block(width: usize) -> Vec<Token>
            = blank_line()*
              a:list_continuation_indent(width) b:line() c:list_block_line()* {
                match a {
                    Some(a) => [vec![a], b, c.into_iter().flatten().collect()].concat(),
                    None => [b, c.into_iter().flatten().collect()].concat(),
                }
            }
        rule list_block_line() -> Vec<Token>
            = !blank_line() !(sp() (bullet() / enumerator()))
              // !horizonatal_rule()
              a:line()
              { a }

        // Paragraph
        rule paragraph() -> Vec<Node>
            = a:inlines() b:blank_lines_eof() {
                let (start, _) = a.first().unwrap().span;
                let (_, end) = a.last().unwrap().span;
                let p = Node::new_block(Kind::Paragraph, start, end, a);
                match b {
                    Some(b) => vec![p, b],
                    None => vec![p],
                }
            }

        // Empty block
        rule empty() -> Vec<Node>
            = a:blank_lines() eof() { vec![a] }

        // Inlines
        rule inlines() -> Vec<Node>
            = v:(
              v:((!end_line() b:inline()+ { b })) /
              v:(a:end_line() b:inline() {
                  vec![Node::new(Kind::Whitespace, a.0, a.1)]
                })
            )+
            end_line()?
            eof()?
            { v.into_iter().flatten().collect() }
        rule inline() -> Node
            = plaintext() /
              whitespace()

        rule plaintext() -> Node
            = a:$[
                Token::Plaintext(..) |
                Token::RightCaret(..) |
                Token::Hash(..) |
                Token::Dash(..) |
                Token::Asterisk(..) |
                Token::Plus(..) |
                Token::NumDot(..) |
                Token::NumParen(..)
            ] {
                let (start, end) = a[0].span();
                Node::new(Kind::Plaintext, start, end)
             } // x
        rule whitespace() -> Node
            = a:$([Token::Whitespace(..)]) {
                let (start, _) = a.first().unwrap().span();
                let (_, end) = a.last().unwrap().span();
                Node::new(Kind::Whitespace, start, end)
            }

        rule blank_lines_eof() -> Option<Node>
            = (a:blank_lines() { Some(a) }) / (eof() { None })
        rule blank_lines() -> Node
            = a:blank_line()+ {
                let (start, _) = a.first().unwrap().span;
                let (_, end) = a.last().unwrap().span;
                Node::new_block(Kind::Empty, start, end, a)
            }
        rule blank_line() -> Node
            = sp() a:newline() {
                let (start, end) = a;
                Node::new(Kind::EmptyLine, start, end)
            }

        rule end_line() -> Span
            = terminal_end_line() / normal_end_line()
        rule normal_end_line() -> Span
            = a:sp()
              b:newline()
              // The next line must not start with a block opener
              !blank_line()
              !block_quote_start()
              !atx_start()
              !bullet() {
                a.map(|span| {
                    let (s, _) = span;
                    let (_, e) = b;
                    (s, e)
                })
                .unwrap_or(b)
            }
        rule terminal_end_line() -> Span
            = a:sp() b:newline() eof() {
                a.map(|span| {
                    let (s, _) = span;
                    let (_, e) = b;
                    (s, e)
                })
                .unwrap_or(b)
            }

        rule newline() -> Span
            = a:$([Token::Newline(..)]) { a[0].span() }
        rule eof()
            = ![_]
        rule sp() -> Option<Span>
            = a:$([Token::Whitespace(..)])? {
                a.map(|a| {
                    let (s, _) = a.first().unwrap().span();
                    let (_, e) = a.last().unwrap().span();
                    (s, e)
                })
            }
        rule non_indent_space() -> Option<Span>
            = a:$([Token::Whitespace((start, end)) if (end - start) < 4])? {
                a.and_then(|a| {
                    if a.is_empty() {
                        None
                    } else {
                        let (s, _) = a.first().unwrap().span();
                        let (_, e) = a.last().unwrap().span();
                        Some((s, e))
                    }
                })
            }
        rule indent() -> Span
            = a:$([Token::Whitespace((start, end)) if (end - start) == 4]) {
                let (s, _) = a.first().unwrap().span();
                let (_, e) = a.last().unwrap().span();
                (s, e)
            }


        rule line() -> Vec<Token>
            = a:(
                b:$((![Token::Newline(..)] [_])*) c:$([Token::Newline(..)]) {
                    Vec::from([b, c].concat())
                } /
                b:$([_]+) eof() { Vec::from(b) }
              )
              { a.into() }
    }
}

pub fn parse(source: &str) -> Node {
    let tokenizer = Tokenizer::new(0, source);
    let tokens = tokenizer.collect::<Vec<_>>();
    md_parser::doc(&tokens).unwrap()
}

#[cfg(test)]
mod test {
    extern crate test;

    use super::*;
    use test::Bencher;

    #[test]
    fn test_empty() {
        // dbg!(parse(""));
        dbg!(parse("\n"));
    }

    #[test]
    fn test_simple() {
        // dbg!(parse("ABC"));
        // dbg!(parse("Hello,\nWorld!\n\n"));
        dbg!(parse("A \n"));
    }

    #[test]
    fn test_heading() {
        // let result = parse("# Hello\nWorld!\n\n");
        // let result = parse("abc\n# Hello\nWorld!\n\n");
        let result = parse("abc\n\n## Hello\nWorld!\n\n");
        // let result = parse("# \n## Heading\n\n\n");
        // let result = parse("* \n# Heading\n\n"); // TODO
        dbg!(&result);
    }

    #[test]
    fn test_block_quote() {
        let result = parse(">\n\n");
        // let result = parse("> Hello");
        // let result = parse("> Hello,\nWorld!\n\n");
        // let result = parse("> A\n>B\n>\n>\n");
        // let result = parse("A\n> B\n");
        // let result = parse("> * Hello,\n> * World!\n\n");
        // let result = parse("> Hello\n\nWorld!");
        // let result = parse(">\n\nABC");
        // let result = parse(">ABC\n>\n>TWO\n");
        // let result = parse("> A\n\nB"); // TODO
        dbg!(&result);
    }

    #[test]
    fn test_unordered_lists() {
        // let result = parse("* A\n* B");
        // let result = parse("A\n* B");
        // let result = parse("* A\n  * B");
        // let result = parse("* List item\n\n* Second list item");
        // let result = parse("* List item\n\n   * Second list item");
        // let result = parse("* List item\n  * Nested list\n\nThird list item");
        // let result = parse("* One list\n- Two list\n+ Three list");
        // let result = parse("> * List\n>   * List\n\nParagraph");
        // let result = parse("* List item\n\n  List item continuation");
        // let result = parse("* List item\n\nNot a list item");
        // let result = parse("* \n\n");
        // let result = parse("* \nABC");
        // let result = parse("* \n* \nABC");
        // let result = parse("* \n\n* \n\nABC");
        let result = parse("* A \n\n");
        // let result = parse("> * A\n>   * B\n> ");
        // let result = parse("* \n* \n\nA");
        dbg!(&result);
    }

    #[test]
    fn test_ordered_lists() {
        let result = parse("1. A\n1. B");
        // let result = parse("1. List item\n\n1. Second list item");
        // let result = parse("1. \n\n1. \n\n");
        dbg!(&result);
    }

    #[bench]
    fn bench_simple_parse(b: &mut Bencher) {
        b.iter(|| parse("> Hello,\nWorld!\n\n"));
    }
}
