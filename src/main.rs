// Vulcan — a small, statically typed language that compiles to native code
// through a C backend. Designed for Rust-class runtime speed with
// dramatically faster compile times.
mod lexer;
mod parser;
mod codegen;
mod codegen_js;
mod codegen_py;
mod cli;
mod modules;
mod pkg;
mod package;

use std::path::{Path, PathBuf};
use std::process::Command;

fn usage() {
    println!("Vulcan 0.4 — a small, fast language with multiple backends

USAGE (single file):
  vulcan run <file.vulcan>     compile + execute
  vulcan build <file.vulcan>  emit native binary (C backend)
  vulcan emit <file.vulcan>   show generated code
  vulcan bench <file.vulcan>  compile, run, time it
  vulcan package <file.vulcan> --platform <p>   ship it: web, desktop,
                               mobile (Capacitor), android (.apk), linux, windows, macos
  vulcan test <file.vulcan>   run every fn test_*() in the file

add --target js or --target py to run/emit/bench for the JavaScript
(Node.js) or Python backend — same program, same output, different
runtime. --target c (default) compiles to native code via C.

add --safe to run/build/bench/test for a sanitizer build
(AddressSanitizer + UndefinedBehaviorSanitizer: memory errors and
integer overflow abort at the exact line). C backend only.

USAGE (project — when a vulcan.toml is present):
  vulcan init <name>          scaffold a new project
  vulcan add <user/repo|path> add a dependency
  vulcan run | build | test   work on the whole project
  vulcan bench                time the project entry point

OTHER:
  vulcan selftest             run the compiler test suite
");
}

fn main() {
    let mut args: Vec<String> = std::env::args().collect();
    // `--safe` builds with AddressSanitizer + UndefinedBehaviorSanitizer:
    // memory errors, integer overflow and UB abort the program at the exact line.
    let safe = match args.iter().position(|a| a == "--safe") {
        Some(i) => { args.remove(i); true }
        None => false,
    };
    // `--target <c|js|py>` picks the backend (default: c)
    let mut target = String::from("c");
    if let Some(i) = args.iter().position(|a| a == "--target") {
        if i + 1 < args.len() {
            target = args[i + 1].clone();
            args.remove(i);
            args.remove(i);
        } else {
            eprintln!("vulcan: --target needs a value (c, js or py)");
            std::process::exit(1);
        }
    }
    if !matches!(target.as_str(), "c" | "js" | "py") {
        eprintln!("vulcan: unknown target `{target}` (expected c, js or py)");
        std::process::exit(1);
    }
    if args.len() < 2 {
        usage();
        std::process::exit(1);
    }
    let cmd = args[1].clone();
    if cmd == "selftest" {
        cli::selftest();
        return;
    }
    if matches!(cmd.as_str(), "help" | "--help" | "-h") {
        usage();
        return;
    }
    if cmd == "init" {
        let name = args.get(2).map(|s| s.as_str()).unwrap_or("app");
        pkg::init(name);
        return;
    }
    if cmd == "add" {
        let source = args.get(2).map(|s| s.as_str())
            .unwrap_or_else(|| { eprintln!("usage: vulcan add <user/repo | local/path>"); std::process::exit(1); });
        let proj = pkg::find_project(Path::new("."))
            .unwrap_or_else(|| { eprintln!("vulcan: no vulcan.toml found (run `vulcan init` first)"); std::process::exit(1); });
        pkg::add(&proj, source);
        return;
    }
    // `--platform <p>` for the package command
    let mut platform = String::from("web");
    if let Some(i) = args.iter().position(|a| a == "--platform") {
        if i + 1 < args.len() {
            platform = args[i + 1].clone();
            args.remove(i);
            args.remove(i);
        } else {
            eprintln!("vulcan: --platform needs a value (web, desktop, mobile, linux, windows, macos)");
            std::process::exit(1);
        }
    }
    if cmd == "package" {
        let file = args.get(2).map(|s| s.as_str())
            .unwrap_or_else(|| { eprintln!("usage: vulcan package <file.vulcan> --platform <web|web3d|android|desktop|mobile|linux|windows|macos> [--assets dir] [--name n]"); std::process::exit(1); });
        if !Path::new(file).exists() {
            eprintln!("error: file not found: {file}");
            std::process::exit(1);
        }
        let assets = args.iter().position(|a| a == "--assets").and_then(|i| args.get(i + 1)).cloned();
        let name = args.iter().position(|a| a == "--name").and_then(|i| args.get(i + 1)).cloned();
        package::package(file, &platform, assets.as_deref(), name.as_deref());
        return;
    }
    if !matches!(cmd.as_str(), "run" | "build" | "emit" | "bench" | "test") {
        eprintln!("unknown command `{cmd}` — try `vulcan help`");
        std::process::exit(1);
    }

    // figure out entry file + project root
    let (entry, proj_root, pkg_name): (PathBuf, Option<PathBuf>, Option<String>) = match args.get(2) {
        Some(arg) if Path::new(arg).is_dir() => {
            let dir = PathBuf::from(arg);
            let root = if dir.join("vulcan.toml").exists() { Some(dir.clone()) } else { pkg::find_project(&dir) };
            (dir.join("src").join("main.vulcan"), root.clone(), root.map(|r| pkg::read_manifest(&r).name))
        }
        Some(arg) => (PathBuf::from(arg), pkg::find_project(Path::new(arg)).filter(|_| false), None), // explicit file: no manifest
        None => {
            let cwd = PathBuf::from(".");
            match pkg::find_project(&cwd) {
                Some(root) => (root.join("src").join("main.vulcan"), Some(root.clone()), None),
                None => {
                    eprintln!("missing file argument (or run inside a project with vulcan.toml)");
                    std::process::exit(1);
                }
            }
        }
    };
    let pkg_name = pkg_name.or_else(|| proj_root.as_ref().map(|r| pkg::read_manifest(r).name));

    if !entry.exists() {
        eprintln!("error: entry point not found: {}", entry.display());
        std::process::exit(1);
    }

    // extra CLI args after the entry file are forwarded to the program
    let prog_args: Vec<String> = match args.get(2) {
        Some(a) if !Path::new(a).is_dir() => args[3.min(args.len())..].to_vec(),
        _ => Vec::new(),
    };

    let program = modules::load_program(&entry);

    if target != "c" {
        if cmd == "test" {
            eprintln!("vulcan: `test` runs on the C backend (use --target c)");
            std::process::exit(1);
        }
        if cmd == "build" {
            eprintln!("vulcan: `build` produces native binaries — use the C backend (default)");
            std::process::exit(1);
        }
        let (src, ext, runner): (String, &str, &str) = match target.as_str() {
            "js" => (codegen_js::generate_js(&program), "js", "node"),
            "py" => (codegen_py::generate_py(&program), "py", "python3"),
            _ => unreachable!(),
        };
        let out_path = match proj_root {
            Some(root) => {
                let build = root.join("build");
                let _ = std::fs::create_dir_all(&build);
                build.join(format!("{}.{}", stem_name(&entry), ext)).to_string_lossy().into_owned()
            }
            None => format!("{}.{}", entry.to_string_lossy().trim_end_matches(".vulcan"), ext),
        };
        match cmd.as_str() {
            "emit" => print!("{src}"),
            "run" | "bench" => {
                std::fs::write(&out_path, &src).unwrap();
                let t0 = std::time::Instant::now();
                let status = Command::new(runner).arg(&out_path).args(&prog_args).status().unwrap();
                let dt = t0.elapsed();
                if cmd == "bench" {
                    println!("exit: {:?}  time: {:.3}s", status.code(), dt.as_secs_f64());
                }
                if !status.success() {
                    std::process::exit(status.code().unwrap_or(1));
                }
            }
            _ => unreachable!(),
        }
        return;
    }

    let c_src = match cmd.as_str() {
        "test" => {
            let tests: Vec<String> = program.iter().filter_map(|i| match i {
                parser::Item::Fn(f) if f.name.starts_with("test_") => Some(f.name.clone()),
                _ => None,
            }).collect();
            if tests.is_empty() {
                eprintln!("vulcan: no `test_*` functions found in {}", entry.display());
                std::process::exit(1);
            }
            codegen::generate_test(&program, &tests)
        }
        _ => codegen::generate(&program),
    };

    // output locations
    let (c_path, bin_path, stem): (String, String, String) = match (&proj_root, &pkg_name) {
        (Some(root), Some(name)) => {
            let build = root.join("build");
            let _ = std::fs::create_dir_all(&build);
            (build.join(format!("{name}.c")).to_string_lossy().into_owned(),
             build.join(name).to_string_lossy().into_owned(),
             name.clone())
        }
        _ => {
            let s = entry.to_string_lossy();
            let stem = s.trim_end_matches(".vulcan").to_string();
            let cext = if cmd == "build" { ".c" } else { ".v.c" };
            (format!("{stem}{cext}"), format!("{stem}.vbin"), stem.clone())
        }
    };

    // exec'ing a relative path like "main.vbin" would search PATH — make it absolute
    let run_path = std::fs::canonicalize(&bin_path).unwrap_or_else(|_| PathBuf::from(&bin_path));

    match cmd.as_str() {
        "emit" => print!("{c_src}"),
        "build" => {
            std::fs::write(&c_path, &c_src).unwrap();
            compile_c(&c_path, &bin_path, safe);
            println!("built {stem} (native, -O3)");
        }
        "run" | "test" => {
            std::fs::write(&c_path, &c_src).unwrap();
            compile_c(&c_path, &bin_path, safe);
            let status = Command::new(&run_path).args(&prog_args).status().unwrap();
            if !status.success() {
                std::process::exit(status.code().unwrap_or(1));
            }
        }
        "bench" => {
            std::fs::write(&c_path, &c_src).unwrap();
            compile_c(&c_path, &bin_path, safe);
            let t0 = std::time::Instant::now();
            let status = Command::new(&run_path).args(&prog_args).status().unwrap();
            let dt = t0.elapsed();
            println!("exit: {:?}  time: {:.3}s", status.code(), dt.as_secs_f64());
        }
        _ => unreachable!(),
    }
}

fn compile_c(c_path: &str, out: &str, safe: bool) {
    let t0 = std::time::Instant::now();
    let out_flag = format!("-o{out}");
    let cc = std::env::var("VULCAN_CC").unwrap_or_else(|_| pick_cc());
    let mut flags: Vec<&str> = vec![
        "-fno-strict-aliasing", "-Wall", "-Wno-unused-variable", "-Wno-unused-function",
        // hardening: stack canaries, fortified libc calls, full PIE/ASLR, strict format strings,
        // and read-only relocations (GOT hardening where the platform supports it)
        "-fstack-protector-strong", "-D_FORTIFY_SOURCE=2", "-fPIE", "-pie",
        "-Wformat", "-Werror=format-security", "-Wl,-z,relro", "-Wl,-z,now",
        c_path, "-lm", &out_flag,
    ];
    if safe {
        // --safe: sanitizer build — memory errors and UB abort at the exact line
        flags.splice(0..0, ["-O1", "-g", "-fsanitize=address,undefined", "-fno-omit-frame-pointer"]);
    } else {
        flags.insert(0, "-O3");
    }
    let status = Command::new(&cc)
        .args(&flags)
        .status()
        .unwrap_or_else(|e| panic!("C compiler `{cc}` failed: {e}"));
    if !status.success() {
        eprintln!("native codegen failed");
        std::process::exit(1);
    }
    let dt = t0.elapsed();
    eprintln!("native compile: {:.0} ms", dt.as_secs_f64() * 1000.0);
}

fn stem_name(entry: &Path) -> String {
    let s = entry.to_string_lossy();
    s.trim_end_matches(".vulcan").to_string()
}

// Prefer clang (LLVM codegen — measurably faster loops), fall back to gcc.
pub fn pick_cc() -> String {
    static CACHE: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    if let Some(c) = CACHE.get() { return c.clone(); }
    let picked = cc_on_path("clang").or_else(|| cc_on_path("gcc"))
        .unwrap_or_else(|| "gcc".into());
    let _ = CACHE.set(picked.clone());
    picked
}

fn cc_on_path(name: &str) -> Option<String> {
    let paths = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&paths) {
        if dir.join(name).exists() {
            return Some(name.into());
        }
    }
    None
}
