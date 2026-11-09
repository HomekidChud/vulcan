# Chapter 4: Structs and enums

In this chapter you'll learn to group data into structs, attach methods with `impl`, and model fixed choices with enums and `match`. Your todo items become first-class values.

## Structs: group data

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

Output:

```
[ ] buy milk
[x] buy milk
```

- `struct Item { ... }` defines the shape; `Item { text: ..., done: ... }` creates a value.
- `impl Item { fn label(self) ... }` attaches methods. Inside, `self` is the value the method was called on. Methods that mutate `self` return it, and callers chain or reassign.

## Value semantics: the one rule to internalize

Assigning a struct **copies** it. Calling a method on a **variable** updates that variable in place. Watch:

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

`q` moved; `p` didn't. But this:

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

Output:

```
11 12
```

Calling `p.move(...)` updates `p` itself — the receiver is the variable, not a copy. The result is returned too, so `let q = p.move(10.0, 10.0)` gives you the moved value in `q` while `p` has also moved to it. To keep the original, copy first (`let q = p`), then mutate one of them. This is consistent on every backend, and it's why the physics engine's API looks like `w = world_step(w, dt)`: pass values in, get values back, reassign.

## Enums: fixed choices

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

Output:

```
not started
finished
true
```

Enums compare with `==` and match with `match` / `when`. `match` also works on strings and numbers, with multiple patterns per arm (`when "sat", "sun"`) and an `else` fallback.

## Enums that carry data

A variant can carry payloads — Vulcan's version of a sum type:

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

Output:

```
12.5664
12
```

Payload enums copy by value like structs. You can't print them directly or compare them with `==` — match on them instead. The [enums reference](../reference/enums.md) has the full rules.

## Structs with struct fields

Structs nest, and arrays of structs work everywhere:

```vulcan
struct Item { text: string, done: bool }

fn main() {
    let items = [Item { text: "x", done: false }]
    print(items[0].text)
    items[0].done = true
    print(items[0].done)
}
```

Output:

```
x
true
```

Where to go next: [Chapter 5: Closures and iterators](05-closures-iterators.md) shows the functional toolkit that makes lists of structs pleasant to work with.
