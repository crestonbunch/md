use super::*;

pub fn consume(node: &mut Node, start: usize, _source: &str) -> Option<usize> {
    // If we consume a non-leaf block that has no open child,
    // we need to push a child to consume.
    if match node.children.last() {
        None => true,
        Some(node) if node.borrow().end.is_some() => true,
        _ => false,
    } {
        node.children.push(Node::new(Kind::Paragraph, start));
    }

    // Containers do not consume anything
    Some(start)
}
