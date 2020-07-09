const WHITESPACE_CHARS: [&str; 2] = [" ", "\t"];
const NUMBER_CHARS: [&str; 10] = ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"];

pub type Span = (usize, usize);

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Token {
    RightCaret(Span),
    Hash(Span),
    Dash(Span),
    Asterisk(Span),
    Plus(Span),
    NumDot(Span),
    NumParen(Span),
    Plaintext(Span),
    Whitespace(Span),
    Newline(Span),
}

impl Token {
    pub fn span(&self) -> Span {
        match self {
            Token::RightCaret(s) => *s,
            Token::Hash(s) => *s,
            Token::Dash(s) => *s,
            Token::Asterisk(s) => *s,
            Token::Plus(s) => *s,
            Token::NumDot(s) => *s,
            Token::NumParen(s) => *s,
            Token::Plaintext(s) => *s,
            Token::Whitespace(s) => *s,
            Token::Newline(s) => *s,
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
    Number,
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
                (TokenizerState::Plaintext, Some(_)) => (TokenizerState::Plaintext, p + 1),
                (TokenizerState::Plaintext, None) => {
                    result = Some(Token::Plaintext((self.start, p)));
                    (TokenizerState::Done, p)
                }
                // Number
                (TokenizerState::Number, Some(c)) if NUMBER_CHARS.contains(&c) => {
                    (TokenizerState::Number, p + 1)
                }
                (TokenizerState::Number, Some(".")) => {
                    result = Some(Token::NumDot((self.start, p + 1)));
                    (TokenizerState::Done, p + 1)
                }
                (TokenizerState::Number, Some(")")) => {
                    result = Some(Token::NumParen((self.start, p + 1)));
                    (TokenizerState::Done, p + 1)
                }
                (TokenizerState::Number, _) => (TokenizerState::Plaintext, p),
                // Hash
                (TokenizerState::Hash, Some("#")) => (TokenizerState::Hash, p + 1),
                (TokenizerState::Hash, _) => {
                    result = Some(Token::Hash((self.start, p)));
                    (TokenizerState::Done, p)
                }
                // Dash
                (TokenizerState::Unset, Some("-")) => {
                    result = Some(Token::Dash((self.start, p + 1)));
                    (TokenizerState::Done, p + 1)
                }
                // Asterisk
                (TokenizerState::Unset, Some("*")) => {
                    result = Some(Token::Asterisk((self.start, p + 1)));
                    (TokenizerState::Done, p + 1)
                }
                // Plus
                (TokenizerState::Unset, Some("+")) => {
                    result = Some(Token::Plus((self.start, p + 1)));
                    (TokenizerState::Done, p + 1)
                }
                // Unset
                (TokenizerState::Unset, Some(c)) if WHITESPACE_CHARS.contains(&c) => {
                    (TokenizerState::Whitespace, p + 1)
                }
                (TokenizerState::Unset, Some(c)) if NUMBER_CHARS.contains(&c) => {
                    (TokenizerState::Number, p + 1)
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
                (TokenizerState::Unset, Some(_)) => (TokenizerState::Plaintext, p + 1),
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

    #[test]
    fn test_ol() {
        let tokenizer = Tokenizer::new(0, "1. Item\n12. Item");
        let result = tokenizer.into_iter().collect::<Vec<_>>();

        assert_eq!(
            result,
            vec![
                Token::NumDot((0, 2)),
                Token::Whitespace((2, 3)),
                Token::Plaintext((3, 7)),
                Token::Newline((7, 8)),
                Token::NumDot((8, 11)),
                Token::Whitespace((11, 12)),
                Token::Plaintext((12, 16)),
            ]
        );
    }

    #[test]
    fn test_numbers() {
        let tokenizer = Tokenizer::new(0, "Test 123 Test");
        let result = tokenizer.into_iter().collect::<Vec<_>>();

        assert_eq!(
            result,
            vec![
                Token::Plaintext((0, 4)),
                Token::Whitespace((4, 5)),
                Token::Plaintext((5, 8)),
                Token::Whitespace((8, 9)),
                Token::Plaintext((9, 13)),
            ]
        );
    }
}
