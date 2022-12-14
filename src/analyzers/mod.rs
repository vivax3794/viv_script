mod definition_analyzer;
mod types_analyzer;

use crate::{ast, CompilerResult};

trait Analyzer {
    fn visit_expression(&mut self, _expression: &mut ast::Expression) -> CompilerResult<()> {
        Ok(())
    }
    fn visit_stmt(&mut self, _statement: &mut ast::Statement) -> CompilerResult<()> {
        Ok(())
    }
    fn visit_toplevel(&mut self, _statement: &mut ast::TopLevelStatement) -> CompilerResult<()> {
        Ok(())
    }
    fn pre_visit_toplevel(
        &mut self,
        _statement: &mut ast::TopLevelStatement,
    ) -> CompilerResult<()> {
        Ok(())
    }

    fn _visit_expression(&mut self, expression: &mut ast::Expression) -> CompilerResult<()> {
        match expression {
            ast::Expression::Binary {
                metadata: _,
                left,
                operator: _,
                right,
            } => {
                self._visit_expression(left.as_mut())?;
                self._visit_expression(right.as_mut())?;
            }
            ast::Expression::ComparisonChain {
                first_element,
                comparisons,
                ..
            } => {
                self._visit_expression(first_element.as_mut())?;
                for (_, expr) in comparisons {
                    self._visit_expression(expr)?;
                }
            },
            ast::Expression::PrefixExpression { expression, ..} => self._visit_expression(expression)?,
            ast::Expression::Var(_, _) | ast::Expression::Literal(_, _) => {}
        }

        self.visit_expression(expression)
    }

    fn _visit_stmt(&mut self, statement: &mut ast::Statement) -> CompilerResult<()> {
        match statement {
            ast::Statement::Print(expr)
            | ast::Statement::Assert(expr)
            | ast::Statement::Assignment {
                expression_location: _,
                var_name: _,
                expression: expr,
            }
            | ast::Statement::Return(expr)
            | ast::Statement::Test(_, expr) => self._visit_expression(expr)?,
            ast::Statement::If { condition, then, otherwise } => {
                self._visit_expression(condition)?;
                self._visit_codebody(then)?;
                self._visit_codebody(otherwise)?;
            }
        }

        self.visit_stmt(statement)
    }

    fn _visit_codebody(&mut self, body: &mut ast::CodeBody) -> CompilerResult<()> {
        body.0
            .iter_mut()
            .try_for_each(|stmt| self._visit_stmt(stmt))
    }

    fn _visit_toplevel(&mut self, statement: &mut ast::TopLevelStatement) -> CompilerResult<()> {
        self.pre_visit_toplevel(statement)?;

        match statement {
            ast::TopLevelStatement::FunctionDefinition { body, .. } => {
                self._visit_codebody(body)?;
            }
        }

        self.visit_toplevel(statement)
    }

    fn visit_file(&mut self, file: &mut ast::File) -> CompilerResult<()> {
        for stmt in &mut file.0 {
            self._visit_toplevel(stmt)?;
        }

        Ok(())
    }
}

pub fn apply_analyzer(code: &mut ast::File) -> CompilerResult<()> {
    let mut type_analyzer = types_analyzer::TypeAnalyzer::new();
    let mut definition_analyzer = definition_analyzer::DefinitionAnalyzer::new();

    definition_analyzer.visit_file(code)?;
    type_analyzer.visit_file(code)?;

    Ok(())
}
