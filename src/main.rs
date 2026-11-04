// Vulcan — a small, statically typed language that compiles to native code
// through a C backend. Designed for Rust-class runtime speed with
// dramatically faster compile times.
mod lexer;
mod parser;
mod codegen;
mod cli;

use std::process::Command;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Vulcan 0.1 — usage:");
        eprintln!("  vulcan run <file.vulcan>    compile + execute");
        eprintln!("  vulcan build <file.vulcan>   emit native binary");
        eprintln!("  vulcan emit <file.vulcan>   show generated C");
        eprintln!("  vulcan bench <file.vulcan>  compile, run, time it");
        std::process::exit(1);
    }
    let cmd = &args[1];
    if cmd == "selftest" {
        cli::selftest();
        return;
    }
    if args.len() < 3 {
        eprintln!("missing file argument");
        std::process::exit(1);
    }
    let path = &args[2];
    let src = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read {path}: {e}");
            std::process::exit(1);
        }
    };
    let tokens = lexer::lex(&src);
    let program = parser::parse_program(&tokens);
    let c_src = codegen::generate(&program);

    let stem = path.trim_end_matches(".vulcan");
    match cmd.as_str() {
        "emit" => print!("{c_src}"),
        "build" => {
            let c_path = format!("{stem}.c");
            std::fs::write(&c_path, &c_src).unwrap();
            compile_c(&c_path, &format!("{stem}"));
            println!("built {stem} (native, -O3)");
        }
        "run" => {
            let c_path = format!("{stem}.v.c");
            std::fs::write(&c_path, &c_src).unwrap();
            compile_c(&c_path, &format!("{stem}.vbin"));
            let status = Command::new(format!("./{stem}.vbin")).status().unwrap();
            if !status.success() {
                std::process::exit(status.code().unwrap_or(1));
            }
        }
        "bench" => {
            let c_path = format!("{stem}.v.c");
            std::fs::write(&c_path, &c_src).unwrap();
            compile_c(&c_path, &format!("{stem}.vbin"));
            let t0 = std::time::Instant::now();
            let status = Command::new(format!("./{stem}.vbin")).status().unwrap();
            let dt = t0.elapsed();
            println!("exit: {:?}  time: {:.3}s", status.code(), dt.as_secs_f64());
        }
        _ => eprintln!("unknown command {cmd}"),
    }
}

fn compile_c(c_path: &str, out: &str) {
    let t0 = std::time::Instant::now();
    let out_flag = format!("-o{out}");
    let status = Command::new("gcc")
        .args(["-O3", "-fno-strict-aliasing", "-Wall", "-Wno-unused-variable", "-Wno-unused-function", c_path, "-lm", &out_flag])
        .status()
        .unwrap_or_else(|e| panic!("gcc failed: {e}"));
    if !status.success() {
        eprintln!("native codegen failed");
        std::process::exit(1);
    }
    let dt = t0.elapsed();
    eprintln!("native compile: {:.0} ms", dt.as_secs_f64() * 1000.0);
}
