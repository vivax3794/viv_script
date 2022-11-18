#![feature(assert_matches)]
#![feature(box_patterns)]

mod parser;
mod llvm_generator;
mod ast;

pub use parser::code_block;
pub use llvm_generator::Compiler;