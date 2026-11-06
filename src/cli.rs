// Embedded self-test suite: compiles and runs sample programs, verifying output.
use std::process::Command;

struct Case {
    name: &'static str,
    src: &'static str,
    expected: &'static str,
}

// Physics engine selftest main — concatenated after lib/physics.vulcan
// (imports are just flattened modules, so this is equivalent to `import physics`).
const PHYSICS_TEST_MAIN: &str = r#"

fn main() -> void {
    let w = world_new(0.0, 900.0)
    w = world_add_bounds(w, 0.5, 0.3)
    w = world_add_circle(w, 40.0, 20.0, 6.0, 1.0, 0.5, 0.2)
    w = world_add_box(w, 120.0, 30.0, 10.0, 8.0, 2.0, 0.2, 0.5)
    w = world_add_spring(w, 4, 5, 50.0, 60.0, 2.0)
    print("bodies:", world_body_count(w), "springs:", world_spring_count(w))
    let f = 0
    while f < 120 {
        w = world_step(w, 1.0 / 60.0)
        f += 1
    }
    let i = 0
    while i < world_body_count(w) {
        print(i, body_x(w, i), body_y(w, i))
        i += 1
    }
}
"#;

const PHYSICS_TEST_EXPECTED: &str = "bodies: 6 springs: 1\n0 128 152\n1 -6 72\n2 262 72\n3 128 -8\n4 59.1166 138.092\n5 108.806 136.102\n";

const CASES: &[Case] = &[
    Case {
        name: "basic (0.1 feature set)",
        src: r#"
struct Vec2 { x: f64, y: f64 }

fn len(v: Vec2) -> f64 {
    return sqrt(v.x * v.x + v.y * v.y)
}

fn fib(n: i64) -> i64 {
    if n < 2 { return n }
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
"#,
        expected: "Vulcan selftest\nlen(3,4) = 5\nfib(20) = 6765\narr sum:\n20\nhello world\n",
    },
    Case {
        name: "elif / compound assign / ++ --",
        src: r#"
fn classify(n: i64) -> string {
    if n < 0 { return "negative" }
    elif n == 0 { return "zero" }
    elif n < 10 { return "small" }
    else { return "big" }
}

fn main() -> void {
    print(classify(-5), classify(0), classify(3), classify(99))
    let x = 10
    x += 5
    x -= 3
    x *= 2
    print(x)
    x++
    x++
    print(x)
    x--
    print(x)
    let f = 2.5
    f /= 0.5
    print(f)
}
"#,
        expected: "negative zero small big\n24\n26\n25\n5\n",
    },
    Case {
        name: "for-in / step ranges / loop-break",
        src: r#"
fn main() -> void {
    let xs = [10, 20, 30]
    let total = 0
    for x in xs { total += x }
    print(total)
    for i in 0 .. 10 by 3 { print(i) }
    for i in 0 ..= 4 by 2 { print(i) }
    for i in 10 .. 0 by -2 { print(i) }
    let n = 0
    loop {
        n++
        if n >= 3 { break }
    }
    print("looped", n)
}
"#,
        expected: "60\n0\n3\n6\n9\n0\n2\n4\n10\n8\n6\n4\n2\nlooped 3\n",
    },
    Case {
        name: "match / enums",
        src: r#"
enum Color { Red, Green, Blue }

fn name(c: Color) -> string {
    match c {
        when Color.Red { return "red" }
        when Color.Green { return "green" }
        else { return "blue" }
    }
}

fn main() -> void {
    print(name(Color.Red), name(Color.Green), name(Color.Blue))
    let c = Color.Blue
    match c {
        when Color.Red { print("nope") }
        when Color.Blue, Color.Green { print("cool color") }
    }
    let m = "b"
    match m {
        when "a" { print("letter a") }
        when "b" { print("letter b") }
        else { print("other") }
    }
}
"#,
        expected: "red green blue\ncool color\nletter b\n",
    },
    Case {
        name: "f-strings / consts / string methods",
        src: r#"
const LIMIT: i64 = 42
const APP: string = "vulcan"

fn main() -> void {
    let name = "ada"
    let n = 7
    print(f"{APP} says: {name} scored {n * 6} points")
    print(f"limit is {LIMIT}")
    print("SHOUT".lower(), "quiet".upper())
    print("  padded  ".trim())
    print(f"nested: {[1,2][1]}")
}
"#,
        expected: "vulcan says: ada scored 42 points\nlimit is 42\nshout QUIET\npadded\nnested: 2\n",
    },
    Case {
        name: "defer / assert",
        src: r#"
fn work() -> i64 {
    defer { print("cleanup") }
    print("working")
    return 5
}

fn main() -> void {
    print(work())
    assert(1 + 1 == 2)
    let items = [1, 2, 3]
    assert(items.len() == 3, "array size")
    print("after asserts")
}
"#,
        expected: "working\ncleanup\n5\nafter asserts\n",
    },
    Case {
        name: "bitwise / hex / binary / chars / casts",
        src: r#"
fn main() -> void {
    let flags = 0b1010
    print(flags)
    print(0xFF)
    print(flags & 0b0110)
    print(flags | 1)
    print(flags ^ 0b1111)
    print(flags << 2)
    print(flags >> 1)
    print(~0)
    print('A')
    let x = 3
    let y = x as f64 / 2.0
    print(y)
    print('z' - 'a')
}
"#,
        expected: "10\n255\n2\n11\n5\n40\n5\n-1\n65\n1.5\n25\n",
    },
    Case {
        name: "array methods / min max / pop",
        src: r#"
fn main() -> void {
    let xs = [5, 3, 8]
    print(xs.pop())
    print(xs.len())
    print(xs.last())
    print(xs.contains(3), xs.contains(99))
    print(min(4, 9), max(4, 9))
    print(min(2.5, 1.5), max(2.5, 1.5))
    print(abs(-7), abs(-2.5))
    print(pow(2.0, 10.0))
    print(floor(2.7), ceil(2.1))
}
"#,
        expected: "8\n2\n3\ntrue false\n4 9\n1.5 2.5\n7 2.5\n1024\n2 3\n",
    },
    Case {
        name: "structs with arrays + compound field update",
        src: r#"
struct Player { name: string, hp: i64 }

fn main() -> void {
    let party = [Player { name: "ada", hp: 100 }, Player { name: "bob", hp: 80 }]
    party[0].hp -= 30
    party[1].hp += 10
    print(party[0].name, party[0].hp)
    print(party[1].name, party[1].hp)
    let v = [1, 2, 3]
    v[1] += 10
    print(v[1])
}
"#,
        expected: "ada 70\nbob 90\n12\n",
    },
    Case {
        name: "test runner (vulcan test)",
        src: r#"
fn add(a: i64, b: i64) -> i64 { return a + b }

fn test_add() -> void {
    assert(add(2, 3) == 5)
}

fn test_negatives() -> void {
    assert(add(-2, 3) == 1)
}

fn main() -> void {
    print("main is not run in test mode")
}
"#,
        expected: "running 2 test(s)\n  test_add ...\n  test_negatives ...\nall tests passed\n",
    },
    Case {
        name: "game stdlib (rand_int, ms_now, bare return)",
        src: r#"
fn main() -> void {
    // rand_int stays in bounds
    let ok = true
    for i in 0 .. 50 {
        let r = rand_int(3, 7)
        if r < 3 || r > 7 { ok = false }
    }
    print("rand bounds:", ok)
    // ms_now is a monotonic millisecond clock
    let a = ms_now()
    let b = ms_now()
    print("clock sane:", b >= a)
    // bare `return` (no value, no semicolon)
    print("before")
    if a >= 0 {
        return
    }
    print("unreachable")
}
"#,
        expected: "rand bounds: true\nclock sane: true\nbefore\n",
    },
    Case {
        name: "snake-style loop + string repeat",
        src: r#"
fn main() -> void {
    let xs: [string] = []
    for c in ["a", "b", "c"] {
        xs.push(c)
    }
    print(xs)
    print(".".repeat(3))
}
"#,
        expected: "[a, b, c]\n...\n",
    },
    Case {
        name: "impl methods (all backends)",
        src: r#"
struct Vec2 { x: f64, y: f64 }

impl Vec2 {
    fn move(self, dx: f64, dy: f64) -> Vec2 {
        self.x = self.x + dx
        self.y = self.y + dy
        return self
    }
    fn len2(self) -> f64 {
        return self.x * self.x + self.y * self.y
    }
    fn scale(self, k: f64) -> Vec2 {
        self.x = self.x * k
        self.y = self.y * k
        return self
    }
}

fn main() -> void {
    let p = Vec2 { x: 1.0, y: 2.0 }
    p.move(3.0, 4.0)
    print("p =", p.x, p.y)
    print("len2 =", p.len2())
    p.scale(2.0)
    print("scaled =", p.x, p.y)
    let pts: [Vec2] = [Vec2 { x: 1.0, y: 1.0 }, Vec2 { x: 2.0, y: 3.0 }]
    print("pts[1].y =", pts[1].y)
}
"#,
        expected: "p = 4 6\nlen2 = 52\nscaled = 8 12\npts[1].y = 3\n",
    },
    Case {
        name: "file I/O, args, string/array stdlib",
        src: r#"
fn main() -> void {
    // file I/O
    file_write("selftest_io.txt", "alpha,beta,gamma")
    print(file_exists("selftest_io.txt"))
    let raw = file_read("selftest_io.txt")
    print(raw)
    // string stdlib
    let parts = raw.split(",")
    print(parts.len())
    print(parts[1])
    print(raw.starts_with("alpha"))
    print(raw.ends_with("gamma"))
    print(raw.substring(6, 10))
    print(raw.find("beta"))
    print("SHOUT".lower())
    print("  pad  ".trim())
    print(to_int("42"), to_int("bad"))
    print(to_float("2.5"))
    // array stdlib
    let xs = [1, 2, 3, 4]
    xs.remove(0)
    xs.insert(1, 99)
    print(xs)
    xs.push(5)
    print(xs.last())
    print(xs.contains(99))
}
"#,
        expected: "true\nalpha,beta,gamma\n3\nbeta\ntrue\ntrue\nbeta\n6\nshout\npad\n42 0\n2.5\n[2, 99, 3, 4]\n5\ntrue\n",
    },
    Case {
        name: "graphics builtins (all backends)",
        src: r#"
fn main() -> void {
    print("canvas:", canvas_width(), "x", canvas_height())
    clear(0x102030)
    draw_rect(10.5, 20.5, 50.0, 30.0, 0xFF5500)
    draw_circle(200.0, 70.0, 25.0, 0x33CCFF)
    draw_line(0.0, 0.0, 255.0, 143.0, 0xFFFFFF)
    draw_text(4.0, 4.0, "VULCAN", 0xFFFFFF)
    draw_text(4.0, 130.0, "count: 42", 0x00FF00)
    print("drawn")
}
"#,
        expected: "canvas: 256 x 144\ndrawn\n",
    },
];

pub fn selftest() {
    let dir = std::env::temp_dir().join("vulcan_selftest");
    let _ = std::fs::create_dir_all(&dir);
    let mut failures = 0;
    // the physics engine case: lib source + test main, verified like any other case
    let physics_src = include_str!("../lib/physics.vulcan").to_string() + PHYSICS_TEST_MAIN;
    failures += run_case("physics engine (all backends)", &physics_src, PHYSICS_TEST_EXPECTED, &dir);
    for case in CASES {
        failures += run_case(case.name, case.src, case.expected, &dir);
    }
    if failures > 0 {
        eprintln!("{failures} selftest case(s) failed");
        std::process::exit(1);
    }
    println!("vulcan selftest: {} cases passed", CASES.len() + 1);
}

fn run_case(name: &str, src: &str, expected: &str, dir: &std::path::Path) -> usize {
    let mut failures = 0;
    let tag = name.replace(['(', ')', ' ', '/'], "_");
    let src_path = dir.join(format!("{tag}.vulcan"));
    std::fs::write(&src_path, src).unwrap();
    let c_path = dir.join(format!("{tag}.v.c"));
    let out_path = dir.join(format!("{tag}.vbin"));
    let tokens = crate::lexer::lex(src);
    let program = crate::parser::parse_program(&tokens);
    let c_src = if name.starts_with("test runner") {
        let tests: Vec<String> = program.iter().filter_map(|i| match i {
            crate::parser::Item::Fn(f) if f.name.starts_with("test_") => Some(f.name.clone()),
            _ => None,
        }).collect();
        crate::codegen::generate_test(&program, &tests)
    } else {
        crate::codegen::generate(&program)
    };
    std::fs::write(&c_path, c_src).unwrap();
    let cc = crate::pick_cc();
    let status = Command::new(&cc)
        .args(["-O3", c_path.to_str().unwrap(), "-lm", "-o", out_path.to_str().unwrap()])
        .output()
        .expect("no C compiler found");
    if !status.status.success() {
        eprintln!("[FAIL] {}: compile failed:\n{}", name, String::from_utf8_lossy(&status.stderr));
        return 1;
    }
    let run = Command::new(&out_path).output().unwrap();
    let stdout = String::from_utf8_lossy(&run.stdout);
    if stdout != expected {
        eprintln!("[FAIL] {}: output mismatch\n--- got ---\n{stdout}--- expected ---\n{}", name, expected);
        failures += 1;
    } else {
        println!("[ok] {}", name);
    }
    // cross-backend check: the JS and Python backends must produce identical output
    if name.starts_with("test runner") { return failures; }
    for backend in ["js", "py"] {
        let (runner, src) = if backend == "js" {
            ("node", crate::codegen_js::generate_js(&program))
        } else {
            ("python3", crate::codegen_py::generate_py(&program))
        };
        let script_path = dir.join(format!("{tag}.{backend}"));
        std::fs::write(&script_path, &src).unwrap();
        match Command::new(runner).arg(&script_path).output() {
            Ok(r) => {
                let stdout = String::from_utf8_lossy(&r.stdout);
                if stdout != expected {
                    eprintln!("[FAIL] {}: {backend} backend output mismatch\n--- got ---\n{stdout}--- expected ---\n{}", name, expected);
                    failures += 1;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                eprintln!("[skip] {}: `{runner}` not found — {backend} backend not verified", name);
            }
            Err(e) => { eprintln!("[FAIL] {}: {backend} runner error: {e}", name); failures += 1; }
        }
    }
    failures
}
