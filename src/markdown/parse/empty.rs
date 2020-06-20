use super::*;

pub fn open(
    parent: &Node,
    a: &Option<Token>,
    b: &Option<Token>,
    c: &Option<Token>,
) -> Option<(Link, usize)> {
    if parent.kind != Kind::Empty {
        match (a, b, c) {
            (Some(Token::Newline((start, end))), ..) => {
                let node = Node::new(Kind::Empty, *start);
                Some((node, *end))
            }
            (Some(Token::Whitespace((start, _))), Some(Token::Newline((_, end))), ..) => {
                let node = Node::new(Kind::Empty, *start);
                Some((node, *end))
            }
            _ => None,
        }
    } else {
        None
    }
}

pub fn consume(node: &mut Node, start: usize, source: &str) -> Option<usize> {
    let tokenizer = Tokenizer::new(start, source);
    if node.end == None {
        let mut p = start;
        let mut empty = true;
        // Consume all empty lines with optional whitespace
        for token in tokenizer {
            match token {
                Token::Whitespace((start, _)) => {
                    p = start;
                }
                Token::Newline((_, end)) => {
                    let empty_line = Node::new(Kind::EmptyLine, p);
                    empty_line.borrow_mut().end = Some(end);
                    node.children.push(empty_line);
                    p = end;
                    empty = false;
                }
                _ => break,
            }
        }
        node.end = Some(p);
        if !empty {
            return node.end;
        }
    }
    None
}
