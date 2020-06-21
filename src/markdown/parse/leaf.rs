use super::*;

pub fn consume(node: &mut Node, start: usize, source: &str) -> Option<usize> {
    // For leaf blocks we consume tokens until the next new line
    let tokenizer = Tokenizer::new(start, source);
    if node.end == None {
        let mut p = start;
        let tokens = tokenizer
            .into_iter()
            .take_while(|t| match t {
                Token::Newline((_, end)) => {
                    p = *end;
                    false
                }
                Token::RightCaret((_, end))
                | Token::Hash((_, end))
                | Token::Dash((_, end))
                | Token::Plus((_, end))
                | Token::NumDot((_, end))
                | Token::NumParen((_, end))
                | Token::Asterisk((_, end))
                | Token::Plaintext((_, end))
                | Token::Whitespace((_, end)) => {
                    p = *end;
                    true
                }
            })
            .map(|t| t.into());

        node.children.extend(tokens);
        return Some(p);
    }
    None
}
