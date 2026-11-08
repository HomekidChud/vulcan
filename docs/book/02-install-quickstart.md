# Chapter 2: Install and quickstart

In this chapter you'll install Vulcan, run your first program on all three backends, and scaffold a project. You'll finish knowing the four commands you'll use every day.

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/HomekidChud/vulcan/master/scripts/vulcanup.sh | sh
```

That downloads the latest prebuilt static binary to `~/.vulcan/bin` and fixes your PATH. Termux on Android is fully supported — the Linux aarch64 build is a static musl binary, and the installer wires PATH through `$PREFIX/etc/profile.d`. To build from source instead: `sh vulcanup.sh --source` (needs [Rust](https://rustup.rs)). Update later with `vulcan self-update`.

The default C backend needs `clang` or `gcc` on your PATH. The other backends need `node` and `python3` if you want to use them.

If anything behaves oddly, run `vulcan doctor` — it checks your toolchain (C
compiler, node, python3, git) and shows exactly what each command will use.

## Your first program

Make a file called `hello.vulcan`:

```vulcan
fn main() {
    print("hello, Vulcan")
}
```

Run it:

```bash
vulcan run hello.vulcan
```

Output:

```
hello, Vulcan
```

`vulcan run` compiles to C, compiles the C with clang, runs the binary, and shows you its output. It takes about a tenth of a second end to end.

## The same program, three runtimes

This is Vulcan's core promise, so check it yourself:

```bash
vulcan run hello.vulcan               # native (through C)
vulcan run --target js hello.vulcan   # Node.js
vulcan run --target py hello.vulcan   # Python 3
```

All three print the same `hello, Vulcan`. Not close to the same — the same bytes, always. `vulcan selftest` runs the compiler's whole test suite this way and fails the build if any backend drifts.

## A real project

`vulcan init` scaffolds a project with a `vulcan.toml` manifest and a `src/main.vulcan`:

```bash
vulcan init hello
cd hello
vulcan run
```

The scaffold's `src/main.vulcan` is:

```vulcan
// Welcome to Vulcan!
const GREETING: string = "world"

fn main() -> void {
    print(f"hello, {GREETING}!")
    let xs = [1, 2, 3, 4, 5]
    let total = 0
    for x in xs {
        total += x * x
    }
    print(f"sum of squares: {total}")
    match total {
        when 55 { print("that is 5 triangular numbers squared") }
        else { print("math still works") }
    }
}

fn test_sum_of_squares() -> void {
    assert(1 * 1 + 2 * 2 + 3 * 3 + 4 * 4 + 5 * 5 == 55, "sum of squares")
    print("test_sum_of_squares passed")
}
```

Output of `vulcan run`:

```
hello, world!
sum of squares: 55
that is 5 triangular numbers squared
```

Inside a project you can also run `vulcan test` (runs every `test_*` function) and `vulcan build` (produces `build/<name>`).

## The four daily commands

| command | what it does |
|---|---|
| `vulcan run file.vulcan` | compile and execute |
| `vulcan build file.vulcan` | emit a native binary |
| `vulcan test file.vulcan` | run every `test_*` function |
| `vulcan check file.vulcan` | type-check without building |

There are more (`emit`, `bench`, `doc`, `fmt`-style tooling via `vulcan doc`, `package`, `ide`) — you'll meet them in later chapters. Run `vulcan` with no arguments to see the full list.

Where to go next: [Chapter 3: A tour of the basics](03-tour-basics.md) covers variables, types, functions, and control flow in depth.
