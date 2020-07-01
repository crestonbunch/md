use super::*;

pub fn probe(
    parent: &Node,
    start: usize,
    a: &Option<Token>,
    b: &Option<Token>,
    c: &Option<Token>,
) -> Option<usize> {
    if let Kind::ListItem(width) = parent.kind {
        return match (a, b, c) {
            (Some(Token::Whitespace((_, end))), ..) if width < (end - start + 1) => {
                // Keep list items open if there is a deeper nested list
                // contained inside of them.
                Some(*end)
            }
            (Some(Token::Newline(..)), ..) => Some(start),
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
    match (parent.kind, a, b, c) {
        // Unordered lists
        (k, Some(Token::Asterisk((start, _))), Some(Token::Whitespace((_, end))), _)
        | (
            k,
            Some(Token::Whitespace((start, _))),
            Some(Token::Asterisk((_, _))),
            Some(Token::Whitespace((_, end))),
        ) if is_ul_child(k, UnorderedListToken::Asterisk, *start, *end) => {
            Some((Node::new(Kind::ListItem(end - start), *start), *end))
        }
        (k, Some(Token::Dash((start, _))), Some(Token::Whitespace((_, end))), _)
        | (
            k,
            Some(Token::Whitespace((start, _))),
            Some(Token::Dash((_, _))),
            Some(Token::Whitespace((_, end))),
        ) if is_ul_child(k, UnorderedListToken::Dash, *start, *end) => {
            Some((Node::new(Kind::ListItem(end - start), *start), *end))
        }
        (k, Some(Token::Plus((start, _))), Some(Token::Whitespace((_, end))), _)
        | (
            k,
            Some(Token::Whitespace((start, _))),
            Some(Token::Plus((_, _))),
            Some(Token::Whitespace((_, end))),
        ) if is_ul_child(k, UnorderedListToken::Plus, *start, *end) => {
            Some((Node::new(Kind::ListItem(end - start), *start), *end))
        }
        // Ordered lists
        (k, Some(Token::NumDot((start, _))), Some(Token::Whitespace((_, end))), _)
        | (
            k,
            Some(Token::Whitespace((start, _))),
            Some(Token::NumDot((_, _))),
            Some(Token::Whitespace((_, end))),
        ) if is_ol_child(k, OrderedListToken::Dot, *start, *end) => {
            Some((Node::new(Kind::ListItem(end - start), *start), *end))
        }
        (k, Some(Token::NumParen((start, _))), Some(Token::Whitespace((_, end))), _)
        | (
            k,
            Some(Token::Whitespace((start, _))),
            Some(Token::NumParen((_, _))),
            Some(Token::Whitespace((_, end))),
        ) if is_ol_child(k, OrderedListToken::Paren, *start, *end) => {
            Some((Node::new(Kind::ListItem(end - start), *start), *end))
        }
        _ => None,
    }
}

pub fn consume(node: &mut Node, start: usize, source: &str) -> Option<usize> {
    container::consume(node, start, source)
}

fn is_ul_child(list_kind: Kind, list_token: UnorderedListToken, start: usize, end: usize) -> bool {
    match list_kind {
        Kind::UnorderedList(UnorderedList { token, width, .. })
            if width == (end - start) && token == list_token =>
        {
            true
        }
        _ => false,
    }
}

fn is_ol_child(list_kind: Kind, list_token: OrderedListToken, start: usize, end: usize) -> bool {
    match list_kind {
        Kind::OrderedList(OrderedList { token, width, .. })
            if width == (end - start) && token == list_token =>
        {
            true
        }
        _ => false,
    }
}
