mod parse;
mod render;

pub use parse::md_parser;
pub use parse::{Kind, Node};

pub fn parse(source: &str) -> Node {
    parse::parse(source)
}

pub use render::json;
