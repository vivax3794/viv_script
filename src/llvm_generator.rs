use std::collections::HashMap;

use inkwell_llvm12::{
    builder::Builder,
    context::Context,
    module::Module,
    passes::PassManager,
    values::{BasicValue, BasicValueEnum, PointerValue},
    AddressSpace,
};

use crate::types::TypeInformation;

use super::ast::*;

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
    }

    fn compile_literal(&self, lit: &LiteralType) -> BasicValueEnum {
        match lit {
            LiteralType::Number(value) => {
                let i32_type = self.context.i32_type();
                i32_type
                    .const_int(*value as u64, false)
                    .as_basic_value_enum()
            }
            LiteralType::String(value) => {
                let global_string =
                    unsafe { self.builder.build_global_string(value, "Literal_String") };
                let ptr_to_string = global_string.as_pointer_value();
                ptr_to_string.as_basic_value_enum()
            }
        }
    }

    fn compile_expression(&self, exp: &Expression) -> BasicValueEnum {
        match exp {
            Expression::Literal(_, lit) => self.compile_literal(lit),
            Expression::Binary(_, left, op, right) => {
                // only numbers support binary
                let left = self.compile_expression(left).into_int_value();
                let right = self.compile_expression(right).into_int_value();

                match op {
                    Operator::Add => self
                        .builder
                        .build_int_add(left, right, "Number_Add")
                        .as_basic_value_enum(),
                    Operator::Sub => self
                        .builder
                        .build_int_sub(left, right, "Number_Sub")
                        .as_basic_value_enum(),
                    Operator::Mul => self
                        .builder
                        .build_int_mul(left, right, "Number_Mul")
                        .as_basic_value_enum(),
                    Operator::Div => self
                        .builder
                        .build_int_signed_div(left, right, "Number_Div")
                        .as_basic_value_enum(),
                }
            }
            Expression::Var(_, ref name) => {
                let function_context = self.function_context.as_ref().unwrap();
                let stack_ptr = function_context.var_pointers.get(name).unwrap();

                match exp.metadata().type_information.unwrap() {
                    TypeInformation::Number => self.builder.build_load(*stack_ptr, "I32_Load"),
                    TypeInformation::StringBorrow => {
                        self.builder.build_load(*stack_ptr, "Str_Heap_Ptr")
                    }
                    TypeInformation::StringOwned => {
                        unreachable!("A var should always produce a Borrowed string")
                    }
                }
            }
        }
    }

    fn compile_print(&self, to_print: Expression) {
        let value = self.compile_expression(&to_print);

        let format_string = match to_print.metadata().type_information.unwrap() {
            TypeInformation::Number => "%d\n",
            TypeInformation::StringBorrow => "%s\n",
            TypeInformation::StringOwned => "%s\n",
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
            value.into(),
        ];

        self.builder
            .build_call(printf_function, &printf_arguments, "Print_Statement");
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
                TypeInformation::StringBorrow => {
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
                TypeInformation::StringOwned => unreachable!("Var is always a string borrowed."),
            };

            function_context.var_pointers.insert(name.clone(), pointer);
        }
    }

    fn compile_assignment(&mut self, name: String, expr: Expression) {
        let function_context = self.function_context.as_ref().unwrap();
        let type_ = function_context.var_types.get(&name).unwrap();
        let pointer = function_context.var_pointers.get(&name).unwrap();

        let expr_value = self.compile_expression(&expr);

        match type_ {
            TypeInformation::Number => {
                self.builder.build_store(*pointer, expr_value);
            }
            TypeInformation::StringBorrow => {
                // Allocate space for new string
                // check is we have a borrowed or owned string
                let existing_heap_pointer = self.builder.build_load(*pointer, "Existing_String");

                match expr.metadata().type_information.unwrap() {
                    TypeInformation::StringOwned => {
                        // We own it, lets just use it!
                        // free existing string
                        let free_function = self.module.get_function("free").unwrap();
                        let free_arguments = [existing_heap_pointer.into()];
                        self.builder
                            .build_call(free_function, &free_arguments, "Free_String");

                        // store new pointer
                        self.builder.build_store(*pointer, expr_value);
                    }
                    TypeInformation::StringBorrow => {
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
                            &[
                                heap_pointer.into(),
                                expr_value.into(),
                                string_length.into(),
                            ],
                            "Memcpy",
                        );

                        // Store new pointer
                        self.builder.build_store(*pointer, heap_pointer);
                    }
                    _ => unreachable!("Should always be string type"),
                }
            }
            TypeInformation::StringOwned => unreachable!(),
        }
    }

    fn free_used_vars(&self) {
        let function_context = self.function_context.as_ref().unwrap();
        for name in function_context.var_pointers.keys() {
            let type_ = function_context.var_types.get(name).unwrap();
            let pointer = function_context.var_pointers.get(name).unwrap();

            match type_ {
                TypeInformation::Number => {}
                TypeInformation::StringBorrow => {
                    let free_function = self.module.get_function("free").unwrap();
                    let free_arguments = [pointer.as_basic_value_enum().into()];
                    self.builder
                        .build_call(free_function, &free_arguments, "Free_String");
                }
                TypeInformation::StringOwned => unreachable!(),
            }
        }
    }

    fn compile_statement(&mut self, stmt: Statement) {
        match stmt {
            Statement::Print(expr) => self.compile_print(expr),
            Statement::Assignment(_, name, exp) => self.compile_assignment(name, exp),
        }
    }

    pub fn compile_code(
        &mut self,
        code: CodeBody,
        var_types: HashMap<String, TypeInformation>,
        optimize: bool,
    ) {
        // Create clib functions
        self.compile_glibc_definitions();

        // Create main function
        let i32_type = self.context.i32_type();
        let main_argument_types = [];
        let main_function_type = i32_type.fn_type(&main_argument_types, false);
        let main_function = self.module.add_function("main", main_function_type, None);

        let entry_block = self.context.append_basic_block(main_function, "entry");
        self.builder.position_at_end(entry_block);

        self.function_context.replace(FunctionContext {
            var_types,
            var_pointers: HashMap::new(),
        });

        self.compile_var_allocations();
        for stmt in code.0 {
            self.compile_statement(stmt);
        }
        self.free_used_vars();

        self.builder
            .build_return(Some(&i32_type.const_int(0, false)));

        if optimize {
            self.fpm.run_on(&self.module);
        }
    }

    pub fn save_in(&self, path: &str) {
        self.module.print_to_file(path).unwrap();
    }
}
