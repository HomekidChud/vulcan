# Functions

A named, reusable block of code with typed parameters and a return value.

## Syntax

```
fn name(param: Type, ...) -> ReturnType {
    statements
}
```

- Parameters: `name: Type`, comma-separated. Each is a copy (arrays copy too — see edge cases).
- Return type after `->`, or omit it (equivalently `-> void`) for no return value.
- `return expr` returns; a `void` function can use bare `return` to exit early.

## Examples

### Basic function

```vulcan
fn add(a: i64, b: i64) -> i64 {
    return a + b
}

fn main() {
    print(add(2, 3))
}
```

```
5
```

### Void function and early return

```vulcan
fn greet(name: string) {
    if name.len() == 0 {
        return
    }
    print("hello,", name)
}

fn main() {
    greet("ada")
    greet("")
    print("done")
}
```

```
hello, ada
done
```

### Forward references — order doesn't matter

```vulcan
fn main() {
    print(double(21))
}

fn double(n: i64) -> i64 {
    return n * 2
}
```

```
42
```

### Copy in, copy out

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

## Semantics

- Calls are expressions: `add(2, 3) * 2` works anywhere a value does.
- Function names live at file scope; two functions with the same name in one program are an error.
- Recursion works, and the C backend inlines pure self-recursive calls three levels deep for speed (semantics unchanged).
- Reference parameters `&T` / `&mut T` are supported for struct types — see [structs](structs.md).

## Edge cases

- **Arrays passed as parameters are copied.** A `push` inside the function does not reach the caller's array. Return the array and reassign — that's the idiom.
- **Structs pass by value too** — the callee gets an independent copy.
- A `-> void` function's `return` carries no value.

## See also

- [Structs](structs.md) — methods with `impl`
- [Types](types.md) — parameter and return types
- [Closures and iterators](closures-iterators.md) — function values
