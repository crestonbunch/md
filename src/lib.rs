#![feature(test)]
#![feature(or_patterns)]

mod markdown;
mod utils;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Compiler {}

impl Default for Compiler {
    fn default() -> Self {
        Compiler::new()
    }
}

#[wasm_bindgen]
impl Compiler {
    pub fn new() -> Self {
        utils::set_panic_hook();
        Compiler {}
    }

    pub fn compile(&self, source: &str) -> String {
        let doc = markdown::parse(source);
        markdown::json::render(source, doc)
    }
}
