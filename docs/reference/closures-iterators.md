# Closures and iterators

A closure is a function value; the iterator methods run a closure over every element of an array.

## Syntax

```
|param| expression
|a, b| expression          // two parameters

array.map(f)                // new array: f of each element
array.filter(f)             // new array: elements where f is true
array.reduce(init, f)       // f(acc, x) folded over the array
array.any(f)                // true if f is true for some element
array.all(f)                // true if f is true for every element
```

## Examples

### Closure capturing a variable

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

### map and filter

```vulcan
struct Item {
    text: string,
    done: bool,
}

fn main() {
    let items = [
        Item { text: "buy milk", done: true },
        Item { text: "walk dog", done: false },
        Item { text: "ship app", done: false },
    ]
    let open = items.filter(|it| it.done == false)
    print("open items:", open.len())
    let labels = open.map(|it| it.text)
    print(labels)
    let texts = items.map(|it| it.text)
    print(texts.join(", "))
    print("all have text:", texts.all(|t| t.len() > 0))
}
```

```
open items: 2
[walk dog, ship app]
buy milk, walk dog, ship app
all have text: true
```

### reduce

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

### Chaining

```vulcan
fn main() {
    let xs: [i64] = [1, 2, 3, 4, 5, 6]
    let result = xs.map(|x| x * 2).filter(|x| x % 3 == 0).reduce(0, |acc, x| acc + x)
    print(result)
    print(xs.any(|x| x > 5), xs.all(|x| x > 0))
}
```

```
18
true true
```

## Semantics

- Closure bodies are **single expressions** — `|x| x * factor`, `|acc, x| acc + x`.
- Closures **capture** outer variables by value at call time.
- The iterator methods **return new arrays** (`map`, `filter`) or a value (`reduce`, `any`, `all`); they never mutate the original.
- They work on every backend with identical results — this is enforced by the selftest suite.

## Edge cases

- Multi-statement closure bodies (`|x| { let y = x + 1; return y }`) are a **syntax error**. Write a single expression, or use a named function / `for` loop.
- Calling a struct **method** inside a closure (`items.map(|it| it.label())`) miscompiles on the JS backend today — a known gap. Use field access (`|it| it.text`) or a loop.
- `reduce` on an empty array returns the initial value.

## See also

- [Arrays and strings](arrays-strings.md) — the other array methods
- [Control flow](control-flow.md) — `for x in arr` as the loop alternative
- [Functions](functions.md)
