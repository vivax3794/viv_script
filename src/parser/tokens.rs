use super::source_location::SourceLocation;

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
    EOF
}

pub struct Token {
    pub value: TokenValue,
    pub source_location: SourceLocation,
}

