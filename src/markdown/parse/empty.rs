use super::*;

pub fn open(
    parent: &mut Node,
    a: &Option<Token>,
    b: &Option<Token>,
    c: &Option<Token>,
) -> Option<(Link, usize)> {
    match parent.kind {
        Kind::Empty | Kind::UnorderedList(..) | Kind::OrderedList(..) => return None,
        Kind::Document => {
            // If a list cannot continue after an empty block, then we close the list.
            let last_child = parent.children.last().map(Rc::clone);
            if let Some((container, child)) = last_child.and_then(find_last_empty) {
                // HACK: move the empty node from the list item parent to the
                // document parent so that empty blocks terminating lists are
                // at the root (this improves the editor experience, but has
                // no actual effect on rendering markdown)
                let start = child.borrow().start;
                parent.close_child(start);

                let end = child.borrow().end.unwrap();
                let mut borrow = container.borrow_mut();
                borrow.children.pop();

                parent.children.push(Rc::clone(&child));
                return Some((child, end));
            }
        }
        _ => (),
    }
    match (a, b, c) {
        (Some(Token::Newline((start, end))), ..) => {
            let node = Node::new(Kind::Empty, *start);
            Some((node, *end))
        }
        (Some(Token::Whitespace((start, _))), Some(Token::Newline((_, end))), ..) => {
            let node = Node::new(Kind::Empty, *start);
            Some((node, *end))
        }
        _ => None,
    }
}

pub fn consume(node: &mut Node, start: usize, source: &str) -> Option<usize> {
    let tokenizer = Tokenizer::new(start, source);
    if node.end == None {
        let mut p = start;
        let mut empty = true;
        // Consume all empty lines with optional whitespace
        for token in tokenizer {
            match token {
                Token::Whitespace((start, _)) => {
                    p = start;
                }
                Token::Newline((_, end)) => {
                    let empty_line = Node::new(Kind::EmptyLine, p);
                    empty_line.borrow_mut().end = Some(end);
                    node.children.push(empty_line);
                    p = end;
                    empty = false;
                }
                _ => break,
            }
        }
        node.end = Some(p);
        if !empty {
            return node.end;
        }
    }
    None
}

fn find_last_empty(node: Link) -> Option<(Link, Link)> {
    let mut parent = node;
    while match {
        let borrow = parent.borrow();
        borrow.kind
    } {
        Kind::ListItem(..) => true,
        Kind::UnorderedList(..) => true,
        Kind::OrderedList(..) => true,
        _ => false,
    } {
        let clone = Rc::clone(&parent);
        let borrow = clone.borrow();
        let child = borrow.children.last();
        if let Some(child) = child {
            let borrow = child.borrow();
            if let Kind::Empty = borrow.kind {
                return Some((Rc::clone(&parent), Rc::clone(child)));
            } else {
                parent = Rc::clone(child);
            }
        } else {
            return None;
        }
    }
    None
}
