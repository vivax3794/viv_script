mod syntax_parser;
mod lexer;
mod source_location;
mod tokens;

pub use source_location::SourceLocation;

use crate::CompilerResult;

pub fn parse_file(code: &str) -> CompilerResult<crate::ast::File> {
    let mut lexer = lexer::Lexer::new(code);
    let tokens = lexer.parse_file()?;

    let mut parser = syntax_parser::SyntaxParser::new(tokens);
    parser.parse_file()
}