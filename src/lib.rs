#![warn(clippy::pedantic)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    // We often convert to u64
    clippy::cast_sign_loss
)]

pub use parser::SourceLocation;
use std::{os::unix::process::ExitStatusExt, path::PathBuf};

mod analyzers;
mod types;

mod ast;
mod llvm_generator;
mod parser;

type CompilerResult<T> = Result<T, (SourceLocation, String)>;

pub fn report_error(code: &str, err: &(SourceLocation, String)) {
    let traceback = err.0.get_line_highlights(code);
    eprintln!("{}\nERROR: {}", traceback, err.1);
}

pub fn compile_to_ir(name: &str, code: &str, output: &str, optimize: bool) -> CompilerResult<()> {
    let mut ast = parser::parse_file(code)?;

    analyzers::apply_analyzer(&mut ast)?;

    let ctx = llvm_generator::Compiler::create_context();
    let mut compiler = llvm_generator::Compiler::new(name, &ctx);

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

#[must_use]
pub fn run_exe(exe: &str) -> i32 {
    let mut exe = PathBuf::from(exe);

    if exe.is_relative() {
        exe = PathBuf::from(".").join(exe);
    }

    let exit = std::process::Command::new(exe)
        .spawn()
        .unwrap()
        .wait()
        .unwrap();

    exit.code().unwrap_or_else(|| exit.signal().unwrap())
}

/// Lists are given in order first
#[must_use]
pub fn find_exe(possible_names: Vec<&str>) -> Option<PathBuf> {
    for name in possible_names {
        if let Some(path) = find_on_path(name) {
            return Some(path);
        }
    }

    None
}

#[must_use]
fn find_on_path(name: &str) -> Option<PathBuf> {
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
