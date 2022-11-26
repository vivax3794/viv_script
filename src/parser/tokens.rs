use super::source_location::SourceLocation;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TokenValue {
    // LITERALS
    Number(String),
    String(String),
    Identifier(String),
    
    // KEYWORDS
    Print,
    
    // SYMBOLS
    Semicolon,
    Minus,
    Plus,
    Star,
    FSlash,
    Eq,
    OpenParen,
    CloseParen,
    Eof
}

#[derive(Clone)]
pub struct Token {
    pub value: TokenValue,
    pub source_location: SourceLocation,
}

