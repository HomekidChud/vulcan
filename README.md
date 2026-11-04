# Vulcan

A small, fast-to-compile language that transpiles to C. Static types, no garbage collector, no runtime — your code becomes tight C, compiled by `gcc -O3`.

```vulcan
fn fib(n: i64) -> i64 {
    if n < 2 { return n }
    return fib(n - 1) + fib(n - 2)
}

fn main() {
    print(fib(35))    // 9227465
}
```

## Features

- functions with typed params and return types (`i64`, `f64`, `bool`, `void`)
- `if` / `else`, `while`, `for` loops
- `struct`s, including arrays of structs
- first-class arrays with bounds-checked element access
- `print` builtin with automatic formatting

## Usage

```bash
cargo build --release
vulcan run examples/fib.vulcan        # compile + execute
vulcan build examples/fib.vulcan     # emit C + native binary
vulcan emit examples/fib.vulcan      # show generated C
vulcan bench examples/fib.vulcan      # time repeated runs
vulcan selftest                       # run the test suite
```

Requires `gcc` on your PATH (Vulcan compiles through C).

## How it works

1. Lexer → parser → AST
2. Codegen emits readable C (see `vulcan emit`)
3. `gcc -O3` produces the native binary

No IR, no LLVM, no runtime library — which is exactly why compile times stay in the milliseconds range.

## Benchmarks (honest)

Best of several runs, 2-core Linux, `gcc -O3` backend vs `rustc -O`:

| benchmark        | Vulcan  | rustc -O | result |
|------------------|---------|----------|--------|
| fib(35)          | 15.4 ms | 23.1 ms | **Vulcan ~1.5x faster** |
| physics 2000x200 | 2.6 ms  | 0.7 ms  | Rust ~3.6x faster |
| compile fib      | 102 ms  | 78 ms    | rustc faster |
| compile physics  | 78 ms   | 97 ms    | Vulcan faster |

The recursive-integer workload wins because Vulcan skips abstractions and compiles straight to tight C. The struct-array workload loses to LLVM's optimizer. That's the honest picture — take it or leave it.

## Layout

```
src/
├── main.rs     CLI (run/build/emit/bench/selftest)
├── lexer.rs
├── parser.rs
└── codegen.rs  AST → C
examples/
├── fib.vulcan
└── physics.vulcan
```
