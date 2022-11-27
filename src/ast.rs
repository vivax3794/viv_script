use std::collections::HashMap;
use crate::parser::SourceLocation;
use crate::types::TypeInformation;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct File(pub Vec<TopLevelStatement>);

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum TopLevelStatement {
    FunctionDefinition(String, CodeBody, FunctionMetadata),
}

#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct FunctionMetadata {
    pub var_types:  HashMap<String, TypeInformation>
}

/// A code body is a collection of statements
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct CodeBody(pub Vec<Statement>);

/// A statement is usually a line of code, but can be more (they are usually defined by being separated by semi colons);
/// A statement is the building blocks of a program, some statements contain more statements (like the body of a loop);
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Statement {
    /// A print statement is used to output the value of a expression
    Print(Expression),
    /// An assignment stores the value of a expression in the provided name
    Assignment(SourceLocation, String, Expression),
}

// An expression is the building block of the language. it usually does stuff.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Expression {
    /// A literal expression always resolves to the same constant and is directly hardcoded into the resulting binary
    /// (unless ofc they are optimized away as part of a constant equation or are just not used)
    Literal(ExpressionMetadata, LiteralType),
    /// A Binary expressions consists of 2 other expressions and an operator
    Binary(
        ExpressionMetadata,
        Box<Expression>,
        Operator,
        Box<Expression>,
    ),
    /// Loads a value as stored by the assignment expression
    Var(ExpressionMetadata, String),
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct ExpressionMetadata {
    pub location: SourceLocation,
    pub type_information: Option<TypeInformation>,
}

impl From<SourceLocation> for ExpressionMetadata {
    fn from(location: SourceLocation) -> Self {
        Self {
            location,
            type_information: None,
        }
    }
}

impl Expression {
    pub fn metadata(&self) -> &ExpressionMetadata {
        match self {
            Expression::Literal(meta, _) => meta,
            Expression::Binary(meta, _, _, _) => meta,
            Expression::Var(meta, _) => meta,
        }
    }

    pub fn location(&self) -> &SourceLocation {
        &self.metadata().location
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
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum LiteralType {
    /// Literal number, these are stored directly in the IR
    Number(i32),
    /// Literal strings are stored as global strings
    String(String),
}
