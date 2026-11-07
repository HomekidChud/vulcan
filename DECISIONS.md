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

## Payload enums, Option/Result, `?`

- Enums with a payload on any variant ("payload enums") are tagged
  structs; payload-less enums stay plain i64 tags (existing output
  unchanged). Each variant gets its own slots (`v<variant>_<slot>`), and
  constructors zero every slot first so eager match bindings can safely
  read an unset slot (well-defined on all backends).
- `Option<T>` / `Result<T, E>` are synthetic payload enums, keyed by the
  mangled type name (`Option<i64>`, `Result<Pixel, string>`). They are
  not user-declarable.
- Construction: `Shape.Circle(2.0)`, `Option.Some(x)`; `Option.None` /
  `Result.Ok` / `Result.Err` need a type hint from the let annotation
  or the function return type (clear panic otherwise).
- `match` on a payload enum compares `__tag` and binds payloads;
  patterns bind identifiers (`when Shape.Circle(r) { ... }`). Payload
  bindings are visible in guards. Bindings are emitted eagerly before
  the tag checks (values unused for non-matching arms).
- `expr?` propagates None/Err from an Option/Result, requires the
  enclosing function to return the same family (Result: same error
  type), and yields the payload value. Propagation constructs a fresh
  None/Err of the return type.
- Match is a statement: arms produce values only via explicit `return`
  (as before). Payload enum values are compared only by matching;
  direct printing and `==` on payload enums are not supported yet
  (noted in TODO.md). Arrays of payload enums / Option / Result are
  gated with a clear compile error (noted in TODO.md).
- Value semantics: payload enum values copy on assignment/argument/
  return (like structs) on all backends.

## Reference parameters (&T / &mut T)

- References are function parameters and call arguments only (Phase 1):
  `fn translate(p: &mut Point, dx: f64)`, called as `translate(&mut p, 1.0, 2.0)`.
  `let r = &mut p`, returning references, and let/field types of ref kinds
  are gated with clear compile errors.
- `&expr` / `&mut expr` are address-of argument expressions (postfix
  allowed: `&Color.Green`). The callee sees the pointee: field reads
  auto-deref (`p->x` on C), writes through `&mut` stick on every backend.
- Call-site rules: a place (or `&`-expression) passed to a `&`/`&mut`
  param shares the object (no clone) — on C the address is taken; a ref
  passed on to a value param is dereferenced and copied (JS/Py clone at
  the boundary to match). A ref can be forwarded to another ref param.
- `let q = p` where p is a ref param copies the pointee (all backends).
- `match` on a reference matches the pointee (copied on C, shared on
  JS/Py — identical observable behavior).
- References to non-struct types (e.g. `&[i64]`, `&i64`) are gated with
  a clear error; struct-typed (incl. enum) refs only for now.
- Reassigning a reference parameter is an error on every backend.
