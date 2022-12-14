use std::collections::{HashMap, VecDeque};

use inkwell::{
    builder::Builder,
    context::Context,
    module::Module,
    passes::PassManager,
    types::{BasicType, BasicTypeEnum},
    values::{BasicValue, BasicValueEnum, IntValue, PointerValue},
    AddressSpace,
};

use crate::ast::{self, Expression};
use crate::types::TypeInformation;

struct FunctionContext<'ctx> {
    var_types: HashMap<String, TypeInformation>,
    var_pointers: HashMap<String, PointerValue<'ctx>>,
}

pub struct Compiler<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    fpm: PassManager<Module<'ctx>>,

    function_context: Option<FunctionContext<'ctx>>,
}

impl<'ctx> Compiler<'ctx> {
    pub fn create_context() -> Context {
        Context::create()
    }

    pub fn new(name: &str, context: &'ctx Context) -> Self {
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

        fpm.add_demote_memory_to_register_pass();
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
            context,
            module,
            builder,
            fpm,
            function_context: None,
        }
    }

    fn compile_glibc_definitions(&self) {
        // types
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::Generic);
        let i32_type = self.context.i32_type();
        let size_type = self.context.i64_type();
        let void_type = self.context.void_type();

        // int printf( const char *format, ... );
        let printf_argument_types = [i8_ptr_type.into()];
        let printf_function_type = i32_type.fn_type(&printf_argument_types, true);
        self.module
            .add_function("printf", printf_function_type, None);

        // void* malloc( size_t size );
        let malloc_argument_types = [size_type.into()];
        let malloc_function_type = i8_ptr_type.fn_type(&malloc_argument_types, false);
        self.module
            .add_function("malloc", malloc_function_type, None);

        // void *realloc( void *ptr, size_t new_size );
        let realloc_argument_types = [i8_ptr_type.into(), size_type.into()];
        let realloc_function_type = i8_ptr_type.fn_type(&realloc_argument_types, false);
        self.module
            .add_function("realloc", realloc_function_type, None);

        // void free( void* ptr );
        let free_argument_types = [i8_ptr_type.into()];
        let free_function_type = void_type.fn_type(&free_argument_types, false);
        self.module.add_function("free", free_function_type, None);

        // size_t strlen( const char *str );
        let strlen_argument_types = [i8_ptr_type.into()];
        let strlen_function_type = size_type.fn_type(&strlen_argument_types, false);
        self.module
            .add_function("strlen", strlen_function_type, None);

        // void* memcpy( void *dest, const void *src, size_t count );
        let memcpy_argument_types = [i8_ptr_type.into(), i8_ptr_type.into(), size_type.into()];
        let memcpy_function_type = i8_ptr_type.fn_type(&memcpy_argument_types, false);
        self.module
            .add_function("memcpy", memcpy_function_type, None);

        // _Noreturn void abort(void);
        let abort_argument_types = [];
        let abort_function_type = void_type.fn_type(&abort_argument_types, false);
        self.module.add_function("abort", abort_function_type, None);
    }

    fn get_type_for(&self, type_: TypeInformation) -> BasicTypeEnum<'ctx> {
        match type_ {
            TypeInformation::Number => self.context.i32_type().as_basic_type_enum(),
            TypeInformation::Boolean => self.context.bool_type().as_basic_type_enum(),
            TypeInformation::String(_) => self
                .context
                .i8_type()
                .ptr_type(AddressSpace::Generic)
                .as_basic_type_enum(),
        }
    }

    fn free_if_needed(&self, value: BasicValueEnum, type_: TypeInformation) {
        if let TypeInformation::String(true) = type_ {
            let free_function = self.module.get_function("free").unwrap();
            self.builder
                .build_call(free_function, &[value.into()], "Free_Tmp_String");
        }
    }

    fn get_owned_string(&self, value: BasicValueEnum<'ctx>) -> BasicValueEnum<'ctx> {
        let strlen = self.module.get_function("strlen").unwrap();
        let string_length = self
            .builder
            .build_call(strlen, &[value.into()], "String_Len")
            .try_as_basic_value()
            .unwrap_left();

        let malloc = self.module.get_function("malloc").unwrap();
        let heap_pointer = self
            .builder
            .build_call(malloc, &[string_length.into()], "Heap_Pointer")
            .try_as_basic_value()
            .unwrap_left();

        let memcpy = self.module.get_function("memcpy").unwrap();
        self.builder
            .build_call(memcpy, &[heap_pointer.into(), value.into()], "Malloc");

        heap_pointer
    }

    fn compile_literal(&self, lit: &ast::LiteralType) -> BasicValueEnum<'ctx> {
        match lit {
            ast::LiteralType::Number(value) => {
                let i32_type = self.context.i32_type();
                i32_type
                    .const_int(*value as u64, false)
                    .as_basic_value_enum()
            }
            ast::LiteralType::String(value) => {
                let global_string =
                    unsafe { self.builder.build_global_string(value, "Literal_String") };
                let ptr_to_string = global_string.as_pointer_value();
                ptr_to_string.as_basic_value_enum()
            }
            ast::LiteralType::Boolean(value) => {
                let bool_type = self.context.bool_type();
                bool_type
                    .const_int(u64::from(*value), false)
                    .as_basic_value_enum()
            }
        }
    }

    fn compile_expression(&self, exp: &ast::Expression) -> BasicValueEnum<'ctx> {
        match exp {
            ast::Expression::Literal(_, lit) => self.compile_literal(lit),
            ast::Expression::Binary {
                metadata: _,
                left,
                operator,
                right,
            } => {
                let left_value = self.compile_expression(left).into_int_value();
                let right_value = self.compile_expression(right).into_int_value();

                match left.type_info() {
                    TypeInformation::Number => match operator {
                        ast::Operator::Add => self
                            .builder
                            .build_int_add(left_value, right_value, "Number_Add")
                            .as_basic_value_enum(),
                        ast::Operator::Sub => self
                            .builder
                            .build_int_sub(left_value, right_value, "Number_Sub")
                            .as_basic_value_enum(),
                        ast::Operator::Mul => self
                            .builder
                            .build_int_mul(left_value, right_value, "Number_Mul")
                            .as_basic_value_enum(),
                        ast::Operator::Div => self
                            .builder
                            .build_int_signed_div(left_value, right_value, "Number_Div")
                            .as_basic_value_enum(),
                    },
                    _ => unreachable!(),
                }
            }
            ast::Expression::ComparisonChain {
                first_element,
                comparisons,
                ..
            } => {
                let mut bool_values: Vec<IntValue> = Vec::with_capacity(comparisons.len());

                let mut left = self.compile_expression(first_element);
                let mut comparisons = VecDeque::from(comparisons.clone());

                // Calculate comparisons
                while !comparisons.is_empty() {
                    let (comp, right) = comparisons.pop_front().unwrap();
                    let right = self.compile_expression(&right);

                    let bool_value = match first_element.type_info() {
                        TypeInformation::Number => self.builder.build_int_compare(
                            match comp {
                                ast::Comparison::Equal => inkwell::IntPredicate::EQ,
                                ast::Comparison::NotEqual => inkwell::IntPredicate::NE,
                                ast::Comparison::GreaterThan => inkwell::IntPredicate::SGT,
                                ast::Comparison::GreaterThanEqual => inkwell::IntPredicate::SGE,
                                ast::Comparison::LessThan => inkwell::IntPredicate::SLT,
                                ast::Comparison::LessThanEqual => inkwell::IntPredicate::SLE,
                            },
                            left.into_int_value(),
                            right.into_int_value(),
                            "Comparison_Chain",
                        ),
                        TypeInformation::Boolean => unreachable!(),
                        TypeInformation::String(_) => unreachable!(),
                    };
                    bool_values.push(bool_value);
                    left = right;
                }

                // 1 == 2 == 3
                // at this point we have the result of 1 == 2, 2 == 3

                bool_values
                    .into_iter()
                    .reduce(|left, right| {
                        self.builder.build_and(left, right, "Comparison_Chain_And")
                    })
                    .unwrap()
                    .as_basic_value_enum()
            }
            ast::Expression::Var(_, ref name) => {
                let function_context = self.function_context.as_ref().unwrap();
                let stack_ptr = function_context.var_pointers.get(name).unwrap();

                match exp.type_info() {
                    TypeInformation::Number
                    | TypeInformation::Boolean
                    | TypeInformation::String(_) => self.builder.build_load(*stack_ptr, "Var_Load"),
                }
            },
            ast::Expression::PrefixExpression { op, expression, .. } => {
                let value = self.compile_expression(expression);
                match expression.type_info() {
                    TypeInformation::Boolean => match op {
                        ast::PrefixOprator::Not => self.builder.build_not(value.into_int_value(), "Not").as_basic_value_enum()
                    },
                    _ => unreachable!()
                }
            }
        }
    }

    fn compile_printf(&self, format_string: &str, value: BasicValueEnum) {
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
            value.into(),
        ];

        self.builder
            .build_call(printf_function, &printf_arguments, "Print_Statement");
    }

    fn compile_print_number(&self, value: BasicValueEnum) {
        self.compile_printf("%d\n", value);
    }

    fn compile_print_string(&self, type_: TypeInformation, value: BasicValueEnum) {
        self.compile_printf("%s\n", value);
        self.free_if_needed(value, type_);
    }

    fn compile_print_bool(&self, value: BasicValueEnum) {
        let current_location = self.builder.get_insert_block().unwrap();
        let true_branch = self
            .context
            .insert_basic_block_after(current_location, "True_Branch");
        let false_branch = self
            .context
            .insert_basic_block_after(true_branch, "False_Branch");
        let continue_branch = self
            .context
            .insert_basic_block_after(false_branch, "Continue_Branch");

        // Jump
        self.builder
            .build_conditional_branch(value.into_int_value(), true_branch, false_branch);

        // True
        self.builder.position_at_end(true_branch);
        let true_string = unsafe {
            self.builder
                .build_global_string("true", "True_String")
                .as_pointer_value()
        };
        self.compile_printf("%s\n", true_string.as_basic_value_enum());
        self.builder.build_unconditional_branch(continue_branch);

        // False
        self.builder.position_at_end(false_branch);
        let false_string = unsafe {
            self.builder
                .build_global_string("false", "False_String")
                .as_pointer_value()
        };
        self.compile_printf("%s\n", false_string.as_basic_value_enum());
        self.builder.build_unconditional_branch(continue_branch);

        // Continue
        self.builder.position_at_end(continue_branch);
    }

    fn compile_print(&self, expression: &ast::Expression) {
        let value = self.compile_expression(expression);
        let type_ = expression.type_info();

        match type_ {
            TypeInformation::Number => self.compile_print_number(value),
            TypeInformation::String(_) => self.compile_print_string(*type_, value),
            TypeInformation::Boolean => self.compile_print_bool(value),
        }
    }

    fn compile_var_allocations(&mut self) {
        let function_context = self.function_context.as_mut().unwrap();
        for name in function_context.var_types.keys() {
            let type_ = function_context.var_types.get(name).unwrap();

            let pointer = match type_ {
                TypeInformation::Number => {
                    let i32_type = self.context.i32_type();
                    self.builder.build_alloca(i32_type, "Stack_Pointer")
                }
                TypeInformation::Boolean => {
                    let bool_type = self.context.bool_type();
                    self.builder.build_alloca(bool_type, "Stack_Pointer")
                }
                TypeInformation::String(_) => {
                    let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::Generic);
                    let size_t = self.context.i64_type();

                    let stack_pointer = self.builder.build_alloca(i8_ptr_type, "Stack_Pointer");

                    let malloc_function = self.module.get_function("malloc").unwrap();
                    let malloc_arguments = [size_t.const_int(0, false).into()];
                    let heap_pointer =
                        self.builder
                            .build_call(malloc_function, &malloc_arguments, "Heap_Pointer");

                    self.builder.build_store(
                        stack_pointer,
                        heap_pointer.try_as_basic_value().unwrap_left(),
                    );

                    stack_pointer
                }
            };

            function_context.var_pointers.insert(name.clone(), pointer);
        }
    }

    fn compile_assignment(&mut self, name: &str, expr: &ast::Expression) {
        let function_context = self.function_context.as_ref().unwrap();
        let type_ = function_context.var_types.get(name).unwrap();
        let pointer = function_context.var_pointers.get(name).unwrap();

        let expr_value = self.compile_expression(expr);

        match type_ {
            TypeInformation::Number | TypeInformation::Boolean => {
                self.builder.build_store(*pointer, expr_value);
            }
            TypeInformation::String(_) => {
                // Allocate space for new string
                // check is we have a borrowed or owned string
                let existing_heap_pointer = self.builder.build_load(*pointer, "Existing_String");
                let expr_value = self.builder.build_pointer_cast(
                    expr_value.into_pointer_value(),
                    self.context.i8_type().ptr_type(AddressSpace::Generic),
                    "Expr Value",
                );

                match expr.type_info() {
                    TypeInformation::String(true) => {
                        // We own it, lets just use it!
                        // free existing string
                        let free_function = self.module.get_function("free").unwrap();
                        let free_arguments = [existing_heap_pointer.into()];
                        self.builder
                            .build_call(free_function, &free_arguments, "Free_String");

                        // store new pointer
                        self.builder.build_store(*pointer, expr_value);
                    }
                    TypeInformation::String(false) => {
                        // get size of new string
                        let strlen_function = self.module.get_function("strlen").unwrap();
                        let string_length = self
                            .builder
                            .build_call(strlen_function, &[expr_value.into()], "String_Length")
                            .try_as_basic_value()
                            .unwrap_left();

                        // Make sure allocated space is large enough
                        let realloc_function = self.module.get_function("realloc").unwrap();
                        let heap_pointer = self
                            .builder
                            .build_call(
                                realloc_function,
                                &[existing_heap_pointer.into(), string_length.into()],
                                "Heap_Pointer",
                            )
                            .try_as_basic_value()
                            .unwrap_left();

                        // Copy string into heap
                        let memcpy_function = self.module.get_function("memcpy").unwrap();
                        self.builder.build_call(
                            memcpy_function,
                            &[heap_pointer.into(), expr_value.into(), string_length.into()],
                            "Memcpy",
                        );

                        // Store new pointer
                        self.builder.build_store(*pointer, heap_pointer);
                    }
                    _ => unreachable!("Should always be string type"),
                }
            }
        }
    }

    fn free_used_vars(&self) {
        let function_context = self.function_context.as_ref().unwrap();
        let free_function = self.module.get_function("free").unwrap();
        for name in function_context.var_pointers.keys() {
            let type_ = function_context.var_types.get(name).unwrap();
            let pointer = function_context.var_pointers.get(name).unwrap();

            match type_ {
                TypeInformation::Number | TypeInformation::Boolean => {}
                TypeInformation::String(_) => {
                    let heap_pointer = self.builder.build_load(*pointer, "HeapPointer");
                    self.builder
                        .build_call(free_function, &[heap_pointer.into()], "Free_String");
                }
            }
        }
    }

    fn compile_return(&self, expr: &ast::Expression) {
        self.free_used_vars();

        let type_ = expr.type_info();
        let value = self.compile_expression(expr);

        match type_ {
            TypeInformation::Number | TypeInformation::Boolean | TypeInformation::String(true) => {
                self.builder.build_return(Some(&value));
            }
            TypeInformation::String(false) => {
                let value = self.get_owned_string(value);
                self.builder.build_return(Some(&value));
            }
        }
    }

    fn compile_assert(&self, expr: &ast::Expression) {
        let abort = self.module.get_function("abort").unwrap();
        let printf = self.module.get_function("printf").unwrap();

        let expr_value = self.compile_expression(expr).into_int_value();
        let line_num = expr.location().line_start;

        let current_block = self.builder.get_insert_block().unwrap();
        let abort_block = self
            .context
            .insert_basic_block_after(current_block, &format!("{}L_Assert_Abort", line_num));
        let success_block = self
            .context
            .insert_basic_block_after(abort_block, &format!("{}L_Assert_Success", line_num));

        self.builder
            .build_conditional_branch(expr_value, success_block, abort_block);

        // Crash and burn
        self.builder.position_at_end(abort_block);

        let format_string = unsafe {
            self.builder
                .build_global_string("%s\n", "Assert_Msg_Format_String")
        };
        let msg_string = unsafe {
            self.builder
                .build_global_string(
                    &format!("Assert on line {} failed", line_num),
                    "Assert_Msg_String",
                )
                .as_pointer_value()
        };
        let printf_arguments = [
            self.builder
                .build_pointer_cast(
                    format_string.as_pointer_value(),
                    self.context.i8_type().ptr_type(AddressSpace::Generic),
                    "Format_String",
                )
                .into(),
            msg_string.into(),
        ];
        self.builder
            .build_call(printf, &printf_arguments, "Assert_Printf");
        self.builder
            .build_call(abort, &[], &format!("{}L_Assert_Abort_Call", line_num));
        self.builder.build_unreachable();

        // Continue to build on the success branch
        self.builder.position_at_end(success_block);
    }

    fn compile_test(&mut self, name: &str, expr: &ast::Expression) {
        // lets prefix the name with the current file
        let padding_length = 20 - (3 + name.len());
        let name = format!(
            "\x1b[36m{}{}\x1b[33m{}\x1b[0m",
            &name,
            " ".repeat(padding_length),
            self.module
                .get_source_file_name()
                .to_str()
                .unwrap()
                .to_owned()
        );

        let abort = self.module.get_function("abort").unwrap();
        let printf = self.module.get_function("printf").unwrap();

        let expr_value = self.compile_expression(expr).into_int_value();
        let line_num = expr.location().line_start;

        let current_block = self.builder.get_insert_block().unwrap();
        let fail_block = self
            .context
            .insert_basic_block_after(current_block, &format!("{}L_Test_Fail", line_num));
        let success_block = self
            .context
            .insert_basic_block_after(fail_block, &format!("{}L_Test_Ok", line_num));

        self.builder
            .build_conditional_branch(expr_value, success_block, fail_block);

        let format_string = unsafe {
            self.builder
                .build_global_string("%s\n", "Test_Msg_Format_String")
        };

        // Crash and burn
        self.builder.position_at_end(fail_block);

        let msg_string = unsafe {
            self.builder
                .build_global_string(
                    &format!("\x1b[31mFAILED {}\x1b[0m", name),
                    "Test_Fail_String",
                )
                .as_pointer_value()
        };
        let printf_arguments = [
            self.builder
                .build_pointer_cast(
                    format_string.as_pointer_value(),
                    self.context.i8_type().ptr_type(AddressSpace::Generic),
                    "Format_String",
                )
                .into(),
            msg_string.into(),
        ];
        self.builder
            .build_call(printf, &printf_arguments, "Test_Fail_Printf");
        self.builder
            .build_call(abort, &[], &format!("{}L_Test_Abort_Call", line_num));
        self.builder.build_unreachable();

        // Continue to build on the success branch
        self.builder.position_at_end(success_block);

        let msg_string = unsafe {
            self.builder
                .build_global_string(&format!("\x1b[32mOK {}\x1b[0m", name), "Test_Ok_String")
                .as_pointer_value()
        };
        let printf_arguments = [
            self.builder
                .build_pointer_cast(
                    format_string.as_pointer_value(),
                    self.context.i8_type().ptr_type(AddressSpace::Generic),
                    "Format_String",
                )
                .into(),
            msg_string.into(),
        ];
        self.builder
            .build_call(printf, &printf_arguments, "Test_Ok_Printf");
    }

    fn compile_if(
        &mut self,
        condition: &Expression,
        then: &ast::CodeBody,
        otherwise: &ast::CodeBody,
    ) {
        let current_block = self.builder.get_insert_block().unwrap();
        let true_case = self
            .context
            .insert_basic_block_after(current_block, "True_Case");
        let false_case = self
            .context
            .insert_basic_block_after(true_case, "False_Case");
        let continue_block = self
            .context
            .insert_basic_block_after(false_case, "Continue");

        let condition_result = self.compile_expression(condition);
        self.builder.build_conditional_branch(
            condition_result.into_int_value(),
            true_case,
            false_case,
        );

        // True
        self.builder.position_at_end(true_case);
        self.compile_codeblock(then);
        self.builder.build_unconditional_branch(continue_block);

        // False
        self.builder.position_at_end(false_case);
        self.compile_codeblock(otherwise);
        self.builder.build_unconditional_branch(continue_block);

        // Continue
        self.builder.position_at_end(continue_block);
    }

    fn compile_statement(&mut self, stmt: &ast::Statement) {
        match stmt {
            ast::Statement::Print(expr) => self.compile_print(expr),

            ast::Statement::Assert(expr) => self.compile_assert(expr),
            ast::Statement::Assignment {
                expression_location: _,
                var_name: name,
                expression: exp,
            } => self.compile_assignment(name, exp),
            ast::Statement::Return(expr) => self.compile_return(expr),
            ast::Statement::Test(name, expr) => self.compile_test(name, expr),
            ast::Statement::If {
                condition,
                then,
                otherwise,
            } => self.compile_if(condition, then, otherwise),
        }
    }

    fn compile_function_definition(&self, name: &str, meta: &ast::FunctionMetadata) {
        let return_type = self.get_type_for(meta.return_type.unwrap());
        let arguments = [];

        let function_type = return_type.fn_type(&arguments, false);
        self.module.add_function(name, function_type, None);
    }

    fn compile_function(&mut self, name: &str, code: &ast::CodeBody, meta: ast::FunctionMetadata) {
        let function = self.module.get_function(name).unwrap();

        let entry_block = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(entry_block);

        self.function_context.replace(FunctionContext {
            var_types: meta.var_types,
            var_pointers: HashMap::new(),
        });

        self.compile_var_allocations();
        self.compile_codeblock(code);
    }

    fn compile_codeblock(&mut self, code: &ast::CodeBody) {
        for stmt in &code.0 {
            self.compile_statement(stmt);
        }
    }

    fn compile_toplevel_statement(&mut self, stmt: ast::TopLevelStatement) {
        match stmt {
            ast::TopLevelStatement::FunctionDefinition {
                function_name: name,
                body,
                metadata: meta,
                ..
            } => self.compile_function(&name, &body, meta),
        }
    }

    pub fn compile_code(&mut self, code: ast::File, optimize: bool) {
        // Create clib functions
        self.compile_glibc_definitions();

        for stmt in &code.0 {
            match stmt {
                ast::TopLevelStatement::FunctionDefinition {
                    function_name: name,
                    metadata: meta,
                    ..
                } => self.compile_function_definition(name, meta),
            }
        }
        for stmt in code.0 {
            self.compile_toplevel_statement(stmt);
        }

        if optimize {
            self.fpm.run_on(&self.module);
        }
    }

    pub fn save_in(&self, path: &str) {
        self.module.print_to_file(path).unwrap();
    }
}
