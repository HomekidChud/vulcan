# Chapter 13: Idiomatic Vulcan

In this chapter you'll learn the patterns experienced Vulcan programmers reach for, and the traps that surprise newcomers. This is the chapter to re-read after your first week.

## Copy in, copy out, reassign

The single most idiomatic pattern in Vulcan. Functions take state, return the new state, callers reassign:

```vulcan
fn add_item(items: [string], text: string) -> [string] {
    items.push(text)
    return items
}

fn main() {
    let items: [string] = []
    items = add_item(items, "buy milk")
    items = add_item(items, "walk dog")
    print(items)
}
```

Output:

```
[buy milk, walk dog]
```

Why: arrays passed as parameters are copied, so mutations inside a function don't reach the caller unless you return the array and reassign. The physics engine uses the same shape at scale (`w = world_step(w, dt)`), and it's what makes one source file behave identically on value-semantics (C) and reference-semantics (JS/Python) backends.

The exception: **method calls on a variable update that variable** (see below), and mutations on arrays you hold directly (`items.push(...)` in the same scope) just work.

## Method calls mutate the receiver variable

```vulcan
struct Point { x: f64, y: f64 }

impl Point {
    fn move(self, dx: f64, dy: f64) -> Point {
        self.x += dx
        self.y += dy
        return self
    }
}

fn main() {
    let p = Point { x: 1.0, y: 2.0 }
    let q = p          // q is an independent copy
    q.move(10.0, 10.0)
    print("p:", p.x, p.y)
    print("q:", q.x, q.y)
}
```

Output:

```
p: 1 2
q: 11 12
```

Assignment copies; a method called on a variable is the variable. To keep an original, copy it first. This is identical on every backend — but it surprises everyone once, so internalize it early.

## `defer` for cleanup

```vulcan
fn main() {
    term_raw()
    defer { term_normal() }
    print("running in raw mode")
}
```

Output:

```
running in raw mode
```

`defer` blocks run when the scope exits — in reverse order if you defer several. Use it for terminal state, socket handles, anything that must be restored on every exit path. It compiles to `try/finally` on JS and Python and behaves the same everywhere.

## Build strings with arrays, join once

String concatenation in a loop copies the whole string each time. The idiom is to collect parts and `join`:

```vulcan
fn main() {
    let parts: [string] = []
    for i in 1 ..= 3 {
        parts.push(f"item {i}")
    }
    print(parts.join(" | "))
}
```

Output:

```
item 1 | item 2 | item 3
```

(`lib/strbuf.vulcan` formalizes this pattern if you want a name for it.)

## Closures are single expressions

`|x| x * factor` — one expression, captures outer variables, works everywhere. Multi-statement closure bodies are not part of the language. When a closure gets too clever, write a named function; forward references are allowed, so order doesn't matter.

Also: don't call struct methods inside closures (`items.map(|it| it.label())`) — a known backend gap. Use field access or a `for` loop.

## Fail loudly with `match`, not silence

For anything that can go wrong, return `Option` or `Result` and force the caller to `match`:

```vulcan
fn item_number(s: string) -> Result<i64, string> {
    if s.len() == 0 {
        return Result.Err("no number given")
    }
    return Result.Ok(to_int(s))
}

fn main() {
    match item_number("2") {
        when Result.Ok(n) { print("item:", n) }
        when Result.Err(e) { print("error:", e) }
    }
}
```

```
item: 2
```

The caller cannot read the value without handling the error case — there is no unwrap to skip the check with. Use `?` to propagate, `to_int`'s `0` fallback only when `0` genuinely means "no number".

## Prefer `for x in arr` over indexing

```vulcan
fn main() {
    let items = ["buy milk", "walk dog"]
    for it in items {
        print(it)
    }
}
```

Output:

```
buy milk
walk dog
```

Index only when you need the number (`f"{i}. {item}"` in a listing). Bounds checks make bad indexes loud, but the right loop makes them impossible.

## Program shape that works everywhere

- State in structs, behavior in `impl`, glue in plain `fn`s.
- File I/O and args for CLIs (`file_read` / `file_write` / `arg` / `arg_count`).
- Tests as `test_*` functions next to the code.
- One file per concern, wired with `import`; `lib/` for shared modules.

The todo app in `examples/todo.vulcan` is this shape end to end — read it as a style reference. It is a complete CLI: subcommands, file persistence, struct lists with `impl` methods, argument parsing, and error messages, in ~130 lines.

## The honest checklist

Before shipping a Vulcan program, run:

```bash
vulcan check file.vulcan        # types, no build
vulcan test                     # every test_* function
vulcan run --safe file.vulcan   # sanitizers on
vulcan run --target js file.vulcan && vulcan run --target py file.vulcan   # backend parity
```

If those four pass, you know what the language can tell you about your program. The rest — logic, design, memory discipline — is yours, as it is in any language without a borrow checker.

Where to go next: [Vulcan by Example](../by-example.md) for a runnable snippet per concept, and the [language reference](../reference/functions.md) for the details.
