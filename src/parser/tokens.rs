use super::source_location::SourceLocation;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TokenValue {
    // LITERALS
    Number(String),
    String(String),
    Identifier(String),
    
    // KEYWORDS
    Print,
    Fn,
    Return,
    
    // SYMBOLS
    Semicolon,
    Minus,
    Plus,
    Star,
    ForwardSlash,
    Equal,
    OpenParen,
    CloseParen,
    OpenBracket,
    CloseBracket,
    Arrow,
    EndOfFile
}

#[derive(Clone)]
pub struct Token {
    pub value: TokenValue,
    pub source_location: SourceLocation,
}

