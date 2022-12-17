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

    fn expect(&mut self, expected_token: &TokenValue) -> CompilerResult<()> {
        let token = self.advance();
        if token.value == *expected_token {
            Ok(())
        } else {
            Err((
                token.source_location,
                format!("expected {:?} found {:?}", expected_token, token.value),
            ))
        }
    }

    fn parse_literal(&mut self) -> CompilerResult<ast::Expression> {
        let token = self.advance();
        let literal = match token.value {
            TokenValue::String(content) => ast::LiteralType::String(content),
            TokenValue::Number(digits) => ast::LiteralType::Number(digits.parse().unwrap()),
            TokenValue::False => ast::LiteralType::Boolean(false),
            TokenValue::True => ast::LiteralType::Boolean(true),
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

        Ok(ast::Expression::Literal(
            token.source_location.into(),
            literal,
        ))
    }

    fn parse_group(&mut self) -> CompilerResult<ast::Expression> {
        match self.peek() {
            TokenValue::OpenParen => {
                self.advance();
                let expression = self.parse_expression()?;
                self.expect(&TokenValue::CloseParen)?;
                Ok(expression)
            }
            _ => self.parse_literal(),
        }
    }

    fn parse_prefix(&mut self) -> CompilerResult<ast::Expression> {
        let op = match self.peek() {
            TokenValue::Bang => ast::PrefixOprator::Not,
            _ => return self.parse_group(),
        };
        let location = self.advance().source_location;

        let expr = self.parse_prefix()?;

        Ok(ast::Expression::PrefixExpression {
            op,
            expression: Box::new(expr),
            metadata: ast::ExpressionMetadata::from(location),
        })
    }

    fn parse_binary_expression(&mut self, level: usize) -> CompilerResult<ast::Expression> {
        let operator_precedence_levels: Vec<Vec<(TokenValue, ast::Operator)>> = vec![
            vec![
                (TokenValue::Plus, ast::Operator::Add),
                (TokenValue::Minus, ast::Operator::Sub),
            ],
            vec![
                (TokenValue::Star, ast::Operator::Mul),
                (TokenValue::ForwardSlash, ast::Operator::Div),
            ],
        ];

        if level >= operator_precedence_levels.len() {
            return self.parse_prefix();
        }

        let mut left_expression = self.parse_binary_expression(level + 1)?;
        'expr: loop {
            let next_token = self.peek();
            for (operator_token, operator) in &operator_precedence_levels[level] {
                if &next_token == operator_token {
                    self.advance();
                    let right_expression = self.parse_binary_expression(level + 1)?;
                    left_expression = ast::Expression::Binary {
                        metadata: SourceLocation::combine(
                            left_expression.location(),
                            right_expression.location(),
                        )
                        .into(),
                        left: Box::new(left_expression),
                        operator: *operator,
                        right: Box::new(right_expression),
                    };
                    continue 'expr;
                }
            }

            // We didn't find a operator
            break;
        }

        Ok(left_expression)
    }

    fn parse_comparison(&mut self) -> CompilerResult<ast::Expression> {
        let first = self.parse_binary_expression(0)?;
        let mut chains = Vec::new();

        loop {
            let comp = match self.peek() {
                TokenValue::EqualEqual => ast::Comparison::Equal,
                TokenValue::BangEqual => ast::Comparison::NotEqual,
                TokenValue::LessThan => ast::Comparison::LessThan,
                TokenValue::LessThanEqual => ast::Comparison::LessThanEqual,
                TokenValue::GreaterThan => ast::Comparison::GreaterThan,
                TokenValue::GreaterThanEqual => ast::Comparison::GreaterThanEqual,
                _ => break,
            };
            self.advance();
            let right = self.parse_binary_expression(0)?;

            chains.push((comp, right));
        }

        if chains.is_empty() {
            Ok(first)
        } else {
            let location = chains
                .iter()
                .map(|(_, expr)| *expr.location())
                .fold(*first.location(), |a, b| SourceLocation::combine(&a, &b));
            Ok(ast::Expression::ComparisonChain {
                first_element: Box::new(first),
                comparisons: chains,
                metadata: ast::ExpressionMetadata::from(location),
            })
        }
    }

    fn parse_expression(&mut self) -> CompilerResult<ast::Expression> {
        self.parse_comparison()
    }

    fn parse_print(&mut self) -> CompilerResult<ast::Statement> {
        self.advance(); // we assume this is only called once we know we have a print
        let expression = self.parse_expression()?;
        self.expect(&TokenValue::Semicolon)?;
        Ok(ast::Statement::Print(expression))
    }

    fn parse_assignment(&mut self) -> CompilerResult<ast::Statement> {
        let name_token = self.advance();
        let var_name = match name_token.value {
            TokenValue::Identifier(name) => name,
            _ => unreachable!("Should have been confirmed before calling!"),
        };

        self.expect(&TokenValue::Equal)?;
        let expression = self.parse_expression()?;
        self.expect(&TokenValue::Semicolon)?;

        Ok(ast::Statement::Assignment {
            expression_location: name_token.source_location,
            var_name,
            expression,
        })
    }

    fn parse_return(&mut self) -> CompilerResult<ast::Statement> {
        self.advance();
        let expression = self.parse_expression()?;
        self.expect(&TokenValue::Semicolon)?;
        Ok(ast::Statement::Return(expression))
    }

    fn parse_assert(&mut self) -> CompilerResult<ast::Statement> {
        self.advance();
        let expression = self.parse_expression()?;
        self.expect(&TokenValue::Semicolon)?;
        Ok(ast::Statement::Assert(expression))
    }

    fn parse_test(&mut self) -> CompilerResult<ast::Statement> {
        self.advance();
        let name = self.advance();
        let name = match name.value {
            TokenValue::String(value) => value,
            _ => {
                return Err((
                    name.source_location,
                    "Expected String for name of test.".to_string(),
                ))
            }
        };

        self.expect(&TokenValue::Arrow)?;
        let left = self.parse_expression()?;
        self.expect(&TokenValue::Semicolon)?;

        Ok(ast::Statement::Test(name, left))
    }

    fn parse_if(&mut self) -> CompilerResult<ast::Statement> {
        self.advance();

        let condition = self.parse_expression()?;
        let then = self.parse_codeblock()?;

        let otherwise = if let TokenValue::Else = self.peek() {
            self.advance();
            match self.peek() {
                TokenValue::If => ast::CodeBody(vec![self.parse_if()?]),
                _ => self.parse_codeblock()?,
            }
        } else {
            ast::CodeBody(Vec::new())
        };

        Ok(ast::Statement::If {
            condition,
            then,
            otherwise,
        })
    }

    fn parse_statement(&mut self) -> CompilerResult<Option<ast::Statement>> {
        match self.peek() {
            TokenValue::Print => self.parse_print().map(Some),
            TokenValue::Assert => self.parse_assert().map(Some),
            TokenValue::Identifier(_) => self.parse_assignment().map(Some),
            TokenValue::Return => self.parse_return().map(Some),
            TokenValue::Test => self.parse_test().map(Some),
            TokenValue::If => self.parse_if().map(Some),
            _ => Ok(None),
        }
    }

    fn parse_codeblock(&mut self) -> CompilerResult<ast::CodeBody> {
        self.expect(&TokenValue::OpenBracket)?;

        let mut statements = Vec::new();
        while let Some(statement) = self.parse_statement()? {
            statements.push(statement);
        }

        self.expect(&TokenValue::CloseBracket)?;
        Ok(ast::CodeBody(statements))
    }

    fn parse_function_definition(&mut self) -> CompilerResult<ast::TopLevelStatement> {
        self.expect(&TokenValue::Fn)?;

        let function_name_token = self.advance();
        let function_name = match function_name_token.value {
            TokenValue::Identifier(name) => name,
            _ => {
                return Err((
                    function_name_token.source_location,
                    format!("expected name, got {:?}", function_name_token.value),
                ))
            }
        };

        self.expect(&TokenValue::OpenParen)?;
        self.expect(&TokenValue::CloseParen)?;

        self.expect(&TokenValue::Arrow)?;

        let return_type_token = self.advance();
        let return_type_name = match return_type_token.value {
            TokenValue::Identifier(name) => name,
            _ => {
                return Err((
                    return_type_token.source_location,
                    format!("expected name, got {:?}", return_type_token.value),
                ))
            }
        };

        let body = self.parse_codeblock()?;

        Ok(ast::TopLevelStatement::FunctionDefinition {
            function_name,
            body,
            return_type_name,
            return_type_location: return_type_token.source_location,
            metadata: ast::FunctionMetadata::default(),
        })
    }

    fn parse_toplevel_statement(&mut self) -> CompilerResult<Option<ast::TopLevelStatement>> {
        match self.peek() {
            TokenValue::Fn => self.parse_function_definition().map(Some),
            TokenValue::EndOfFile => Ok(None),
            _ => {
                let token = self.advance();
                Err((
                    token.source_location,
                    format!(
                        "expected start of top level definition or end of file, got {:?}",
                        token.value
                    ),
                ))
            }
        }
    }

    pub fn parse_file(&mut self) -> CompilerResult<ast::File> {
        let mut statements = Vec::new();

        while let Some(statement) = self.parse_toplevel_statement()? {
            statements.push(statement);
        }

        Ok(ast::File(statements))
    }
}
