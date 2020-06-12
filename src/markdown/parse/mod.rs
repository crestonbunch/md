extern crate test;

mod token;

use std::cell::RefCell;
use std::rc::Rc;

use token::{Token, Tokenizer};

// mod block_quote;
// mod document;
// mod ordered_list;
// mod paragraph;
// mod unordered_list;

// use block_quote::BlockQuoteProbe;
// use document::DocumentProbe;
// use paragraph::ParagraphProbe;

const DEFAULT_CAPACITY: usize = 32;

type Link = Rc<RefCell<Node>>;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum UnorderedListToken {
    Plus,
    Asterisk,
    Dash,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum OrderedListToken {
    CloseParen,
    Period,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct UnorderedList {
    token: UnorderedListToken,
    width: usize,
    tight: bool,
}

impl UnorderedList {
    fn new(token: UnorderedListToken, width: usize) -> Kind {
        Kind::UnorderedList(UnorderedList {
            token,
            width,
            tight: false,
        })
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct OrderedList {
    token: OrderedListToken,
    width: usize,
    tight: bool,
    start: usize,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Kind {
    // Container block tokens
    Document,
    BlockQuote,
    Empty,
    UnorderedList(UnorderedList),
    OrderedList(OrderedList),
    ListItem(usize),
    // Leaf block tokens
    Heading(usize),
    Paragraph,
    EmptyLine,
    // Inline tokens
    Plaintext,
    Whitespace,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub kind: Kind,
    pub start: usize,
    pub end: Option<usize>,
    pub children: Vec<Link>,
}

impl Node {
    pub fn new(kind: Kind, start: usize) -> Link {
        Rc::new(RefCell::new(Node {
            kind,
            start,
            end: None,
            // TODO: change the capacity based on kind to better
            // preemptively allocate appropriate space for each
            // kind of token.
            children: Vec::with_capacity(DEFAULT_CAPACITY),
        }))
    }

    pub fn new_inline(kind: Kind, start: usize, end: usize) -> Link {
        Rc::new(RefCell::new(Node {
            kind,
            start,
            end: Some(end),
            children: vec![],
        }))
    }

    fn close_child(&self, p: usize) {
        if let Some(open) = self.children.last() {
            let mut open = open.borrow_mut();
            open.end = Some(p);
            open.close_child(p);
        }
    }

    fn probe(&self, start: usize, source: &str) -> Option<usize> {
        let mut tokenizer = Tokenizer::new(start, source);
        let (a, b, c) = (tokenizer.next(), tokenizer.next(), tokenizer.next());
        match (self.kind, a, b, c) {
            // Block quote
            (Kind::BlockQuote, Some(Token::RightCaret((_, end))), _, _) => Some(end),
            // Paragraph
            (Kind::Paragraph, Some(Token::Newline(..)), _, _) => None,
            (Kind::Paragraph, Some(Token::Whitespace(..)), Some(Token::Newline(..)), _) => None,
            (Kind::Paragraph, Some(_), _, _) => Some(start),
            // Unordered list
            (
                Kind::UnorderedList(UnorderedList {
                    width,
                    token: UnorderedListToken::Asterisk,
                    ..
                }),
                Some(Token::Asterisk((_, end))),
                Some(Token::Whitespace((_, _))),
                _,
            ) if (width <= end - start + 1) => Some(start),
            (
                Kind::UnorderedList(UnorderedList {
                    width,
                    token: UnorderedListToken::Asterisk,
                    ..
                }),
                Some(Token::Whitespace((_, _))),
                Some(Token::Asterisk((_, end))),
                Some(Token::Whitespace((_, _))),
            ) if (width <= end - start + 1) => Some(start),
            (
                Kind::UnorderedList(UnorderedList { width, .. }),
                Some(Token::Whitespace((_, end))),
                _,
                _,
            ) if (width <= end - start + 1) => Some(start),
            // List item
            (Kind::ListItem(width), Some(Token::Whitespace((_, end))), _, _)
                if width < (end - start + 1) =>
            {
                // Keep list items open if there is a deeper nested list
                // contained inside of them.
                Some(end)
            }
            (Kind::Document, Some(_), _, _) => None,
            _ => None,
        }
    }

    fn open(&self, start: usize, source: &str) -> Option<(Link, usize)> {
        let mut tokenizer = Tokenizer::new(start, source);
        let (a, b, c) = (tokenizer.next(), tokenizer.next(), tokenizer.next());
        match (self.kind, a, b, c) {
            // Block quote open
            (
                _,
                Some(Token::Whitespace((_, _))),
                Some(Token::RightCaret((start, _))),
                Some(Token::Whitespace((_, end))),
            ) => Some((Node::new(Kind::BlockQuote, start), end)),
            (_, Some(Token::Whitespace((_, _))), Some(Token::RightCaret((start, end))), _) => {
                Some((Node::new(Kind::BlockQuote, start), end))
            }
            (_, Some(Token::RightCaret((start, _))), Some(Token::Whitespace((_, end))), _) => {
                Some((Node::new(Kind::BlockQuote, start), end))
            }
            (_, Some(Token::RightCaret((start, end))), _, _) => {
                Some((Node::new(Kind::BlockQuote, start), end))
            }
            // Heading open
            (_, Some(Token::Hash((start, end))), Some(Token::Whitespace((_, _))), _)
                if (end - start) <= 6 =>
            {
                Some((Node::new(Kind::Heading(end - start), start), end))
            }
            // Unordered list open
            (
                k,
                Some(Token::Whitespace((start, _))),
                Some(Token::Asterisk((_, _))),
                Some(Token::Whitespace((_, end))),
            ) if match k {
                Kind::UnorderedList(..) => false,
                _ => true,
            } =>
            {
                Some((
                    Node::new(
                        UnorderedList::new(UnorderedListToken::Asterisk, end - start),
                        start,
                    ),
                    end,
                ))
            }
            (k, Some(Token::Asterisk((start, _))), Some(Token::Whitespace((_, end))), _)
                if match k {
                    Kind::UnorderedList(..) => false,
                    _ => true,
                } =>
            {
                Some((
                    Node::new(
                        UnorderedList::new(UnorderedListToken::Asterisk, end - start),
                        start,
                    ),
                    end,
                ))
            }
            // List item open
            (k, Some(Token::Asterisk((start, _))), Some(Token::Whitespace((_, end))), _)
                if match k {
                    Kind::UnorderedList(UnorderedList {
                        token: UnorderedListToken::Asterisk,
                        width,
                        ..
                    }) if width == (end - start) => true,
                    _ => false,
                } =>
            {
                Some((Node::new(Kind::ListItem(end - start), start), end))
            }
            (
                k,
                Some(Token::Whitespace((start, _))),
                Some(Token::Asterisk((_, _))),
                Some(Token::Whitespace((_, end))),
            ) if match k {
                Kind::UnorderedList(UnorderedList {
                    token: UnorderedListToken::Asterisk,
                    width,
                    ..
                }) if width == (end - start) => true,
                _ => false,
            } =>
            {
                Some((Node::new(Kind::ListItem(end - start), start), end))
            }
            _ => None,
        }
    }

    fn consume(&mut self, start: usize, source: &str) -> Option<usize> {
        match self.kind {
            Kind::Document => container::consume(self, start, source),
            Kind::BlockQuote => container::consume(self, start, source),
            Kind::UnorderedList(..) => list::consume(self, start, source),
            Kind::ListItem(..) => container::consume(self, start, source),
            Kind::Paragraph => leaf::consume(self, start, source),
            Kind::Heading(..) => {
                if let Some(p) = leaf::consume(self, start, source) {
                    // Headings cannot be continued onto the next line
                    // so we close it immediately.
                    self.end = Some(p);
                    Some(p)
                } else {
                    self.end = Some(start);
                    None
                }
            }
            // Inline nodes cannot be consumed
            _ => None,
        }
    }

    pub fn probe_all(node: Link, start: usize, source: &str) -> (Link, usize) {
        let mut p = start;
        let mut node = node;
        while let Some(open) = {
            let borrow = node.borrow_mut();
            // Probe the last child if this node is still open
            match borrow.end {
                None => borrow.children.last().map(Rc::clone),
                _ => None,
            }
        } {
            let borrow = open.borrow();
            match borrow.kind {
                // Leaf nodes cannot be probed since they can't
                // contain any block elements.
                Kind::Paragraph | Kind::Heading(..) => break,
                _ => (),
            };
            if let Some(new_p) = borrow.probe(p, source) {
                p = new_p;
            } else {
                // Any remaining unmatched tokens will either
                // be continued or closed in the next step.
                break;
            }
            node = Rc::clone(&open);
        }
        (node, start)
    }

    pub fn open_all(node: Link, start: usize, source: &str) -> (Link, usize) {
        let mut p = start;
        let mut node = node;

        // Now push all of the new open blocks into the tree
        while let Some((open, new_p)) = {
            let borrow = node.borrow();
            borrow.open(p, source)
        } {
            {
                let mut borrow = node.borrow_mut();
                borrow.children.push(Rc::clone(&open));
            }
            node = open;
            p = new_p;
        }

        (node, p)
    }

    pub fn consume_all(node: Link, start: usize, source: &str) -> (Link, usize) {
        let mut p = start;
        let mut node = node;

        if let Some(new_p) = {
            let mut borrow = node.borrow_mut();
            borrow.consume(p, source)
        } {
            p = new_p;
        } else {
            // We did not consume anything at this node, so
            // it must not be a continuation after all.
            // Now we close it.
            node.borrow_mut().end = Some(p);
        }

        (node, p)
    }
}

pub fn parse(source: &str) -> Link {
    let doc = Node::new(Kind::Document, 0);
    let mut p = 0;

    while let None = {
        // Loop until the document block is closed
        let borrow = doc.borrow();
        borrow.end
    } {
        let mut node = Rc::clone(&doc);

        // First we iterate through the open blocks, matching each block
        // with a token in the source. Any remaining unmatched tokens will either
        // be continued or closed in the next step.
        let (new_node, new_p) = Node::probe_all(node, p, source);
        node = new_node;
        p = new_p;

        if let Some(_) = {
            let borrow = node.borrow();
            borrow.open(p, source)
        } {
            // We found a new block opener, so let's close any open blocks
            // before we open new ones.
            node.borrow().close_child(p);

            let (_, new_p) = Node::open_all(Rc::clone(&node), p, source);
            p = new_p;
        }

        // There are no more blocks to open, so this
        // is a continuation of the last open block.
        let (_, new_p) = Node::consume_all(node, p, source);
        p = new_p;
    }

    doc
}

mod container {
    use super::*;

    pub fn consume(node: &mut Node, start: usize, source: &str) -> Option<usize> {
        // If we consume a non-leaf block that has no open child,
        // we need to push a child to consume.
        if match node.children.last() {
            None => true,
            Some(node) if node.borrow().end.is_some() => true,
            _ => false,
        } {
            node.children.push(Node::new(Kind::Paragraph, start));
        }

        if let Some(open) = node.children.last() {
            if let Some(p) = {
                let mut borrow = open.borrow_mut();
                borrow.consume(start, source)
            } {
                return Some(p);
            } else {
                // We did not consume anything, so that
                // means we can close this child.
                open.borrow_mut().end = Some(start);
                return empty::consume(node, start, source);
            }
        }

        None
    }
}

mod list {
    use super::*;

    pub fn consume(node: &mut Node, start: usize, source: &str) -> Option<usize> {
        if match node.children.last() {
            None => true,
            Some(node) if node.borrow().end.is_some() => true,
            _ => false,
        } {
            if let Kind::UnorderedList(UnorderedList { width, .. }) = node.kind {
                node.children.push(Node::new(Kind::ListItem(width), start));
            }
        }

        container::consume(node, start, source)
    }
}

mod leaf {
    use super::*;

    pub fn consume(node: &mut Node, start: usize, source: &str) -> Option<usize> {
        // For leaf blocks we consume tokens until the next new line
        let tokenizer = Tokenizer::new(start, source);
        if node.end == None {
            let mut p = start;
            let mut empty = true;
            let tokens = tokenizer
                .into_iter()
                .take_while(|t| match t {
                    Token::Newline((_, end)) => {
                        p = *end;
                        false
                    }
                    Token::RightCaret((_, end))
                    | Token::Hash((_, end))
                    | Token::Dash((_, end))
                    | Token::Plus((_, end))
                    | Token::Number((_, end))
                    | Token::Period((_, end))
                    | Token::CloseParen((_, end))
                    | Token::Asterisk((_, end))
                    | Token::Plaintext((_, end))
                    | Token::Whitespace((_, end)) => {
                        p = *end;
                        empty = false;
                        true
                    }
                })
                .map(|t| t.into());

            node.children.extend(tokens);

            if !empty {
                return Some(p);
            }
        }

        None
    }
}

mod empty {
    use super::*;

    pub fn consume(node: &mut Node, start: usize, source: &str) -> Option<usize> {
        let tokenizer = Tokenizer::new(start, source);
        let mut p = start;

        // Empty tokens are easily confused with paragraphs. In order to reduce
        // the complexity of parsing empty lines in the normal probe-open-consume
        // cycle, we break out and consume entire chunks of empty lines
        // before returning to the main loop.
        let mut consumed = false;
        let empty = Node::new(Kind::Empty, start);
        for token in tokenizer {
            match token {
                Token::Whitespace((start, _)) => {
                    p = start;
                }
                Token::Newline((_, end)) => {
                    let empty_line = Node::new(Kind::EmptyLine, p);
                    empty_line.borrow_mut().end = Some(end);
                    empty.borrow_mut().children.push(empty_line);
                    p = end;
                    consumed = true;
                }
                _ => break,
            }
        }

        if consumed {
            let mut borrow = empty.borrow_mut();
            borrow.end = Some(p);
            if let Some(open) = borrow.children.last_mut() {
                open.borrow_mut().end = Some(p);
            }
            node.children.push(Rc::clone(&empty));

            Some(p)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;

    #[test]
    fn test_empty() {
        let result = parse("");
        assert_eq!(
            Rc::try_unwrap(result).unwrap().into_inner(),
            Node {
                kind: Kind::Document,
                start: 0,
                end: Some(0),
                children: vec![],
            }
        );
    }

    #[test]
    fn test_blockquote() {
        let result = parse("> Hello,\nWorld!");
        dbg!(&result);
    }

    #[test]
    fn test_heading() {
        let result = parse("# Hello\nWorld!");
        dbg!(&result);
    }

    #[test]
    fn test_multiple_paragraphs() {
        let result = parse("Hello\nWorld!\n \n\nHello\nWorld");
        dbg!(&result);
    }

    #[test]
    fn test_unordered_lists() {
        // let result = parse("* List item\n* Second list item");
        // let result = parse("* List item\n  * Second list item");
        // let result = parse("* List item\n\nTestTest\n* Second list item");
        // let result = parse("* List item\n\n   * Second list item");
        let result = parse("* List item\n  * Nested list\n* Third list item");
        dbg!(&result);
    }

    #[bench]
    fn bench_simple_parse(b: &mut Bencher) {
        b.iter(|| parse("> Hello,\nWorld!"));
    }
}
