use std::collections::VecDeque;

use super::source_location::SourceLocation;
use super::tokens::{Token, TokenValue};
use crate::CompilerResult;

pub struct Lexer {
    code: VecDeque<char>,

    current_line: usize,
    current_colum: usize,

    tokens: Vec<Token>,
}

impl Lexer {
    pub fn new(code: &str) -> Self {
        Self {
            code: code.chars().collect(),
            current_line: 1,
            current_colum: 0,
            tokens: Vec::with_capacity(code.len() / 2),
        }
    }

    fn advance(&mut self) -> Option<char> {
        let char = self.code.pop_front();

        self.current_colum += 1;

        if let Some(c) = char {
            if c == '\n' {
                self.current_line += 1;
                self.current_colum = 0;
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
        while self.peek().map_or(false, &predicate) {
            self.advance();
        }
    }

    fn take_while<P>(&mut self, predicate: P) -> String
    where
        P: Fn(char) -> bool,
    {
        let mut chars = Vec::new();
        while self.peek().map_or(false, &predicate){
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
            self.current_colum - len + 1,
            self.current_colum,
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
                '-' => {
                    match self.peek() {
                        Some('>') => {
                            self.advance();
                            self.emit_token(2, TokenValue::Arrow);
                        },
                        _ => self.emit_token(1, TokenValue::Minus),
                    }
                },
                '*' => self.emit_token(1, TokenValue::Star),
                '/' => {
                    match self.peek() {
                        Some('/') => {
                            // comments
                            self.advance();
                            self.take_while(|c| c != '\n');
                        },
                        _ => self.emit_token(1, TokenValue::ForwardSlash),
                    }
                },
                '=' => {
                    match self.peek() {
                        Some('=') => {
                            self.advance();
                            self.emit_token(2, TokenValue::EqualEqual);
                        },
                        _ => self.emit_token(1, TokenValue::Equal)
                    }
                },
                ',' => self.emit_token(1, TokenValue::Comma),
                '(' => self.emit_token(1, TokenValue::OpenParen),
                ')' => self.emit_token(1, TokenValue::CloseParen),
                '{' => self.emit_token(1, TokenValue::OpenBracket),
                '}' => self.emit_token(1, TokenValue::CloseBracket),
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
                                self.current_colum,
                                self.current_colum,
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
                        "assert" => self.emit_token(6, TokenValue::Assert),
                        "fn" =>  self.emit_token(2, TokenValue::Fn),
                        "return" => self.emit_token(6, TokenValue::Return),
                        "true" => self.emit_token(4, TokenValue::True),
                        "false" => self.emit_token(5, TokenValue::False),
                        "test" => self.emit_token(4, TokenValue::Test),
                        "is" => self.emit_token(2, TokenValue::Is),
                        _ => self.emit_token(word.len(), TokenValue::Identifier(word)),
                    }
                }
                _ => {
                    error = Err((
                        SourceLocation::new(
                            self.current_line,
                            self.current_colum,
                            self.current_colum,
                        ),
                        format!("invalid char {char}"),
                    ));
                    break;
                }
            }

            self.eat_whitespace();
        }

        if error.is_ok() {
            self.emit_token(1, TokenValue::EndOfFile);
        }

        error.map(|_| self.tokens.clone())
    }
}
