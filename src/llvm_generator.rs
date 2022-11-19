use inkwell_llvm12::{
    builder::Builder, context::Context, module::Module, values::{IntValue, PointerValue, FunctionValue}, AddressSpace, passes::PassManager,
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
    fpm: PassManager<Module<'ctx>>,
}

impl<'code, 'ctx> Compiler<'code, 'ctx> {
    pub fn create_context() -> Context {
        Context::create()
    }

    pub fn new(name: &str, code: &'code str, context: &'ctx Context) -> Self {
        let module = context.create_module(name);
        let builder = context.create_builder();

        let fpm = PassManager::create(());

        fpm.add_ipsccp_pass();
        fpm.add_new_gvn_pass();
        fpm.add_ind_var_simplify_pass();
        fpm.add_instruction_simplify_pass();
        fpm.add_instruction_combining_pass();

        fpm.add_constant_merge_pass();
        fpm.add_global_optimizer_pass();

        fpm.add_merge_functions_pass();
        fpm.add_dead_arg_elimination_pass();
        fpm.add_argument_promotion_pass();
        fpm.add_function_attrs_pass();
        fpm.add_function_inlining_pass();
        fpm.add_tail_call_elimination_pass();

        fpm.add_licm_pass();
        fpm.add_loop_unswitch_pass();

        fpm.add_cfg_simplification_pass();

        fpm.add_global_dce_pass();
        fpm.add_aggressive_dce_pass();
        fpm.add_loop_deletion_pass();

        Self {
            raw_code: code,
            context,
            module,
            builder,
            fpm
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

    fn compile_expression(&self, exp: Expression) -> Value {
        match exp {
            Expression::Literal(lit) => self.compile_literal(lit),
            Expression::Binary(left, op, right) => {
                let left = self.compile_expression(*left);
                let right = self.compile_expression(*right);

                match (left, right) {
                    (Value::Number(left), Value::Number(right)) => Value::Number(match op {
                        Operator::Add => self.builder.build_int_add(left, right, "Number_Add"),
                        Operator::Sub => self.builder.build_int_sub(left, right, "Number_Sub"),
                        Operator::Mul => self.builder.build_int_mul(left, right, "Number_Mul"),
                        Operator::Div => self.builder.build_int_signed_div(left, right, "Number_Div"),
                    }),
                    _ => panic!("Can not these types of values!"),
                }
            }
        }
    }

    fn compile_print(&self, to_print: Expression) {
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

    fn compile_statement(&self, stmt: Statement) {
        match stmt {
            Statement::Print(expr) => self.compile_print(expr),
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
            self.compile_statement(stmt);
        }

        self.builder
            .build_return(Some(&i32_type.const_int(0, false)));
        
        self.fpm.run_on(&self.module);
    }

    pub fn save_in(&self, path: &str) {
        self.module.print_to_file(path).unwrap();
    }
}
