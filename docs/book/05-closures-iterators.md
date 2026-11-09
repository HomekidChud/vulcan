# Chapter 5: Closures and iterators

In this chapter you'll learn to write closures and use the iterator methods every array has: `map`, `filter`, `reduce`, `any`, and `all`. Your todo list gains searching and reporting in a handful of lines.

## Closures

A closure is a function value written inline between pipes:

```vulcan
fn main() {
    let factor = 3
    let xs = [1, 2, 3]
    print(xs.map(|x| x * factor))
}
```

Output:

```
[3, 6, 9]
```

`|x| x * factor` is a function of one parameter `x` whose body is a single expression. Closures capture outer variables (`factor` above), and they work identically on every backend.

## The iterator methods

Every array has five iterator methods, each taking a closure:

| method | what it does |
|---|---|
| `map(f)` | a new array: `f` applied to each element |
| `filter(f)` | a new array: elements where `f` is `true` |
| `reduce(init, f)` | fold: `f(acc, x)` over all elements, starting at `init` |
| `any(f)` | `true` if `f` is `true` for some element |
| `all(f)` | `true` if `f` is `true` for every element |

On the todo list:

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

Output:

```
open items: 2
[walk dog, ship app]
buy milk, walk dog, ship app
all have text: true
```

`reduce` folds a list down to one value:

```vulcan
fn main() {
    let marks = [3, 1, 4, 1, 5]
    let total = marks.reduce(0, |acc, x| acc + x)
    print("sum:", total)
    let doubled = marks.map(|x| x * 2)
    print(doubled)
}
```

Output:

```
sum: 14
[6, 2, 8, 2, 10]
```

## Chaining

Iterator methods return arrays, so they chain:

```vulcan
fn main() {
    let xs: [i64] = [1, 2, 3, 4, 5, 6]
    let result = xs.map(|x| x * 2).filter(|x| x % 3 == 0).reduce(0, |acc, x| acc + x)
    print(result)
    print(xs.any(|x| x > 5), xs.all(|x| x > 0))
}
```

Output:

```
18
true true
```

## Limits to know about

- Closure bodies are **expressions or blocks**. A block body ends with the value expression: `|x| { let y = x + 1; y }` (the last statement is the closure's value). `return` is not used inside closures.
- Multi-statement blocks can't contain `return`, `break` or `continue` — the value is the trailing expression.

Where to go next: [Chapter 6: Errors](06-errors.md) — `Option`, `Result`, and the `?` operator.
