use viv_script::Compiler;

const TEST_CODE: &str = include_str!("../test.viv");

fn main() {
    let (rest, ast) = viv_script::code_block(TEST_CODE.into()).unwrap();
    assert_eq!(*rest.fragment(), "");
    let ctx = Compiler::create_context();
    let compiler = Compiler::new(TEST_CODE, &ctx);
    compiler.compile_code(ast);
    compiler.save_in("test.ll");
}
