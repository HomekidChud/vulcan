// Vulcan — a small, statically typed language that compiles to native code
// through a C backend. Designed for Rust-class runtime speed with
// dramatically faster compile times.
mod lexer;
mod parser;
mod codegen;
mod sema;
mod diag;
mod codegen_js;
mod codegen_py;
mod cli;
mod modules;
mod pkg;
mod package;
mod ide;
mod selfhost;

use std::path::{Path, PathBuf};
use std::process::Command;

// Single source of truth for the version: Cargo.toml (via CARGO_PKG_VERSION).
// The README, usage banner and `vulcan --version` all come from here.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

fn usage() {
    println!("Vulcan {VERSION} — a small, fast language with multiple backends,
a 2D physics engine and a browser IDE

USAGE (single file):
  vulcan run <file.vulcan>     compile + execute
  vulcan build <file.vulcan>  emit native binary (C backend)
  vulcan emit <file.vulcan>   show generated code
  vulcan bench <file.vulcan>  compile, run, time it
  vulcan package <file.vulcan> --platform <p>   ship it: web, desktop, android, ios,
                               mobile (iOS+Android), linux, windows, macos
  vulcan apk <file.vulcan>    build an installable Android APK (Termux-native
                               pipeline: aapt + ecj + d8 + apksigner, no Gradle)
  vulcan check <file.vulcan>  type-check without building (file:line:col errors)
  vulcan test <file.vulcan>   run every fn test_*() in the file
  vulcan doc <file|dir>       print the public API (structs, fns, consts)

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

USAGE (IDE):
  vulcan ide [port]           launch the browser IDE (editor + live canvas,
                               physics engine ready via `import physics`)

SELF-HOSTING (the compiler is written in Vulcan — self/vulc.vulcan):
  vulcan self <file.vulcan>   compile + run a program with the self-hosted
                               compiler (vulc compiles it, no cargo involved)
  vulcan bootstrap            verify the language builds itself: stage1
                               (Rust-built) -> stage2 -> stage3, with a
                               byte-identical fixpoint check

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
    if cmd == "self" {
        if args.len() < 3 {
            eprintln!("usage: vulcan self <file.vulcan> [args...] — compile + run with the self-hosted compiler");
            std::process::exit(1);
        }
        let file = args[2].clone();
        let prog_args: Vec<String> = args[3..].to_vec();
        selfhost::self_cmd(&file, &prog_args);
        return;
    }
    if cmd == "bootstrap" {
        selfhost::bootstrap_cmd();
        return;
    }
    if cmd == "doc" {
        if args.len() < 3 {
            eprintln!("usage: vulcan doc <file.vulcan|dir>");
            std::process::exit(1);
        }
        cli::doc(&args[2]);
        return;
    }
    if cmd == "ide" {
        let port = args.get(2).and_then(|a| a.parse().ok()).unwrap_or(7788);
        ide::serve(port);
        return;
    }
    if matches!(cmd.as_str(), "version" | "--version" | "-V") {
        println!("vulcan {VERSION}");
        return;
    }
    if cmd == "check" {
        if args.len() < 3 {
            eprintln!("usage: vulcan check <file.vulcan>");
            std::process::exit(1);
        }
        check_file(&args[2]);
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
    if cmd == "apk" {
        let file = args.get(2).map(|s| s.as_str())
            .unwrap_or_else(|| { eprintln!("usage: vulcan apk <file.vulcan>"); std::process::exit(1); });
        if !Path::new(file).exists() {
            eprintln!("error: file not found: {file}");
            std::process::exit(1);
        }
        package::apk(file);
        return;
    }
    if cmd == "package" {
        let file = args.get(2).map(|s| s.as_str())
            .unwrap_or_else(|| { eprintln!("usage: vulcan package <file.vulcan> --platform <web|desktop|mobile|android|ios|linux|windows|macos>"); std::process::exit(1); });
        if !Path::new(file).exists() {
            eprintln!("error: file not found: {file}");
            std::process::exit(1);
        }
        package::package(file, &platform);
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
                let status = Command::new(runner).arg(&out_path).args(&prog_args).status()
                    .unwrap_or_else(|e| {
                        eprintln!("vulcan: cannot run `{runner}` ({e}) — is it installed and on your PATH?");
                        std::process::exit(1);
                    });
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
        _ => codegen::generate_with_libs(&program).0,
    };
    let link_libs = crate::codegen::last_link_libs();

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
            compile_c(&c_path, &bin_path, safe, &link_libs);
            println!("built {stem} (native, -O3)");
        }
        "run" | "test" => {
            std::fs::write(&c_path, &c_src).unwrap();
            compile_c(&c_path, &bin_path, safe, &link_libs);
            let status = Command::new(&run_path).args(&prog_args).status()
                    .unwrap_or_else(|e| {
                        eprintln!("vulcan: cannot run {}: {e}", run_path.display());
                        std::process::exit(1);
                    });
            if !status.success() {
                std::process::exit(status.code().unwrap_or(1));
            }
        }
        "bench" => {
            std::fs::write(&c_path, &c_src).unwrap();
            compile_c(&c_path, &bin_path, safe, &link_libs);
            let t0 = std::time::Instant::now();
            let status = Command::new(&run_path).args(&prog_args).status()
                    .unwrap_or_else(|e| {
                        eprintln!("vulcan: cannot run {}: {e}", run_path.display());
                        std::process::exit(1);
                    });
            let dt = t0.elapsed();
            println!("exit: {:?}  time: {:.3}s", status.code(), dt.as_secs_f64());
        }
        _ => unreachable!(),
    }
}

fn compile_c(c_path: &str, out: &str, safe: bool, libs: &[String]) {
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
    // `link "name"` from the program -> -l<name> (e.g. FFI libraries)
    let lib_flags: Vec<String> = libs.iter().map(|l| format!("-l{l}")).collect();
    let status = Command::new(&cc)
        .args(&flags)
        .args(&lib_flags)
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

// `vulcan check <file>` — lex, parse (with imports) and run the semantic
// checks WITHOUT building. Reports in file:line:col format with a caret,
// collecting as many errors as possible per run.
fn check_file(path_str: &str) {
    let path = Path::new(path_str);
    let src = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => { eprintln!("vulcan: cannot read {path_str}: {e}"); std::process::exit(1); }
    };
    let mut diags: Vec<diag::Diag> = Vec::new();

    // 1. lexer errors (collected, not first-abort). If the source does not
    // lex cleanly, parsing it is meaningless — report and stop here.
    let (_, lex_errs) = lexer::lex_diags(&src);
    if !lex_errs.is_empty() {
        eprint!("{}", diag::render(&src, path_str, &lex_errs));
        eprintln!("vulcan check: {} error(s) in {path_str}", lex_errs.len());
        std::process::exit(1);
    }

    // 2. parse + imports + semantic checks; a panic (syntax error, missing
    // import) is converted into a Diag with its line number if we can find one
    let result = {
        let prev_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {})); // silence the default panic printer
        let r = std::panic::catch_unwind(|| {
            let program = modules::load_program(path);
            sema::check_program(&program)
        });
        std::panic::set_hook(prev_hook);
        r
    };
    match result {
        Ok(mut d) => diags.extend(d),
        Err(payload) => {
            let msg = if let Some(s) = payload.downcast_ref::<String>() { s.clone() }
                else if let Some(s) = payload.downcast_ref::<&str>() { s.to_string() }
                else { "unknown error".to_string() };
            let msg = msg.strip_prefix("vulcan: ").unwrap_or(&msg).to_string();
            let line = msg.split("line ").nth(1)
                .and_then(|rest| rest.chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse().ok())
                .unwrap_or(0);
            diags.push(diag::Diag::new(line, 0, msg));
        }
    }

    if diags.is_empty() {
        println!("vulcan check: ok ({path_str})");
    } else {
        eprint!("{}", diag::render(&src, path_str, &diags));
        eprintln!("vulcan check: {} error(s) in {path_str}", diags.len());
        std::process::exit(1);
    }
}
