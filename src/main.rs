use std::process::exit;

use viv_script::{compile_to_exe, compile_to_ir, compile_to_obj, find_exe, report_error, run_exe};

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long)]
    no_optimize: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
#[command()]
enum Command {
    Build {
        input_file: String,
        output_file: String,
    },
    Run {
        input_file: String,
    },
    Ir {
        input_file: String,
        output_fie: String,
    },
}

fn ir(optimize: bool, input_file: &str, output_file: &str) {
    let code = std::fs::read_to_string(input_file).unwrap();
    if let Err(err) = compile_to_ir(input_file, &code, output_file, optimize) {
        report_error(&code, &err);
        // This is not good error handling, but :P
        exit(1);
    }
}

fn build(optimize: bool, input_file: &str, output_file: &str) {
    let ir_file = temp_file::empty();
    let ir_file = ir_file.path().to_str().unwrap();

    let obj_file = temp_file::empty();
    let obj_file = obj_file.path().to_str().unwrap();

    ir(optimize, input_file, ir_file);

    let llc = find_exe(&["llc-14", "llc"]).expect("llc binary not found");
    let gcc = find_exe(&["clang", "gcc"]).expect("gcc/clang not found on system");

    compile_to_obj(llc, ir_file, obj_file);
    compile_to_exe(gcc, obj_file, output_file);
}

fn run(optimize: bool, input_file: &str) -> i32 {
    let exe_file = temp_file::empty();
    let exe_file = exe_file.path().to_str().unwrap();

    build(optimize, input_file, exe_file);
    run_exe(exe_file)
}

fn main() {
    let args = Args::parse();

    match args.command {
        Command::Build {
            input_file,
            output_file,
        } => build(!args.no_optimize, &input_file, &output_file),
        Command::Run { input_file } => exit(run(!args.no_optimize, &input_file)),
        Command::Ir {
            input_file,
            output_fie,
        } => ir(!args.no_optimize, &input_file, &output_fie),
    }
}
