use std::cell::RefCell;
use std::rc::Rc;

use crate::markdown::parse::{Kind, Node};

const WHITESPACE_CHARS: [&str; 2] = [" ", "\t"];

type Slice = (usize, usize);

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Token {
    RightCaret(Slice),
    Hash(Slice),
    Plaintext(Slice),
    Whitespace(Slice),
    Newline(Slice),
}

impl Into<Rc<RefCell<Node>>> for Token {
    fn into(self) -> Rc<RefCell<Node>> {
        match self {
            Token::RightCaret((start, end)) => Node::new_inline(Kind::Plaintext, start, end),
            Token::Hash((start, end)) => Node::new_inline(Kind::Plaintext, start, end),
            Token::Plaintext((start, end)) => Node::new_inline(Kind::Plaintext, start, end),
            Token::Whitespace((start, end)) => Node::new_inline(Kind::Whitespace, start, end),
            Token::Newline((start, end)) => Node::new_inline(Kind::Whitespace, start, end),
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum TokenizerState {
    Unset,
    Done,
    Hash,
    Plaintext,
    Whitespace,
}

pub struct Tokenizer<'a> {
    start: usize,
    source: &'a str,
}

impl<'a> Tokenizer<'a> {
    pub fn new(start: usize, source: &'a str) -> Self {
        Tokenizer { start, source }
    }
}

impl<'a> Iterator for Tokenizer<'a> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        let mut p = self.start;
        let mut state = TokenizerState::Unset;
        let mut result = None;

        while state != TokenizerState::Done {
            let (new_state, new_p) = match (state, self.source.get(p..p + 1)) {
                // Whitespace
                (TokenizerState::Whitespace, Some(c)) if WHITESPACE_CHARS.contains(&c) => {
                    (TokenizerState::Whitespace, p + 1)
                }
                (TokenizerState::Whitespace, _) => {
                    result = Some(Token::Whitespace((self.start, p)));
                    (TokenizerState::Done, p)
                }
                // Plaintext
                (TokenizerState::Plaintext, Some(c)) if WHITESPACE_CHARS.contains(&c) => {
                    result = Some(Token::Plaintext((self.start, p)));
                    (TokenizerState::Done, p)
                }
                (TokenizerState::Plaintext, Some("\n")) => {
                    result = Some(Token::Plaintext((self.start, p)));
                    (TokenizerState::Done, p)
                }
                (TokenizerState::Plaintext, Some(_)) => {
                    (TokenizerState::Plaintext, p + 1)
                }
                (TokenizerState::Plaintext, None) => {
                    result = Some(Token::Plaintext((self.start, p)));
                    (TokenizerState::Done, p)
                }
                // Hash
                (TokenizerState::Hash, Some("#")) => {
                    (TokenizerState::Hash, p + 1)
                }
                (TokenizerState::Hash, _) => {
                    result = Some(Token::Hash((self.start, p)));
                    (TokenizerState::Done, p)
                }
                // Unset
                (TokenizerState::Unset, Some(c)) if WHITESPACE_CHARS.contains(&c) => {
                    (TokenizerState::Whitespace, p + 1)
                }
                (TokenizerState::Unset, Some("\n")) => {
                    result = Some(Token::Newline((self.start, p + 1)));
                    (TokenizerState::Done, p + 1)
                }
                (TokenizerState::Unset, Some(">")) => {
                    result = Some(Token::RightCaret((self.start, p + 1)));
                    (TokenizerState::Done, p + 1)
                }
                (TokenizerState::Unset, Some("#")) => {
                    result = Some(Token::Hash((self.start, p + 1)));
                    (TokenizerState::Hash, p + 1)
                }
                (TokenizerState::Unset, Some(_)) => {
                    (TokenizerState::Plaintext, p + 1)
                }
                // Done
                _ => (TokenizerState::Done, p),
            };
            state = new_state;
            p = new_p;
        }

        self.start = p;
        result
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_plaintext() {
        let tokenizer = Tokenizer::new(0, "Hello, World!");
        let result = tokenizer.into_iter().collect::<Vec<_>>();

        assert_eq!(
            result,
            vec![
                Token::Plaintext((0, 6)),
                Token::Whitespace((6, 7)),
                Token::Plaintext((7, 13)),
            ]
        );
    }

    #[test]
    fn test_hash() {
        let tokenizer = Tokenizer::new(0, "### Header Text");
        let result = tokenizer.into_iter().collect::<Vec<_>>();

        assert_eq!(
            result,
            vec![
                Token::Hash((0, 3)),
                Token::Whitespace((3, 4)),
                Token::Plaintext((4, 10)),
                Token::Whitespace((10, 11)),
                Token::Plaintext((11, 15)),
            ]
        );
    }
}
