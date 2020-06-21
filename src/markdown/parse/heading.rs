use super::*;

pub fn open(
    parent: &Node,
    a: &Option<Token>,
    b: &Option<Token>,
    c: &Option<Token>,
) -> Option<(Link, usize)> {
    match parent.kind {
        Kind::Empty => return None,
        _ => (),
    }
    match (a, b, c) {
        // Headers are not rendered without some content
        (Some(Token::Hash(..)), Some(Token::Whitespace(..)), Some(Token::Newline(..))) => None,
        (Some(Token::Hash((start, x))), Some(Token::Whitespace((_, end))), _)
            if (x - start) <= 6 =>
        {
            Some((Node::new(Kind::Heading(x - start), *start), *end))
        }
        _ => None,
    }
}

pub fn consume(node: &mut Node, start: usize, source: &str) -> Option<usize> {
    if let Some(p) = leaf::consume(node, start, source) {
        // Headings cannot be continued onto the next line
        // so we close it immediately.
        node.end = Some(p);
        Some(p)
    } else {
        node.end = Some(start);
        Some(start)
    }
}
