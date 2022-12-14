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
        let left_type = *left_expression.type_info();
        let right_type = *right_expression.type_info();

        let source_location =
            SourceLocation::combine(left_expression.location(), right_expression.location());

        if !TypeInformation::same_type(left_type, right_type) {
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

    fn analyze_comparison(
        metadata: &mut ast::ExpressionMetadata,
        first: &ast::Expression,
        chains: &Vec<(ast::Comparison, ast::Expression)>,
    ) -> crate::CompilerResult<()> {
        let type_ = *first.type_info();

        let valid_comparisons = match type_ {
            TypeInformation::Number => vec![
                ast::Comparison::Equal,
                ast::Comparison::NotEqual,
                ast::Comparison::GreaterThan,
                ast::Comparison::GreaterThanEqual,
                ast::Comparison::LessThan,
                ast::Comparison::LessThanEqual,
            ],
            TypeInformation::Boolean => vec![],
            TypeInformation::String(_) => vec![],
        };

        for (comp, value) in chains {
            let value_type = *value.type_info();
            if !TypeInformation::same_type(type_, value_type) {
                return Err((
                    SourceLocation::combine(first.location(), value.location()),
                    format!("Expected all expression in comparison chain to have same type, got {:?} and {:?}", type_, value_type)
                ));
            }

            if !valid_comparisons.contains(comp) {
                return Err((
                    metadata.location,
                    format!("Not a valid comparison for {type_:?}, valid comps are {valid_comparisons:?}")
                ));
            }
        }

        metadata.type_information = Some(TypeInformation::Boolean);

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
            ast::Expression::ComparisonChain {
                first_element,
                comparisons,
                metadata,
            } => TypeAnalyzer::analyze_comparison(metadata, first_element, comparisons)?,
            ast::Expression::Var(metadata, var_name) => match self.var_types.get(var_name) {
                Some(type_) => metadata.type_information = Some(*type_),
                None => return Err((metadata.location, format!("Name {} not defined", var_name))),
            },
            ast::Expression::PrefixExpression {
                op,
                expression,
                metadata,
            } => {
                let type_ = match (op, expression.type_info()) {
                    (ast::PrefixOprator::Not, TypeInformation::Boolean) => TypeInformation::Boolean,
                    _ => {
                        return Err((
                            *expression.location(),
                            format!(
                                "Invalid prefix operator for {:?}",
                                expression.type_info()
                            ),
                        ))
                    }
                };
                metadata.type_information = Some(type_);
            }
        }

        Ok(())
    }

    fn visit_stmt(&mut self, stmt: &mut ast::Statement) -> crate::CompilerResult<()> {
        match stmt {
            ast::Statement::Print(_) => {}
            ast::Statement::Assert(expr) | ast::Statement::Test(_, expr) => {
                let expr_type = *expr.type_info();
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
                    let type_ = expression.type_info();
                    let type_ = type_.mark_borrowed();
                    self.var_types.insert(var_name.clone(), type_);
                }
                Some(expected_type) => {
                    let expression_type = *expression.type_info();

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
                if self.return_type != *return_expression.type_info() {
                    return Err((
                        *return_expression.location(),
                        format!(
                            "expected {:?}, got {:?}",
                            self.return_type,
                            return_expression.type_info()
                        ),
                    ));
                }
            }
            ast::Statement::If { condition, .. } => {
                let condition_type = *condition.type_info();
                if !TypeInformation::same_type(condition_type, TypeInformation::Boolean) {
                    return Err((
                        *condition.location(),
                        format!("Expected condition to be bool, got {:?}", condition_type),
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
