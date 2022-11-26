use std::collections::VecDeque;

use super::source_location::SourceLocation;
use super::tokens::{Token, TokenValue};
use crate::CompilerResult;

pub struct Lexer {
    code: VecDeque<char>,

    current_line: usize,
    current_char_on_line: usize,

    tokens: Vec<Token>,
}

impl Lexer {
    pub fn new(code: &str) -> Self {
        Self {
            code: code.chars().collect(),
            current_line: 1,
            current_char_on_line: 0,
            tokens: Vec::with_capacity(code.len() / 2),
        }
    }

    fn advance(&mut self) -> Option<char> {
        let char = self.code.pop_front();

        self.current_char_on_line += 1;

        if let Some(c) = char {
            if c == '\n' {
                self.current_line += 1;
                self.current_char_on_line = 0;
            }
        }

        char
    }

    fn peek(&self) -> Option<char> {
        self.code.get(0).copied()
    }

    fn advance_until<P>(&mut self, predicate: P)
    where
        P: Fn(char) -> bool,
    {
        while self.peek().map(&predicate).unwrap_or(false) {
            self.advance();
        }
    }

    fn take_while<P>(&mut self, predicate: P) -> String
    where
        P: Fn(char) -> bool,
    {
        let mut chars = Vec::new();
        while self.peek().map(&predicate).unwrap_or(false) {
            chars.push(self.advance().unwrap());
        }

        chars.into_iter().collect()
    }

    fn eat_whitespace(&mut self) {
        self.advance_until(char::is_whitespace);
    }

    fn emit_token(&mut self, len: usize, value: TokenValue) {
        let location = SourceLocation::new(
            self.current_line,
            self.current_char_on_line - len + 1,
            self.current_char_on_line,
        );
        self.tokens.push(Token {
            value,
            source_location: location,
        });
    }

    pub fn parse_file(&mut self) -> CompilerResult<Vec<Token>> {
        self.eat_whitespace();
        let mut error = Ok(());
        while let Some(char) = self.advance() {
            match char {
                ';' => self.emit_token(1, TokenValue::Semicolon),
                '+' => self.emit_token(1, TokenValue::Plus),
                '-' => self.emit_token(1, TokenValue::Minus),
                '*' => self.emit_token(1, TokenValue::Star),
                '/' => self.emit_token(1, TokenValue::FSlash),
                '=' => self.emit_token(1, TokenValue::Eq),
                '(' => self.emit_token(1, TokenValue::OpenParen),
                ')' => self.emit_token(1, TokenValue::CloseParen),
                char if char.is_ascii_digit() => {
                    let digits = char.to_string() + &self.take_while(|c| c.is_ascii_digit());
                    self.emit_token(digits.len(), TokenValue::Number(digits));
                }
                '"' => {
                    let string_content = self.take_while(|c| c != '"' && c != '\n');
                    let end = self.advance();
                    if end != Some('"') {
                        error = Err((
                            SourceLocation::new(
                                self.current_line,
                                self.current_char_on_line,
                                self.current_char_on_line,
                            ),
                            "Unclosed String".to_string(),
                        ));
                        break;
                    }

                    self.emit_token(string_content.len() + 2, TokenValue::String(string_content));
                }
                char if char.is_alphabetic() || char == '_' => {
                    let word = char.to_string() + &self.take_while(|c| c.is_alphabetic() || c == '_');
                    match word.as_str() {
                        "print" => self.emit_token(5, TokenValue::Print),
                        _ => self.emit_token(word.len(), TokenValue::Identifier(word)),
                    }
                }
                _ => {
                    error = Err((
                        SourceLocation::new(
                            self.current_line,
                            self.current_char_on_line,
                            self.current_char_on_line,
                        ),
                        format!("invalid char {char}"),
                    ));
                    break;
                }
            }

            self.eat_whitespace();
        }

        self.emit_token(1, TokenValue::Eof);

        error.map(|_| self.tokens.clone())
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::tokens::TokenValue;

    use super::Lexer;

    fn parse_file(lex: &mut Lexer) -> Vec<TokenValue> {
        lex.parse_file()
            .unwrap()
            .into_iter()
            .map(|t| t.value)
            .collect()
    }

    #[test]
    fn symbols() {
        let mut lexer = Lexer::new("    +  -  * / =  ; ");
        let tokens = parse_file(&mut lexer);

        assert_eq!(
            tokens,
            vec![
                TokenValue::Plus,
                TokenValue::Minus,
                TokenValue::Star,
                TokenValue::FSlash,
                TokenValue::Eq,
                TokenValue::Semicolon,
                TokenValue::Eof
            ]
        )
    }

    #[test]
    fn digit() {
        let mut lexer = Lexer::new("1234");
        let tokens = parse_file(&mut lexer);

        assert_eq!(
            tokens,
            vec![TokenValue::Number("1234".to_string()), TokenValue::Eof]
        )
    }

    #[test]
    fn string() {
        let mut lexer = Lexer::new("\"hello\"");
        let tokens = parse_file(&mut lexer);

        assert_eq!(
            tokens,
            vec![TokenValue::String("hello".to_string()), TokenValue::Eof]
        )
    }

    #[test]
    fn keyword() {
        let mut lexer = Lexer::new("print");
        let tokens = parse_file(&mut lexer);

        assert_eq!(tokens, vec![TokenValue::Print, TokenValue::Eof])
    }

    #[test]
    fn identifier() {
        let mut lexer = Lexer::new("hello");
        let tokens = parse_file(&mut lexer);

        assert_eq!(
            tokens,
            vec![TokenValue::Identifier("hello".to_string()), TokenValue::Eof]
        )
    }
}
