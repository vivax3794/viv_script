use crate::ast;
use crate::types::TypeInformation;

pub struct DefinitionAnalyzer {}

impl DefinitionAnalyzer {
    pub     fn new() -> Self {
        Self {}
    }

    fn get_type(&self, type_name: &str) -> Option<TypeInformation> {
        match type_name {
            "Num" => Some(TypeInformation::Number),
            // This would be different in different contexts, but owned can be for all...
            "String" => Some(TypeInformation::StringOwned),
            "Bool" => Some(TypeInformation::Boolean),
            _ => None,
        }
    }
}

impl super::Analyzer for DefinitionAnalyzer {
    fn visit_toplevel(&mut self, statement: &mut ast::TopLevelStatement) -> crate::CompilerResult<()> {
        match statement {
            ast::TopLevelStatement::FunctionDefinition {
                return_type_name,
                return_type_location,
                metadata,
                ..
            } => {
                let return_type = match self.get_type(return_type_name) {
                    Some(type_) => type_,
                    None => return Err((*return_type_location, "Invalid type name".to_string())),
                };
                metadata.return_type.replace(return_type);
            }
        }

        Ok(())
    }
}
