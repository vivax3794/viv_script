use std::collections::VecDeque;

use super::{
    tokens::{Token, TokenValue},
    SourceLocation,
};
use crate::{ast, CompilerResult};

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

    fn expect(&mut self, token: TokenValue) -> CompilerResult<()> {
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

    fn parse_literal(&mut self) -> CompilerResult<ast::Expression> {
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
            // Lets just special case this since this is a convenient place to parse this
            TokenValue::Identifier(name) => {
                return Ok(ast::Expression::Var(token.source_location.into(), name))
            }
            value => {
                return Err((
                    token.source_location,
                    format!("Expected String(_), Number(_) or Minus got {:?}", value),
                ))
            }
        };

        Ok(ast::Expression::Literal(token.source_location.into(), lit))
    }

    fn parse_group(&mut self) -> CompilerResult<ast::Expression> {
        match self.peek() {
            TokenValue::OpenParen => {
                self.advance();
                let exp = self.parse_expression()?;
                self.expect(TokenValue::CloseParen)?;
                Ok(exp)
            }
            _ => self.parse_literal(),
        }
    }

    fn parse_binary_expression(&mut self, level: usize) -> CompilerResult<ast::Expression> {
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
                    self.advance();
                    let right = self.parse_binary_expression(level + 1)?;
                    left = ast::Expression::Binary(
                        SourceLocation::combine(left.location(), right.location()).into(),
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

    fn parse_expression(&mut self) -> CompilerResult<ast::Expression> {
        self.parse_binary_expression(0)
    }

    fn parse_print(&mut self) -> CompilerResult<ast::Statement> {
        self.advance(); // we assume this is only called once we know we have a print
        let expr = self.parse_expression()?;
        self.expect(TokenValue::Semicolon)?;
        Ok(ast::Statement::Print(expr))
    }

    fn parse_assignment(&mut self) -> CompilerResult<ast::Statement> {
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

    fn parse_return(&mut self) -> CompilerResult<ast::Statement> {
        self.advance();
        let expr = self.parse_expression()?;
        self.expect(TokenValue::Semicolon)?;
        Ok(ast::Statement::Return(expr))
    }

    fn parse_statement(&mut self) -> CompilerResult<Option<ast::Statement>> {
        match self.peek() {
            TokenValue::Print => self.parse_print().map(Some),
            TokenValue::Identifier(_) => self.parse_assignment().map(Some),
            TokenValue::Return => self.parse_return().map(Some),
            _ => Ok(None),
        }
    }

    fn parse_codeblock(&mut self) -> CompilerResult<ast::CodeBody> {
        self.expect(TokenValue::OpenBracket)?;

        let mut statements = Vec::new();
        while let Some(stmt) = self.parse_statement()? {
            statements.push(stmt);
        }

        self.expect(TokenValue::CloseBracket)?;
        Ok(ast::CodeBody(statements))
    }

    fn parse_function_definition(&mut self) -> CompilerResult<ast::TopLevelStatement> {
        self.expect(TokenValue::Fn)?;

        let name = self.advance();
        let name = match name.value {
            TokenValue::Identifier(name) => name,
            _ => {
                return Err((
                    name.source_location,
                    format!("expected name, got {:?}", name.value),
                ))
            }
        };

        self.expect(TokenValue::OpenParen)?;
        self.expect(TokenValue::CloseParen)?;

        self.expect(TokenValue::Arrow)?;

        let return_type_tk = self.advance();
        let return_type = match return_type_tk.value {
            TokenValue::Identifier(name) => name,
            _ => {
                return Err((
                    return_type_tk.source_location,
                    format!("expected name, got {:?}", return_type_tk.value),
                ))
            }
        };

        let block = self.parse_codeblock()?;

        Ok(ast::TopLevelStatement::FunctionDefinition {
            name,
            body: block,
            return_type,
            return_type_location: return_type_tk.source_location,
            meta: Default::default(),
        })
    }

    fn parse_toplevel_statement(&mut self) -> CompilerResult<Option<ast::TopLevelStatement>> {
        match self.peek() {
            TokenValue::Fn => self.parse_function_definition().map(Some),
            TokenValue::Eof => Ok(None),
            _ => {
                let tk = self.advance();
                Err((
                    tk.source_location,
                    format!(
                        "expected start of top level definition or end of file, got {:?}",
                        tk.value
                    ),
                ))
            }
        }
    }

    pub fn parse_file(&mut self) -> CompilerResult<ast::File> {
        let mut statements = Vec::new();

        while let Some(stmt) = self.parse_toplevel_statement()? {
            statements.push(stmt);
        }

        Ok(ast::File(statements))
    }
}
