# Chapter 12: How it works

In this chapter you'll learn how a Vulcan program becomes a running process, why the compiler is written in Vulcan itself, and what `vulcan bootstrap` proves. You'll run a program with the self-hosted compiler.

## The pipeline

```
source (.vulcan)
  |
  v
lexer            hand-written, line-tagged tokens
  |
  v
modules          expands `import` transitively (./, src/, lib/, deps/...) into one flat item list
  |
  v
parser           items: fn, struct, enum, const, impl, extern, import (no type checking yet)
  |
  v
one of three code generators
  |-- C:       readable C, compiled by clang -O3 to a native binary
  |-- JS:      plain Node.js-compatible JavaScript
  |-- Python:  Python 3 source
  |
  v
run             C: invoke binary | JS: invoke node | Python: invoke python3
```

Each backend re-infers types independently — the same program analyzed three times, which is part of why the outputs are kept identical by force: `vulcan selftest` compiles and runs the whole test suite on all three backends and diffs, byte for byte.

The C backend has one extra trick worth knowing: since 1.18 it inlines calls that a **pure** function makes to itself, three levels deep, into the emitted C. On recursion-heavy code (like `fib`) that is a ~40x speedup, because clang's optimizer sees the recursion tree instead of opaque calls. Purity is required, not assumed — functions that print, touch arrays, divide, or use `match`/`defer`/`assert` keep the plain call, with identical semantics on every backend.

## Self-hosting: the compiler is written in Vulcan

`self/vulc.vulcan` (~3,100 lines) is a complete lexer, parser, type checker, and C code generator for **Vulcan Core** — the stable subset described in [docs/selfhost.md](../selfhost.md). It emits the same C dialect the Rust compiler emits.

The chain:

```
stage 0  the Rust compiler in this repo (src/*.rs)   — bootstrap seed
stage 1  vulc built by stage 0   (Rust -> C -> binary)
stage 2  vulc built by stage 1   (vulc -> C -> binary)
stage 3  vulc built by stage 2   (output only, for the fixpoint check)
```

`vulcan bootstrap` runs the whole chain and asserts that **stage2.c and stage3.c are byte-identical** — the compiler, compiled by itself, reproduces itself exactly. This is the same fixpoint verification rustc performs on its stage2/stage3 artifacts, and the approach Go has used since 1.5.

After stage 1 exists, the Rust compiler is not needed to build the compiler from source: stage 1 compiles `vulc.vulcan`, forever. The Rust compiler stays in the tree as the fast path for everyday development.

## Running a program with the self-hosted compiler

```vulcan
fn add(a: i64, b: i64) -> i64 {
    return a + b
}

fn main() {
    print(add(20, 22))
}
```

```bash
vulcan self file.vulcan
```

Output:

```
42
```

`vulcan self` compiles and runs the program with the self-hosted compiler — no cargo involved. It's not just a demo: `./scripts/self-test.sh` compiles every `self/test/*.vulcan` with both compilers and requires identical behavior, so the self-hosted compiler can't drift from the Rust one.

## Vulcan Core: what `vulc` compiles

The self-hosted compiler covers the stable core of the language:

- **Items**: `fn`, `struct`, `impl`, `const`
- **Types**: `i64`, `f64`, `bool`, `string`, `void`, user structs, one-dimensional `[T]` arrays
- **Statements**: `let`, assignments, `if`/`elif`/`else`, `while`, `for i in a..b` / `a..=b`, `for x in arr`, `return`/`break`/`continue`
- **Expressions**: literals, casts, struct literals, array literals, method calls, field access, indexing, the usual operators
- **Builtins**: `print`, `to_string`, `to_int`, `to_float`, `char_at`, `len`, `arg_count`, `arg`, `file_read`, `file_write`, `file_exists`, `exit`, `sleep_ms`, `clock`, `ms_now`, `input`, `rand_int`

Features outside Core — payload enums, `Option`/`Result`, references, f-strings — are compiled by the Rust compiler (the default `vulcan run`). Try a Core-incompatible feature under `vulcan self` and you'll get a clear parse error rather than silent misbehavior.

## Architecture notes worth reading

The self-hosting design is documented in [docs/selfhost.md](../selfhost.md) — arena ASTs with integer indices, copy-in/copy-out context, embedded prelude — and the overall system design in [docs/architecture.md](../architecture.md). Both are short and written to be read.

Where to go next: [Chapter 13: Idiomatic Vulcan](13-idiomatic-vulcan.md) — the patterns and habits that tie it all together.
