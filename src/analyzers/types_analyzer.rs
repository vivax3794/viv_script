use std::collections::HashMap;

use crate::{ast, types::TypeInformation, SourceLocation};

pub struct TypeAnalyzer {
    var_types: HashMap<String, TypeInformation>,
}

impl TypeAnalyzer {
    pub fn new() -> Self {
        Self {
            var_types: HashMap::new(),
        }
    }

    fn analyze_binary(
        &mut self,
        meta: &mut ast::ExpressionMetadata,
        left: &mut ast::Expression,
        right: &mut ast::Expression,
    ) -> crate::CompilerResult<()> {
        let left_type = left.metadata().type_information.unwrap();
        let right_type = right.metadata().type_information.unwrap();

        if left_type != right_type {
            return Err((
                SourceLocation::combine(left.location(), right.location()),
                format!(
                    "Expected left and right to have same type, got {:?} and {:?}",
                    left_type, right_type
                ),
            ));
        }

        if left_type != TypeInformation::Number {
            return Err((
                SourceLocation::combine(left.location(), right.location()),
                format!("can only do operations on numbers, got {:?}", left_type),
            ));
        }

        meta.type_information = Some(TypeInformation::Number);

        Ok(())
    }
}

impl super::Analyzer for TypeAnalyzer {
    fn visit_expression(&mut self, expr: &mut crate::ast::Expression) -> crate::CompilerResult<()> {
        match expr {
            ast::Expression::Literal(meta, lit) => {
                meta.type_information = Some(match lit {
                    ast::LiteralType::Number(_) => TypeInformation::Number,
                    ast::LiteralType::String(_) => TypeInformation::StringBorrow,
                })
            }
            ast::Expression::Binary(meta, left, _, right) => {
                self.analyze_binary(meta, left, right)?;
            }
            ast::Expression::Var(meta, name) => match self.var_types.get(name) {
                Some(type_) => meta.type_information = Some(*type_),
                None => return Err((meta.location, format!("Name {} not defined", name))),
            },
        }

        Ok(())
    }

    fn visit_stmt(&mut self, stmt: &mut ast::Statement) -> crate::CompilerResult<()> {
        match stmt {
            ast::Statement::Print(_) => {}
            ast::Statement::Assignment(_, name, expr) => match self.var_types.get(name) {
                None => {
                    let type_ = expr.metadata().type_information.unwrap();
                    self.var_types.insert(name.clone(), type_);
                }
                Some(expected_type) => {
                    let expr_type = expr.metadata().type_information.unwrap();

                    let expr_type = match expr_type {
                        // The var will own the value, but when the var is used it will always result in a borrow
                        // so the value stored in `abc` is a `StringOwned`, but the expression `abc` will result in a StringBorrow
                        TypeInformation::StringOwned => TypeInformation::StringBorrow,
                        _ => expr_type,
                    };

                    if expr_type != *expected_type {
                        return Err((
                            *expr.location(),
                            format!("expected {:?}, but got {:?}", expected_type, expr_type),
                        ));
                    }
                }
            },
        }

        Ok(())
    }

    fn visit_toplevel(&mut self, stmt: &mut ast::TopLevelStatement) -> crate::CompilerResult<()> {
        match stmt {
            ast::TopLevelStatement::FunctionDefinition(_, _, metadata) => {
                metadata.var_types = self.var_types.clone();
                self.var_types.clear();
           },
        }

        Ok(())
    }
}
