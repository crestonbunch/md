mod parse;
mod parse2;
mod render;

// pub use parse::parse;
// pub use parse::{Kind, Node, OrderedList, UnorderedList};
pub use parse2::md_parser;
pub use parse2::{Kind, Node};

pub fn parse(source: &str) -> Node {
    parse2::parse(source)
}

pub use render::json;
