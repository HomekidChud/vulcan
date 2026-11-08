# Types

The set of value types Vulcan programs use, and the conversions between them.

## Syntax

```
i64  f64  bool  string  void
i8 i16 i32 u8 u16 u32 f32        (small numeric kinds)
[T]                              (array of T)
StructName                       (user struct)
EnumName                         (user enum)
Option<T>  Result<T, E>          (synthetic payload enums)
```

## Examples

### The core four

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

### Small numeric kinds wrap

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
}
```

```
6
4
1.5
```

### Casts with `as`

```vulcan
fn main() {
    print(3.99 as i64, -3.99 as i64)
    print(3 as f64)
    print('A', 'A' as i64)
}
```

```
3 -3
3
65 65
```

### Literals

```vulcan
fn main() {
    print(42, 3.14, 0xFF, 0b1010, true, false)
}
```

```
42 3.14 255 10 true false
```

## Semantics

- **Literals are `i64` or `f64`.** A small kind only comes from an annotation or an `as` cast.
- **Arithmetic computes wide.** `+ - * / %` and bitwise ops promote integers to `i64` and floats to `f64`; narrowing back to a small kind happens at storage points (annotated `let`, assignment, argument passing, field/element stores) and wraps modulo the target width (two's complement for signed kinds).
- **Implicit conversions, no `as` needed:** int → int (wraps), int → float, float → float (f64 → f32 rounds to nearest). Float → int always requires `as`.
- **Integer division truncates toward zero:** `10 / 3` is `3`, `-7 / 2` is `-3`. Division or modulo by zero aborts with `vulcan: division by zero` on every backend.
- **Float printing uses 6 significant digits** on every backend: `1.0 / 3.0` prints `0.333333`, `10.0` prints `10`.
- **`void`** is the type of functions that return nothing.
- **`char` literals** like `'A'` are integer values (an `i64` holding the character code); `'A' as i64` is `65`. They are not a separate type.
- **Arrays of small kinds** are backed by wide vectors in C — semantics match, but there's no memory saving for `[i32]` on the C backend.

## Edge cases

- `u8` arithmetic wraps: `250 + 10` stored into a `u8` is `4`.
- `3.99 as i64` truncates (not rounds): `3`. `-3.99 as i64` is `-3`.
- Overflow of `i64` wraps silently in normal builds; under `--safe` it aborts (UBSan).
- Hex and binary literals (`0xFF`, `0b1010`) are integers.

## See also

- [Arrays and strings](arrays-strings.md)
- [Structs](structs.md) and [Enums](enums.md)
- [Builtins](builtins.md) — `to_int`, `to_float`, `to_string` conversions
