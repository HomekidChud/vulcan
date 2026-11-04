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

## Vulcan 0.3.0 — what's new

- **`--safe` build mode** — `vulcan run --safe` (or build/bench/test) compiles with AddressSanitizer + UndefinedBehaviorSanitizer: out-of-bounds heap access, use-after-free, and signed integer overflow abort the program at the exact line instead of corrupting memory.
- **Game/terminal stdlib** — everything you need to build any kind of game:
  - timing: `ms_now()` (monotonic ms clock), `sleep_ms(n)` (frame caps)
  - input: `key_ready()`, `read_key()`, `term_raw()` / `term_normal()` (with `defer` cleanup)
  - output: `print_at(x, y, s)`, `clear_screen()`, `cursor_visible(0)`, `set_color(fg, bg)` (8 ANSI colors, `-1` skips)
  - screen: `screen_width()`, `screen_height()` (live terminal size)
  - randomness: `rand_int(lo, hi)`
- **Three example games** in pure Vulcan, covering three genres:
  - `examples/snake.vulcan` — arcade (grid, growth, collision)
  - `examples/pong.vulcan` — real-time physics (ball angles, AI opponent, frame loop)
  - `examples/tictactoe.vulcan` — turn-based strategy (full minimax AI — it cannot be beaten)
- **A first-person 3D engine** — `examples/raycaster.vulcan` is a Wolfenstein-style raycaster written in pure Vulcan: one ray per screen column, distance-falloff wall heights, fisheye correction, 256-color depth shading, collision detection, live minimap, ~30fps. This is the same core technique that powered Wolfenstein 3D.
- **Language**: bare `return` (no semicolon, no value), string indexing (`s[i]` yields the char code), trailing commas in array literals, `.repeat(n)`, array printing, typed empty arrays

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

## Security posture (an honest comparison)

Is Vulcan "securer than Rust"? No. Rust's borrow checker proves memory safety at compile time; Vulcan compiles through C and inherits C's model, so use-after-free, leaks, and dangling pointers are possible in ways Rust rules out by construction. If your threat model is "untrusted input, hostile environment", use Rust.

What Vulcan does give you:

- **`--safe` sanitizer builds** — `vulcan run --safe app.vulcan` compiles with AddressSanitizer + UndefinedBehaviorSanitizer: out-of-bounds heap access, use-after-free, and integer overflow abort at the exact line with a stack trace. Develop with `--safe`, ship hardened.
- **Bounds-checked arrays** — every element access goes through a checked `get`/`set`, so indexing bugs abort instead of corrupting memory.
- **Hardened binaries by default** — every vulcan build compiles with `-fstack-protector-strong` (stack smashing detection), `-D_FORTIFY_SOURCE=2` (fortified libc calls), full PIE/ASLR (`-fPIE -pie`), full RELRO (`-z relro -z now`), and `-Werror=format-security` (format string injection rejected at compile time).
- **No hidden runtime** — the generated C is readable (`vulcan emit`), so the attack surface is auditable.

The hardening is free: with all flags on, vulcan still beats `rustc -O` on all four benchmarks above.

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
vulcan run --safe examples/fib.vulcan # sanitizer build (ASan + UBSan)
vulcan selftest                       # run the test suite
vulcan init myproj                    # scaffold a new project
```

Requires `clang` or `gcc` on your PATH (Vulcan compiles through C; clang is preferred for faster loops).

## Making games

The full game stack is built in — timing (`ms_now`, `sleep_ms`), non-blocking keyboard input (`key_ready`, `read_key`, `term_raw`), drawing (`print_at`, `clear_screen`, `set_color`, `cursor_visible`), screen size (`screen_width`, `screen_height`), and randomness (`rand_int`). See `examples/pong.vulcan` for the canonical game loop:

```vulcan
term_raw()
defer { term_normal() }        // terminal state always restored
cursor_visible(0)
// per frame: drain keys, update state, draw at print_at(x, y, ...), sleep_ms(70)
```

Three complete games ship in `examples/`: Snake (arcade), Pong (real-time physics + AI), and Tic-Tac-Toe (unbeatable minimax AI).

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
