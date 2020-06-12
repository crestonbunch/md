use super::*;

pub fn consume(node: &mut Node, start: usize, source: &str) -> Option<usize> {
    if start >= source.len() {
        return None;
    }

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
