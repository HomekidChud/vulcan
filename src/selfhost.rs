// Self-hosting support: `vulcan self` and `vulcan bootstrap`.
//
// Vulcan's compiler is written in Vulcan (self/vulc.vulcan). The Rust
// compiler in this repo is the bootstrap "stage 0": its only job is to
// compile vulc.vulcan once. After that, the language builds itself:
//
//   stage0 (Rust)  --compiles-->  stage1 vulc (C, built by Rust)
//   stage1 vulc    --compiles-->  stage2 vulc (built by stage1)
//   stage2 vulc    --compiles-->  stage3 vulc (built by stage2)
//
// A build is "self-hosted" when stage2 and stage3 agree byte-for-byte
// (the same fixpoint check rustc uses for its stage2/stage3 comparison).

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::codegen;
use crate::modules;

// Locate the self-hosted compiler source (self/vulc.vulcan).
pub fn find_self_dir() -> Option<PathBuf> {
    let mut cands: Vec<PathBuf> = vec![PathBuf::from("self")];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(d) = exe.parent() {
            cands.push(d.join("self"));
            cands.push(d.join("..").join("self"));
            cands.push(d.join("..").join("..").join("self"));
        }
    }
    for c in cands {
        if c.join("vulc.vulcan").exists() {
            return Some(c);
        }
    }
    None
}

fn pick_cc() -> String {
    for cc in ["clang", "gcc", "cc"] {
        if Command::new(cc).arg("--version").output().is_ok() {
            return cc.into();
        }
    }
    "cc".into()
}

fn cc(c_path: &Path, out: &Path) {
    let cc = std::env::var("VULCAN_CC").unwrap_or_else(|_| pick_cc());
    let status = Command::new(&cc)
        .args(["-O2", "-fno-strict-aliasing"])
        .arg(c_path)
        .arg("-lm")
        .arg("-o").arg(out)
        .status()
        .unwrap_or_else(|e| panic!("vulcan: cannot run C compiler `{cc}`: {e}"));
    if !status.success() {
        eprintln!("vulcan: C compile failed for {}", c_path.display());
        std::process::exit(1);
    }
}

// Where build artifacts go (target/self/ next to the repo, else a temp dir).
fn build_dir(self_dir: &Path) -> PathBuf {
    let cand = self_dir.parent().map(|p| p.join("target").join("self"));
    match cand {
        Some(d) if std::fs::create_dir_all(&d).is_ok() => d,
        _ => std::env::temp_dir().join("vulcan-self"),
    }
}

// Emit the self-hosted compiler's C source using the Rust (stage 0) compiler.
fn emit_vulc_c(self_dir: &Path, c_path: &Path) {
    let program = modules::load_program(&self_dir.join("vulc.vulcan"));
    let c_src = codegen::generate_with_libs(&program).0;
    std::fs::write(c_path, c_src).unwrap();
}

// Build (or reuse) the stage-1 vulc binary: compiled by the Rust compiler.
// Rebuilt whenever self/vulc.vulcan is newer than the cached binary.
pub fn stage1_binary(self_dir: &Path) -> PathBuf {
    let bdir = build_dir(self_dir);
    let bin = bdir.join("vulc-stage1");
    let c_path = bdir.join("vulc-stage1.c");
    let src = self_dir.join("vulc.vulcan");
    let stale = match bin.metadata().and_then(|m| m.modified()) {
        Ok(bin_time) => match src.metadata().and_then(|m| m.modified()) {
            Ok(src_time) => src_time > bin_time,
            Err(_) => true,
        },
        Err(_) => true,
    };
    if stale {
        eprintln!("vulcan: building stage-1 self compiler (Rust -> C)...");
        emit_vulc_c(self_dir, &c_path);
        cc(&c_path, &bin);
    }
    bin
}

// `vulcan self <file.vulcan> [args...]` — compile and run a program with the
// self-hosted compiler (vulc compiles it to C, cc builds it, we run it).
pub fn self_cmd(file: &str, prog_args: &[String]) {
    let self_dir = find_self_dir()
        .unwrap_or_else(|| { eprintln!("vulcan: cannot find self/vulc.vulcan (self-hosted compiler source)"); std::process::exit(1); });
    if !Path::new(file).exists() {
        eprintln!("vulcan: file not found: {file}");
        std::process::exit(1);
    }
    let stage1 = stage1_binary(&self_dir);
    let bdir = build_dir(&self_dir);
    let stem = Path::new(file).file_stem().unwrap().to_string_lossy().into_owned();
    let c_path = bdir.join(format!("{stem}.self.c"));
    let bin = bdir.join(format!("{stem}.self"));

    // vulc <input> <output.c>
    let status = Command::new(&stage1).arg(file).arg(&c_path).status()
        .unwrap_or_else(|e| { eprintln!("vulcan: cannot run {}: {e}", stage1.display()); std::process::exit(1); });
    if !status.success() { std::process::exit(1); }
    cc(&c_path, &bin);
    let run_path = std::fs::canonicalize(&bin).unwrap_or_else(|_| bin.clone());
    let status = Command::new(&run_path).args(prog_args).status()
        .unwrap_or_else(|e| { eprintln!("vulcan: cannot run {}: {e}", run_path.display()); std::process::exit(1); });
    if !status.success() { std::process::exit(status.code().unwrap_or(1)); }
}

// `vulcan bootstrap` — prove the language builds itself:
//   1. stage1 (Rust-built) compiles vulc.vulcan -> stage2.c, cc -> stage2
//   2. stage2 compiles vulc.vulcan -> stage3.c
//   3. stage2.c and stage3.c must be byte-identical (the fixpoint)
//   4. smoke-test: stage2 compiles and runs self/test/smoke.vulcan,
//      output must match the Rust compiler's output for the same file
pub fn bootstrap_cmd() {
    let self_dir = find_self_dir()
        .unwrap_or_else(|| { eprintln!("vulcan: cannot find self/vulc.vulcan (self-hosted compiler source)"); std::process::exit(1); });
    let bdir = build_dir(&self_dir);
    let t0 = std::time::Instant::now();

    // stage 1: built by the Rust compiler
    let stage1 = stage1_binary(&self_dir);
    println!("stage 1: vulc built by the Rust compiler ({})", stage1.display());

    // stage 2: stage1 compiles vulc.vulcan
    let s2c = bdir.join("vulc-stage2.c");
    run(&stage1, &[self_dir.join("vulc.vulcan"), s2c.clone()]);
    let stage2 = bdir.join("vulc-stage2");
    cc(&s2c, &stage2);
    println!("stage 2: vulc built by stage-1 vulc ({})", stage2.display());

    // stage 3: stage2 compiles vulc.vulcan again
    let s3c = bdir.join("vulc-stage3.c");
    run(&stage2, &[self_dir.join("vulc.vulcan"), s3c.clone()]);

    // the fixpoint check: stage2 and stage3 must agree byte-for-byte
    let s2 = std::fs::read(&s2c).expect("read stage2.c");
    let s3 = std::fs::read(&s3c).expect("read stage3.c");
    if s2 != s3 {
        eprintln!("bootstrap FAILED: stage2.c and stage3.c differ ({} vs {} bytes)",
                  s2.len(), s3.len());
        std::process::exit(1);
    }
    println!("fixpoint: stage2.c == stage3.c ({} bytes) — the compiler reproduces itself exactly", s2.len());

    // smoke test: a user program through the self-hosted compiler
    let smoke = self_dir.join("test").join("smoke.vulcan");
    if smoke.exists() {
        let out_c = bdir.join("smoke.self.c");
        let out_bin = bdir.join("smoke.self");
        run(&stage2, &[smoke.clone(), out_c.clone()]);
        cc(&out_c, &out_bin);
        let self_out = Command::new(&out_bin).output().expect("run smoke");
        let self_txt = String::from_utf8_lossy(&self_out.stdout).into_owned();

        // same program through the Rust compiler
        let program = modules::load_program(&smoke);
        let c_src = codegen::generate_with_libs(&program).0;
        let rc = bdir.join("smoke.rust.c");
        let rbin = bdir.join("smoke.rust");
        std::fs::write(&rc, c_src).unwrap();
        cc(&rc, &rbin);
        let rust_out = Command::new(&rbin).output().expect("run smoke (rust)");
        let rust_txt = String::from_utf8_lossy(&rust_out.stdout).into_owned();

        if self_txt == rust_txt {
            println!("smoke test: self-hosted and Rust compilers agree ({} bytes of output)", self_txt.len());
        } else {
            eprintln!("bootstrap FAILED: smoke test output differs between compilers");
            eprintln!("--- self-hosted ---\n{self_txt}--- rust ---\n{rust_txt}");
            std::process::exit(1);
        }
    }

    println!("bootstrap ok in {:.1}s — Vulcan builds itself", t0.elapsed().as_secs_f64());
}

fn run(bin: &Path, args: &[PathBuf]) {
    let status = Command::new(bin).args(args).status()
        .unwrap_or_else(|e| { eprintln!("vulcan: cannot run {}: {e}", bin.display()); std::process::exit(1); });
    if !status.success() {
        eprintln!("bootstrap FAILED: {} exited with {:?}", bin.display(), status.code());
        std::process::exit(1);
    }
}
