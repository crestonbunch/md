use super::*;

pub fn probe(
    parent: &Node,
    start: usize,
    a: &Option<Token>,
    b: &Option<Token>,
    c: &Option<Token>,
) -> Option<usize> {
    if let Kind::UnorderedList(UnorderedList { width, token, .. }) = parent.kind {
        return match (token, a, b, c) {
            (
                UnorderedListToken::Asterisk,
                Some(Token::Asterisk((_, end))),
                Some(Token::Whitespace(..)),
                _,
            )
            | (
                UnorderedListToken::Dash,
                Some(Token::Dash((_, end))),
                Some(Token::Whitespace(..)),
                _,
            )
            | (
                UnorderedListToken::Plus,
                Some(Token::Plus((_, end))),
                Some(Token::Whitespace(..)),
                _,
            ) if (width <= end - start + 1) => Some(start),
            (
                UnorderedListToken::Asterisk,
                Some(Token::Whitespace(..)),
                Some(Token::Asterisk((_, end))),
                Some(Token::Whitespace(..)),
            )
            | (
                UnorderedListToken::Dash,
                Some(Token::Whitespace(..)),
                Some(Token::Dash((_, end))),
                Some(Token::Whitespace(..)),
            )
            | (
                UnorderedListToken::Plus,
                Some(Token::Whitespace(..)),
                Some(Token::Plus((_, end))),
                Some(Token::Whitespace(..)),
            ) if (width <= end - start + 1) => Some(start),
            (_, Some(Token::Whitespace((_, end))), ..) if (width <= end - start + 1) => Some(start),
            (_, Some(Token::Newline(..)), ..) => Some(start),
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
        Kind::UnorderedList(..) | Kind::Empty => return None,
        _ => (),
    }
    let result = match (a, b, c) {
        (Some(Token::Asterisk((start, _))), Some(Token::Whitespace((_, end))), _)
        | (
            Some(Token::Whitespace((start, _))),
            Some(Token::Asterisk((_, _))),
            Some(Token::Whitespace((_, end))),
        ) => Some((
            Node::new(
                UnorderedList::new(UnorderedListToken::Asterisk, end - start),
                *start,
            ),
            *end,
        )),
        (Some(Token::Dash((start, _))), Some(Token::Whitespace((_, end))), _)
        | (
            Some(Token::Whitespace((start, _))),
            Some(Token::Dash((_, _))),
            Some(Token::Whitespace((_, end))),
        ) => Some((
            Node::new(
                UnorderedList::new(UnorderedListToken::Dash, end - start),
                *start,
            ),
            *end,
        )),
        (Some(Token::Plus((start, _))), Some(Token::Whitespace((_, end))), _)
        | (
            Some(Token::Whitespace((start, _))),
            Some(Token::Plus((_, _))),
            Some(Token::Whitespace((_, end))),
        ) => Some((
            Node::new(
                UnorderedList::new(UnorderedListToken::Plus, end - start),
                *start,
            ),
            *end,
        )),
        _ => None,
    };

    result
}

pub fn consume(node: &mut Node, start: usize, _source: &str) -> Option<usize> {
    if match node.children.last() {
        None => true,
        Some(node) if node.borrow().end.is_some() => true,
        _ => false,
    } {
        if let Kind::UnorderedList(UnorderedList { width, .. }) = node.kind {
            node.children.push(Node::new(Kind::ListItem(width), start));
        }
    }

    Some(start)
}
