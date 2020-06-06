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

const NON_LEAF_KINDS: [Kind; 3] = [Kind::Document, Kind::BlockQuote, Kind::Empty];

const DEFAULT_CAPACITY: usize = 32;

type Link = Rc<RefCell<Node>>;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Kind {
    // Container block tokens
    Document,
    BlockQuote,
    Empty,
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

    fn open_child(&self) -> Option<Link> {
        if self.end.is_none() {
            self.children.last().map(Rc::clone)
        } else {
            None
        }
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
        let (a, b) = (tokenizer.next(), tokenizer.next());
        match (self.kind, a, b) {
            (Kind::BlockQuote, Some(Token::RightCaret((_, end))), _) => Some(end),
            (Kind::Paragraph, Some(Token::Newline(..)), _) => None,
            (Kind::Paragraph, Some(Token::Whitespace(..)), Some(Token::Newline(..))) => None,
            (Kind::Paragraph, Some(_), _) => Some(start),
            (Kind::Empty, Some(Token::Newline((_, end))), _) => Some(end),
            (Kind::Empty, Some(Token::Whitespace(..)), Some(Token::Newline((_, end)))) => Some(end),
            (Kind::Document, Some(_), _) => None,
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
            // Empty lines
            (k, Some(Token::Newline((start, end))), _, _) if k != Kind::Empty => {
                Some((Node::new(Kind::Empty, start), end))
            }
            (k, Some(Token::Whitespace((start, _))), Some(Token::Newline((_, _))), _)
                if k != Kind::Empty =>
            {
                Some((Node::new(Kind::Empty, start), start))
            }
            _ => None,
        }
    }

    fn consume(&mut self, start: usize, source: &str) -> Option<usize> {
        match self.kind {
            Kind::Document => container::consume(self, start, source),
            Kind::BlockQuote => container::consume(self, start, source),
            Kind::Empty => empty::consume(self, start, source),
            Kind::EmptyLine => empty_line::consume(self, start, source),
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
            borrow.open_child()
        } {
            if let Some(new_p) = open.borrow().probe(p, source) {
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

        if let Some(open) = {
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
            let mut open = open.borrow_mut();
            if let Some(p) = open.consume(start, source) {
                return Some(p);
            } else {
                // We did not consume anything, so that
                // means we can close this child.
                open.end = Some(start);
            }
        }

        None
    }
}

mod empty {
    use super::*;

    pub fn consume(node: &mut Node, start: usize, source: &str) -> Option<usize> {
        // If we consume a non-leaf block that has no open child,
        // we need to push a child to consume.
        if match node.children.last() {
            None => true,
            Some(node) if node.borrow().end.is_some() => true,
            _ => false,
        } {
            node.children.push(Node::new(Kind::EmptyLine, start))
        }

        if let Some(open) = node.children.last() {
            let mut open = open.borrow_mut();
            if let Some(p) = open.consume(start, source) {
                let mut tokenizer = Tokenizer::new(p, source);
                // An empty empty block is always closed if it is not
                // continued by a new line or whitespace.
                match tokenizer.next() {
                    Some(Token::Newline(..)) | Some(Token::Whitespace(..)) => (),
                    _ => node.end = Some(p),
                }
                return Some(p);
            }
        }

        None
    }
}

mod empty_line {
    use super::*;

    pub fn consume(node: &mut Node, start: usize, source: &str) -> Option<usize> {
        // For leaf blocks we consume tokens until the next new line
        let tokenizer = Tokenizer::new(start, source);
        let mut p = start;
        let mut empty = true;
        let tokens = tokenizer
            .into_iter()
            .take_while(|t| match t {
                Token::Whitespace((_, end)) => {
                    p = *end;
                    empty = false;
                    true
                }
                Token::RightCaret((_, end))
                | Token::Hash((_, end))
                | Token::Plaintext((_, end))
                | Token::Newline((_, end)) => {
                    p = *end;
                    false
                }
            })
            .map(|t| t.into());

        // An empty line is always closed once we hit a new line token
        node.children.extend(tokens);
        node.end = Some(p);
        return node.end;
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

    #[bench]
    fn bench_simple_parse(b: &mut Bencher) {
        b.iter(|| parse("> Hello,\nWorld!"));
    }
}
