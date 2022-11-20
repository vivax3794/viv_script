use std::collections::HashMap;

use inkwell_llvm12::{
    attributes::{Attribute, AttributeLoc},
    basic_block::BasicBlock,
    builder::Builder,
    context::Context,
    module::Module,
    passes::PassManager,
    values::{IntValue, PointerValue},
    AddressSpace,
};

use std::cell::RefCell;

use super::ast::*;

enum PointerOwnership {
    Owned,
    Borrow,
}

#[derive(Clone, Copy)]
enum Var<'ctx> {
    Number(PointerValue<'ctx>),
    String(PointerValue<'ctx>),
}

enum Value<'ctx> {
    Number(IntValue<'ctx>),
    String(PointerValue<'ctx>, PointerOwnership),
}

struct FunctionContext<'ctx> {
    /// We also need to modify this over time, but we want to keep the compiler as imutabal references only
    vars: HashMap<String, Var<'ctx>>,
    current_block: BasicBlock<'ctx>,
    /// This is where stuff that should only run ONCE is defined,
    /// Like stack allocations for used variables.
    allocate_block: BasicBlock<'ctx>,
}

pub struct Compiler<'code, 'ctx> {
    raw_code: &'code str,
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    fpm: PassManager<Module<'ctx>>,
    function_context: RefCell<Option<FunctionContext<'ctx>>>,
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
            raw_code: code,
            context,
            module,
            builder,
            fpm,
            function_context: RefCell::new(None),
        }
    }

    fn compile_glibc_definitions(&self) {
        // types
        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::Generic);
        let i32_type = self.context.i32_type();
        let size_type = self.context.i64_type();
        let void_type = self.context.void_type();

        let no_inline = self.context.create_string_attribute("noinline", "");

        // printf
        let printf_argument_types = [i8_ptr_type.into()];
        let printf_function_type = i32_type.fn_type(&printf_argument_types, true);
        self.module
            .add_function("printf", printf_function_type, None)
            .add_attribute(AttributeLoc::Function, no_inline);

        // void* malloc( size_t size );
        let malloc_argument_types = [size_type.into()];
        let malloc_function_type = i8_ptr_type.fn_type(&malloc_argument_types, false);
        self.module
            .add_function("malloc", malloc_function_type, None)
            .add_attribute(AttributeLoc::Function, no_inline);

        // void free( void* ptr );
        let free_argument_types = [i8_ptr_type.into()];
        let free_function_type = void_type.fn_type(&free_argument_types, false);
        self.module
            .add_function("free", free_function_type, None)
            .add_attribute(AttributeLoc::Function, no_inline);

        // size_t strlen( const char *str );
        let strlen_argument_types = [i8_ptr_type.into()];
        let strlen_function_type = size_type.fn_type(&strlen_argument_types, false);
        self.module
            .add_function("strlen", strlen_function_type, None)
            .add_attribute(AttributeLoc::Function, no_inline);

        // char *strcpy( char *dest, const char *src );
        let strcpy_argument_types = [i8_ptr_type.into(), i8_ptr_type.into()];
        let strcpy_function_type = i8_ptr_type.fn_type(&strcpy_argument_types, false);
        self.module
            .add_function("strcpy", strcpy_function_type, None)
            .add_attribute(AttributeLoc::Function, no_inline);

        // void* memcpy( void *dest, const void *src, size_t count );
        let memcpy_argument_types = [i8_ptr_type.into(), i8_ptr_type.into(), size_type.into()];
        let memcpy_function_type = i8_ptr_type.fn_type(&memcpy_argument_types, false);
        self.module
            .add_function("memcpy", memcpy_function_type, None)
            .add_attribute(AttributeLoc::Function, no_inline);
    }

    fn compile_literal(&self, lit: LiteralType) -> Value {
        match lit {
            LiteralType::Number(value) => {
                let i32_type = self.context.i32_type();
                Value::Number(i32_type.const_int(value as u64, false))
            }
            LiteralType::String(value) => {
                let global_string =
                    unsafe { self.builder.build_global_string(&value, "Literal_String") };
                let ptr_to_string = global_string.as_pointer_value();
                Value::String(ptr_to_string, PointerOwnership::Borrow)
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
                        Operator::Div => {
                            self.builder.build_int_signed_div(left, right, "Number_Div")
                        }
                    }),
                    _ => panic!("Can not these types of values!"),
                }
            }
            Expression::Var(name) => {
                let function_context = self.function_context.borrow();
                let function_context = function_context.as_ref().unwrap();
                let var = function_context.vars.get(&name).expect("Var not found");

                match var {
                    Var::Number(stack_ptr) => Value::Number(
                        self.builder
                            .build_load(*stack_ptr, "I32_Load")
                            .into_int_value(),
                    ),
                    Var::String(stack_ptr) => {
                        let heap_ptr = self.builder.build_load(*stack_ptr, "Str_Heap_Ptr");
                        Value::String(heap_ptr.into_pointer_value(), PointerOwnership::Borrow)
                    }
                }
            }
        }
    }

    fn compile_print(&self, to_print: Expression) {
        let value_hinted = self.compile_expression(to_print);
        let value = match value_hinted {
            Value::Number(val) => val.into(),
            Value::String(val, _) => val.into(),
        };

        let format_string = match value_hinted {
            Value::Number(_) => "%d\n",
            Value::String(_, _) => "%s\n",
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

    fn compile_assignment_store_value(&self, var: &Var, value: Value) {
        let malloc = self.module.get_function("malloc").unwrap();
        let free = self.module.get_function("free").unwrap();

        match value {
            Value::Number(value) => {
                let stack_ptr = match var {
                    Var::Number(ptr) => *ptr,
                    _ => panic!("this variable is of type number"),
                };
                self.builder.build_store(stack_ptr, value);
            }
            Value::String(value, ownership) => {
                let stack_ptr = match var {
                    Var::String(ptr) => *ptr,
                    _ => panic!("this variable is of type string"),
                };

                // used functions and types
                let size_type = self.context.i64_type();
                let strlen = self.module.get_function("strlen").unwrap();
                let memcpy = self.module.get_function("memcpy").unwrap();

                // Get owned pointer to string
                let heap_ptr = match ownership {
                    PointerOwnership::Owned => {
                        // This value is not used anywhere else and we can safly take ownership of it
                        value
                    }
                    PointerOwnership::Borrow => {
                        // We dont own this string so we are gonna make a copy of it!
                        let i8_ptr_type = self.context.i8_type().ptr_type(AddressSpace::Generic);

                        // get lenght
                        let strln_arguments = [self
                            .builder
                            .build_pointer_cast(value, i8_ptr_type, "Str_Len_Arg")
                            .into()];
                        let lenght =
                            self.builder
                                .build_call(strlen, &strln_arguments, "String_Lenght");
                        let lenght = lenght.try_as_basic_value().left().unwrap().into_int_value();
                        // strlen does not include the null string 
                        let lenght = self.builder.build_int_add(
                            lenght,
                            size_type.const_int(1, false),
                            "String_Space",
                        );

                        // Allocate space for string
                        let malloc_arguments = [lenght.into()];
                        let heap_ptr =
                            self.builder
                                .build_call(malloc, &malloc_arguments, "Heap_Pointer");
                        let heap_ptr = heap_ptr
                            .try_as_basic_value()
                            .left()
                            .unwrap()
                            .into_pointer_value();

                        // Copy string
                        let memcpy_arguments = [
                            self.builder
                                .build_pointer_cast(heap_ptr, i8_ptr_type, "Strcpy_Dest")
                                .into(),
                            self.builder
                                .build_pointer_cast(value, i8_ptr_type, "Strcpy_Src")
                                .into(),
                            lenght.into(),
                        ];
                        self.builder.build_call(memcpy, &memcpy_arguments, "Memcpy");

                        heap_ptr
                    }
                };

                // Free current pointer
                // We did the above first incase it is a pointer to the same string (might happen for some reason if you did `x = x` :P)
                // We could optimize that to a no-op, but if you are doing that you are asking to get bad code xD
                let current_pointer = self
                    .builder
                    .build_load(stack_ptr, "Current_Str")
                    .into_pointer_value();
                let free_arguments = [current_pointer.into()];
                self.builder
                    .build_call(free, &free_arguments, "Free_Current_Str");

                // Store new string
                self.builder.build_store(stack_ptr, heap_ptr);
            }
        };
    }

    fn compile_assignment(&self, name: String, exp: Expression) {
        // The expression might need the function context
        let value = self.compile_expression(exp);

        let mut function_context = self.function_context.borrow_mut();
        let function_context = function_context.as_mut().unwrap();

        let var = match function_context.vars.get(&name) {
            Some(var) => *var,
            None => {
                // We need to allocate this, but we cant do it at this position because reasons
                self.builder
                    .position_at_end(function_context.allocate_block);

                let var = match value {
                    Value::Number(_) => {
                        // we dont need to preload it with anything
                        Var::Number(
                            self.builder
                                .build_alloca(self.context.i32_type(), "I32_Stack_Pointer"),
                        )
                    }
                    Value::String(_, _) => {
                        // we need to malloc something because assigment always frees the current value
                        let malloc = self.module.get_function("malloc").unwrap();
                        let heap_ptr = self.builder.build_call(
                            malloc,
                            &[self.context.i64_type().const_int(0, false).into()],
                            "Temp_Heap_Ptr",
                        );
                        let heap_ptr = heap_ptr
                            .try_as_basic_value()
                            .unwrap_left()
                            .into_pointer_value();

                        let stack_ptr = self.builder.build_alloca(
                            self.context.i8_type().ptr_type(AddressSpace::Generic),
                            "Str_Stack_Pointer",
                        );

                        self.builder.build_store(stack_ptr, heap_ptr);
                        self.builder.position_at_end(function_context.current_block);

                        Var::String(stack_ptr)
                    }
                };

                self.builder.position_at_end(function_context.current_block);

                function_context.vars.insert(name, var);
                var
            }
        };
        self.compile_assignment_store_value(&var, value);
    }

    fn free_used_vars(&self) {
        let mut function_context = self.function_context.borrow_mut();
        let function_context = function_context.as_mut().unwrap();

        let free = self.module.get_function("free").unwrap();

        for var in function_context.vars.values() {
            match var {
                Var::Number(_) => (),
                Var::String(stack_ptr) => {
                    let heap_ptr = self.builder.build_load(*stack_ptr, "Str_Heap_Ptr");
                    let free_arguments = [heap_ptr.into()];
                    self.builder.build_call(free, &free_arguments, "Free_Str");
                }
            }
        }
    }

    fn compile_statement(&self, stmt: Statement) {
        match stmt {
            Statement::Print(expr) => self.compile_print(expr),
            Statement::Assignment(name, exp) => self.compile_assignment(name, exp),
        }
    }

    pub fn compile_code(&self, code: CodeBody, optimize: bool) {
        // Create clib functions
        self.compile_glibc_definitions();

        // Create main function
        let i32_type = self.context.i32_type();
        let main_argument_types = [];
        let main_function_type = i32_type.fn_type(&main_argument_types, false);
        let main_function = self.module.add_function("main", main_function_type, None);

        let entry_block = self.context.append_basic_block(main_function, "entry");
        let code_block = self.context.append_basic_block(main_function, "code");
        self.builder.position_at_end(code_block);

        self.function_context.replace(Some(FunctionContext {
            vars: HashMap::new(),
            allocate_block: entry_block,
            current_block: code_block,
        }));

        for stmt in code.statements {
            self.compile_statement(stmt);
        }

        self.free_used_vars();

        self.builder
            .build_return(Some(&i32_type.const_int(0, false)));

        // We need to add this at the end of the block
        // We could get away with not having two of the block if we could easialy add to the start
        // but the IR optimizer will optimize this away anyway :D 
        self.builder.position_at_end(entry_block);
        self.builder.build_unconditional_branch(code_block);

        if optimize {
            self.fpm.run_on(&self.module);
        }
    }

    pub fn save_in(&self, path: &str) {
        self.module.print_to_file(path).unwrap();
    }
}
