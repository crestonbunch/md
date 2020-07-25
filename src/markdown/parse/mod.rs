mod token;

use peg;
use token::{Span, Token, Tokenizer};

#[derive(Debug, PartialEq)]
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

#[derive(Debug, PartialEq)]
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
        rule atx_start() -> Span
            = a:$([Token::Hash((a, b)) if (b - a) <= 6]) { a[0].span() }
        rule atx_empty() -> Vec<Node>
            = s:atx_start() t:sp() !atx_inline() b:blank_lines_eof() {
                let (x, start) = t.unwrap_or(s);
                let (_, end) = (&b).as_ref().map(|b| b.span).unwrap_or((x, start));
                let n = Node::new(Kind::Heading(s.1 - s.0), start, end);
                match b {
                    Some(b) => vec![n, b],
                    None => vec![n],
                }
            }
        rule atx_heading() -> Vec<Node>
            = s:atx_start() t:ws() a:atx_inline()* b:blank_lines_eof() {
                let (_, x) = t;
                let (_, y) = (&b).as_ref().map(|b| b.span).unwrap_or(t);
                let start = a.first().map(|a| a.span.0).unwrap_or(x);
                let end = a.last().map(|y| y.span.1).unwrap_or(y);
                let n = Node::new_block(Kind::Heading(s.1 - s.0), start, end, a);
                match b {
                    Some(b) => vec![n, b],
                    None => vec![n],
                }
            }
        rule heading() -> Vec<Node>
            = h:atx_heading() / h:atx_empty()
            { h }

        // Block quote
        rule block_quote_start() -> Vec<Token>
            =  z:non_indent_space()?
               x:$([Token::RightCaret(..)] [Token::Whitespace(..)]?) {
                   Vec::from(x)
               }
        rule block_quote() -> Vec<Node>
            = x:block_quote_start() a:line()
              b:(
                non_indent_space()? [Token::RightCaret(..)] b:$([Token::Whitespace(..)])? c:line() { (b, c) } /
                non_indent_space()? ![Token::RightCaret(..)] !blank_line() c:line() { (None, c) }
              )*
              c:blank_lines_eof()
              {
                let b = b.into_iter().map(|(b, line)| {
                    match b {
                        Some([Token::Whitespace((s, e)), ..]) if *s < e - 1 => {
                            // Trim off exactly one space character from each line
                            // leaving the remaining whitespace to be included in
                            // the parsed body of the block quote
                            [vec![Token::Whitespace((s + 1, *e))], line].concat()
                        },
                        _ => line,
                    }
                });
                let (start, _) = x[0].span();
                let s = [a, b.flatten().collect()].concat();
                let (_, end) = s.last().map(|n| n.span()).unwrap_or(x[0].span());
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
              b:$([Token::Plus(..) | Token::Asterisk(..) | Token::Dash(..)])
              c:whitespace() {
                let (a, _) = a.unwrap_or(b[0].span());
                let (_, c) = c.span;
                c - a
              }
        rule enumerator() -> usize
            = a:non_indent_space()
              b:$([Token::NumDot(..) | Token::NumParen(..)])
              c:whitespace() {
                let (a, _) = a.unwrap_or(b[0].span());
                let (_, c) = c.span;
                c - a
              }
        rule unordered_list() -> Vec<Node>
            = &bullet()
              a:(
                b:list_tight() { (b, false) } /
                b:list_loose() { (b, true) }
              )
              b:blank_lines_eof()? !bullet() {
                let (children, loose) = a;
                let (start, _) = children.first().unwrap().span;
                let (_, end) = children.last().unwrap().span;
                let n = Node::new_block(Kind::UnorderedList(loose), start, end, children);
                match b.flatten() {
                    Some(b) => vec![n, b],
                    None => vec![n],
                }
            }
        rule ordered_list() -> Vec<Node>
            = &enumerator()
              a:(
                  b:list_tight() { (b, false) } /
                  b:list_loose() { (b, true) }
                )
              b:blank_lines_eof()? !enumerator() {
                let (children, loose) = a;
                let (start, _) = children.first().unwrap().span;
                let (_, end) = children.last().unwrap().span;
                let n = Node::new_block(Kind::OrderedList(loose), start, end, children);
                match b.flatten() {
                    Some(b) => vec![n, b],
                    None => vec![n],
                }
            }
        rule list_tight() -> Vec<Node>
            = (list_item_tight())+
        rule list_loose() -> Vec<Node>
            = (list_item())+
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
        rule list_block() -> Vec<Token>
            = a:line() b:list_block_line()* {
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
              v:(a:end_line() &inline() {
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
            = a:sp() b:newline() {
                let (start, _) = a.unwrap_or(b);
                let (_, end) = b;
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
              !bullet()
              !enumerator() {
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
        rule ws() -> Span
            = a:$([Token::Whitespace(..)]) {
                let (s, _) = a.first().unwrap().span();
                let (_, e) = a.last().unwrap().span();
                (s, e)
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

    macro_rules! doc {
        ($start:literal $end:literal $($child:expr )*) => {
           Node::new_block(Kind::Document, $start, $end, vec![$($child),*])
        };
    }

    macro_rules! empty {
        ($start:literal $end:literal $($child:expr )*) => (
           Node::new_block(Kind::Empty, $start, $end, vec![$($child),*]);
        );
    }

    macro_rules! empty_line {
        ($start:literal $end:literal $($child:expr )*) => (
           Node::new_block(Kind::EmptyLine, $start, $end, vec![$($child),*]);
        );
    }

    macro_rules! p {
        ($start:literal $end:literal $($child:expr )*) => {
            Node::new_block(Kind::Paragraph, $start, $end, vec![$($child),*])
        };
    }

    macro_rules! ws {
        ($start:literal $end:literal) => {
            Node::new(Kind::Whitespace, $start, $end)
        };
    }

    macro_rules! plain {
        ($start:literal $end:literal) => {
            Node::new(Kind::Plaintext, $start, $end)
        };
    }

    macro_rules! h {
        (# $start:literal $end:literal $($child:expr )*) => {
            Node::new_block(Kind::Heading(1), $start, $end, vec![$($child),*])
        };
        (## $start:literal $end:literal $($child:expr )*) => {
            Node::new_block(Kind::Heading(2), $start, $end, vec![$($child),*])
        };
    }

    #[test]
    fn test_empty() {
        assert_eq!(parse(""), doc!(0 0));
        assert_eq!(parse("\n"), doc!(0 1 empty!(0 1 empty_line!(0 1))));
        assert_eq!(
            parse("\n\n"),
            doc!(0 2 empty!(0 2 empty_line!(0 1) empty_line!(1 2)))
        );
        assert_eq!(parse("   \n"), doc!(0 4 empty!(0 4 empty_line!(0 4))));
    }

    #[test]
    fn test_plaintext() {
        assert_eq!(parse("ABC"), doc!(0 3 p!(0 3 plain!(0 3))));
        assert_eq!(
            parse("Hello,\nWorld!"),
            doc!(0 13 p!(0 13 plain!(0 6) ws!(6 7) plain!(7 13)))
        );
        // TODO: should doc end at 3?
        assert_eq!(parse("A \n"), doc!(0 2 p!(0 2 plain!(0 1) ws!(1 2))));
    }

    #[test]
    fn test_heading() {
        assert_eq!(parse("# Hello"), doc!(0 7 h!(# 2 7 plain!(2 7))));
        assert_eq!(
            parse("# Hello\nWorld!"),
            doc!(0 14
                h!(# 2 7 plain!(2 7))
                // TODO: these aren't empty lines
                empty!(7 8 empty_line!(7 8))
                p!(8 14 plain!(8 14))
            )
        );
        assert_eq!(
            parse("Hello\n# World\n\n"),
            doc!(0 15
                p!(0 5 plain!(0 5))
                empty!(5 6 empty_line!(5 6))
                h!(# 8 13 plain!(8 13))
                empty!(13 15 empty_line!(13 14) empty_line!(14 15))
            )
        );
        assert_eq!(
            parse("# Hello\n## World"),
            doc!(
                0 16
                h!(# 2 7 plain!(2 7))
                empty!(7 8 empty_line!(7 8))
                h!(## 11 16 plain!(11 16))
            )
        );
        assert_eq!(
            parse("# \n## A B C"),
            doc!(
                0 11
                h!(# 2 3)
                empty!(2 3 empty_line!(2 3))
                h!(## 6 11 plain!(6 7) ws!(7 8) plain!(8 9) ws!(9 10) plain!(10 11))
            )
        );
        assert_eq!(
            parse("#\n#A"),
            doc!(
                0 4
                h!(# 1 2)
                empty!(1 2 empty_line!(1 2))
                // TODO: should we merge adjacent plaintext tokens?
                p!(2 4 plain!(2 3) plain!(3 4))
            )
        );
        // let result = parse("* \n# Heading\n\n");
        // dbg!(&result);
    }

    #[test]
    fn test_block_quote() {
        // let result = parse(">\n\n");
        // let result = parse("> Hello");
        // let result = parse("> Hello,\nWorld!\n\n");
        // let result = parse("> A\n>B\n>\n>\n");
        // let result = parse("A\n> B\n");
        // let result = parse("> * Hello,\n> * World!\n\n");
        // let result = parse("> * A\n>   * B\n> \n> \n\n");
        let result = parse("> * A\n>   * B\n>   * \n\n");
        // let result = parse("> Hello\n\nWorld!");
        // let result = parse(">\n\nABC");
        // let result = parse(">ABC\n>\n>TWO\n");
        // let result = parse("> A\n\nB");
        dbg!(&result);
    }

    #[test]
    fn test_unordered_lists() {
        // let result = parse("* A\n* B");
        // let result = parse("A\n* B");
        // let result = parse("* A\n  * B\n  * \n\n");
        // let result = parse("* A\n* \n  * B");
        let result = parse("* >A\n  >B");
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
        // let result = parse("* A \n* \nABC");
        // let result = parse("* \n\n* \n\nABC");
        // let result = parse("* A \n\n");
        // let result = parse("> * A\n>   * B\n> ");
        // let result = parse("* \n* \n\nA");
        dbg!(&result);
    }

    #[test]
    fn test_ordered_lists() {
        // let result = parse("1. A\n1. B");
        // let result = parse("A\n1. B");
        // let result = parse("1. A\n1. B\n   1. B");
        // let result = parse("1.  A\n   1. B");
        let result = parse("1. A\n\n1. B"); // TODO

        // let result = parse("1. List item\n\n1. Second list item");
        // let result = parse("1. \n\n1. \n\n");
        dbg!(&result);
    }

    #[bench]
    fn bench_simple_parse(b: &mut Bencher) {
        b.iter(|| parse("> Hello,\nWorld!\n\n"));
    }
}
