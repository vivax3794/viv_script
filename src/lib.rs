use std::path::PathBuf;

mod ast;
mod llvm_generator;
mod parser;

pub fn compile_to_ir(name: &str, code: &str, output: &str, optimize: bool) {
    let (_, ast) = parser::code_block(code).unwrap();
    let ctx = llvm_generator::Compiler::create_context();
    let compiler = llvm_generator::Compiler::new(name, code, &ctx);

    compiler.compile_code(ast, optimize);
    compiler.save_in(output);
}

pub fn compile_to_obj(llc: PathBuf, from: &str, to: &str) {
    std::process::Command::new(llc)
        .args([from, "-filetype=obj", "-o", to])
        .spawn()
        .unwrap()
        .wait()
        .unwrap()
        .success()
        .then_some(())
        .unwrap();
}

pub fn compile_to_exe(gcc: PathBuf, from: &str, to: &str) {
    std::process::Command::new(gcc)
        .args([from, "-no-pie", "-o", to])
        .spawn()
        .unwrap()
        .wait()
        .unwrap()
        .success()
        .then_some(())
        .unwrap();
}

/// Lists are given in order first
pub fn find_exe(posible_names: Vec<&str>) -> Option<PathBuf> {
    for name in posible_names.into_iter() {
        if let Some(path) = is_on_path(name) {
            return Some(path);
        }
    }

    None
}

fn is_on_path(name: &str) -> Option<PathBuf> {
    let path_env = std::env::var("PATH").expect("PATH env var not found!");
    let path_env = std::env::split_paths(&path_env);

    for path in path_env {
        let to_check = path.join(name);
        if to_check.exists() {
            return Some(to_check);
        }
    }

    None
}
