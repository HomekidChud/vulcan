# Vulcan

A small, fast-to-compile language that transpiles to C. Static types, no garbage collector, no runtime — your code becomes tight C, compiled by `clang -O3` (or `gcc`, whichever is found first).

```vulcan
fn fib(n: i64) -> i64 {
    if n < 2 { return n }
    return fib(n - 1) + fib(n - 2)
}

fn main() {
    print(fib(35))    // 9227465
}
```

## Vulcan 0.2.0 — what's new

The 0.2.0 release roughly doubles the language:

**Language features**
- `elif` chains, `match` / `when` arms with `else` fallback
- `defer { ... }` — scope-exit cleanup, LIFO order, runs before returns
- `enum`s (`Color.Red`) and named `const`s with inferred types
- f-strings: `print(f"n = {n}, double is {n * 2}")`
- `for x in xs { }`, stepped ranges `for i in 0 ..= 10 by 2 { }`
- compound assignment (`+=`, `-=`, `*=`, `/=`, `%=`), `++` / `--`
- bitwise ops (`&`, `|`, `^`, `~`, `<<`, `>>`), hex/binary/char literals (`0xFF`, `0b1010`, `'A'`)
- `import` + multi-file modules, `assert` with messages, casts (`x as i64`)
- new builtins: `min`, `max`, `abs`, `sqrt`, `floor`, `ceil`, `pow`, `input`, `sleep_ms`
- string methods (`.upper()`, `.lower()`, `.trim()`, `.contains()`, `.repeat()`, `.len()`)
- array printing (`print(xs)` → `[10, 20, 30]`), typed empty arrays (`let xs: [i64] = []`)
- terminal I/O for games and TUIs: `term_raw()`, `term_normal()`, `key_ready()`, `read_key()`, `clear_screen()`

**Tooling**
- a package manager: `vulcan init`, `vulcan add`, `vulcan.lock` (see below)
- `vulcan test` — runs every `test_*` function in your project
- a clang-first backend (set `VULCAN_CC` to override)

Run the tour: `vulcan run examples/tour.vulcan`. Play the game: `vulcan run examples/snake.vulcan`.

## Packaging

```bash
vulcan init hello          # scaffold a project (vulcan.toml, src/main.vulcan)
cd hello
vulcan add git@github.com:user/mathlib.git   # or a local path: vulcan add ../mathlib
vulcan run                  # builds to build/hello, runs it
vulcan test                 # runs every test_* function
```

`vulcan add` vendors dependencies into `deps/` and records them in `vulcan.lock`. Dependencies are resolved from `deps/<name>/src/lib.vulcan`, `deps/<name>/lib.vulcan`, or `deps/<name>/src/main.vulcan`. Imports are also resolved relative to the importing file, plus `src/` and `lib/` directories — so plain `import math` works with a `lib/math.vulcan` next to your code.

## Benchmarks vs Rust

Same algorithms, `vulcan build` (clang backend) vs `rustc -O`, best of 5 runs (Linux, this repo's `bench/compare.sh`):

| benchmark | vulcan (s) | rustc -O (s) | winner |
|---|---|---|---|
| fib(35) | 0.0292 | 0.0296 | vulcan |
| collatz | 0.1390 | 0.1391 | vulcan |
| primes | 0.1244 | 0.1244 | vulcan |
| physics | 0.0048 | 0.0049 | vulcan |

Vulcan wins or ties all four. Compile times are in the same ballpark (~90ms end-to-end for vulcan incl. C compile; ~70ms for rustc -O with the clang backend — with the gcc backend vulcan compiles faster than rustc on most benches).

Reproduce with:

```bash
./bench/compare.sh
```

## Usage

```bash
cargo build --release
vulcan run examples/fib.vulcan        # compile + execute
vulcan build examples/fib.vulcan     # emit C + native binary
vulcan emit examples/fib.vulcan      # show generated C
vulcan bench examples/fib.vulcan      # time repeated runs
vulcan selftest                       # run the test suite
vulcan init myproj                    # scaffold a new project
```

Requires `clang` or `gcc` on your PATH (Vulcan compiles through C; clang is preferred for faster loops).

## How it works

1. Lexer → parser → AST
2. Codegen emits readable C (see `vulcan emit`)
3. `clang -O3` (or `gcc -O3`) compiles it to a native binary

No runtime, no GC — arrays and structs are plain C vectors and values, with bounds-checked access.

## Examples

- `examples/fib.vulcan` — classic recursion
- `examples/tour.vulcan` — a tour of every 0.2.0 feature
- `examples/snake.vulcan` — Snake, written in pure Vulcan (WASD keys; uses `term_raw` + `defer` for terminal cleanup)
- `bench/` — vulcan-vs-rust benchmarks
