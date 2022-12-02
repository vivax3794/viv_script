use std::collections::HashMap;

use crate::{ast, types::TypeInformation, SourceLocation};

pub struct TypeAnalyzer {
    var_types: HashMap<String, TypeInformation>,
    return_type: TypeInformation,
}

impl TypeAnalyzer {
    pub fn new() -> Self {
        Self {
            var_types: HashMap::new(),
            return_type: TypeInformation::Number, // Temp value,
        }
    }

    fn analyze_binary(
        &mut self,
        metadata: &mut ast::ExpressionMetadata,
        left_expression: &mut ast::Expression,
        right_expression: &mut ast::Expression,
    ) -> crate::CompilerResult<()> {
        let left_type = left_expression.metadata().type_information.unwrap();
        let right_type = right_expression.metadata().type_information.unwrap();

        if left_type != right_type {
            return Err((
                SourceLocation::combine(left_expression.location(), right_expression.location()),
                format!(
                    "Expected left and right to have same type, got {:?} and {:?}",
                    left_type, right_type
                ),
            ));
        }

        if left_type != TypeInformation::Number {
            return Err((
                SourceLocation::combine(left_expression.location(), right_expression.location()),
                format!("can only do operations on numbers, got {:?}", left_type),
            ));
        }

        metadata.type_information = Some(TypeInformation::Number);

        Ok(())
    }
}

impl super::Analyzer for TypeAnalyzer {
    fn visit_expression(&mut self, expr: &mut crate::ast::Expression) -> crate::CompilerResult<()> {
        match expr {
            ast::Expression::Literal(metadata, literal) => {
                metadata.type_information = Some(match literal {
                    ast::LiteralType::Number(_) => TypeInformation::Number,
                    ast::LiteralType::String(_) => TypeInformation::StringBorrow,
                    ast::LiteralType::Boolean(_) => TypeInformation::Boolean,
                })
            }
            ast::Expression::Binary { metadata, left, operator: _, right } => {
                self.analyze_binary(metadata, left, right)?;
            }
            ast::Expression::Var(metadata, var_name) => match self.var_types.get(var_name) {
                Some(type_) => metadata.type_information = Some(*type_),
                None => return Err((metadata.location, format!("Name {} not defined", var_name))),
            },
        }

        Ok(())
    }

    fn visit_stmt(&mut self, stmt: &mut ast::Statement) -> crate::CompilerResult<()> {
        match stmt {
            ast::Statement::Print(_) => {}
            ast::Statement::Assignment { var_name, expression, .. } => match self.var_types.get(var_name) {
                None => {
                    let type_ = expression.metadata().type_information.unwrap();
                    self.var_types.insert(var_name.clone(), type_);
                }
                Some(expected_type) => {
                    let expression_type = expression.metadata().type_information.unwrap();

                    // Handle string special case.
                    let expression_type = match expression_type {
                        // The var will own the value, but when the var is used it will always result in a borrow
                        // so the value stored in `abc` is a `StringOwned`, but the expression `abc` will result in a StringBorrow
                        TypeInformation::StringOwned => TypeInformation::StringBorrow,
                        _ => expression_type,
                    };

                    if expression_type != *expected_type {
                        return Err((
                            *expression.location(),
                            format!("expected {:?}, but got {:?}", expected_type, expression_type),
                        ));
                    }
                }
            },
            ast::Statement::Return(return_expression) => {
                if self.return_type != return_expression.metadata().type_information.unwrap() {
                    return Err((
                        *return_expression.location(),
                        format!(
                            "expected {:?}, got {:?}",
                            self.return_type,
                            return_expression.metadata().type_information.unwrap()
                        ),
                    ));
                }
            }
        }

        Ok(())
    }

    fn pre_visit_toplevel(
        &mut self,
        statement: &mut ast::TopLevelStatement,
    ) -> crate::CompilerResult<()> {
        // Clear/Setup function context
        match statement {
            ast::TopLevelStatement::FunctionDefinition { metadata, .. } => {
                self.var_types.clear();
                self.return_type = metadata.return_type.unwrap();
            }
        }

        Ok(())
    }

    fn visit_toplevel(&mut self, statement: &mut ast::TopLevelStatement) -> crate::CompilerResult<()> {
        match statement {
            ast::TopLevelStatement::FunctionDefinition { metadata, .. } => {
                metadata.var_types = self.var_types.clone();
            }
        }

        Ok(())
    }
}
