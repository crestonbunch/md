use super::*;

pub fn probe(
    parent: &Node,
    start: usize,
    a: &Option<Token>,
    b: &Option<Token>,
    c: &Option<Token>,
) -> Option<usize> {
    if let Kind::OrderedList(OrderedList { width, token, .. }) = parent.kind {
        return match (token, a, b, c) {
            (
                OrderedListToken::Dot,
                Some(Token::NumDot((_, end))),
                Some(Token::Whitespace(..)),
                _,
            )
            | (
                OrderedListToken::Paren,
                Some(Token::NumParen((_, end))),
                Some(Token::Whitespace(..)),
                _,
            ) if (width <= end - start + 1) => Some(start),
            (
                OrderedListToken::Dot,
                Some(Token::Whitespace(..)),
                Some(Token::NumDot((_, end))),
                Some(Token::Whitespace(..)),
            )
            | (
                OrderedListToken::Paren,
                Some(Token::Whitespace(..)),
                Some(Token::NumParen((_, end))),
                Some(Token::Whitespace(..)),
            ) if (width <= end - start + 1) => Some(start),
            (_, Some(Token::Whitespace((_, end))), ..) if (width <= end - start + 1) => Some(start),
            _ => None,
        };
    }
    None
}

pub fn open(
    parent: &Node,
    a: &Option<Token>,
    b: &Option<Token>,
    c: &Option<Token>,
) -> Option<(Link, usize)> {
    if let Kind::OrderedList(..) = parent.kind {
        // We cannot open another list inside a list
        return None;
    }
    match (a, b, c) {
        (Some(Token::NumDot((start, _))), Some(Token::Whitespace((_, end))), _)
        | (
            Some(Token::Whitespace((start, _))),
            Some(Token::NumDot(..)),
            Some(Token::Whitespace((_, end))),
        ) => Some((
            Node::new(OrderedList::new(OrderedListToken::Dot, end - start), *start),
            *end,
        )),
        (Some(Token::NumParen((start, _))), Some(Token::Whitespace((_, end))), _)
        | (
            Some(Token::Whitespace((start, _))),
            Some(Token::NumParen((_, _))),
            Some(Token::Whitespace((_, end))),
        ) => Some((
            Node::new(
                OrderedList::new(OrderedListToken::Paren, end - start),
                *start,
            ),
            *end,
        )),
        _ => None,
    }
}

pub fn consume(node: &mut Node, start: usize, source: &str) -> Option<usize> {
    if match node.children.last() {
        None => true,
        Some(node) if node.borrow().end.is_some() => true,
        _ => false,
    } {
        if let Kind::OrderedList(OrderedList { width, .. }) = node.kind {
            node.children.push(Node::new(Kind::ListItem(width), start));
        }
    }
    container::consume(node, start, source)
}
