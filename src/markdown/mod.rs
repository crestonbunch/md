pub mod ast;
pub mod parse;
pub mod token;

use parse::Parser;
use token::Tokenizer;

pub fn parse(tokenizer: &Tokenizer, source: &str) -> String {
    let tokens = tokenizer.tokenize(source);

    let mut parser = Parser::new();

    // TODO: only consume and parse one token at a time, so we
    // make one pass over the input instead of two

    // for token in tokens {
    // parser.parse(token).unwrap();
    // }

    let result = parser.end_of_input().unwrap();

    serde_json::to_string(&result).unwrap()
}
