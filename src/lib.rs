use std::path::PathBuf;

mod ast;
mod llvm_generator;
mod parser;

pub use parser::SourceLocation;
type CompilerResult<T> = Result<T, (SourceLocation, String)>;

pub fn report_error(code: &str, err: (SourceLocation, String)) {
    let traceback = err.0.get_line_highlights(code);
    eprintln!("{}\nERROR: {}", traceback, err.1);
}

pub fn compile_to_ir(name: &str, code: &str, output: &str, optimize: bool) -> CompilerResult<()> {
    let ast = parser::parse_file(code)?;

    let ctx = llvm_generator::Compiler::create_context();
    let compiler = llvm_generator::Compiler::new(name, &ctx);

    compiler.compile_code(ast, optimize);
    compiler.save_in(output);

    Ok(())
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
        .expect("Non zero exit code");
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
        .expect("Non zero exit code");
}

pub fn run_exe(exe: &str) {
    let mut exe = PathBuf::from(exe);

    if exe.is_relative() {
        exe = PathBuf::from(".").join(exe);
    }

    std::process::Command::new(exe)
        .spawn()
        .unwrap()
        .wait()
        .unwrap()
        .success()
        .then_some(())
        .expect("Non zero exit code");
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
