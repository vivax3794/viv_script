
/// A code body is a collection of statemnts 
#[derive(Debug, PartialEq, Eq)]
pub struct CodeBody(pub Vec<Statement>);

/// A statement is usually a line of code, but can be more (they are usually defined by being seperated by semi colons);
/// A statement is the building blocks of a program, some staments contain more statements (like the body of a loop);
#[derive(Debug, PartialEq, Eq)]
pub enum Statement {
    /// A print statement is used to output the value of a expression
    Print(Expression),
    /// An assignement stores the value of a expression in the provided name 
    Assignment(String, Expression),
}

// An expression is the building block of the languague. it usually does stuff.
#[derive(Debug, PartialEq, Eq)]
pub enum Expression {
    /// A literal expression always resolves to the same constant and is directly hardcoded into the resulting binary
    /// (unless ofc they are optimized away as part of a constant equation or are just not used)
    Literal(LiteralType),
    /// A Binary expressions consists of 2 other expressions and an operator
    Binary(Box<Expression>, Operator, Box<Expression>),
    /// Loads a value as stored by the assignment expression
    Var(String),
}


/// A operator describes what action should be taken on the expressions of a binary-exp
/// These are relativly self explainatory
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Operator {
    Add,
    Sub,
    Mul,
    Div,
}

/// A literal is a hardcoded value
#[derive(Debug, PartialEq, Eq)]
pub enum LiteralType {
    /// Literal number, these are stored directly in the IR
    Number(i32),
    /// Literal strings are stored as global strings
    String(String),
}
