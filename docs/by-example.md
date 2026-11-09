# Vulcan by Example

Every snippet below is a complete, runnable program. Save it as `example.vulcan`, run `vulcan run example.vulcan`, and you'll see exactly the output shown. Verified on all three backends (C, JS, Python) unless a note says otherwise.

The book builds these concepts progressively — start at [Chapter 1](book/01-why-vulcan.md) if you're new. This page is for looking things up.

## Hello world

```vulcan
fn main() {
    print("hello, Vulcan")
}
```

```
hello, Vulcan
```

## Variables and types

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

```
3 4.5 true buy milk
4
```

## F-strings

```vulcan
fn main() {
    let item = "buy milk"
    let qty = 2
    print(f"todo: {qty} x {item}")
    print(f"total: {qty * 2}")
    print(f"shouted: {item.upper()}")
}
```

```
todo: 2 x buy milk
total: 4
shouted: BUY MILK
```

## Operators and division

```vulcan
fn main() {
    print(10 / 3, 10 % 3, 10.0 / 4.0, 1.0 / 3.0)
    print(2 + 3 * 4, (2 + 3) * 4)
    print(1 << 10, 1024 >> 3, 12 & 10, 12 | 10, ~5)
}
```

```
3 1 2.5 0.333333
14 20
1024 128 8 14 -6
```

## If / elif / else

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

## Loops

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

## Ranges with a step

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

```
[buy milk, walk dog]
```

Arrays passed to functions are copied — return the array and reassign.

## Comments

```vulcan
fn main() {
    // one line
    /* a block that
       spans lines */
    print("comments everywhere")
}
```

```
comments everywhere
```

## Structs and impl

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

## Value semantics

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

## Enums and match

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

## Enums with payloads

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

## Match on strings and numbers

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

## Arrays: create, push, pop

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

## Array bounds are checked

```vulcan
fn main() {
    let items = ["a", "b"]
    print(items[5])
}
```

```
vulcan: index out of bounds
```

(The message goes to stderr and the program exits with code 1.)

## Closures and iterators

```vulcan
fn main() {
    let factor = 3
    let xs = [1, 2, 3]
    print(xs.map(|x| x * factor))
}
```

```
[3, 6, 9]
```

```vulcan
fn main() {
    let marks = [3, 1, 4, 1, 5]
    let total = marks.reduce(0, |acc, x| acc + x)
    print("sum:", total)
    let doubled = marks.map(|x| x * 2)
    print(doubled)
}
```

```
sum: 14
[6, 2, 8, 2, 10]
```

## Strings

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

## Numbers: small types and casts

```vulcan
fn main() {
    let small: i32 = 5
    small += 1
    print(small)
    let byte: u8 = 250
    byte += 10
    print(byte)
    let f: f32 = 1.5
    print(f)
    print(3.99 as i64, -3.99 as i64)
    print(3 as f64)
    print('A', 'A' as i64)
}
```

```
6
4
1.5
3 -3
3
65 65
```

(`u8` wraps: 250 + 10 is 4, modulo 256.)

## Hex and binary literals

```vulcan
fn main() {
    print(0xFF, 0b1010)
}
```

```
255 10
```

## Option

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

## Result and `?`

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

```
marking item 2 done
error: no number given
```

## to_int fallback

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

```
43
that was not a number
```

## assert

```vulcan
fn main() {
    let n = 3
    assert(n == 4, "n must be 4")
    print("never printed")
}
```

```
vulcan: assertion failed at line 3: n must be 4
```

## defer

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

## Nested arrays and arrays of structs

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

## Imports

```vulcan
import math

fn main() {
    print(square(6.0))
    print(clamp(150.0, 0.0, 100.0))
    print(PI)
}
```

```
36
100
3.14159
```

(With `lib/math.vulcan` next to your file, or a project that has it in `deps/`.)

## Math builtins

```vulcan
fn main() {
    print(abs(-7), min(2, 9), max(2, 9))
    print(sqrt(25.0), pow(2.0, 10.0), atan2(1.0, 1.0))
    print(clamp(15, 0, 10), lerp(0.0, 10.0, 0.5), sign(-3))
}
```

```
7 2 9
5 1024 0.785398
10 5 -1
```

## FFI: calling C

```vulcan
extern "c" {
    fn sqrt(x: f64) -> f64
    fn floor(x: f64) -> f64
}

fn main() {
    print("sqrt(2) =", sqrt(2.0))
    print("floor(3.9) =", floor(3.9))
}
```

```
sqrt(2) = 1.41421
floor(3.9) = 3
```

## Command-line arguments

```vulcan
fn main() {
    print(arg_count(), arg(0), arg(1))
}
```

```
2 alpha beta
```

(Run as `vulcan run file.vulcan alpha beta` — everything after the file reaches your program.)

## Files

```vulcan
fn main() {
    file_write("notes.txt", "line1\nline2")
    print(file_exists("notes.txt"))
    print(file_read("notes.txt"))
}
```

```
true
line1
line2
```

## Tests

```vulcan
fn add(a: i64, b: i64) -> i64 {
    return a + b
}

fn test_add() {
    assert(add(2, 3) == 5)
    print("test_add passed")
}

fn test_add_negative() {
    assert(add(-2, 3) == 1, "negative numbers work")
    print("test_add_negative passed")
}

fn main() {
    print(add(2, 3))
}
```

Run with `vulcan test file.vulcan`:

```
running 2 test(s)
  test_add ...
test_add passed
  test_add_negative ...
test_add_negative passed
all tests passed
```

## Same output on three backends

```vulcan
struct Item { text: string, done: bool }

impl Item {
    fn label(self) -> string {
        if self.done { return f"[x] {self.text}" }
        return f"[ ] {self.text}"
    }
}

fn main() {
    let items = [
        Item { text: "buy milk", done: false },
        Item { text: "ship app", done: true },
    ]
    for it in items {
        print(it.label())
    }
    let open = items.filter(|it| it.done == false)
    print("open items:", open.len())
    print("total:", items.len())
}
```

```
[ ] buy milk
[x] ship app
open items: 1
total: 2
```

Byte-identical via `vulcan run`, `vulcan run --target js`, and `vulcan run --target py`.
