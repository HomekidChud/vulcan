# Builtins

The functions available in every Vulcan program without an import.

## Syntax

```
print(values...)                to_string(v)  to_int(s)  to_float(s)
char_at(s, i)                   is_prime(n)
len(arr_or_string)
arg_count()  arg(i)
file_read(path)  file_write(path, data)  file_exists(path)
exit(code)  sleep_ms(ms)  clock()  ms_now()  input()
rand_int(lo, hi)
abs(x)  min(a, b)  max(a, b)  sqrt(x)  pow(x, y)  atan2(y, x)
clamp(x, lo, hi)  lerp(a, b, t)  sign(x)
assert(cond)  assert(cond, "message")
```

## Examples

### Conversions

```vulcan
fn main() {
    print(to_string(42), to_int("17"), to_float("2.5"))
    print("num: " + to_string(7))
    let bad = to_int("not a number")
    print(bad)
}
```

```
42 17 2.5
num: 7
0
```

`to_int` returns `0` when the string isn't a number — use `Result` when you must tell "0" from garbage.

### Characters

```vulcan
fn main() {
    print(char_at("ABC", 1))
}
```

```
66
```

`char_at(s, i)` returns the character code at index `i`. It also works as a method: `s.char_at(i)`.

### Primes

```vulcan
fn main() {
    print(is_prime(2), is_prime(97), is_prime(100))
    let n = 0
    for i in 2 .. 1000 {
        if is_prime(i) { n++ }
    }
    print("primes below 1000:", n)
}
```

```
true true false
primes below 1000: 168
```

`is_prime(n)` uses 6k±1 wheel factorization — about 3x fewer divisions than naive trial division, identical results. It is pure (safe inside inlined functions) and works on every backend.

### Command-line arguments

```vulcan
fn main() {
    print(arg_count(), arg(0), arg(1))
}
```

```
2 alpha beta
```

(Run as `vulcan run file.vulcan alpha beta`. Everything after the file name is forwarded to your program.)

### Files

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

### Math

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

### assert

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

## Semantics

- `print` takes any number of values of any type and joins them with single spaces.
- `exit(code)` terminates the program immediately with that exit code.
- `sleep_ms(n)` pauses for n milliseconds; `clock()` returns CPU seconds as `f64`; `ms_now()` returns wall-clock milliseconds.
- `input()` reads one line from stdin (strips the newline).
- `rand_int(lo, hi)` returns a random integer in `[lo, hi]` (inclusive).
- `file_exists` returns a `bool`.
- All of these behave identically on the three backends. In the web bundle, file I/O is backed by an in-memory filesystem — same API, different environment.

## Time and randomness

```vulcan
fn main() {
    print(ms_now() > 0)
    sleep_ms(10)
    let r = rand_int(1, 6)
    print(r >= 1 && r <= 6)
}
```

```
true
true
```

## Beyond builtins: the libraries

Networking (`net_listen`, `net_connect`, `net_send`, ...), terminal games (`key_ready`, `read_key`, `term_raw`, `print_at`, `clear_screen`, `set_color`, `screen_width`), graphics (`canvas_resize`, `clear`, `draw_rect`, `draw_circle`, `draw_line`, `draw_text`, `present`), and UI (`ui_button`, `ui_joystick`, `ui_html`, `ui_css`, `ui_title`) are also built in — they're documented in the [README](https://github.com/HomekidChud/vulcan) and [docs/net.md](../net.md), and work on every backend (no-ops where there is no shell). Importable libraries add more: `lib/math.vulcan` (`PI`, `square`, `clamp`), `lib/http.vulcan` (HTTP/1.1 client), `lib/strbuf.vulcan` (string builder), `lib/physics.vulcan`, `lib/render3d.vulcan`, `lib/engine.vulcan`.

## Edge cases

- `to_int` never fails — it returns `0` on garbage.
- `char_at` works both ways: `char_at(s, 0)` and `s.char_at(0)`.
- `assert` aborts the program; it's for tests and invariants, not input validation.
- Division and modulo by zero abort with `vulcan: division by zero` — that's a language rule, not a builtin.

## See also

- [Arrays and strings](arrays-strings.md) — the method set on arrays and strings
- [Functions](functions.md)
- The [book](../book/01-why-vulcan.md) for where each builtin is used in practice
