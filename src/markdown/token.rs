use std::ops::Range;

use crate::markdown::parse::Span;
use crate::markdown::parse::Token;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Probe {
    Empty(Range<usize>),
    Eof(Range<usize>),
    Blockquote(Range<usize>),
    Header(Range<usize>, usize),
    Ul(Range<usize>, usize),
    Ol(Range<usize>, usize),
}

#[derive(Debug, Copy, Clone)]
enum ConsumeState {
    Plaintext,
    Whitespace,
}

pub struct Tokenizer {
    source: String,
    start_idx: usize,
    end_idx: usize,
    open: Vec<Token>,
}

impl Tokenizer {
    pub fn new(source: &str) -> Self {
        Tokenizer {
            source: source.into(),
            start_idx: 0,
            end_idx: 0,
            open: vec![],
        }
    }

    /// Parse the input string into a list of lines of tokens. The lines
    /// can then be run through the parser to generate a syntax tree.
    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = vec![];

        while match tokens.last() {
            Some(&Token::Eof(_)) => false,
            _ => true,
        } {
            let new_tokens = self.tokenize_line();
            for t in new_tokens {
                if let Some(peek) = self.open.last() {
                    if match (peek, &t) {
                        (Token::UnorderedListStart(_), Token::UnorderedListEnd(_)) => true,
                        (Token::OrderedListStart(_), Token::OrderedListEnd(_)) => true,
                        (Token::ListItemStart(_), Token::ListItemEnd(_)) => true,
                        (Token::BlockquoteStart(_), Token::BlockquoteEnd(_)) => true,
                        (Token::ParagraphStart(_), Token::ParagraphEnd(_)) => true,
                        (Token::EmptyStart(_), Token::EmptyEnd(_)) => true,
                        _ => false,
                    } {
                        self.open.pop();
                    }
                }

                if match &t {
                    Token::UnorderedListStart(_) => true,
                    Token::OrderedListStart(_) => true,
                    Token::ListItemStart(_) => true,
                    Token::BlockquoteStart(_) => true,
                    Token::ParagraphStart(_) => true,
                    Token::EmptyStart(_) => true,
                    _ => false,
                } {
                    self.open.push(t.clone());
                }

                tokens.push(t);
            }
        }

        tokens
    }

    fn tokenize_line(&mut self) -> Vec<Token> {
        let probes = self.probe();
        let (probes, mut unmatched, max_ol, max_ul) = self.match_probes(&probes);
        let end_idx = self.end_idx;
        // let span = Span::new(start_idx, end_idx);
        // let text = &source[start_idx..end_idx].trim_end_matches("\n");

        // Any unconsumed probes are now considered the start of
        // new blocks. If we're starting a new block we need to
        // close any unmatched blocks.
        if !probes.is_empty() {
            let tokens = unmatched
                .iter()
                .rev()
                .map(|b| {
                    let span = Span::new(end_idx, end_idx);
                    match b {
                        Token::ListItemStart(_) => Token::ListItemEnd(span),
                        Token::BlockquoteStart(_) => Token::BlockquoteEnd(span),
                        Token::UnorderedListStart(_) => Token::UnorderedListEnd(span),
                        Token::OrderedListStart(_) => Token::OrderedListEnd(span),
                        Token::ParagraphStart(_) => Token::ParagraphEnd(span),
                        Token::EmptyStart(_) => Token::EmptyEnd(span),
                        _ => unreachable!(),
                    }
                })
                .chain(probes.iter().flat_map(|p| match p {
                    Probe::Header(range, width) => vec![Token::Header((range.into(), *width))],
                    Probe::Blockquote(range) => vec![Token::BlockquoteStart(range.into())],
                    Probe::Ul(range, width) if width > &max_ul => vec![
                        Token::UnorderedListStart((range.into(), *width)),
                        Token::ListItemStart((range.into(), *width)),
                    ],
                    Probe::Ol(range, width) if width > &max_ol => vec![
                        Token::OrderedListStart((range.into(), *width)),
                        Token::ListItemStart((range.into(), *width)),
                    ],
                    Probe::Ol(range, width) | Probe::Ul(range, width) => {
                        vec![Token::ListItemStart((range.into(), *width))]
                    }
                    Probe::Empty(range) => vec![Token::EmptyStart(range.into())],
                    Probe::Eof(range) => vec![Token::Eof(range.into())],
                }))
                // Following the last probe we sometimes need to start a paragraph
                .chain({
                    let p = probes.last().unwrap();
                    match p {
                        Probe::Header(_, _) => self.consume_line(),
                        Probe::Blockquote(range) => [
                            vec![Token::ParagraphStart(range.into())],
                            self.consume_line(),
                        ]
                        .concat(),
                        Probe::Ul(range, _) => [
                            vec![Token::ParagraphStart(range.into())],
                            self.consume_line(),
                        ]
                        .concat(),
                        Probe::Ol(range, _) => [
                            vec![Token::ParagraphStart(range.into())],
                            self.consume_line(),
                        ]
                        .concat(),
                        Probe::Empty(range) => vec![],
                        Probe::Eof(_) => vec![],
                    }
                })
                .collect();

            return tokens;
        }

        if unmatched.is_empty() {
            if let Some(token) = self.consume_empty_line() {
                // Everything is empty so this is an empty continuation
                return vec![token];
            }
        }

        let mut t = vec![];

        // The line is not empty, so we should close all open
        // empty blocks before pushing the paragraph/plaintext blocks.
        // TODO: is there a more elegant place to put this?
        let range = self.end_idx..self.end_idx;
        dbg!(self.start_idx, self.end_idx);
        if let Some(Token::EmptyStart(_)) = unmatched.get(0) {
            t.push(Token::EmptyEnd((&range).into()));
            unmatched = (&unmatched[1..]).into();
        } else if let Some(Token::EmptyStart(_)) = self.open.get(0) {
            t.push(Token::EmptyEnd((&range).into()));
        }

        let range = self.start_idx..self.start_idx;
        let mut tokens = self.consume_line();
        if unmatched.is_empty() {
            // There are no unmatched blocks, so this is a paragraph
            let p = Token::ParagraphStart((&range).into());
            t.push(p);
            t.append(&mut tokens);
        } else {
            // Something is already open, so just append the plaintext
            t.append(&mut tokens);
        }

        t
    }

    fn match_probes(&mut self, probes: &[Probe]) -> (Vec<Probe>, Vec<Token>, usize, usize) {
        // First we look for open blocks and match the probes against
        // them. In order to remain open, a block needs to have an
        // appropriate continuation at the start of the line. For lists
        // any list item on the same level or greater matches.
        let mut unmatched = &self.open[..];
        let mut probes = &probes[..];
        let (mut max_ol, mut max_ul) = (0, 0);
        while !unmatched.is_empty() && !probes.is_empty() {
            let probe = &probes[0];
            let open = &unmatched[0];
            match (open, probe) {
                // TODO: Handle non-list probes that match the level of
                // indentation. Right now a list item can only have one
                // block child, but should support any number.
                // As long as there exists a list item of equal
                // or greater spacing, we can match the list
                (Token::UnorderedListStart((_, a)), Probe::Ul(..)) => max_ul = *a,
                (Token::OrderedListStart((_, a)), Probe::Ol(..)) => max_ol = *a,
                // However, list items must strictly increase in
                // spacing or we close them.
                (Token::ListItemStart((_, a)), Probe::Ul(_, b)) if a < b => (),
                (Token::ListItemStart((_, a)), Probe::Ol(_, b)) if a < b => (),
                // > can only be continued with another >
                (Token::BlockquoteStart(_), Probe::Blockquote(_)) => probes = &probes[1..],
                // Empy blocks are continued by another empty line
                (Token::EmptyStart(_), Probe::Empty(_)) => probes = &probes[1..],
                _ => {
                    // The first time any block unmatched, all
                    // remaining blocks are unmatched
                    break;
                }
            }
            unmatched = &unmatched[1..];
        }

        (probes.into(), unmatched.into(), max_ol, max_ul)
    }

    fn probe(&mut self) -> Vec<Probe> {
        let start_idx = self.start_idx;
        let line_start = start_idx;
        let mut probes = vec![];
        // Start probing for line indicators
        loop {
            if start_idx >= self.source.len() {
                // The Eof probe is a special case indicating the end of
                // the source and we should close all open blocks.
                probes.push(Probe::Eof(start_idx..start_idx));
                return probes;
            }

            match self.probe_block(line_start) {
                // Stop as soon as we find plaintext -- any remaining
                // tokens are parsed as part of the line.
                None => return probes,
                // Track empty lines as tokens so that we can render
                // them in the editor frontend.
                Some(Probe::Empty(range)) => {
                    probes.push(Probe::Empty(range));
                    return probes;
                }
                // Push the probed token onto the stack. We will use
                // the stack of tokens to decide which blocks to open/close.
                Some(probe) => probes.push(probe),
            }
        }
    }

    fn probe_block(&mut self, line_start: usize) -> Option<Probe> {
        let start_idx = self.start_idx;
        let source = &self.source[..];
        // Split the input source into three chunks: whitespace, token,
        // whitespace. The token may be a block start token.
        let (_, a) = Tokenizer::probe_whitespace(start_idx, source);
        let (token, b) = Tokenizer::probe_non_whitespace(a, source);
        let (ws, end_idx) = Tokenizer::probe_whitespace(b, source);

        // TODO: make sure there is whitespace after block starters
        // TODO: support any number for ordered lists
        // TODO: tabs are not the same width as spaces

        let width = end_idx - line_start;
        let range = start_idx..end_idx;
        let probe = match token {
            "-" | "+" | "*" if !ws.is_empty() => Probe::Ul(range, width),
            "1." | "1)" if !ws.is_empty() => Probe::Ol(range, width),
            "#" if !ws.is_empty() => Probe::Header(range, 1),
            "##" if !ws.is_empty() => Probe::Header(range, 2),
            "###" if !ws.is_empty() => Probe::Header(range, 3),
            "####" if !ws.is_empty() => Probe::Header(range, 4),
            "#####" if !ws.is_empty() => Probe::Header(range, 5),
            "######" if !ws.is_empty() => Probe::Header(range, 6),
            ">" => Probe::Blockquote(range),
            "" => Probe::Empty(range),
            // We did not consume a block token, so the remainder
            // of the line is just plaintext.
            _ => return None,
        };

        // Update the start pointer only if we found a probe
        self.start_idx = end_idx;
        Some(probe)
    }

    fn probe_whitespace(start_idx: usize, source: &str) -> (&str, usize) {
        let mut p = start_idx;
        while match &source.get(p..p + 1) {
            Some(" ") | Some("\t") => true,
            _ => false,
        } {
            p += 1;
        }
        (&source[start_idx..p], p)
    }

    fn probe_non_whitespace(start_idx: usize, source: &str) -> (&str, usize) {
        let mut p = start_idx;
        while match &source.get(p..p + 1) {
            None | Some(" ") | Some("\t") | Some("\n") => false,
            _ => true,
        } {
            p += 1;
        }
        (&source[start_idx..p], p)
    }
    fn consume_line(&mut self) -> Vec<Token> {
        let mut tokens = vec![];
        self.end_idx = self.start_idx;
        let mut state = ConsumeState::Plaintext;
        while match self.source.get(self.end_idx..self.end_idx + 1) {
            None => false,
            Some("\n") => false,
            _ => true,
        } {
            let range = self.start_idx..self.end_idx;
            let slice = &self.source[range.clone()];
            let char = &self.source[self.end_idx..self.end_idx + 1];
            match (char, state) {
                (" ", ConsumeState::Whitespace) | ("\t", ConsumeState::Whitespace) => (),
                (" ", ConsumeState::Plaintext) | ("\t", ConsumeState::Plaintext) => {
                    tokens.push(Token::Plaintext(((&range).into(), slice.into())));
                    self.start_idx = self.end_idx;
                    state = ConsumeState::Whitespace;
                }
                (_, ConsumeState::Whitespace) => {
                    tokens.push(Token::Plaintext(((&range).into(), slice.into())));
                    self.start_idx = self.end_idx;
                    state = ConsumeState::Plaintext;
                }
                (_, ConsumeState::Plaintext) => (),
            }

            self.end_idx += 1;
        }

        let range = self.start_idx..self.end_idx;
        let text = &self.source[range.clone()];
        tokens.push(Token::Plaintext(((&range).into(), text.into())));
        // Move the start index past the new line so we start on the
        // next line when we begin again. NB: the end index does
        // not change since we need a pointer to the end of the last line.
        self.start_idx = self.end_idx + 1;
        tokens
    }

    fn consume_empty_line(&mut self) -> Option<Token> {
        let p = self.start_idx;
        match self.source.get(p..p + 1) {
            Some("\n") => {
                self.end_idx = self.start_idx;
                self.start_idx = p + 1;
                Some(Token::Empty((&(p..p)).into()))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(start: usize, end: usize) -> Span {
        Span::new(start, end)
    }

    #[test]
    fn test_empty_source() {
        let source = "";
        let mut tokenizer = Tokenizer::new(source);
        let result = tokenizer.tokenize();

        assert_eq!(vec![Token::Eof(s(0, 0))], result);
    }

    #[test]
    fn test_empty_lines() {
        // TODO: trim empty lines from source?
        let source = "\n\nHello, World!\n\n";
        let mut tokenizer = Tokenizer::new(source);
        let result = tokenizer.tokenize();
        assert_eq!(
            vec![
                Token::EmptyStart(s(0, 0)),
                Token::Empty(s(0, 0)),
                Token::Empty(s(1, 1)),
                Token::EmptyEnd(s(1, 1)),
                Token::ParagraphStart(s(2, 2)),
                Token::Plaintext((s(2, 8), "Hello,".into())),
                Token::Plaintext((s(8, 9), " ".into())),
                Token::Plaintext((s(9, 15), "World!".into())),
                Token::ParagraphEnd(s(15, 15)),
                Token::EmptyStart(s(16, 16)),
                Token::Empty(s(16, 16)),
                Token::EmptyEnd(s(16, 16)),
                Token::Eof(s(17, 17))
            ],
            result
        );
    }

    #[test]
    fn test_single_paragraph() {
        let source = "Hello, World!";
        let mut tokenizer = Tokenizer::new(source);
        let result = tokenizer.tokenize();
        assert_eq!(
            vec![
                Token::ParagraphStart(s(0, 0)),
                Token::Plaintext((s(0, 6), "Hello,".into())),
                Token::Plaintext((s(6, 7), " ".into())),
                Token::Plaintext((s(7, 13), "World!".into())),
                Token::ParagraphEnd(s(13, 13)),
                Token::Eof(s(14, 14))
            ],
            result
        );
    }

    #[test]
    fn test_header_paragraph() {
        let source = "# Title\nHello, World!";
        let mut tokenizer = Tokenizer::new(source);
        let result = tokenizer.tokenize();
        assert_eq!(
            vec![
                Token::Header((s(0, 2), 1)),
                Token::Plaintext((s(2, 7), "Title".into())),
                Token::ParagraphStart(s(8, 8)),
                Token::Plaintext((s(8, 14), "Hello,".into())),
                Token::Plaintext((s(14, 15), " ".into())),
                Token::Plaintext((s(15, 21), "World!".into())),
                Token::ParagraphEnd(s(21, 21)),
                Token::Eof(s(22, 22))
            ],
            result
        );
    }

    #[test]
    fn test_nested_lists() {
        let source = "- One\n- Two\n  - Three\n- Four";
        let mut tokenizer = Tokenizer::new(source);
        let result = tokenizer.tokenize();
        assert_eq!(
            vec![
                Token::UnorderedListStart((s(0, 2), 2)),
                Token::ListItemStart((s(0, 2), 2)),
                Token::ParagraphStart(s(0, 2)),
                Token::Plaintext((s(2, 5), "One".into())),
                Token::ParagraphEnd(s(5, 5)),
                Token::ListItemEnd(s(5, 5)),
                Token::ListItemStart((s(6, 8), 2)),
                Token::ParagraphStart(s(6, 8)),
                Token::Plaintext((s(8, 11), "Two".into())),
                Token::ParagraphEnd(s(11, 11)),
                Token::UnorderedListStart((s(12, 16), 4)),
                Token::ListItemStart((s(12, 16), 4)),
                Token::ParagraphStart(s(12, 16)),
                Token::Plaintext((s(16, 21), "Three".into())),
                Token::ParagraphEnd(s(21, 21)),
                Token::ListItemEnd(s(21, 21)),
                Token::UnorderedListEnd(s(21, 21)),
                Token::ListItemEnd(s(21, 21)),
                Token::ListItemStart((s(22, 24), 2)),
                Token::ParagraphStart(s(22, 24)),
                Token::Plaintext((s(24, 28), "Four".into())),
                Token::ParagraphEnd(s(28, 28)),
                Token::ListItemEnd(s(28, 28)),
                Token::UnorderedListEnd(s(28, 28)),
                Token::Eof(s(29, 29))
            ],
            result
        );
    }

    #[test]
    fn test_blockquote_list() {
        let source = [
            "> Lorem ipsum dolor",
            "sit amet.",
            "> - Qui *quodsi iracundia*",
            "> - aliquando id",
        ]
        .join("\n");
        let mut tokenizer = Tokenizer::new(&source[..]);
        let result = tokenizer.tokenize();
        assert_eq!(
            vec![
                Token::BlockquoteStart(s(0, 2)),
                Token::ParagraphStart(s(0, 2)),
                Token::Plaintext((s(2, 7), "Lorem".into())),
                Token::Plaintext((s(7, 8), " ".into())),
                Token::Plaintext((s(8, 13), "ipsum".into())),
                Token::Plaintext((s(13, 14), " ".into())),
                Token::Plaintext((s(14, 19), "dolor".into())),
                Token::Plaintext((s(20, 23), "sit".into())),
                Token::Plaintext((s(23, 24), " ".into())),
                Token::Plaintext((s(24, 29), "amet.".into())),
                Token::ParagraphEnd(s(29, 29)),
                Token::UnorderedListStart((s(32, 34), 4)),
                Token::ListItemStart((s(32, 34), 4)),
                Token::ParagraphStart(s(32, 34)),
                Token::Plaintext((s(34, 37), "Qui".into())),
                Token::Plaintext((s(37, 38), " ".into())),
                Token::Plaintext((s(38, 45), "*quodsi".into())),
                Token::Plaintext((s(45, 46), " ".into())),
                Token::Plaintext((s(46, 56), "iracundia*".into())),
                Token::ParagraphEnd(s(56, 56)),
                Token::ListItemEnd(s(56, 56)),
                Token::ListItemStart((s(59, 61), 4)),
                Token::ParagraphStart(s(59, 61)),
                Token::Plaintext((s(61, 70), "aliquando".into())),
                Token::Plaintext((s(70, 71), " ".into())),
                Token::Plaintext((s(71, 73), "id".into())),
                Token::ParagraphEnd(s(73, 73)),
                Token::ListItemEnd(s(73, 73)),
                Token::UnorderedListEnd(s(73, 73)),
                Token::BlockquoteEnd(s(73, 73)),
                Token::Eof(s(74, 74))
            ],
            result
        );
    }
}
