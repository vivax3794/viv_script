use viv_script::Compiler;

const TEST_CODE: &str = "
print 1 + 2 + 3;
";

fn main() {
    let (_, ast) = viv_script::code_block(TEST_CODE.into()).unwrap();
    let ctx = Compiler::create_context();
    let compiler = Compiler::new(&ctx);
    compiler.compile_code(ast);
    compiler.save_in("test.ll");
}
