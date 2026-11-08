# Vulcan documentation

Vulcan is a small, statically-typed language that transpiles to C, JavaScript, and Python — same program, same output on every backend. This folder is the documentation home. Start with the book, look things up in the reference.

## The Vulcan Book

A tutorial that builds one todo-list CLI, one skill per chapter. For a reader who knows maybe one language (Python or JavaScript).

| chapter | what you'll learn |
|---|---|
| [1. Why Vulcan?](book/01-why-vulcan.md) | what it is, what it trades away, whether it fits you |
| [2. Install and quickstart](book/02-install-quickstart.md) | install, first program, all three backends, first project |
| [3. A tour of the basics](book/03-tour-basics.md) | variables, types, f-strings, control flow, functions |
| [4. Structs and enums](book/04-structs-enums.md) | `struct`, `impl` methods, `enum`, `match`, value semantics |
| [5. Closures and iterators](book/05-closures-iterators.md) | `map` / `filter` / `reduce` / `any` / `all` |
| [6. Errors](book/06-errors.md) | `Option`, `Result`, the `?` operator, `assert` |
| [7. Packages and projects](book/07-packages.md) | `import`, `vulcan init`, the registry, `vulcan publish` |
| [8. Backends and FFI](book/08-backends.md) | C/JS/Python targets, `extern "c"`, portability rules |
| [9. Shipping](book/09-shipping.md) | `vulcan package` to web/desktop/mobile/APK |
| [10. Testing](book/10-testing.md) | `test_*` functions, `vulcan test`, `assert` |
| [11. Safety](book/11-safety.md) | bounds checks, `--safe` sanitizers, hardening, vs Rust |
| [12. How it works](book/12-how-it-works.md) | the pipeline, self-hosting, the bootstrap fixpoint |
| [13. Idiomatic Vulcan](book/13-idiomatic-vulcan.md) | the patterns and the traps |

## Vulcan by Example

[by-example.md](by-example.md) — one complete, runnable program per concept, with the real output shown after each. Every snippet was verified on all three backends.

## Language reference

One page per construct — summary, syntax, semantics, tested examples, edge cases.

- [Functions](reference/functions.md)
- [Types](reference/types.md)
- [Structs](reference/structs.md)
- [Enums](reference/enums.md)
- [Control flow](reference/control-flow.md)
- [Closures and iterators](reference/closures-iterators.md)
- [Arrays and strings](reference/arrays-strings.md)
- [Builtins](reference/builtins.md)

## Internals and subsystem docs

Older, deeper documents written by and for compiler developers:

- [selfhost.md](selfhost.md) — the self-hosted compiler, Vulcan Core, the bootstrap chain
- [architecture.md](architecture.md) — the compiler pipeline, runtime model, tooling
- [net.md](net.md) — the TCP socket layer and the HTTP client
- [baseline.md](baseline.md) — recorded benchmark and version baseline (Phase 0)

## Elsewhere

- [README.md](../README.md) — the project overview, benchmarks, graphics/physics/game API reference
- [CHANGELOG.md](../CHANGELOG.md) — per-version history
- [TODO.md](../TODO.md) — the roadmap
- [DECISIONS.md](../DECISIONS.md) — design decisions and their rationale
