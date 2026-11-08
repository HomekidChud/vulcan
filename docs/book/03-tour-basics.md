# Chapter 3: A tour of the basics

In this chapter you'll learn variables, the four core types, f-strings, operators, control flow, and functions. You'll also start the book's running example: a todo list.

## Variables: `let`

`let` declares a variable. You can annotate the type or let Vulcan infer it:

```vulcan
fn main() {
    let count = 3          // inferred: i64
    let price: f64 = 4.5   // annotated: f64
    let open = true        // bool
    let title = "buy milk" // string
    print(count, price, open, title)
    count = count + 1
    print(count)
}
```

Output:

```
3 4.5 true buy milk
4
```

Variables are mutable — no `mut` keyword. `print` accepts any number of values and joins them with single spaces.

The core types are `i64` (integers), `f64` (floats), `bool` (`true`/`false`), and `string`. There are also small numeric types (`i8` `i16` `i32` `u8` `u16` `u32` `f32`) — [Chapter 8](08-backends.md) and the [types reference](../reference/types.md) cover them, but you can go far with `i64` and `f64` alone.

## Strings and f-strings

Put `{expression}` inside a double-quoted string to interpolate it:

```vulcan
fn main() {
    let item = "buy milk"
    let qty = 2
    print(f"todo: {qty} x {item}")
    print(f"total: {qty * 2}")
    print(f"shouted: {item.upper()}")
}
```

Output:

```
todo: 2 x buy milk
total: 4
shouted: BUY MILK
```

The `{}` can hold any expression, including method calls. Strings concatenate with `+`, and have methods like `.upper()`, `.len()`, `.split(",")` — the full list is in the [arrays and strings reference](../reference/arrays-strings.md).

## Operators

Arithmetic (`+ - * / %`), comparisons (`== != < <= > >=`), logical (`&& || !`), and bitwise (`& | ^ ~ << >>`) all work as you'd expect. Integer division truncates toward zero: `10 / 3` is `3`, `-7 / 2` is `-3`. Division or modulo by zero aborts with an error instead of crashing or producing `Infinity` — on every backend.

## Control flow

`if` / `elif` / `else`:

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

Output:

```
nothing to do — add something!
```

Three loops — `for` over a range, `while`, and `loop` (infinite, use `break`):

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

Output:

```
item 1
item 2
item 3
countdown done: 0
loop stopped at 5
```

`1 ..= 3` includes 3; `1 .. 3` stops before it. Add `by 2` to step: `for i in 0 .. 10 by 3` visits 0, 3, 6, 9. `for x in arr` walks an array directly. `break` and `continue` work in every loop.

## Functions

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

Two things to notice, because they shape every Vulcan program:

1. **Arrays passed to a function are copied.** `add_item` pushes onto its own copy and returns it; the caller reassigns `items = add_item(items, ...)`. This "copy in, copy out, reassign" pattern is idiomatic Vulcan — you'll see it everywhere, from the todo app to the physics engine (`w = world_step(w, dt)`).
2. **Functions can be defined in any order.** `main` can call a function defined below it.

A function that returns nothing has no `->` (or `-> void`). `return` is optional in that case.

## Comments

`//` for a line, `/* ... */` for a block:

```vulcan
fn main() {
    // one line
    /* a block that
       spans lines */
    print("comments everywhere")
}
```

Output:

```
comments everywhere
```

Where to go next: [Chapter 4: Structs and enums](04-structs-enums.md) gives your todo items real shape.
