use std::collections::VecDeque;

use super::{
    tokens::{Token, TokenValue},
    SourceLocation,
    PResult
};
use crate::ast;


pub struct SyntaxParser {
    tokens: VecDeque<Token>,
}

impl SyntaxParser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens: tokens.into_iter().collect(),
        }
    }

    fn advance(&mut self) -> Token {
        self.tokens.pop_front().unwrap()
    }

    fn peek(&self) -> TokenValue {
        self.tokens[0].clone().value
    }

    fn expect(&mut self, token: TokenValue) -> PResult<()> {
        let tk = self.advance();
        if tk.value != token {
            Err((
                tk.source_location,
                format!("expected {:?} found {:?}", token, tk.value),
            ))
        } else {
            Ok(())
        }
    }

    fn parse_literal(&mut self) -> PResult<ast::Expression> {
        let token = self.advance();
        let lit = match token.value {
            TokenValue::String(content) => ast::LiteralType::String(content),
            TokenValue::Number(digits) => ast::LiteralType::Number(digits.parse().unwrap()),
            TokenValue::Minus => {
                let digits = self.advance();
                match digits.value {
                    TokenValue::Number(digits) => {
                        ast::LiteralType::Number(-digits.parse::<i32>().unwrap())
                    }
                    _ => {
                        return Err((
                            digits.source_location,
                            format!("Expected Number(_) got {:?}", digits.value),
                        ))
                    }
                }
            }
            value => {
                return Err((
                    token.source_location,
                    format!("Expected String(_), Number(_) or Sign got {:?}", value),
                ))
            }
        };

        Ok(ast::Expression::Literal(token.source_location, lit))
    }

    fn parse_group(&mut self) -> PResult<ast::Expression> {
        match self.peek() {
            TokenValue::OpenParen => {
                self.advance();
                let exp = self.parse_expression()?;
                let closing = self.advance();
                self.expect(TokenValue::CloseParen)?;
                Ok(exp)
            }
            _ => self.parse_literal(),
        }
    }

    fn parse_binary_expression(&mut self, level: usize) -> PResult<ast::Expression> {
        let expressions: Vec<Vec<(TokenValue, ast::Operator)>> = vec![
            vec![
                (TokenValue::Plus, ast::Operator::Add),
                (TokenValue::Minus, ast::Operator::Sub),
            ],
            vec![
                (TokenValue::Star, ast::Operator::Mul),
                (TokenValue::FSlash, ast::Operator::Div),
            ],
        ];

        if level >= expressions.len() {
            return self.parse_group();
        }

        let mut left = self.parse_binary_expression(level + 1)?;
        'expr: loop {
            let next = self.peek();
            for (token, op) in expressions[level].iter() {
                if &next == token {
                    let right = self.parse_binary_expression(level)?;
                    left = ast::Expression::Binary(
                        SourceLocation::combine(left.location(), right.location()),
                        Box::new(left),
                        *op,
                        Box::new(right),
                    );
                    continue 'expr;
                }
            }

            // We didn't find a operator
            break;
        }

        Ok(left)
    }

    fn parse_expression(&mut self) -> PResult<ast::Expression> {
        self.parse_binary_expression(0)
    }

    fn parse_print(&mut self) -> PResult<ast::Statement> {
        self.advance(); // we assume this is only called once we know we have a print
        let expr = self.parse_expression()?;
        self.expect(TokenValue::Semicolon)?;
        Ok(ast::Statement::Print(expr))
    }

    fn parse_assignment(&mut self) -> PResult<ast::Statement> {
        let name_tk = self.advance();
        let name = match name_tk.value {
            TokenValue::Identifier(name) => name,
            _ => unreachable!("Should have been confirmed before calling!"),
        };

        self.expect(TokenValue::Eq)?;
        let expr = self.parse_expression()?;
        self.expect(TokenValue::Semicolon)?;

        Ok(ast::Statement::Assignment(
            name_tk.source_location,
            name,
            expr,
        ))
    }

    fn parse_statement(&mut self) -> PResult<Option<ast::Statement>> {
        match self.peek() {
            TokenValue::Print => self.parse_print().map(Some),
            TokenValue::Identifier(_) => self.parse_assignment().map(Some),
            TokenValue::EOF => Ok(None),
            _ => {
                let token = self.advance();
                Err((
                    token.source_location,
                    format!("Expected start of statement or EOF, got {:?}", token.value),
                ))
            }
        }
    }

    pub fn parse_file(&mut self) -> PResult<ast::CodeBody> {
        let mut statements = Vec::new();

        while let Some(stmt) = self.parse_statement()? {
            statements.push(stmt);
        }

        Ok(ast::CodeBody(statements))
    }
}
