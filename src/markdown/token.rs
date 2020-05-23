use crate::markdown::parse::Span;
use crate::markdown::parse::Token;

#[derive(Debug, Copy, Clone)]
enum Terminal {
    NewLine,
    PlainText,
    Whitespace,
    Hash,
    Atx(usize),
    Asterisk,
    Underscore,
    Dash,
    Eq,
    Hr,
    HrOrSetext,
    Setext,
}

impl Terminal {
    fn as_token(self, span: Span, src: &str) -> Token {
        match self {
            Terminal::NewLine => Token::NewLine((span, src.to_string())),
            Terminal::PlainText
            | Terminal::Dash
            | Terminal::Eq
            | Terminal::Hash
            | Terminal::Asterisk
            | Terminal::Underscore => Token::PlainText((span, src.to_string())),
            Terminal::Whitespace => Token::Whitespace((span, src.to_string())),
            Terminal::Atx(s) => Token::Atx((span, (s, src.to_string()))),
            Terminal::Hr => Token::LineHr((span, src.to_string())),
            Terminal::HrOrSetext => Token::LineHrOrSetext((span, src.to_string())),
            Terminal::Setext => Token::LineSetext((span, src.to_string())),
        }
    }
}

pub struct Tokenizer {}

impl Default for Tokenizer {
    fn default() -> Self {
        Tokenizer::new()
    }
}

impl<'a> Tokenizer {
    pub fn new() -> Self {
        Tokenizer {}
    }

    /// Parse the input string into a list of lines of tokens. The lines
    /// can then be run through the parser to generate a syntax tree.
    pub fn tokenize(&self, source: &str) -> Vec<Token> {
        let mut extraction = self.extract(source);
        let mut lines = vec![];

        let mut line_no = 0;
        let mut start_col = 0;
        let mut end_col = 0;

        let mut tokens = vec![];
        while let Some((terminal, slice, rem)) = extraction {
            match terminal {
                // When we hit a newline, push the tokens onto the line
                // vec, then start accumulating a new line.
                Terminal::NewLine => {
                    let span = Span::single_line(line_no, start_col, end_col);
                    lines.push(Tokenizer::line_token(span, tokens));
                    tokens = vec![];
                    line_no += 1;
                    start_col = 0;
                    end_col = 0;
                }
                t => {
                    end_col = start_col + slice.len();
                    let span = Span::single_line(line_no, start_col, end_col);
                    tokens.push(t.as_token(span, slice));
                    start_col = end_col;
                }
            }
            extraction = self.extract(rem);
        }

        // Push remaining tokens onto the end if the source did not end
        // with a new line.
        let span = Span::single_line(line_no, start_col, end_col);
        lines.push(Tokenizer::line_token(span, tokens));

        lines
    }

    /// Extract the next available token from the source string. The
    /// returned tuple contains the extracted terminal type, and matching
    /// token string from the start of the source.
    fn extract(&self, source: &'a str) -> Option<(Terminal, &'a str, &'a str)> {
        if source == "" {
            return None;
        }

        let result = self.scan(source);
        if let Some((terminal, slice, rem)) = result {
            Some((terminal, slice, rem))
        } else {
            let (terminal, slice, rem) = self.scan_plaintext(source);
            Some((terminal, slice, rem))
        }
    }

    fn line_token(token_span: Span, tokens: Vec<Token>) -> Token {
        let line_span = Span::single_line(token_span.start_line, 0, token_span.end_col);
        match tokens.first() {
            None => Token::LineEmpty(line_span),
            Some(Token::Atx((_, (s, _)))) => Token::LineHeader((line_span, (*s, tokens))),
            Some(Token::LineHrOrSetext(t)) => Token::LineHrOrSetext(t.clone()),
            Some(Token::LineHr(t)) => Token::LineHr(t.clone()),
            Some(Token::LineSetext(t)) => Token::LineSetext(t.clone()),
            Some(_) => Token::LinePlain((line_span, tokens)),
        }
    }

    fn scan(&self, source: &'a str) -> Option<(Terminal, &'a str, &'a str)> {
        let mut p = 0;
        let mut rem = &source[p..];
        let mut result = None;

        while !rem.is_empty() {
            p += 1;
            rem = &source[p..];
            let slice = &source[..p];

            let terminal = match slice {
                "\n" => Terminal::NewLine,
                " " | "\t" => Terminal::Whitespace,
                "###### " | "######\t" => Terminal::Atx(6),
                "##### " | "#####\t" => Terminal::Atx(5),
                "#### " | "####\t" => Terminal::Atx(4),
                "### " | "###\t" => Terminal::Atx(3),
                "## " | "##\t" => Terminal::Atx(2),
                "# " | "#\t" => Terminal::Atx(1),
                "===" => Terminal::Setext,
                "---" => Terminal::HrOrSetext,
                "___" | "***" => Terminal::Hr,
                // We need to make to make sure sure partial terminals still
                // trigger another loop iteration so it doesn't break the
                // loop and return PlainText. This is a little more
                // cumbersome than writing regex, but it's much more efficient.
                "#" | "##" | "###" | "####" | "#####" | "######" => Terminal::Hash,
                "*" | "**" => Terminal::Asterisk,
                "_" | "__" => Terminal::Underscore,
                "=" | "==" => Terminal::Eq,
                "-" | "--" => Terminal::Dash,
                _ => return result,
            };

            result = Some((terminal, slice, rem));
        }

        result
    }

    fn scan_plaintext(&self, source: &'a str) -> (Terminal, &'a str, &'a str) {
        let mut p = 1;
        let (mut slice, mut rem) = (&source[..p], &source[p..]);
        let mut result = (Terminal::PlainText, slice, rem);

        while !rem.is_empty() {
            match self.scan(rem) {
                Some(_) => return result,
                None => {
                    p += 1;
                    slice = &source[..p];
                    rem = &source[p..];
                    result = (Terminal::PlainText, slice, rem);
                }
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::parse::Token;

    #[test]
    fn test_basic_tokenization() {
        let tokenizer = Tokenizer::new();

        let test = "# Hello, World\n\nTest.\n";
        let tokens = tokenizer.tokenize(test);

        assert_eq!(
            vec![
                Token::LineHeader((
                    Span::single_line(0, 0, 14),
                    (
                        1,
                        vec![
                            Token::Atx((Span::single_line(0, 0, 2), (1, "# ".into()))),
                            Token::PlainText((Span::single_line(0, 2, 8), "Hello,".into())),
                            Token::Whitespace((Span::single_line(0, 8, 9), " ".into())),
                            Token::PlainText((Span::single_line(0, 9, 14), "World".into())),
                        ]
                    ),
                )),
                Token::LineEmpty(Span::single_line(1, 0, 0)),
                Token::LinePlain((
                    Span::single_line(2, 0, 5),
                    vec![Token::PlainText((
                        Span::single_line(2, 0, 5),
                        "Test.".into()
                    ))],
                )),
                Token::LineEmpty(Span::single_line(3, 0, 0)),
            ],
            tokens
        )
    }
}
