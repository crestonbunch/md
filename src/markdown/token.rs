use regex::{Regex, RegexSet};

use crate::markdown::parse::Span;
use crate::markdown::parse::Token;
#[derive(Debug, Copy, Clone)]
enum Terminal {
    NewLine,
    PlainText,
    Whitespace,
    Hash1,
}

impl Terminal {
    fn as_token(self, span: Span, src: &str) -> Token {
        match self {
            Terminal::NewLine => Token::NewLine((span, src.to_string())),
            Terminal::PlainText => Token::PlainText((span, src.to_string())),
            Terminal::Whitespace => Token::Whitespace((span, src.to_string())),
            Terminal::Hash1 => Token::Hash1((span, src.to_string())),
        }
    }
}

const TOKEN_REGEX: [(&str, Terminal); 3] = [
    (r"^\n|\r\n|\r", Terminal::NewLine),
    (r"^[ \t]+", Terminal::Whitespace),
    (r"^#", Terminal::Hash1),
    // Anything not matched is Token::PlainText
];

pub struct Tokenizer {
    regex_set: RegexSet,
    regexes: Vec<Regex>,
}

impl<'a> Tokenizer {
    pub fn new() -> Tokenizer {
        let regex_set = RegexSet::new(TOKEN_REGEX.iter().map(|(re, _)| re)).unwrap();
        let regexes = TOKEN_REGEX
            .iter()
            .map(|(re, _)| Regex::new(re).unwrap())
            .collect();
        Tokenizer { regex_set, regexes }
    }

    /// Parse the input string into a list of lines of tokens. The lines
    /// can then be run through the parser to generate a syntax tree.
    pub fn tokenize(&self, source: &str) -> Vec<Token> {
        let mut source = source;
        let mut extraction = self.extract(source);
        let mut lines = vec![];

        let mut line_no = 0;
        let mut col_no = 0;

        let mut tokens = vec![];
        while let Some((terminal, token_str)) = extraction {
            let token_len = token_str.len();
            let start_col = col_no;
            let end_col = start_col + token_len;
            let span = Span::new(line_no, start_col, end_col);
            // Move the pointer after we've extracted the token
            col_no = end_col;
            source = &source[token_len..];
            match terminal {
                // When we hit a newline, push the tokens onto the line
                // vec, then start accumulating a new line.
                Terminal::NewLine => {
                    lines.push(Tokenizer::line_token(span, tokens));
                    tokens = vec![];
                    line_no += 1;
                    col_no = 0;
                }
                t => tokens.push(t.as_token(span, token_str)),
            }
            extraction = self.extract(source);
        }

        lines
    }

    /// Extract the next available token from the source string. The
    /// returned tuple contains the extracted terminal type, and matching
    /// token string from the start of the source.
    fn extract(&self, source: &'a str) -> Option<(Terminal, &'a str)> {
        if source == "" {
            return None;
        }

        let matches = self.regex_set.matches(&source[..]);
        if !matches.matched_any() {
            return self.extract_plaintext(source);
        }

        let m = matches.iter().next().unwrap();
        let matching_regex = &self.regexes[m];
        let (_, matching_terminal) = &TOKEN_REGEX[m];
        let mat = matching_regex.find(&source[..]).unwrap();
        // let remainder = &source[mat.end()..];
        Some((*matching_terminal, &source[..mat.end()]))
    }

    fn extract_plaintext(&self, source: &'a str) -> Option<(Terminal, &'a str)> {
        let mut end = 1;
        let mut matches = self.regex_set.matches(&source[end..]);

        while !matches.matched_any() {
            end += 1;
            matches = self.regex_set.matches(&source[end..]);
        }

        Some((Terminal::PlainText, &source[..end]))
    }

    fn line_token(token_span: Span, tokens: Vec<Token>) -> Token {
        let line_span = Span::new(token_span.start_line, 0, token_span.end_col);
        match tokens.first() {
            None => Token::LineEmpty(line_span),
            Some(Token::Hash1(_)) => Token::LineHeader((line_span, (1, tokens))),
            Some(_) => Token::LinePlain((line_span, tokens)),
        }
    }
}
