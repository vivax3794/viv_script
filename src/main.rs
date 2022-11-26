use viv_script::{compile_to_exe, compile_to_ir, compile_to_obj, find_exe, run_exe, report_error};

use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    input_file: String,
    output_file: String,
    #[arg(short, long)]
    ir: bool,
    #[arg(short, long)]
    no_optimize: bool,
    #[arg(short, long)]
    run: bool,
}

fn main() {
    let args = Args::parse();

    let ir_file = temp_file::empty();
    let ir_file = ir_file.path().to_str().unwrap();

    let obj_file = temp_file::empty();
    let obj_file = obj_file.path().to_str().unwrap();

    let exe_file = args.output_file;

    let code = std::fs::read_to_string(&args.input_file).unwrap();

    if args.ir {
        if let Err(err) = compile_to_ir(&args.input_file, &code, &exe_file, !args.no_optimize) {
            report_error(&code, err)
        }
    } else {
        let llc = find_exe(vec!["llc-12", "llc"]).expect("llc binary not found");
        let gcc = find_exe(vec!["clang", "gcc"]).expect("gcc/clang not found on system");

        if let Err(err) = compile_to_ir(&args.input_file, &code, ir_file, !args.no_optimize) {
            report_error(&code, err);
            return;
        }
        compile_to_obj(llc, ir_file, obj_file);
        compile_to_exe(gcc, obj_file, &exe_file);

        if args.run {
            run_exe(&exe_file);
        }
    }
}
