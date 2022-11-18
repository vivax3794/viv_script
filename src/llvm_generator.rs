use inkwell_llvm12::{
    builder::Builder, context::Context, module::Module, values::{IntValue, PointerValue}, AddressSpace,
};

use super::ast::*;

enum Value<'ctx> {
    Number(IntValue<'ctx>),
    String(PointerValue<'ctx>)
}

pub struct Compiler<'code, 'ctx> {
    raw_code: &'code str,
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
}

impl<'code, 'ctx> Compiler<'code, 'ctx> {
    pub fn create_context() -> Context {
        Context::create()
    }

    pub fn new(code: &'code str, context: &'ctx Context) -> Self {
        let module = context.create_module("TMP");
        let builder = context.create_builder();

        Self {
            raw_code: code,
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

    fn compile_literal(&self, lit: LiteralType) -> Value {
        match lit {
            LiteralType::Number(value) => {
                let i32_type = self.context.i32_type();
                Value::Number(i32_type.const_int(value as u64, false))
            },
            LiteralType::String(value) => {
                let global_string = unsafe {
                    self.builder.build_global_string(&value, "Literal_String")
                };
                let ptr_to_string = global_string.as_pointer_value();
                Value::String(ptr_to_string)
            }
        }
    }

    fn compile_expression(&self, exp: ExpressionType) -> Value {
        match exp {
            ExpressionType::Literal(lit) => self.compile_literal(lit),
            ExpressionType::Binary(left, op, right) => {
                let left = self.compile_expression(left.specifics);
                let right = self.compile_expression(right.specifics);

                match (left, right) {
                    (Value::Number(left), Value::Number(right)) => Value::Number(match op {
                        Operator::Plus => self.builder.build_int_add(left, right, "+"),
                        Operator::Minus => self.builder.build_int_sub(left, right, "-"),
                    }),
                    _ => panic!("Can not these types of values!"),
                }
            }
        }
    }

    fn compile_print(&self, to_print: ExpressionType) {
        let value_hinted = self.compile_expression(to_print);
        let value = match value_hinted {
            Value::Number(val) => val.into(),
            Value::String(val) => val.into(),
        };

        let format_string = match value_hinted {
            Value::Number(_) => "%d\n",
            Value::String(_) => "%s\n"
        };

        let printf_function = self.module.get_function("printf").unwrap();
        let format_string = unsafe {
            self.builder
                .build_global_string(format_string, "Print_Format_String")
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
            value,
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
