use viv_script::{compile_to_exe, compile_to_ir, compile_to_obj, find_exe};

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    input_file: String,
    output_file: String,
}

fn main() {
    let args = Args::parse();

    let ir_file = temp_file::empty();
    let ir_file = ir_file.path().to_str().unwrap();

    let obj_file = temp_file::empty();
    let obj_file = obj_file.path().to_str().unwrap();

    let exe_file = args.output_file;

    let code = std::fs::read_to_string(&args.input_file).unwrap();
    compile_to_ir(&args.input_file, &code, ir_file);

    let llc = find_exe(vec!["llc-12", "llc"]).expect("llc binary not found");
    let gcc = find_exe(vec!["clang", "gcc"]).expect("gcc/clang not found on system");

    compile_to_obj(llc, ir_file, obj_file);
    compile_to_exe(gcc, obj_file, &exe_file);
}
