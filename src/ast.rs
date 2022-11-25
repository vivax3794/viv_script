use crate::parser::SourceLocation;

/// A code body is a collection of statements 
#[derive(Debug, PartialEq, Eq)]
pub struct CodeBody(pub Vec<Statement>);

/// A statement is usually a line of code, but can be more (they are usually defined by being separated by semi colons);
/// A statement is the building blocks of a program, some statements contain more statements (like the body of a loop);
#[derive(Debug, PartialEq, Eq)]
pub enum Statement {
    /// A print statement is used to output the value of a expression
    Print(Expression),
    /// An assignment stores the value of a expression in the provided name 
    Assignment(SourceLocation, String, Expression),
}

// An expression is the building block of the language. it usually does stuff.
#[derive(Debug, PartialEq, Eq)]
pub enum Expression {
    /// A literal expression always resolves to the same constant and is directly hardcoded into the resulting binary
    /// (unless ofc they are optimized away as part of a constant equation or are just not used)
    Literal(SourceLocation, LiteralType),
    /// A Binary expressions consists of 2 other expressions and an operator
    Binary(SourceLocation, Box<Expression>, Operator, Box<Expression>),
    /// Loads a value as stored by the assignment expression
    Var(SourceLocation, String),
}

impl Expression {
    pub fn location(&self) -> &SourceLocation {
        match self {
            Expression::Literal(loc, _) => loc,
            Expression::Binary(loc, _, _, _) => loc,
            Expression::Var(loc, _) => loc
        }
    }
}


/// A operator describes what action should be taken on the expressions of a binary-exp
/// These are relatively self explanatory
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
