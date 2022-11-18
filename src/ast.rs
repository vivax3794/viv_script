use crate::parser::Span;


#[derive(Debug, PartialEq, Eq)]
pub struct CodeBody<'a> {
    pub statements: Vec<Statement<'a>>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct  Statement<'a> {
    pub pos: Span<'a>,
    pub specifics: StatementType<'a>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum StatementType<'a> {
    Print(Expression<'a>)
}

#[derive(Debug, PartialEq, Eq)]
pub struct Expression<'a> {
    pub pos: Span<'a>,
    pub specifics: ExpressionType<'a>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ExpressionType<'a> {
    Literal(LiteralType),
    Binary(Box<Expression<'a>>, Operator, Box<Expression<'a>>)
}

#[derive(Debug, PartialEq, Eq)]
pub enum Operator {
    Plus,
}

#[derive(Debug, PartialEq, Eq)]
pub enum LiteralType {
    LitI32(i32)
}