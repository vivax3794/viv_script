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
        metadata: &mut ast::ExpressionMetadata,
        left_expression: &mut ast::Expression,
        operator: ast::Operator,
        right_expression: &mut ast::Expression,
    ) -> crate::CompilerResult<()> {
        let left_type = left_expression.metadata().type_information.unwrap();
        let right_type = right_expression.metadata().type_information.unwrap();

        let source_location =
            SourceLocation::combine(left_expression.location(), right_expression.location());

        if left_type != right_type {
            return Err((
                source_location,
                format!(
                    "Expected left and right to have same type, got {:?} and {:?}",
                    left_type, right_type
                ),
            ));
        }

        let resulting_type = match left_type {
            TypeInformation::Number => match operator {
                ast::Operator::Add
                | ast::Operator::Sub
                | ast::Operator::Mul
                | ast::Operator::Div => TypeInformation::Number,
                ast::Operator::Equal => TypeInformation::Boolean,
            },
            TypeInformation::Boolean => {
                return Err((
                    source_location,
                    format!("Unsupported operator for boolean {:?}", operator),
                ))
            }
            TypeInformation::String(_) => {
                return Err((
                    source_location,
                    format!("Unsupported operator for String {:?}", operator),
                ))
            }
        };

        metadata.type_information = Some(resulting_type);

        Ok(())
    }
}

impl super::Analyzer for TypeAnalyzer {
    fn visit_expression(&mut self, expr: &mut crate::ast::Expression) -> crate::CompilerResult<()> {
        match expr {
            ast::Expression::Literal(metadata, literal) => {
                metadata.type_information = Some(match literal {
                    ast::LiteralType::Number(_) => TypeInformation::Number,
                    ast::LiteralType::String(_) => TypeInformation::String(false),
                    ast::LiteralType::Boolean(_) => TypeInformation::Boolean,
                });
            }
            ast::Expression::Binary {
                metadata,
                left,
                operator,
                right,
            } => {
                TypeAnalyzer::analyze_binary(metadata, left, *operator, right)?;
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
            ast::Statement::Assert(expr) | ast::Statement::Test(_, expr) => {
                let expr_type = expr.metadata().type_information.unwrap();
                if expr_type != TypeInformation::Boolean {
                    return Err((
                        *expr.location(),
                        format!("Expected Boolean, got {:?}", expr_type),
                    ));
                }
            }
            ast::Statement::Assignment {
                var_name,
                expression,
                ..
            } => match self.var_types.get(var_name) {
                None => {
                    let type_ = expression.metadata().type_information.unwrap();
                    let type_ = type_.mark_borrowed();
                    self.var_types.insert(var_name.clone(), type_);
                }
                Some(expected_type) => {
                    let expression_type = expression.metadata().type_information.unwrap();

                    if expression_type != *expected_type {
                        return Err((
                            *expression.location(),
                            format!(
                                "expected {:?}, but got {:?}",
                                expected_type, expression_type
                            ),
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

    fn visit_toplevel(
        &mut self,
        statement: &mut ast::TopLevelStatement,
    ) -> crate::CompilerResult<()> {
        match statement {
            ast::TopLevelStatement::FunctionDefinition { metadata, .. } => {
                metadata.var_types = self.var_types.clone();
            }
        }

        Ok(())
    }
}
