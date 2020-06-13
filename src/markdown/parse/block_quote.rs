use super::*;

pub fn probe(
    parent: &Node,
    _start: usize,
    a: &Option<Token>,
    b: &Option<Token>,
    c: &Option<Token>,
) -> Option<usize> {
    match (parent.kind, a, b, c) {
        (Kind::BlockQuote, Some(Token::RightCaret((_, end))), ..) => Some(*end),
        _ => None,
    }
}

pub fn open(
    _parent: &Node,
    a: &Option<Token>,
    b: &Option<Token>,
    c: &Option<Token>,
) -> Option<(Link, usize)> {
    match (a, b, c) {
        (Some(Token::Whitespace((_, _))), Some(Token::RightCaret((start, end))), _) => {
            Some((Node::new(Kind::BlockQuote, *start), *end))
        }
        (Some(Token::RightCaret((start, end))), ..) => {
            Some((Node::new(Kind::BlockQuote, *start), *end))
        }
        _ => None,
    }
}

pub fn consume(node: &mut Node, start: usize, source: &str) -> Option<usize> {
    container::consume(node, start, source)
}
