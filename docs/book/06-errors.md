# Chapter 6: Errors

In this chapter you'll learn Vulcan's error handling: `Option` for "maybe nothing", `Result` for "value or error", the `?` operator for propagation, and plain old checks with `to_int`. Your todo app learns to survive bad input.

## The problem

Some operations can fail. A search finds nothing. A user types "done" without a number. Vulcan has no exceptions — failures are values you handle explicitly. That sounds like work until you notice it makes every failure visible in the function's type.

## `Option<T>`: maybe nothing

```vulcan
struct Item {
    text: string,
    done: bool,
}

fn find(items: [Item], want: string) -> Option<Item> {
    for it in items {
        if it.text == want {
            return Option.Some(it)
        }
    }
    return Option.None
}

fn main() {
    let items = [Item { text: "buy milk", done: false }]
    match find(items, "buy milk") {
        when Option.Some(it) { print("found:", it.text) }
        when Option.None { print("not found") }
    }
    match find(items, "nope") {
        when Option.Some(it) { print("found:", it.text) }
        when Option.None { print("not found") }
    }
}
```

Output:

```
found: buy milk
not found
```

`Option.Some(value)` holds a value; `Option.None` holds nothing. You get the value out with `match` — there is no "unwrap" that can crash, because you never skip the check.

## `Result<T, E>`: value or error

```vulcan
fn item_number(s: string) -> Result<i64, string> {
    if s.len() == 0 {
        return Result.Err("no number given")
    }
    return Result.Ok(to_int(s))
}

fn done_command(s: string) -> Result<string, string> {
    let n = item_number(s)?
    return Result.Ok(f"marking item {n} done")
}

fn main() {
    match done_command("2") {
        when Result.Ok(msg) { print(msg) }
        when Result.Err(e) { print("error:", e) }
    }
    match done_command("") {
        when Result.Ok(msg) { print(msg) }
        when Result.Err(e) { print("error:", e) }
    }
}
```

Output:

```
marking item 2 done
error: no number given
```

Two new things here:

- `Result.Ok(value)` / `Result.Err(error)`. The error type is up to you — `string` is the common choice.
- **The `?` operator.** `let n = item_number(s)?` means: if `item_number` returned `Err`, return that error from `done_command` right now; otherwise `n` is the payload. It's the propagation operator, and it requires the enclosing function to return a `Result` with the same error type.

## `Option` and `Result` are payload enums

They're not special machinery — they're enums with payloads, exactly like the `Shape` enum from [Chapter 4](04-structs-enums.md). That's why `match` handles them: `Option` is `enum { Some(T), None }`, `Result` is `enum { Ok(T), Err(E) }`.

## When you don't need them: `to_int`

`to_int` converts a string to an `i64`, and returns `0` when the string isn't a number:

```vulcan
fn main() {
    let good = to_int("42")
    print(good + 1)
    let bad = to_int("not a number")
    if bad == 0 {
        print("that was not a number")
    }
}
```

Output:

```
43
that was not a number
```

For a CLI tool where `0` is an acceptable fallback, that's often all you need. Use `Result` when you must tell the difference between "0" and garbage.

## `assert`: crash early, in tests

`assert(condition)` aborts with the file line if the condition is false; `assert(condition, "message")` adds your message:

```vulcan
fn main() {
    let n = 3
    assert(n == 4, "n must be 4")
    print("never printed")
}
```

Output:

```
vulcan: assertion failed at line 3: n must be 4
```

Use `assert` in tests ([Chapter 10](10-testing.md)), not for input validation — it kills the program.

## Runtime errors abort, uniformly

Some errors aren't values — they abort the program with a message on stderr and exit code 1, identically on every backend:

- Array index out of bounds: `vulcan: index out of bounds`
- Division or modulo by zero: `vulcan: division by zero`
- Integer overflow in division: `vulcan: integer overflow in division (INT64_MIN / -1)`

## Compile errors help you (1.19)

Misspell a variable and the compiler suggests the fix:

```vulcan
fn main() {
    let count = 10
    print(cout)
}
```

```
file.vulcan:3:11: error: use of undeclared variable 'cout' — did you mean 'count'?
```

Unknown function names get the same treatment (builtins included — `pront`
suggests `print`). And if you write a Rust/Java-style return type by habit
(`fn f(x: i64): i64 {`), the error tells you the vulcan spelling (`-> i64`)
instead of a generic "expected `{`".

## A note on limits

Payload enums (including `Option` and `Result`) can't be printed directly or compared with `==` yet, and arrays of them are rejected with a clear compile error. Match on them instead. These gaps are on the roadmap.

Where to go next: [Chapter 7: Packages and projects](07-packages.md) — splitting your program into files and using libraries.
