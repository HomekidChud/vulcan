# Design Decisions

Simplest-option calls made while implementing the upgrade phases, per the
working rules: if something is ambiguous, pick the simplest option, write it
down here, and keep going.

## Small numeric types (i8/i16/i32/u8/u16/u32/f32)

- **Literals stay i64/f64.** A small kind only comes from an annotation
  (`let x: i32 = 5`) or an `as` cast (`n as u8`). No literal suffixes.
- **Arithmetic computes wide.** `+ - * / %` and bitwise ops promote every
  int kind to i64 and every float kind to f64 before computing; the result
  is i64/f64. Narrowing back to a small kind happens at storage points
  (let with annotation, assignment, argument passing, field/element stores)
  and wraps modulo the target width (two's complement for signed kinds).
  This avoids C signed-overflow UB and keeps all three backends identical.
- **f32 rounds to nearest** on narrowing (`(float)` in C, `Math.fround` in
  JS, `struct.pack('f')` in Python). Printing an f32 widens it to f64 and
  uses the existing 6-significant-digit float formatter on every backend.
- **Implicit conversions** (no `as` needed): int→int (narrowing wraps),
  int→float, float→float (f64→f32 rounds). Float→int still requires `as`.
- **Arrays of small kinds are backed by the wide vectors in C** (`[i32]`
  uses VecI64, `[f32]` uses VecF64); values are wrapped to the element kind
  on every store, so semantics match — but there is no memory saving for
  small-element arrays in C. JS/Py have no storage distinction.
- `+=`, `++` etc. on small kinds wrap back to the target kind after the
  operation on all three backends.

## Checked division

- `/` or `%` by zero — int or float, including compound forms — aborts
  with `vulcan: division by zero` (or `modulo by zero`) on stderr, exit 1,
  on all three backends. Previously C crashed or produced inf, JS produced
  Infinity/NaN, Python raised ZeroDivisionError. Float division by zero is
  now an error rather than inf everywhere.

## Strings

- `s.replace(old, new)` is a literal (non-regex) replacement; an empty
  `old` pattern is defined as a no-op on all backends.
- `lib/strbuf.vulcan` is the string builder: a `[string]` parts array with
  amortized O(1) push and one `join` at the end.

## Value semantics (earlier in Phase 1)

- C structs copy on assignment (shallow — array fields are shared, because
  C arrays are reference-like headers). JS/Py deep-copy structs but share
  arrays to match the C behavior exactly.
