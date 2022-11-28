use crate::ast;
use crate::types::TypeInformation;

pub struct DefinitionAnalyzer {}

impl DefinitionAnalyzer {
    pub     fn new() -> Self {
        Self {}
    }

    fn get_type(&self, type_: &str) -> Option<TypeInformation> {
        match type_ {
            "Num" => Some(TypeInformation::Number),
            // This would be different in different contexts, but owned can be for all...
            "String" => Some(TypeInformation::StringOwned),
            _ => None,
        }
    }
}

impl super::Analyzer for DefinitionAnalyzer {
    fn visit_toplevel(&mut self, stmt: &mut ast::TopLevelStatement) -> crate::CompilerResult<()> {
        match stmt {
            ast::TopLevelStatement::FunctionDefinition {
                return_type,
                return_type_location,
                meta,
                ..
            } => {
                let return_type = match self.get_type(return_type) {
                    Some(type_) => type_,
                    None => return Err((*return_type_location, "Invalid type name".to_string())),
                };
                meta.return_type.replace(return_type);
            }
        }

        Ok(())
    }
}
