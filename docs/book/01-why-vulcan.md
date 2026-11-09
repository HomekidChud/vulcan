# Chapter 1: Why Vulcan?

In this chapter you'll learn what Vulcan is, what it trades away on purpose, and whether it fits your project. You'll read a complete Vulcan program before installing anything.

## What Vulcan is

Vulcan is a small, statically-typed language that compiles the same source file to three targets:

- **Native code** through C (`clang -O3`), with no garbage collector and no runtime.
- **JavaScript** that runs on Node.js.
- **Python 3** source.

The same program produces the same output on every backend — not "similar" output, byte-for-byte identical output. The compiler enforces this itself: `vulcan selftest` compiles its whole test suite on all three backends and fails if any output differs.

```vulcan
fn fib(n: i64) -> i64 {
    if n < 2 { return n }
    return fib(n - 1) + fib(n - 2)
}

fn main() {
    print(fib(35))    // 9227465
}
```

Output:

```
9227465
```

That is the whole language in one shape: `fn` functions, `i64` integers, `if` with braces, and a `print` builtin. If you know Python or JavaScript, most of Vulcan will feel familiar within an afternoon.

## What you get

- **One language, three runtimes.** Write it once, run it as a native binary, in Node, or in Python 3.
- **No garbage collector, no runtime.** The C backend emits readable C (see it yourself with `vulcan emit`) and compiles it with clang. Nothing hides between your program and the metal.
- **Bounds-checked arrays.** Reading past the end of an array aborts with an error message instead of corrupting memory.
- **Hardened binaries by default.** Every native build turns on stack smashing detection, fortified libc calls, full PIE/ASLR, full RELRO, and format-string hardening.
- **`--safe` sanitizer builds.** `vulcan run --safe app.vulcan` compiles with AddressSanitizer + UndefinedBehaviorSanitizer, so memory errors and integer overflow abort at the exact line.
- **A complete ecosystem.** Package registry (`vulcan add` / `vulcan publish`), browser IDE (`vulcan ide`), shipping to web/desktop/mobile/APK (`vulcan package --platform ...`), and a self-hosted compiler written in Vulcan itself.

## What you don't get (the honest part)

- **Vulcan is not safer than Rust.** Rust's borrow checker proves memory safety at compile time. Vulcan compiles through C and inherits C's memory model, so use-after-free and leaks are possible in ways Rust rules out by construction. If your threat model is "untrusted input, hostile environment", use Rust.
- **No garbage collector also means no automatic memory management.** You manage the lifetime of heap data yourself.
- **Vulcan is a young language.** Parts of the surface (networking, graphics) exist and work on all backends; other parts (nested arrays of structs as struct fields, for example) are still being built. The [roadmap](https://github.com/HomekidChud/vulcan/blob/master/TODO.md) tracks what's done and what's next.
- **Small type system.** You get integers, floats, bools, strings, arrays, structs, and enums. No traits, no generics, no pattern matching beyond `match` on enums, strings, and numbers.

## Who it's for

Vulcan is a good fit when you want:

- a small fast language for CLI tools, small apps, and games (it ships a full terminal game stack: timing, input, canvas, 3D renderer, physics);
- one source file that runs natively, in Node, and in Python with identical behavior;
- to read the C your language produces, and audit it.

It's a poor fit when you need compile-time memory safety guarantees, a large ecosystem of libraries, or generics.

## The rest of this book

The book builds one program with you: a todo-list CLI, the same domain as `examples/todo.vulcan` in the repository. Each chapter adds one skill — variables, structs, closures, error handling — and by the end you'll have seen every core feature of the language.

Where to go next: [Chapter 2: Install and quickstart](02-install-quickstart.md), or jump ahead to [Vulcan by Example](../by-example.md) for a snippet-per-concept reference.
