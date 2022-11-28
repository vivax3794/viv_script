mod definition_analyzer;
mod types_analyzer;

use crate::{ast, CompilerResult};

trait Analyzer {
    fn visit_expression(&mut self, _expr: &mut ast::Expression) -> CompilerResult<()> {
        Ok(())
    }
    fn visit_stmt(&mut self, _stmt: &mut ast::Statement) -> CompilerResult<()> {
        Ok(())
    }
    fn visit_toplevel(&mut self, _stmt: &mut ast::TopLevelStatement) -> CompilerResult<()> {
        Ok(())
    }
    fn pre_visit_toplevel(&mut self, _stmt: &mut ast::TopLevelStatement) -> CompilerResult<()> {
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
            ast::Statement::Return(expr) => self._visit_expression(expr)?,
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
        self.pre_visit_toplevel(stmt)?;

        match stmt {
            ast::TopLevelStatement::FunctionDefinition { body, ..} => self._visit_codebody(body)?,
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
    let mut definition_analyzer = definition_analyzer::DefinitionAnalyzer::new();

    definition_analyzer.visit_file(code)?;
    type_analyzer.visit_file(code)?;

    Ok(())
}
