mod types_analyzer;

use crate::{ast, types::TypeInformation, CompilerResult};
use std::collections::HashMap;

trait Analyzer {
    fn visit_expression(&mut self, expr: &mut ast::Expression) -> CompilerResult<()> {
        Ok(())
    }
    fn visit_stmt(&mut self, stmt: &mut ast::Statement) -> CompilerResult<()> {
        Ok(())
    }
    fn visit_toplevel(&mut self, stmt: &mut ast::TopLevelStatement) -> CompilerResult<()> {
        Ok(())
    }

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

    fn _visit_codebody(&mut self, body: &mut ast::CodeBody) -> CompilerResult<()> {
        for stmt in body.0.iter_mut() {
            self._visit_stmt(stmt)?;
        }

        Ok(())
    }

    fn _visit_toplevel(&mut self, stmt: &mut ast::TopLevelStatement) -> CompilerResult<()> {
        match stmt {
            ast::TopLevelStatement::FunctionDefinition(_, body, _) => self._visit_codebody(body)?,
        }

        self.visit_toplevel(stmt)
    }

    fn visit_file(&mut self, file: &mut ast::File) -> CompilerResult<()> {
        for stmt in file.0.iter_mut() {
            self._visit_toplevel(stmt)?;
        }

        Ok(())
    }
}

pub fn apply_analyzer(code: &mut ast::File) -> CompilerResult<()> {
    let mut type_analyzer = types_analyzer::TypeAnalyzer::new();
    type_analyzer.visit_file(code)?;

    Ok(())
}
