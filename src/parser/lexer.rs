use std::collections::VecDeque;

use super::source_location::SourceLocation;
use super::tokens::{Token, TokenValue};

pub struct Lexer {
    code: VecDeque<char>,

    current_line: usize,
    current_char_on_line: usize,

    tokens: Vec<Token>,
}

impl Lexer {
    fn new(code: &str) -> Self {
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
        P: FnMut(char) -> bool,
    {
        while self.peek().map(predicate).unwrap_or(false) {
            self.advance();
        }
    }

    fn take_while<P>(&mut self, predicate: P) -> String
    where
        P: FnMut(char) -> bool,
    {
        let chars = Vec::new();
        while self.peek().map(predicate).unwrap_or(false) {
            chars.push(self.advance().unwrap());
        }

        chars.into_iter().collect()
    }

    fn take_until_whitespace(&mut self) -> String {
        self.take_while(|c| !c.is_whitespace())
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

    pub fn parse_file(&mut self) -> Result<Vec<Token>, (String, SourceLocation)> {
        self.eat_whitespace();
        let error = Ok(());
        while let Some(char) = self.advance() {
            match char {
                ';' => self.emit_token(1, TokenValue::Semicolon),
                '+' => self.emit_token(1, TokenValue::Plus),
                '-' => self.emit_token(1, TokenValue::Minus),
                '*' => self.emit_token(1, TokenValue::Star),
                '/' => self.emit_token(1, TokenValue::FSlash),
                '=' => self.emit_token(1, TokenValue::Eq),
                char if char.is_digit(10) => {
                    let digits = char.to_string() + &self.take_while(|c| c.is_digit(10));
                    self.emit_token(digits.len(), TokenValue::Number(digits));
                }
                '"' => {
                    let string_content = self.take_while(|c| c != '"' && c != '\n');
                    let end = self.advance();
                    if end != Some('"') {
                        error = Err((
                            "Unclosed String".to_string(),
                            SourceLocation::new(
                                self.current_line,
                                self.current_char_on_line,
                                self.current_char_on_line,
                            ),
                        ));
                        break;
                    }
                }
                char if char.is_alphabetic() => {
                    let word = char.to_string() + &self.take_while(char::is_alphabetic);
                    match word.as_str() {
                        "print" => self.emit_token(5, TokenValue::Print),
                        _ => self.emit_token(word.len(), TokenValue::Identifier(word)),
                    }
                }
                _ => {
                    error = Err((
                        format!("invalid char {char}"),
                        SourceLocation::new(
                            self.current_line,
                            self.current_char_on_line,
                            self.current_char_on_line,
                        ),
                    ));
                    break;
                }
            }

            self.eat_whitespace();
        }

        self.emit_token(1, TokenValue::EOF);

        error.map(|_| self.tokens)
    }
}
