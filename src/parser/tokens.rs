use super::source_location::SourceLocation;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TokenValue {
    // LITERALS
    Number(String),
    String(String),
    Identifier(String),
    True,
    False,
    
    // KEYWORDS
    Print,
    Assert,
    Test,
    Is,
    
    // SYMBOLS
    Semicolon,
    Minus,
    Plus,
    Star,
    ForwardSlash,
    Comma,

    Equal,
    EqualEqual,
    BangEqual,
    LessThan,
    LessThanEqual,
    GreaterThan,
    GreaterThanEqual,

    OpenParen,
    CloseParen,
    OpenBracket,
    CloseBracket,

    Arrow,
    Fn,
    Return,

    EndOfFile
}

#[derive(Clone)]
pub struct Token {
    pub value: TokenValue,
    pub source_location: SourceLocation,
}

