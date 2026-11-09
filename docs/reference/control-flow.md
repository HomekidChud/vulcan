# Control flow

How Vulcan programs decide and repeat: `if`/`elif`/`else`, three loops, `break`/`continue`, `match`/`when`, and `defer`.

## Syntax

```
if cond { } elif cond { } else { }
while cond { }
loop { }
for i in a .. b { }          // exclusive
for i in a ..= b { }         // inclusive
for i in a ..= b by step { }
for x in array { }
break
continue
match value {
    when pattern, pattern { }
    else { }
}
defer { }
```

## Examples

### if / elif / else

```vulcan
fn main() {
    let items = 0
    if items == 0 {
        print("nothing to do — add something!")
    } elif items < 5 {
        print("a short list")
    } else {
        print("a long list")
    }
}
```

```
nothing to do — add something!
```

### while, loop, break

```vulcan
fn main() {
    for i in 1 ..= 3 {
        print("item", i)
    }
    let n = 3
    while n > 0 {
        n -= 1
    }
    print("countdown done:", n)
    let i = 0
    loop {
        i += 1
        if i == 5 { break }
    }
    print("loop stopped at", i)
}
```

```
item 1
item 2
item 3
countdown done: 0
loop stopped at 5
```

### Ranges with a step

```vulcan
fn main() {
    for i in 0 .. 10 by 3 {
        print(i)
    }
}
```

```
0
3
6
9
```

### for-in over an array, with continue

```vulcan
fn main() {
    let items = ["buy milk", "walk dog"]
    for it in items {
        if it == "walk dog" {
            continue
        }
        print(it)
    }
}
```

```
buy milk
```

### match on strings and numbers

```vulcan
fn main() {
    let day = "sat"
    match day {
        when "sat", "sun" { print("weekend") }
        else { print("weekday") }
    }
    match 3 {
        when 1 { print("one") }
        when 3 { print("three") }
        else { print("many") }
    }
}
```

```
weekend
three
```

### defer

```vulcan
fn main() {
    term_raw()
    defer { term_normal() }
    print("running in raw mode")
}
```

```
running in raw mode
```

## Semantics

- Conditions are `bool` expressions; there is no truthiness — `if 1 { }` is a type error.
- `for i in a .. b` iterates `a` up to (not including) `b`; `a ..= b` includes `b`; `by step` sets the stride. An empty range (`5 .. 3`) runs zero iterations.
- `loop` runs forever until `break`; `break` and `continue` work in all three loop forms.
- `match` compares a value against patterns: enum variants (with optional payload bindings), string literals, and integer literals. Multiple patterns per arm, comma-separated. `else` catches everything else.
- `defer` blocks run when their scope exits — function return, early `return`, or normal fall-through — in reverse order if several are deferred. It compiles to `try/finally` on the JS and Python backends.

## Edge cases

- Ranges are not expressions — `print(1 .. 3)` is a syntax error. Use them in `for` headers.
- `match` is a statement, not an expression: it can't produce a value. Return from inside arms.
- A `match` with no matching arm and no `else` silently does nothing — cover your cases.

## See also

- [Enums](enums.md) — matching enums with payloads
- [Functions](functions.md) — `return` inside loops and matches
- [Closures and iterators](closures-iterators.md) — a functional alternative to loops
