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
    match parent.kind {
        // We cannot open another list inside a list or empty block
        Kind::OrderedList(..) | Kind::Empty => return None,
        _ => (),
    }
    match (a, b, c) {
        (Some(Token::NumDot((start, _))), Some(Token::Whitespace((_, end))), _)
        | (
            Some(Token::Whitespace((start, _))),
            Some(Token::NumDot(..)),
            Some(Token::Whitespace((_, end))),
        ) => Some((
            {
                let ol = Node::new(OrderedList::new(OrderedListToken::Dot, end - start), *start);
                ol.borrow_mut()
                    .children
                    .push(Node::new(Kind::ListItem(end - start), *start));
                ol
            },
            *end,
        )),
        (Some(Token::NumParen((start, _))), Some(Token::Whitespace((_, end))), _)
        | (
            Some(Token::Whitespace((start, _))),
            Some(Token::NumParen((_, _))),
            Some(Token::Whitespace((_, end))),
        ) => Some((
            {
                let ol = Node::new(
                    OrderedList::new(OrderedListToken::Paren, end - start),
                    *start,
                );
                ol.borrow_mut()
                    .children
                    .push(Node::new(Kind::ListItem(end - start), *start));
                ol
            },
            *end,
        )),
        _ => None,
    }
}

pub fn consume(_node: &mut Node, start: usize, _source: &str) -> Option<usize> {
    Some(start)
}
