# Chapter 11: Safety

In this chapter you'll learn what Vulcan actually protects you from — bounds checks, checked arithmetic aborts, sanitizer builds, hardened binaries — and what it cannot protect you from. The comparison with Rust is honest because the language is honest about it.

## The honest headline

Is Vulcan securer than Rust? **No.** Rust's borrow checker proves memory safety at compile time; Vulcan compiles through C and inherits C's model, so use-after-free, leaks, and dangling pointers are possible in ways Rust rules out by construction. If your threat model is "untrusted input, hostile environment", use Rust.

What Vulcan gives you is a set of specific, testable guarantees:

## Bounds-checked arrays

Every element access goes through a checked get/set. Reading past the end aborts instead of corrupting memory:

```vulcan
fn main() {
    let items = ["a", "b"]
    print(items[5])
}
```

Output (stderr, exit code 1 — identical on C, JS, and Python):

```
vulcan: index out of bounds
```

No undefined behavior, no silent wrong answer, no heap corruption. The program stops and tells you.

## Checked arithmetic aborts

Division and modulo by zero abort with a clear message on every backend — instead of C's crash, JavaScript's `Infinity`, or Python's `ZeroDivisionError`:

```vulcan
fn main() {
    let a: i64 = 1
    let b: i64 = 0
    print(a / b)
}
```

Output (stderr, exit code 1):

```
vulcan: division by zero
```

This is the "same program, same output" principle applied to failure: errors are part of the contract.

## `--safe` sanitizer builds

`vulcan run --safe app.vulcan` compiles with AddressSanitizer + UndefinedBehaviorSanitizer (C backend only). Memory errors and integer overflow abort at the exact line with a stack trace:

```vulcan
fn main() {
    let big: i64 = 9223372036854775807
    print(big + 1)
}
```

Without `--safe`, this silently wraps (the C behavior). With it:

```
SUMMARY: UndefinedBehaviorSanitizer: undefined-behavior file.v.c:510:39
-9223372036854775808
```

The habit to build: develop with `--safe`, ship hardened. Sanitizer builds are slower — don't ship them.

## `--checked` builds (1.19)

`--safe` is thorough but pays for ASan too. When you want integer-overflow
detection without the sanitizer tax, use `--checked`:

```bash
vulcan run --checked app.vulcan
```

It compiles with UBSan only (`-fsanitize=undefined -fno-sanitize-recover`), so
signed overflow, bad shifts and the like abort at the exact line — at a small
fraction of the runtime cost of `--safe`. The Zig "ReleaseSafe" posture fits
vulcan: develop with `--checked`, hunt memory bugs with `--safe`, ship the
default hardened build.

## `INT64_MIN / -1` is defined (1.19)

The one division C leaves undefined — `INT64_MIN / -1` — is a clean runtime
panic in vulcan (`vulcan: integer overflow in division (INT64_MIN / -1)`)
instead of a crash on x86 or garbage on ARM. The JS and Python backends were
already safe.

## Hardened binaries by default

Every native build compiles with all of these on:

- `-fstack-protector-strong` — stack smashing detection
- `-D_FORTIFY_SOURCE=2` — fortified libc calls
- `-fPIE -pie` — full PIE/ASLR
- `-z relro -z now` — full RELRO
- `-Werror=format-security` — format string injection rejected at compile time

The hardening is free: with all flags on, vulcan still beats `rustc -O` on the repo's benchmarks.

## No hidden runtime

The generated C is readable (`vulcan emit`), so the attack surface is auditable. Every builtin is emitted inline into your program's C file — there is no runtime library to audit separately, and no code you can't see.

## What remains your job

- **Heap discipline.** No GC means you decide when data dies. Leaks and (via FFI) use-after-free are possible.
- **Integer wrap.** `i64` overflow wraps silently in normal builds — use `--safe` in development if you depend on it not wrapping.
- **FFI is C.** `extern "c"` calls into real C libraries with real C semantics. The types are checked at the boundary, but the library's behavior is its own.

Where to go next: [Chapter 12: How it works](12-how-it-works.md) — the self-hosting compiler and the bootstrap fixpoint.
