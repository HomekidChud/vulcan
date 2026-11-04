// Embedded self-test: compiles and runs a sample program, verifying output.
use std::process::Command;

const SAMPLE: &str = r#"
struct Vec2 {
    x: f64,
    y: f64,
}

fn len(v: Vec2) -> f64 {
    return sqrt(v.x * v.x + v.y * v.y)
}

fn fib(n: i64) -> i64 {
    if n < 2 {
        return n
    }
    return fib(n - 1) + fib(n - 2)
}

fn main() -> void {
    print("Vulcan selftest")
    let v = Vec2 { x: 3.0, y: 4.0 }
    print("len(3,4) =", len(v))
    print("fib(20) =", fib(20))
    let arr = [1, 1, 2, 3, 5, 8]
    print("arr sum:")
    let total = 0
    for i in 0 .. arr.len() {
        total = total + arr[i]
    }
    print(total)
    let msg = "hello" + " " + "world"
    print(msg)
}
"#;

const EXPECTED: &str = "Vulcan selftest\nlen(3,4) = 5\nfib(20) = 6765\narr sum:\n20\nhello world\n";

pub fn selftest() {
    let dir = std::env::temp_dir().join("vulcan_selftest");
    let _ = std::fs::create_dir_all(&dir);
    let src_path = dir.join("selftest.vulcan");
    std::fs::write(&src_path, SAMPLE).unwrap();
    let c_path = dir.join("selftest.v.c");
    let out_path = dir.join("selftest.vbin");
    let tokens = crate::lexer::lex(SAMPLE);
    let program = crate::parser::parse_program(&tokens);
    let c_src = crate::codegen::generate(&program);
    std::fs::write(&c_path, c_src).unwrap();
    let status = Command::new("gcc")
        .args(["-O3", c_path.to_str().unwrap(), "-lm", "-o", out_path.to_str().unwrap()])
        .output()
        .expect("gcc not found");
    if !status.status.success() {
        eprintln!("selftest compile failed:\n{}", String::from_utf8_lossy(&status.stderr));
        std::process::exit(1);
    }
    let run = Command::new(out_path).output().unwrap();
    let stdout = String::from_utf8_lossy(&run.stdout);
    if stdout != EXPECTED {
        eprintln!("selftest output mismatch!\n--- got ---\n{stdout}\n--- expected ---\n{EXPECTED}");
        std::process::exit(1);
    }
    println!("vulcan selftest: OK (compile + run + verified output)");
}
