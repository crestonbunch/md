use super::*;

pub fn consume(node: &mut Node, start: usize, source: &str) -> Option<usize> {
    let tokenizer = Tokenizer::new(start, source);
    let mut p = start;

    // Empty tokens are easily confused with paragraphs. In order to reduce
    // the complexity of parsing empty lines in the normal probe-open-consume
    // cycle, we break out and consume entire chunks of empty lines
    // before returning to the main loop.
    let mut consumed = false;
    let empty = Node::new(Kind::Empty, start);
    for token in tokenizer {
        match token {
            Token::Whitespace((start, _)) => {
                p = start;
            }
            Token::Newline((_, end)) => {
                let empty_line = Node::new(Kind::EmptyLine, p);
                empty_line.borrow_mut().end = Some(end);
                empty.borrow_mut().children.push(empty_line);
                p = end;
                consumed = true;
            }
            _ => break,
        }
    }

    if consumed {
        let mut borrow = empty.borrow_mut();
        borrow.end = Some(p);
        if let Some(open) = borrow.children.last_mut() {
            open.borrow_mut().end = Some(p);
        }
        node.children.push(Rc::clone(&empty));

        Some(p)
    } else {
        None
    }
}
