use super::*;

pub fn probe(
    parent: &Node,
    start: usize,
    a: &Option<Token>,
    b: &Option<Token>,
    c: &Option<Token>,
) -> Option<usize> {
    if let Kind::Paragraph = parent.kind {
        return match (a, b, c) {
            (Some(Token::Newline(..)), ..) => None,
            (Some(Token::Whitespace(..)), Some(Token::Newline(..)), ..) => None,
            (Some(_), ..) => Some(start),
            _ => None,
        };
    }
    None
}

pub fn open(parent: &Node, start: usize) -> Option<(Link, usize)> {
    if let Kind::Document = parent.kind {
        // If a list cannot continue after an empty block, then we
        // close the list and open a paragraph.
        let last_child = parent.children.last().map(Rc::clone);
        if let Some(_) = last_child.and_then(find_last_empty) {
            return Some((Node::new(Kind::Paragraph, start), start));
        }
    }
    None
}

pub fn consume(node: &mut Node, start: usize, source: &str) -> Option<usize> {
    leaf::consume(node, start, source)
}

fn find_last_empty(node: Link) -> Option<Link> {
    let borrow = node.borrow();
    match borrow.kind {
        Kind::ListItem(..) | Kind::UnorderedList(..) | Kind::OrderedList(..) => {
            let child = borrow.children.last();
            if let Some(child) = child {
                let borrow = child.borrow();
                if let Kind::Empty = borrow.kind {
                    Some(Rc::clone(child))
                } else {
                    find_last_empty(Rc::clone(child))
                }
            } else {
                None
            }
        }
        _ => None,
    }
}
