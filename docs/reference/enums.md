# Enums

An enum is a type with a fixed set of named values — plain enums — optionally each carrying data — payload enums.

## Syntax

```
enum Name { Variant, Variant, ... }                  // plain enum
enum Name { Variant(Type, ...), Variant, ... }       // payload enum

match value {
    when Name.Variant { ... }
    when Name.Variant(field) { ... }                 // binds payload
    when "str", "str2" { ... }                       // strings/numbers
    else { ... }
}
```

## Examples

### Plain enum with match

```vulcan
enum State { Todo, Doing, Done }

fn describe(s: State) -> string {
    match s {
        when State.Todo { return "not started" }
        when State.Doing { return "in progress" }
        else { return "finished" }
    }
}

fn main() {
    print(describe(State.Todo))
    print(describe(State.Done))
    let s = State.Doing
    print(s == State.Doing)
}
```

```
not started
finished
true
```

### Payload enum

```vulcan
enum Shape { Circle(f64), Rect(f64, f64) }

fn area(s: Shape) -> f64 {
    match s {
        when Shape.Circle(r) { return 3.14159 * r * r }
        when Shape.Rect(w, h) { return w * h }
    }
}

fn main() {
    print(area(Shape.Circle(2.0)))
    print(area(Shape.Rect(3.0, 4.0)))
}
```

```
12.5664
12
```

### Enum as integer

```vulcan
enum Color { Red, Green, Blue }

fn main() {
    let c = Color.Red
    print(c as i64)
}
```

```
0
```

## Option and Result

`Option<T>` and `Result<T, E>` are synthetic payload enums — not user-declarable, but matched the same way:

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

```
found: buy milk
not found
```

The `?` operator propagates `None`/`Err` from an `Option`/`Result` expression; the enclosing function must return the same family (for `Result`, the same error type), and it yields the payload value. See [the errors chapter](../book/06-errors.md).

## Semantics

- **Plain enums** (no payloads anywhere) are integer tags: comparable with `==`, printable via `as i64`, zero-cost.
- **Payload enums** are tagged structs: each variant gets its own slots; constructors zero every slot first. They copy by value on assignment/argument/return, like structs.
- **`match`** compares the tag and binds payload identifiers (`when Shape.Circle(r)`). Bindings are visible in the arm's body. `match` also works on strings and numbers, arms can list multiple patterns (`when "sat", "sun"`), and `else` is the fallback.
- `match` is a **statement**: arms produce values only via explicit `return`.
- `Option.None` / `Result.Ok` / `Result.Err` constructions need a type hint from a `let` annotation or the function's return type.

## Edge cases

- Payload enums can't be printed directly or compared with `==` — match on them. (Plain enums can't be printed directly either; use `as i64`.)
- Arrays of payload enums / `Option` / `Result` are rejected with a clear compile error (on the roadmap).
- Missing an arm with no `else` falls through silently — cover every variant or add `else`.

## See also

- [Control flow](control-flow.md) — `match` and `when` in full
- [Structs](structs.md) — the value semantics payload enums share
- [Errors chapter](../book/06-errors.md) — `Option`/`Result`/`?` as error handling
