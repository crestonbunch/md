use crate::markdown::parse::Span;
use crate::markdown::parse::Token;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Probe<'a> {
    Plaintext(Span, &'a str),
    Eof(Span),
    Blockquote(Span),
    Header(Span, usize),
    Ul(Span, usize),
    Ol(Span, usize),
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
        let mut source = source;
        let mut open_tokens: Vec<Token> = vec![];
        let mut tokens = vec![];
        let mut line_no = 0;

        while match tokens.last() {
            Some(&Token::Eof(_)) => false,
            _ => true,
        } {
            let (new_tokens, new_source) = self.tokenize_line(line_no, source, open_tokens.clone());
            source = new_source;
            for t in new_tokens {
                if let Some(peek) = open_tokens.last() {
                    if match (peek, &t) {
                        (Token::UnorderedListStart(_), Token::UnorderedListEnd(_)) => true,
                        (Token::OrderedListStart(_), Token::OrderedListEnd(_)) => true,
                        (Token::ListItemStart(_), Token::ListItemEnd(_)) => true,
                        (Token::BlockquoteStart(_), Token::BlockquoteEnd(_)) => true,
                        (Token::ParagraphStart(_), Token::ParagraphEnd(_)) => true,
                        _ => false,
                    } {
                        open_tokens.pop();
                    }
                }

                if match &t {
                    Token::UnorderedListStart(_) => true,
                    Token::OrderedListStart(_) => true,
                    Token::ListItemStart(_) => true,
                    Token::BlockquoteStart(_) => true,
                    Token::ParagraphStart(_) => true,
                    _ => false,
                } {
                    open_tokens.push(t.clone());
                }

                tokens.push(t);
            }

            line_no += 1;
        }

        tokens
    }

    fn tokenize_line(
        &'a self,
        line_no: usize,
        source: &'a str,
        open_tokens: Vec<Token>,
    ) -> (Vec<Token>, &'a str) {
        let open_tokens = open_tokens;
        let (probes, span, text, len) = self.probe(line_no, source);
        let source = &source[len..];

        // First we look for open blocks and match the probes against
        // them. In order to remain open, a block needs to have an
        // appropriate continuation at the start of the line. For lists
        // any list item on the same level or greater matches.
        let mut unmatched = &open_tokens[..];
        let mut probes = &probes[..];
        let (mut max_ol, mut max_ul) = (0, 0);
        while !unmatched.is_empty() && !probes.is_empty() {
            let probe = probes[0];
            let open = &unmatched[0];
            match (open, probe) {
                // As long as there exists a list item of equal
                // or greater spacing, we can match the list
                (Token::UnorderedListStart((_, a)), Probe::Ul(..)) => max_ul = *a,
                (Token::OrderedListStart((_, a)), Probe::Ol(..)) => max_ol = *a,
                // However, list items must strictly increase in
                // spacing or we close them.
                (Token::ListItemStart((_, a)), Probe::Ul(_, b)) if *a < b => (),
                (Token::ListItemStart((_, a)), Probe::Ol(_, b)) if *a < b => (),
                // > can only be continued with another >
                (Token::BlockquoteStart(_), Probe::Blockquote(_)) => probes = &probes[1..],
                _ => {
                    // The first time any block unmatched, all
                    // remaining blocks are unmatched
                    break;
                }
            }
            unmatched = &unmatched[1..];
        }

        // Any unconsumed probes are now considered the start of
        // new blocks. If we're starting a new block we need to
        // close any unmatched blocks.
        if !probes.is_empty() {
            let tail = &probes[probes.len() - 1..];
            let tokens = unmatched
                .into_iter()
                .rev()
                .map(|b| {
                    let span = Span::single_line(line_no, 0, 0);
                    match b {
                        Token::ListItemStart(_) => Token::ListItemEnd(span),
                        Token::BlockquoteStart(_) => Token::BlockquoteEnd(span),
                        Token::UnorderedListStart(_) => Token::UnorderedListEnd(span),
                        Token::OrderedListStart(_) => Token::OrderedListEnd(span),
                        Token::ParagraphStart(_) => Token::ParagraphEnd(span),
                        _ => unreachable!(),
                    }
                })
                .chain(probes.into_iter().flat_map(|p| match p {
                    Probe::Header(span, size) => vec![Token::Header((*span, *size))],
                    Probe::Plaintext(_, _) => vec![],
                    Probe::Blockquote(span) => vec![Token::BlockquoteStart((*span, ()))],
                    Probe::Ul(span, level) if level > &max_ul => vec![
                        Token::UnorderedListStart((*span, *level)),
                        Token::ListItemStart((*span, *level)),
                    ],
                    Probe::Ol(span, level) if level > &max_ol => vec![
                        Token::OrderedListStart((*span, *level)),
                        Token::ListItemStart((*span, *level)),
                    ],
                    Probe::Ol(span, i) | Probe::Ul(span, i) => {
                        vec![Token::ListItemStart((*span, *i))]
                    }
                    Probe::Eof(span) => vec![Token::Eof(*span)],
                }))
                // Following the last probe we include the plaintext and
                // sometimes a paragraph start.
                .chain(tail.into_iter().flat_map(|p| {
                    let text = Token::Plaintext((span, text.into()));
                    match p {
                        Probe::Header(_, _) => vec![text],
                        Probe::Plaintext(span, t) => vec![Token::Plaintext((*span, (*t).into()))],
                        Probe::Blockquote(span) => vec![Token::ParagraphStart((*span, ())), text],
                        Probe::Ul(span, _) => vec![Token::ParagraphStart((*span, ())), text],
                        Probe::Ol(span, _) => vec![Token::ParagraphStart((*span, ())), text],
                        Probe::Eof(_) => vec![],
                    }
                }))
                .collect();
            return (tokens, source);
        }

        // If probes is empty, this is a continuation if we have
        // unmatched blocks, and a paragraph otherwise.
        if !unmatched.is_empty() {
            let text = Token::Plaintext((span, text.into()));
            (vec![text], source)
        } else {
            let text = Token::Plaintext((span, text.into()));
            let p = Token::ParagraphStart((Span::single_line(line_no, 0, 0), ()));
            (vec![p, text], source)
        }
    }

    fn probe(&'a self, line_no: usize, source: &'a str) -> (Vec<Probe<'a>>, Span, &'a str, usize) {
        let orig = source;
        let mut source = source;
        let mut probes = vec![];
        let mut col = 0;
        // Start probing for line indicators
        loop {
            match self.probe_block(line_no, col, source) {
                // Stop as soon as we find plaintext -- any remaining
                // tokens are parsed as part of the line. NB: plaintext
                // are not pushed onto the probes stack
                (Probe::Plaintext(span, plaintext), end) => {
                    return (probes, span, plaintext, end);
                }
                // The Eof probe is a special case indicating the end of
                // the source and we should close all open blocks.
                (Probe::Eof(span), end) => {
                    probes.push(Probe::Eof(span));
                    return (probes, span, "", end);
                }
                // Push the probed token onto the stack. We will use
                // the stack of tokens to decide which blocks to open/close.
                (probe, end) => {
                    probes.push(probe);
                    source = &orig[end..];
                    col = end;
                }
            }
        }
    }

    /// Probe for the next token, returning the token and how many
    /// characters of the source were consumed.
    fn probe_block(&'a self, line_no: usize, col: usize, source: &'a str) -> (Probe<'a>, usize) {
        // Split the input source into three chunks: whitespace, token,
        // whitespace. The token may be a block start token.
        let (_, a) = self.probe_whitespace(source);
        let (token, b) = self.probe_non_whitespace(&source[a..]);
        let (_, c) = self.probe_whitespace(&source[b..]);
        let end_col = col + a + b + c;

        let span = Span::single_line(line_no, col, end_col);
        match token {
            "-" | "+" | "*" => (Probe::Ul(span, end_col), end_col),
            "1." | "1)" => (Probe::Ol(span, end_col), end_col),
            "#" => (Probe::Header(span, 1), end_col),
            "##" => (Probe::Header(span, 2), end_col),
            "###" => (Probe::Header(span, 3), end_col),
            "####" => (Probe::Header(span, 4), end_col),
            "#####" => (Probe::Header(span, 5), end_col),
            "######" => (Probe::Header(span, 6), end_col),
            ">" => (Probe::Blockquote(span), end_col),
            "" => (Probe::Eof(span), end_col),
            // We did not consume a block token, so the remainder
            // of the line is just plaintext.
            _ => {
                // If the line ends in a new line character, we don't
                // want to count that as the end column so probe_line
                // returns two numbers: one for the column and one for
                // the remaining source slice
                let (text, b, rem) = self.probe_line(&source[a..]);
                let span = Span::single_line(line_no, col, col + a + b);
                (Probe::Plaintext(span, text), col + a + rem)
            }
        }
    }

    fn probe_whitespace(&'a self, source: &'a str) -> (&'a str, usize) {
        let mut ws = "";
        let mut p: usize = 0;
        while p < source.len()
            && match &source[p..p + 1] {
                " " | "\t" => true,
                _ => false,
            }
        {
            p += 1;
            ws = &source[..p];
        }
        (ws, p)
    }

    fn probe_non_whitespace(&'a self, source: &'a str) -> (&'a str, usize) {
        let mut token = "";
        let mut p: usize = 0;
        while p < source.len()
            && match &source[p..p + 1] {
                " " | "\t" | "\n" => false,
                _ => true,
            }
        {
            p += 1;
            token = &source[..p];
        }
        (token, p)
    }

    fn probe_line(&'a self, source: &'a str) -> (&'a str, usize, usize) {
        let mut p: usize = 0;
        while match source.get(p..p + 1) {
            None => return (&source[..p], p, p),
            Some("\n") => return (&source[..p], p, p + 1),
            _ => true,
        } {
            p += 1;
        }

        (&source[..p], p, p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(line: usize, start: usize, end: usize) -> Span {
        Span::single_line(line, start, end)
    }

    #[test]
    fn test_empty_source() {
        let tokenizer = Tokenizer::default();
        let source = "";
        let result = tokenizer.tokenize(source);

        assert_eq!(vec![Token::Eof(s(0, 0, 0))], result);
    }

    #[test]
    fn test_single_paragraph() {
        let tokenizer = Tokenizer::default();
        let source = "Hello, World!";
        let result = tokenizer.tokenize(source);

        assert_eq!(
            vec![
                Token::ParagraphStart((s(0, 0, 0), ())),
                Token::Plaintext((s(0, 0, 13), "Hello, World!".into())),
                Token::ParagraphEnd(s(1, 0, 0)),
                Token::Eof(s(1, 0, 0))
            ],
            result
        );
    }

    #[test]
    fn test_header_paragraph() {
        let tokenizer = Tokenizer::default();
        let source = "# Title\nHello, World!";
        let result = tokenizer.tokenize(source);

        assert_eq!(
            vec![
                Token::Header((s(0, 0, 2), 1)),
                Token::Plaintext((s(0, 2, 7), "Title".into())),
                Token::ParagraphStart((s(1, 0, 0), ())),
                Token::Plaintext((s(1, 0, 13), "Hello, World!".into())),
                Token::ParagraphEnd(s(2, 0, 0)),
                Token::Eof(s(2, 0, 0))
            ],
            result
        );
    }

    #[test]
    fn test_nested_lists() {
        let tokenizer = Tokenizer::default();
        let source = "- One\n- Two\n  - Three\n- Four";
        let result = tokenizer.tokenize(source);

        assert_eq!(
            vec![
                Token::UnorderedListStart((s(0, 0, 2), 2)),
                Token::ListItemStart((s(0, 0, 2), 2)),
                Token::ParagraphStart((s(0, 0, 2), ())),
                Token::Plaintext((s(0, 2, 5), "One".into())),
                Token::ParagraphEnd(s(1, 0, 0)),
                Token::ListItemEnd(s(1, 0, 0)),
                Token::ListItemStart((s(1, 0, 2), 2)),
                Token::ParagraphStart((s(1, 0, 2), ())),
                Token::Plaintext((s(1, 2, 5), "Two".into())),
                Token::ParagraphEnd(s(2, 0, 0)),
                Token::UnorderedListStart((s(2, 0, 4), 4)),
                Token::ListItemStart((s(2, 0, 4), 4)),
                Token::ParagraphStart((s(2, 0, 4), ())),
                Token::Plaintext((s(2, 4, 9), "Three".into())),
                Token::ParagraphEnd(s(3, 0, 0)),
                Token::ListItemEnd(s(3, 0, 0)),
                Token::UnorderedListEnd(s(3, 0, 0)),
                Token::ListItemEnd(s(3, 0, 0)),
                Token::ListItemStart((s(3, 0, 2), 2)),
                Token::ParagraphStart((s(3, 0, 2), ())),
                Token::Plaintext((s(3, 2, 6), "Four".into())),
                Token::ParagraphEnd(s(4, 0, 0)),
                Token::ListItemEnd(s(4, 0, 0)),
                Token::UnorderedListEnd(s(4, 0, 0)),
                Token::Eof(s(4, 0, 0))
            ],
            result
        );
    }

    #[test]
    fn test_blockquote_list() {
        let tokenizer = Tokenizer::default();
        let source = [
            "> Lorem ipsum dolor",
            "sit amet.",
            "> - Qui *quodsi iracundia*",
            "> - aliquando id",
        ]
        .join("\n");
        let result = tokenizer.tokenize(&source[..]);

        assert_eq!(
            vec![
                Token::BlockquoteStart((s(0, 0, 2), ())),
                Token::ParagraphStart((s(0, 0, 2), ())),
                Token::Plaintext((s(0, 2, 19), "Lorem ipsum dolor".into())),
                Token::Plaintext((s(1, 0, 9), "sit amet.".into())),
                Token::ParagraphEnd(s(2, 0, 0)),
                Token::UnorderedListStart((s(2, 2, 4), 4)),
                Token::ListItemStart((s(2, 2, 4), 4)),
                Token::ParagraphStart((s(2, 2, 4), ())),
                Token::Plaintext((s(2, 4, 26), "Qui *quodsi iracundia*".into())),
                Token::ParagraphEnd(s(3, 0, 0)),
                Token::ListItemEnd(s(3, 0, 0)),
                Token::ListItemStart((s(3, 2, 4), 4)),
                Token::ParagraphStart((s(3, 2, 4), ())),
                Token::Plaintext((s(3, 4, 16), "aliquando id".into())),
                Token::ParagraphEnd(s(4, 0, 0)),
                Token::ListItemEnd(s(4, 0, 0)),
                Token::UnorderedListEnd(s(4, 0, 0)),
                Token::BlockquoteEnd(s(4, 0, 0)),
                Token::Eof(s(4, 0, 0))
            ],
            result
        );
    }
}
