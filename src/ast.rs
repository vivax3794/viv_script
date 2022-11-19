#[derive(Debug, PartialEq, Eq)]
pub struct CodeBody {
    pub statements: Vec<Statement>,
}


#[derive(Debug, PartialEq, Eq)]
pub enum Statement {
    Print(Expression)
}

#[derive(Debug, PartialEq, Eq)]
pub enum Expression {
    Literal(LiteralType),
    Binary(Box<Expression>, Operator, Box<Expression>)
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Operator {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, PartialEq, Eq)]
pub enum LiteralType {
    Number(i32),
    String(String)
}