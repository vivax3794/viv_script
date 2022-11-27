mod types_analyzer;

use std::collections::HashMap;
use crate::{ast, CompilerResult, types::TypeInformation};

trait Analyzer {
    fn visit_expression(&mut self, expr: &mut ast::Expression) -> CompilerResult<()>;
    fn visit_stmt(&mut self, stmt: &mut ast::Statement) -> CompilerResult<()>;

    fn _visit_expression(&mut self, expr: &mut ast::Expression) -> CompilerResult<()> {
        match expr {
            ast::Expression::Literal(_, _) => {}
            ast::Expression::Binary(_, left, _, right) => {
                self._visit_expression(left.as_mut())?;
                self._visit_expression(right.as_mut())?;
            }
            ast::Expression::Var(_, _) => {}
        }

        self.visit_expression(expr)
    }

    fn _visit_stmt(&mut self, stmt: &mut ast::Statement) -> CompilerResult<()> {
        match stmt {
            ast::Statement::Print(expr) => self._visit_expression(expr)?,
            ast::Statement::Assignment(_, _, expr) => self._visit_expression(expr)?,
        }

        self.visit_stmt(stmt)
    }

    fn visit_code_block(&mut self, block: &mut ast::CodeBody) -> CompilerResult<()> {
        for stmt in block.0.iter_mut() {
            self._visit_stmt(stmt)?;
        }

        Ok(())
    }
}

pub fn apply_analyzer(code: &mut ast::CodeBody) -> CompilerResult<HashMap<String, TypeInformation>> {
    let mut type_analyzer = types_analyzer::TypeAnalyzer::new();
    type_analyzer.visit_code_block(code)?;

    Ok(type_analyzer.var_types)
}