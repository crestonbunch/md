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

pub fn consume(node: &mut Node, start: usize, source: &str) -> Option<usize> {
    leaf::consume(node, start, source)
}
