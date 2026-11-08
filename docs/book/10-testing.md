# Chapter 10: Testing

In this chapter you'll learn Vulcan's built-in test runner, how `assert` reports failures, and how the compiler tests itself on all three backends. You'll add tests to the todo app in minutes.

## `test_*` functions

Any function whose name starts with `test_` is a test. `vulcan test` finds and runs them all:

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

Run it:

```bash
vulcan test file.vulcan
```

Output:

```
running 2 test(s)
  test_add ...
test_add passed
  test_add_negative ...
test_add_negative passed
all tests passed
```

In a project (with `vulcan.toml`), plain `vulcan test` runs every `test_*` function in the project.

## What a failure looks like

`assert(condition)` aborts with the line number; `assert(condition, "message")` adds your words:

```vulcan
fn main() {
    let n = 3
    assert(n == 4, "n must be 4")
    print("never printed")
}
```

Output:

```
vulcan: assertion failed at line 3: n must be 4
```

The failing test stops with the file line in the message, so a red test is a thirty-second fix, not an investigation.

## Testing patterns

**Test the function, not the print.** Give the function a return value and assert on it:

```vulcan
struct Item { text: string, done: bool }

fn count_open(items: [Item]) -> i64 {
    let n = 0
    for it in items {
        if it.done == false { n += 1 }
    }
    return n
}

fn test_count_open() {
    let items = [
        Item { text: "a", done: true },
        Item { text: "b", done: false },
    ]
    assert(count_open(items) == 1, "one open item")
    print("test_count_open passed")
}

fn main() {
    let items = [
        Item { text: "a", done: true },
        Item { text: "b", done: false },
    ]
    print(count_open(items))
}
```

`vulcan test` output:

```
running 1 test(s)
  test_count_open ...
test_count_open passed
all tests passed
```

**Test the edges.** Empty list, single item, already-done item. The todo app's `load()` reading an empty file, `to_int` on garbage input, an index one past the end — those are where bugs live, and each is a two-line test.

**Tests share the program.** Test functions live next to the code in the same file, can call any function, and can use the same structs. No test framework, no imports, no ceremony.

## The compiler tests itself

`vulcan selftest` runs the compiler's own test suite — dozens of cases covering the language, the libraries, and the game stack — on **all three backends**, and diffs the output. If C, JS, and Python ever disagree by one byte, the build fails. That's the same guarantee your program gets for free, and it's why "same output on every backend" is a tested property, not a hope.

In a project, `vulcan bench` times repeated runs of the entry point — useful to catch performance regressions too.

Where to go next: [Chapter 11: Safety](11-safety.md) — bounds checks, sanitizers, and an honest comparison with Rust.
