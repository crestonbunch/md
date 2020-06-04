pub mod parse;

pub fn parse(source: &str) -> String {
    source.into()
    /*
    let mut tokenizer = Tokenizer::new(source);
    let tokens = tokenizer.tokenize();

    let mut parser = Parser::new();

    for token in tokens {
        parser.parse(token).unwrap();
    }

    let result = parser.end_of_input().unwrap();

    // TODO: use native structs to avoid serialization?
    serde_json::to_string(&result).unwrap()
    */
}
