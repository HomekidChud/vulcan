# Structs

A struct is a fixed group of named fields — Vulcan's way to give one value many parts.

## Syntax

```
struct Name {
    field: Type,
    field: Type,
}

impl Name {
    fn method(self, params) -> ReturnType {
        ...
    }
}

let value = Name { field: expr, field: expr }
```

## Examples

### Define, create, read fields

```vulcan
struct Item {
    text: string,
    done: bool,
}

impl Item {
    fn label(self) -> string {
        if self.done {
            return f"[x] {self.text}"
        }
        return f"[ ] {self.text}"
    }
    fn complete(self) -> Item {
        self.done = true
        return self
    }
}

fn main() {
    let a = Item { text: "buy milk", done: false }
    print(a.label())
    let b = a.complete()
    print(b.label())
}
```

```
[ ] buy milk
[x] buy milk
```

### Assignment copies

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

```
p: 1 2
q: 11 12
```

### A method called on a variable updates that variable

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
    p.move(10.0, 10.0)   // called on the variable itself
    print(p.x, p.y)
}
```

```
11 12
```

### Arrays of structs, struct field access through indexing

```vulcan
struct Item { text: string, done: bool }

fn main() {
    let items = [Item { text: "x", done: false }]
    print(items[0].text)
    items[0].done = true
    print(items[0].done)
}
```

```
x
true
```

## Semantics

- Structs **copy on assignment, argument passing, and return** (shallow copy — array fields are shared, because arrays are reference-like).
- **A method call on a variable mutates that variable.** The receiver is the variable's storage, not a copy. The mutated value is also returned, so chaining works.
- `impl` blocks can appear anywhere at file scope; methods take `self` as their first parameter (a copy that the method can mutate and return).
- Methods are called with dot syntax: `value.method(args)`. Method calls work on any expression receiver (`pts[0].move(...)`, `get_pos(e).move(...)`), where they mutate a temporary copy.
- Structs can nest: `struct Rect { p1: Point, p2: Point }`.

## Edge cases

- Copying is shallow: if a struct has an array field, both copies share the array's elements.
- Nested arrays of structs **as struct fields** (`struct W { grid: [[Cell]] }`) are not supported yet — a clear compile error. Grids of scalars and local `[[T]]` variables work.
- Structs don't compare with `==`; compare fields, or use an enum if you need identity.
- Calling a struct method inside a closure (`items.map(|it| it.label())`) is not reliable across backends yet — the JS backend miscompiles it. Use a `for` loop or field access.

## See also

- [Functions](functions.md) — plain functions
- [Enums](enums.md) — sum types
- [Types](types.md) — field types
