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

const NON_LEAF_KINDS: [Kind; 2] = [Kind::Document, Kind::BlockQuote];

const DEFAULT_CAPACITY: usize = 32;

type Link = Rc<RefCell<Node>>;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Kind {
    // Block tokens
    Document,
    BlockQuote,
    Paragraph,
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
        let next = tokenizer.next();
        match (self.kind, next) {
            (Kind::BlockQuote, Some(Token::RightCaret((_, end)))) => Some(end),
            (Kind::Paragraph, Some(_)) => Some(start),
            (Kind::Document, Some(_)) => None,
            _ => None,
        }
    }

    fn open(&self, start: usize, source: &str) -> Option<(Link, usize)> {
        let mut tokenizer = Tokenizer::new(start, source);
        let (a, b, c) = (tokenizer.next(), tokenizer.next(), tokenizer.next());
        match (self.kind, a, b, c) {
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
            _ => None,
        }
    }

    fn consume(&mut self, start: usize, source: &str) -> Option<usize> {
        // If we consume a non-leaf block that has no children,
        // we need to push a child to consume it.
        if self.children.is_empty() {
            match self.kind {
                Kind::Document | Kind::BlockQuote => {
                    self.children.push(Node::new(Kind::Paragraph, self.start))
                }
                _ => (),
            }
        }

        // If this is a non-leaf block, we need to move to
        // its last child to consume it.
        if NON_LEAF_KINDS.contains(&self.kind) {
            if let Some(open) = self.children.last() {
                let mut open = open.borrow_mut();
                if let Some(p) = open.consume(start, source) {
                    return Some(p);
                } else {
                    // We did not consume anything, so that
                    // means we can close this child.
                    open.end = Some(start);
                }
            }
        }

        // For leaf blocks we consume tokens until the next new line
        let tokenizer = Tokenizer::new(start, source);
        match self.kind {
            Kind::Paragraph => {
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
                        | Token::Plaintext((_, end))
                        | Token::Whitespace((_, end)) => {
                            p = *end;
                            empty = false;
                            true
                        }
                    })
                    .map(|t| t.into());
                self.children.extend(tokens);
                if empty {
                    None
                } else {
                    Some(p)
                }
            }
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
        let parent = Rc::clone(&node);
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
        dbg!(p);
        let mut node = Rc::clone(&doc);

        // First we iterate through the open blocks, matching each block
        // with a token in the source. Any remaining unmatched tokens will either
        // be continued or closed in the next step.
        let (new_node, new_p) = Node::probe_all(node, p, source);
        node = new_node;
        p = new_p;
        dbg!(&node, p);

        if let Some(open) = {
            let borrow = node.borrow();
            borrow.open(p, source)
        } {
            dbg!(&open);
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
mod test {
    use super::*;

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
}
