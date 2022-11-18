use inkwell_llvm12::{
    builder::Builder, context::Context, module::Module, values::IntValue, AddressSpace,
};

use super::ast::*;

pub struct Compiler<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
}

impl<'ctx> Compiler<'ctx> {
    pub fn create_context() -> Context {
        Context::create()
    }

    pub fn new(context: &'ctx Context) -> Self {
        let module = context.create_module("TMP");
        let builder = context.create_builder();

        Self {
            context,
            module,
            builder,
        }
    }

    fn compile_glibc_definitions(&self) {
        // types
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::Generic);
        let i32_type = self.context.i32_type();

        // printf
        let printf_argument_types = [i8_ptr_type.into()];
        let printf_function_type = i32_type.fn_type(&printf_argument_types, true);
        self.module
            .add_function("printf", printf_function_type, None);
    }

    fn compile_literal(&self, lit: LiteralType) -> IntValue {
        match lit {
            LiteralType::LitI32(value) => {
                let i32_type = self.context.i32_type();
                i32_type.const_int(value as u64, false)
            }
        }
    }

    fn compile_expression(&self, exp: ExpressionType) -> IntValue {
        match exp {
            ExpressionType::Literal(lit) => self.compile_literal(lit),
            ExpressionType::Binary(left, op, right) => {
                let left = self.compile_expression(left.specifics);
                let right = self.compile_expression(right.specifics);

                match op {
                    Operator::Plus => self.builder.build_int_add(left, right, "+"),
                }
            }
        }
    }

    fn compile_print(&self, to_print: ExpressionType) {
        let value = self.compile_expression(to_print);

        let printf_function = self.module.get_function("printf").unwrap();
        let format_string = unsafe {
            self.builder
                .build_global_string("%d\n", "I32_Print_Format_String")
        };
        let printf_arguments = [
            // Format string
            self.builder
                .build_pointer_cast(
                    format_string.as_pointer_value(),
                    self.context.i8_type().ptr_type(AddressSpace::Generic),
                    "Format",
                )
                .into(),
            value.into(),
        ];

        self.builder
            .build_call(printf_function, &printf_arguments, "Print_Statement");
    }

    fn compile_statement(&self, stmt: StatementType) {
        match stmt {
            StatementType::Print(expr) => self.compile_print(expr.specifics),
        }
    }

    pub fn compile_code(&self, code: CodeBody) {
        // Create clib functions
        self.compile_glibc_definitions();

        // Create main function
        let i32_type = self.context.i32_type();
        let main_argument_types = [];
        let main_function_type = i32_type.fn_type(&main_argument_types, false);
        let main_function = self.module.add_function("main", main_function_type, None);

        let entry = self.context.append_basic_block(main_function, "entry");
        self.builder.position_at_end(entry);

        for stmt in code.statements {
            self.compile_statement(stmt.specifics);
        }

        self.builder
            .build_return(Some(&i32_type.const_int(0, false)));
    }

    pub fn save_in(&self, path: &str) {
        self.module.print_to_file(path).unwrap();
    }
}
