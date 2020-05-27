pub mod ast;
pub mod parse;
pub mod token;

use parse::Parser;
use token::Tokenizer;

pub fn parse(source: &str) -> String {
    let tokens = Tokenizer::new(source);

    let mut parser = Parser::new();

    for token in tokens {
        parser.parse(token).unwrap();
    }

    let result = parser.end_of_input().unwrap();

    // TODO: use native structs to avoid serialization?
    serde_json::to_string(&result).unwrap()
}
