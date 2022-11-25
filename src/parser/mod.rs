mod syntax_parser;
mod lexer;
mod source_location;
mod tokens;

pub use source_location::SourceLocation;

type PResult<T> = Result<T, (SourceLocation, String)>;

pub fn parse_file(code: &str) -> PResult<crate::ast::CodeBody> {
    let mut lexer = lexer::Lexer::new(code);
    let tokens = lexer.parse_file()?;

    let mut parser = syntax_parser::SyntaxParser::new(tokens);
    parser.parse_file()
}