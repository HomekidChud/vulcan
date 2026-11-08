# Arrays and strings

Arrays are growable, bounds-checked lists (`[T]`). Strings are immutable text with a method set of their own.

## Syntax

```
let xs: [i64] = []                // empty, annotated
let xs = [1, 2, 3]                 // literal
xs[i]                             // read element (bounds-checked)
xs[i] = value                     // write element (bounds-checked)
[[T]]                             // nested arrays (locals/params)
```

## Array examples

### The method set

```vulcan
fn main() {
    let xs: [i64] = []
    xs.push(10)
    xs.push(20)
    xs.push(30)
    print(xs.len(), xs.last(), xs.contains(20))
    print(xs.pop())
    print(xs.shift())
    xs.insert(0, 5)
    print(xs)
    xs.remove(0)
    print(xs)
    xs[0] = 99
    print(xs)
}
```

```
3 30 true
30
10
[5, 20]
[20]
[99]
```

### Bounds are checked on every backend

```vulcan
fn main() {
    let items = ["a", "b"]
    print(items[5])
}
```

```
vulcan: index out of bounds
```

(stderr; exit code 1 — identical on C, JS, and Python.)

### Nested arrays and arrays of structs

```vulcan
struct Item { text: string, done: bool }

fn main() {
    let lists: [[string]] = [["a", "b"], ["c"]]
    print(lists[0][1], lists.len())
    let items = [Item { text: "x", done: false }]
    print(items[0].text)
    items[0].done = true
    print(items[0].done)
}
```

```
b 2
x
true
```

## String examples

### Methods

```vulcan
fn main() {
    let s = "Vulcan"
    print(s.len(), s.upper(), s.lower())
    print(s.contains("ulc"), s.find("u"), s.find("z"))
    print(s.substring(0, 3), s.substring(3, s.len()))
    print("a,b,c".split(","))
    print("a-b-c".replace("-", "+"))
    print("ha".repeat(3))
    print(s.starts_with("Vul"), s.ends_with("can"))
    print(char_at("Vulcan", 4))
}
```

```
6 VULCAN vulcan
true 1 -1
Vul can
[a, b, c]
a+b+c
hahaha
true true
97
```

### Concatenation and f-strings

```vulcan
fn main() {
    let item = "buy milk"
    let qty = 2
    print("todo: " + item)
    print(f"{qty} x {item}")
}
```

```
todo: buy milk
2 x buy milk
```

## Semantics

### Arrays

- Growable (`push`), with `pop` (remove last), `shift` (remove first), `insert`, `remove`, `contains`, `last`, `join`.
- **Every access is bounds-checked** — an out-of-range index aborts with an error instead of corrupting memory.
- Arrays are **reference-like when passed around**: a `push` on an array you hold in the same scope mutates it, but an array **passed as a function parameter is copied** — mutations inside the function don't reach the caller. Return the array and reassign (the copy-in/copy-out idiom).
- Elements print as `[a, b, c]` — with a space after each comma.
- The iterator methods (`map` / `filter` / `reduce` / `any` / `all`) are covered in [closures-iterators](closures-iterators.md).

### Strings

- Immutable: every method returns a new string.
- `substring(start, end)` — end exclusive. Out-of-range values are clamped.
- `find` returns the first index of the substring, or `-1`.
- `split(sep)` returns an array; `join(sep)` is the array method that glues them back.
- `replace(old, new)` is a literal (non-regex) replacement; an empty `old` is a no-op.
- `char_at(s, i)` is a builtin (not a method) and returns the character code.
- `+` concatenates; f-strings interpolate any expression.
- String comparison with `==` and `!=` works.

## Edge cases

- Nested arrays `[[T]]` work as locals, parameters, and returns. **Nested arrays of structs as struct fields** (`struct W { grid: [[Cell]] }`) are rejected with a clear compile error — on the roadmap.
- `pop()` on an empty array returns a zero value (0, `""`) rather than aborting, and backends differ on this edge — guard with `xs.len() > 0` before popping.
- Building strings by `+` in a loop is slow (each `+` copies); collect parts in a `[string]` and `join` once.

## See also

- [Closures and iterators](closures-iterators.md)
- [Types](types.md) — element types
- [Builtins](builtins.md) — `len`, `char_at`
