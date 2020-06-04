#![feature(test)]

mod markdown;
mod utils;

use wasm_bindgen::prelude::*;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

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
        markdown::parse(source)
    }
}
