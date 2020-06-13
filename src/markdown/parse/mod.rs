extern crate test;

mod block_quote;
mod container;
mod empty;
mod heading;
mod leaf;
mod list_item;
mod ordered_list;
mod paragraph;
mod token;
mod unordered_list;

use std::cell::RefCell;
use std::rc::Rc;

use token::{Token, Tokenizer};

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
    Paren,
    Dot,
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

impl OrderedList {
    fn new(token: OrderedListToken, width: usize) -> Kind {
        Kind::OrderedList(OrderedList {
            token,
            width,
            tight: false,
            start: 1, // TODO
        })
    }
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
        if let Some(p) = paragraph::probe(self, start, &a, &b, &c) {
            return Some(p);
        }
        if let Some(p) = block_quote::probe(self, start, &a, &b, &c) {
            return Some(p);
        }
        if let Some(p) = unordered_list::probe(self, start, &a, &b, &c) {
            return Some(p);
        }
        if let Some(p) = ordered_list::probe(self, start, &a, &b, &c) {
            return Some(p);
        }
        if let Some(p) = list_item::probe(self, start, &a, &b, &c) {
            return Some(p);
        }
        None
    }

    fn open(&self, start: usize, source: &str) -> Option<(Link, usize)> {
        let mut tokenizer = Tokenizer::new(start, source);
        let (a, b, c) = (tokenizer.next(), tokenizer.next(), tokenizer.next());

        if let Some((node, p)) = block_quote::open(self, &a, &b, &c) {
            return Some((node, p));
        }
        if let Some((node, p)) = heading::open(self, &a, &b, &c) {
            return Some((node, p));
        }
        if let Some((node, p)) = unordered_list::open(self, &a, &b, &c) {
            return Some((node, p));
        }
        if let Some((node, p)) = ordered_list::open(self, &a, &b, &c) {
            return Some((node, p));
        }
        if let Some((node, p)) = list_item::open(self, &a, &b, &c) {
            return Some((node, p));
        }

        None
    }

    fn consume(&mut self, start: usize, source: &str) -> Option<usize> {
        match self.kind {
            Kind::Document => container::consume(self, start, source),
            Kind::BlockQuote => block_quote::consume(self, start, source),
            Kind::UnorderedList(..) => unordered_list::consume(self, start, source),
            Kind::OrderedList(..) => ordered_list::consume(self, start, source),
            Kind::ListItem(..) => list_item::consume(self, start, source),
            Kind::Paragraph => paragraph::consume(self, start, source),
            Kind::Heading(..) => heading::consume(self, start, source),
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
        // let result = parse("* List item\n  * Nested list\n* Third list item");
        let result = parse("* One list\n- Two list\n+ Three list");
        dbg!(&result);
    }

    #[test]
    fn test_ordered_lists() {
        let result = parse("1. List item\n1. Second list item");
        dbg!(&result);
    }

    #[bench]
    fn bench_simple_parse(b: &mut Bencher) {
        b.iter(|| parse("> Hello,\nWorld!"));
    }
}
