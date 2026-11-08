# Baseline (Phase 0)

Recorded 2026-11-07, on the head of master at commit 757308d. Machine:
Linux sandbox (x86_64), gcc, Node 22, Python 3.13, Rust
stable. Numbers are indicative, not lab-grade; the point is to catch drift.

## Build and test

| measurement                          | result   |
|--------------------------------------|----------|
| `cargo build --release` (cold)       | 5.9 s    |
| `cargo build --release` (incremental)| ~0.6 s   |
| `cargo test --release`               | 0 tests  |
| `VULCAN_CC=gcc ./target/release/vulcan selftest` | **24/24 cases pass, 5.7 s** |

Selftest covers: basic, elif/compound assign, for-in/step ranges, match/enums,
f-strings/consts/string methods, defer/assert, bitwise/hex/binary/chars/casts,
array methods, structs with arrays, test runner, game stdlib, snake-style loop,
impl methods, method calls on expressions, forward references, file I/O,
graphics builtins, FFI, math builtins, nested arrays, clamp/lerp/sign, physics,
3D renderer, engine. All byte-identical across C, JS and Python.

## Benchmarks (`vulcan bench`, single run)

| benchmark | C (gcc -O3) | JS (Node) | Python 3 |
|-----------|------------|-----------|----------|
| fib(35)   | 0.016 s    | 2.167 s   | 1.093 s  |
| collatz   | 0.197 s    | 1.547 s   | 24.127 s |
| primes    | 0.200 s    | (not run) | (not run)|
| physics   | 0.003 s    | (not run) | (not run)|

(All four benches were run on C; fib and collatz additionally on JS and Python.
A proper tracked matrix is Phase 10 work.)

## Version state (before reconciliation)

- Cargo.toml: 1.2.0
- README.md: "Vulcan 1.10"
- TODO.md: "Current state (v1.9)"
- `vulcan` usage banner: "Vulcan 1.9"
- No `vulcan --version` command exists.

## Confirmed bugs (all verified in source at head of master)

1. `src/pkg.rs` `add()` (lines ~135-165): the dependency folder name comes from
   the last path segment of the source, then `remove_dir_all` is called on
   `deps/<name>`. A source ending in `/..` (or a name like `..`) can resolve
   outside `deps/` and delete project files. Also the local-path branch uses
   `src.file_name()`, same class of problem.
2. `src/pkg.rs` line ~145: `git clone --depth 1 <url> <target>` — the URL is
   not preceded by `--`, so a URL starting with `-` would be parsed as a git
   option.
3. `src/lexer.rs` (hex/binary branches): `v = v * 16 + digit` / `v = v * 2 +
   bit` with no overflow check — literals like `0xFFFFFFFFFFFFFFFFFF` silently
   wrap to garbage. Decimal literals use `s.parse::<i64>().unwrap()` which
   panics on overflow instead of a diagnostic.
4. `src/cli.rs` / `src/main.rs`: `Command::new(...).status().unwrap()` and
   `.output().unwrap()` on compiler/interpreter spawns (cli.rs ~801-827,
   main.rs ~206-303) — a missing `node`/`python3` panics with an ugly Rust
   backtrace instead of a clean error message.

## Code shape (for Phase 1 planning; counted at head of master)

- `field_ty` and `const_ty` are duplicated 3x (codegen.rs:598,1158;
  codegen_js.rs:362,445; codegen_py.rs:382,485).
- panic! counts: C 114, JS 104, Python 104. Parser `expect(` count: 39.
- Total: ~10.6k lines across src/ + lib/ (lexer 245, parser 752, codegen 1906,
  codegen_js 1444, codegen_py 1484, cli 842, main 339, modules 64, package 953,
  pkg 207, ide 717; lib/engine 1103, physics 386, render3d 146, math 16).

## CI (before Phase 0)

`.github/workflows/ci.yml`: single ubuntu-latest job — apt install gcc nodejs
python3, `cargo build --release`, selftest with VULCAN_CC=gcc. No clang
matrix, no `cargo test`, no unit tests exist yet.
