# Vulcan

A small, fast-to-compile language with three backends: native code through C (`clang -O3`), JavaScript (Node.js), and Python. Static types, no garbage collector, no runtime — and the same program produces the same output on every backend.

```vulcan
fn fib(n: i64) -> i64 {
    if n < 2 { return n }
    return fib(n - 1) + fib(n - 2)
}

fn main() {
    print(fib(35))    // 9227465
}
```

## Vulcan 1.4 — what's new

Nested arrays, game math builtins, and the first release of the **Vulcan Engine**.

- **Nested arrays** — `[[i64]]`, `[[f64]]`, `[[string]]`, `[[bool]]`, `[[Point]]`,
  and deeper (`[[[i64]]]`), on all three backends with identical output:
  ```vulcan
  let grid: [[i64]] = []
  for r in 0 .. 8 {
      let row: [i64] = []
      for c in 0 .. 8 { row.push(r * 8 + c) }
      grid.push(row)
  }
  grid[3][5] = 99        // read, write, compound assign (grid[3][5] += 1)
  ```
  Struct fields support nested scalar grids (`tiles: [[i64]]`). The C backend
  generates `Vec_VecI64`-style vector-of-vector types on demand.
- **`clamp(x, lo, hi)`, `lerp(a, b, t)`, `sign(x)`** — game math builtins on all
  backends (i64/f64 overloads, matching `min`/`max`).
- **`lib/engine.vulcan` — the Vulcan Engine v0.1** — a game-engine layer in pure
  vulcan on top of `render3d` and the canvas: entity registry (`scene_spawn` /
  `scene_find` / `scene_reap`), heightmap terrain (`[[f64]]` grids + ground
  collision), gravity/friction simulation, a WASD + jump player controller with
  third-person chase camera, HUD helpers (health bar, crosshair, minimap), and
  a fixed-timestep loop (`engine_tick`). Like the physics engine it follows
  copy-in/copy-out (`s = scene_step(s, dt)`) so one source runs on value-semantics
  C and reference-semantics JS/Python.
  ```vulcan
  import engine
  fn main() -> void {
      let s = scene_new()
      s = scene_flat_terrain(s, 16, 0.0)
      s = scene_spawn(s, "player", ENTITY_KIND_PLAYER, Vec3 { x: 0.0, y: 5.0, z: 0.0 })
      let player = scene_last_id(s)
      while true {
          s = player_control(s, player, 1.0 / 60.0)
          s = engine_tick(s, 1.0 / 60.0, 1.0 / 60.0)
          // draw with the canvas + render3d builtins, then present()
      }
  }
  ```
- Selftest grew to 22 cases (nested arrays, clamp/lerp/sign, engine smoke test —
  all byte-identical across backends). Roadmap now lives in `TODO.md`.

## Vulcan 1.3 — what's new

The repo no longer ships pre-bundled demo games — `examples/` is now pure language and tooling
reference code (`tour`, `fib`, `todo`, `impl_demo`, `ffi_demo`), and the IDE ships the same set.
The compiler gained the math builtins that real simulation, steering and aiming code needs:

- **`atan2(y, x)`** — two-argument arctangent, the function bot AI, projectile and camera code
  can't live without. Available as a free function on all three backends (libm `atan2`,
  `Math.atan2`, `math.atan2`), with identical `%.6g` output everywhere.
- **`tan(x)`, `exp(x)`, `log(x)`** — join `sqrt` / `sin` / `cos` / `pow` as free functions on
  every backend.
- New selftest case covering all four — 19 cases, each verified byte-for-byte on C, JS, and Python.

## Vulcan 1.2 — what's new

Vulcan now talks to the outside world: FFI, C libraries, host-language interop and a plugin hook.

- **`extern "c" { ... }`** — declare C functions and call them like ordinary Vulcan fns:
  ```vulcan
  extern "c" {
      fn sqrt(x: f64) -> f64
      fn sin(x: f64) -> f64
  }
  fn main() -> void { print(sqrt(2.0)) }
  ```
  On the native backend each declaration becomes a real C prototype and the call links
  straight against libc/libm — no wrapper, native speed. Parameter/return types are
  i64/f64/bool/string. Declarations must match the C header's ABI (double-returning
  functions like sqrt/sin/cos/floor/ceil/fabs are the portable set).
- **`link "name"`** — a top-level statement that adds `-l<name>` to the native linker,
  so any C library is reachable (curl, sqlite, raylib, SDL...). Ignored on JS/Python.
- **Host-language interop** — the same `extern` names resolve to a host table instead:
  JS uses the `__ffi` table (Math.sqrt, ...), Python the `_ffi` dict (math.sqrt, ...).
  A single Vulcan program runs identically on all three backends (verified byte-identical
  in the selftest).
- **Plugin hook** — hosts can extend or override the table at runtime before running
  Vulcan code:
  ```js
  globalThis.__vulcan_ffi = { my_plugin_fn: (x) => x * 2 };
  ```
  ```python
  _vulcan_ffi = { "my_plugin_fn": lambda x: x * 2 }
  ```
  `extern`-declared fns with those names then call into the host plugin.
- New example `examples/ffi_demo.vulcan` (also in the IDE), new selftest case — 18 cases.
- The IDE is hostable: `VULCAN_IDE_ADDR=0.0.0.0 vulcan ide 8000` binds all interfaces, the
  frontend honors a `window.__API_BASE__` proxy prefix, and the JS backend now compiles to
  async functions with promise-based sleeps — so live keyboard/mouse input works even in
  hosted previews without SharedArrayBuffer (input falls back to postMessage).

## Vulcan 1.1 — what's new

Vulcan now has a **software 3D renderer**:

- **`draw_tri` — depth-buffered triangles** — the canvas gained a real 3D pipeline: barycentric rasterization with a per-pixel z-buffer, pixel-center sampling, and winding-insensitive coverage. Draw far triangles first or last — the depth buffer makes the result identical. Works on all three backends, byte-for-byte.
- **`import render3d`** — `lib/render3d.vulcan` gives you `Vec3` math (add/sub/scaled/dot/cross/length/normalized via impl methods), a `Camera` (position + yaw/pitch), `project()` (world point → screen x/y + camera depth + visibility flag), `draw_quad()`, and color helpers (`color_shade`, `color_lerp`). Together with `draw_tri` that's a complete software renderer: perspective projection, flat shading, fog, and hidden-surface removal — in ~130 lines of pure Vulcan.
- `to_string(n)` now works on the JS and Python backends too (it was C-only).
- The Python backend no longer breaks when builtins like `sqrt()` are called inside `impl` methods (Python name-mangled the runtime helpers; they're now mangle-proof).
- The selftest suite now covers all of the above (17 cases, each verified on C, JS, and Python — the animated examples produce identical output on all backends, ANSI frames included).

  ```bash
  vulcan ide                # http://127.0.0.1:7788 — open examples/tour.vulcan, Run
  ```

## Vulcan 1.0 — what's new

Vulcan is a physics-engine scripting IDE:

- **`vulcan ide`** — launch a full browser IDE (localhost, zero dependencies): syntax-highlighted editor, examples, file sidebar, console panel, and a **live canvas**. Programs run in a sandboxed Web Worker; `present()` streams 256x144 frames to the canvas at your `sleep_ms` frame rate, and `mouse_x()` / `mouse_y()` / `mouse_down()` / `key_down(code)` feed live input back into your program. Ctrl+Enter runs. Your files are saved in `.ide/`, and `import physics` is always available.

  ```bash
  vulcan ide          # http://127.0.0.1:7788 — live canvas + physics engine
  ```
- **A 2D rigid-body physics engine** — `import physics` gives you circles, boxes, static walls, gravity, restitution, friction, damped springs, impulse-based collision resolution with iterative solving, and positional correction for stable stacking. Deterministic, byte-for-byte identical on all three backends, and written in pure Vulcan (~400 lines, `lib/physics.vulcan`).

  ```vulcan
  import physics

  fn main() -> void {
      let w = world_new(0.0, 900.0)          // gravity
      w = world_add_bounds(w, 0.75, 0.1)     // walls + floor
      w = world_add_circle(w, 40.0, 20.0, 8.0, 1.0, 0.8, 0.1)
      w = world_add_box(w, 120.0, 30.0, 10.0, 8.0, 2.0, 0.2, 0.5)
      w = world_add_spring(w, 1, 2, 50.0, 60.0, 2.0)
      while true {
          w = world_step(w, 1.0 / 60.0)
          render(w)                          // draw with the canvas builtins
      }
  }
  ```
- **A graphics layer for every backend** — a 256x144 canvas with `clear(color)`, `draw_rect`, `draw_circle`, `draw_line`, `draw_text` (3x5 pixel font), `canvas_width()`, `canvas_height()` and `present()`. The same program renders to the IDE canvas, or to your terminal as truecolor half-block ANSI — byte-for-byte identical across C, JS, and Python.
- **Compiler fixes** — whole-number float literals (`1.0`) no longer degrade to integer division on the C backend (this was a real constant-folding bug: `1.0 / 60.0` emitted `1 / 60`), and struct fields holding arrays of structs (`bodies: [Body]`) now work on the C backend (forward declarations + dependency-ordered struct emission).
- The selftest suite now covers all of the above (16 cases, each verified on C, JS, and Python — the five animated examples produce identical output on all backends, ANSI frames included).

## Vulcan 0.5.0 — what's new

- **Package once, run anywhere**: `vulcan package app.vulcan --platform <web|desktop|mobile|linux|windows|macos>` emits a ready-to-ship bundle for that target (see Packaging below).
- **File I/O and CLI args**: `file_read`, `file_write`, `file_exists`, `arg_count()`, `arg(i)` — identical on C, JS, and Python backends (args after the entry file are forwarded to your program).
- **Full string stdlib**: `split`, `starts_with`, `ends_with`, `substring(start, end)`, `find`, plus `to_int(s)` / `to_float(s)` parsing.
- **Full array stdlib**: `remove(index)`, `insert(index, item)` join `push`, `pop`, `shift`, `last`, `contains`.
- **Not just games**: `examples/todo.vulcan` is a complete CLI todo app — file persistence, subcommands, struct lists, impl methods — running identically on all three backends.
- The selftest suite now covers all of the above (14 cases, each verified on C, JS, and Python).

## Vulcan 0.4.0 — what's new

- **Multiple backends** — the same Vulcan program now compiles to three targets:
  - `--target c` (default): native code via C, `-O3` + full hardening
  - `--target js`: plain JavaScript for Node.js — zero-install distribution, same output
  - `--target py`: Python 3 source — structs become classes, `impl` methods become real class methods
  - Cross-backend output is byte-for-byte identical for the benchmark suite (fib, collatz, primes, physics — including `%.6g` float formatting), and `vulcan selftest` now verifies every test case on all three backends automatically.
- **`impl` blocks** — attach methods to your structs, Rust-style:

  ```vulcan
  struct Point { x: f64, y: f64 }

  impl Point {
      fn translate(self, dx: f64, dy: f64) -> Point {
          self.x = self.x + dx
          self.y = self.y + dy
          return self
      }
  }

  fn main() {
      let p = Point { x: 1.0, y: 2.0 }
      p.translate(3.0, 4.0)      // methods run on every backend
      print(p.x, p.y)            // 4 6
  }
  ```

  Methods take `self` (mutated in place on the C backend via a pointer), can call each other in any order, and user-defined methods take priority over builtins of the same name.
- **Backend semantics note** — on the C backend struct assignment copies (value semantics); on the JS/Python backends structs are objects (reference semantics). Everything else — integer division, `%.6g` float printing, bounds-checked arrays, `defer`, f-strings, `match` — behaves identically.

## Vulcan 0.3.0 — what's new

- **`--safe` build mode** — `vulcan run --safe` (or build/bench/test) compiles with AddressSanitizer + UndefinedBehaviorSanitizer: out-of-bounds heap access, use-after-free, and signed integer overflow abort the program at the exact line instead of corrupting memory.
- **Game/terminal stdlib** — everything you need to build any kind of game:
  - timing: `ms_now()` (monotonic ms clock), `sleep_ms(n)` (frame caps)
  - input: `key_ready()`, `read_key()`, `term_raw()` / `term_normal()` (with `defer` cleanup)
  - output: `print_at(x, y, s)`, `clear_screen()`, `cursor_visible(0)`, `set_color(fg, bg)` (8 ANSI colors, `-1` skips)
  - screen: `screen_width()`, `screen_height()` (live terminal size)
  - randomness: `rand_int(lo, hi)`
- **Language**: bare `return` (no semicolon, no value), string indexing (`s[i]` yields the char code), trailing commas in array literals, `.repeat(n)`, array printing, typed empty arrays

## Vulcan 0.2.0 — what's new

The 0.2.0 release roughly doubles the language:

**Language features**
- `elif` chains, `match` / `when` arms with `else` fallback
- `defer { ... }` — scope-exit cleanup, LIFO order, runs before returns
- `enum`s (`Color.Red`) and named `const`s with inferred types
- f-strings: `print(f"n = {n}, double is {n * 2}")`
- `for x in xs { }`, stepped ranges `for i in 0 ..= 10 by 2 { }`
- compound assignment (`+=`, `-=`, `*=`, `/=`, `%=`), `++` / `--`
- bitwise ops (`&`, `|`, `^`, `~`, `<<`, `>>`), hex/binary/char literals (`0xFF`, `0b1010`, `'A'`)
- `import` + multi-file modules, `assert` with messages, casts (`x as i64`)
- new builtins: `min`, `max`, `abs`, `sqrt`, `floor`, `ceil`, `pow`, `input`, `sleep_ms`
- string methods (`.upper()`, `.lower()`, `.trim()`, `.contains()`, `.repeat()`, `.len()`)
- array printing (`print(xs)` → `[10, 20, 30]`), typed empty arrays (`let xs: [i64] = []`)
- terminal I/O for games and TUIs: `term_raw()`, `term_normal()`, `key_ready()`, `read_key()`, `clear_screen()`

**Tooling**
- a package manager: `vulcan init`, `vulcan add`, `vulcan.lock` (see below)
- `vulcan test` — runs every `test_*` function in your project
- a clang-first backend (set `VULCAN_CC` to override)

Run the tour: `vulcan run examples/tour.vulcan`.

## Packaging

```bash
vulcan init hello          # scaffold a project (vulcan.toml, src/main.vulcan)
cd hello
vulcan add git@github.com:user/mathlib.git   # or a local path: vulcan add ../mathlib
vulcan run                  # builds to build/hello, runs it
vulcan test                 # runs every test_* function
```

`vulcan add` vendors dependencies into `deps/` and records them in `vulcan.lock`. Dependencies are resolved from `deps/<name>/src/lib.vulcan`, `deps/<name>/lib.vulcan`, or `deps/<name>/src/main.vulcan`. Imports are also resolved relative to the importing file, plus `src/` and `lib/` directories — so plain `import math` works with a `lib/math.vulcan` next to your code.

### Shipping to devices

`vulcan package` turns one `.vulcan` file into a bundle for a target platform — no project setup needed:

```bash
vulcan package examples/todo.vulcan --platform web       # single index.html: ANSI terminal emulator + web-worker runtime
vulcan package examples/todo.vulcan --platform desktop   # Electron scaffold (npm install; packages .dmg/.exe/AppImage)
vulcan package examples/todo.vulcan --platform mobile    # Capacitor scaffold (npx cap add ios / android)
vulcan package examples/todo.vulcan --platform linux     # embedded C source + ./build.sh (cc -O3 + hardening)
vulcan package examples/todo.vulcan --platform windows   # mingw-w64 cross-compile build.sh
vulcan package examples/todo.vulcan --platform macos      # universal (arm64 + x86_64) build.sh
```

The **web** bundle is a single self-contained `index.html`: your program compiles to JS, runs in a Web Worker, and renders through an 80x24 ANSI terminal emulator with full color support — keyboard input, `sleep_ms` pacing, and in-memory `file_read`/`file_write` all work in the browser. The **desktop** and **mobile** bundles are standard Electron/Capacitor scaffolds around the same runtime, and the **native-OS** bundles ship the generated C with a one-command build script.

## Security posture (an honest comparison)

Is Vulcan "securer than Rust"? No. Rust's borrow checker proves memory safety at compile time; Vulcan compiles through C and inherits C's model, so use-after-free, leaks, and dangling pointers are possible in ways Rust rules out by construction. If your threat model is "untrusted input, hostile environment", use Rust.

What Vulcan does give you:

- **`--safe` sanitizer builds** — `vulcan run --safe app.vulcan` compiles with AddressSanitizer + UndefinedBehaviorSanitizer: out-of-bounds heap access, use-after-free, and integer overflow abort at the exact line with a stack trace. Develop with `--safe`, ship hardened.
- **Bounds-checked arrays** — every element access goes through a checked `get`/`set`, so indexing bugs abort instead of corrupting memory.
- **Hardened binaries by default** — every vulcan build compiles with `-fstack-protector-strong` (stack smashing detection), `-D_FORTIFY_SOURCE=2` (fortified libc calls), full PIE/ASLR (`-fPIE -pie`), full RELRO (`-z relro -z now`), and `-Werror=format-security` (format string injection rejected at compile time).
- **No hidden runtime** — the generated C is readable (`vulcan emit`), so the attack surface is auditable.

The hardening is free: with all flags on, vulcan still beats `rustc -O` on all four benchmarks above.

## Benchmarks vs Rust

Same algorithms, `vulcan build` (clang backend) vs `rustc -O`, best of 5 runs (Linux, this repo's `bench/compare.sh`):

| benchmark | vulcan (s) | rustc -O (s) | winner |
|---|---|---|---|
| fib(35) | 0.0292 | 0.0296 | vulcan |
| collatz | 0.1390 | 0.1391 | vulcan |
| primes | 0.1244 | 0.1244 | vulcan |
| physics | 0.0048 | 0.0049 | vulcan |

Vulcan wins or ties all four. Compile times are in the same ballpark (~90ms end-to-end for vulcan incl. C compile; ~70ms for rustc -O with the clang backend — with the gcc backend vulcan compiles faster than rustc on most benches).

Reproduce with:

```bash
./bench/compare.sh
```

## Usage

```bash
cargo build --release
vulcan ide                            # launch the browser IDE (physics + canvas)
vulcan run examples/fib.vulcan        # compile + execute (native, via C)
vulcan build examples/fib.vulcan     # emit C + native binary
vulcan emit examples/fib.vulcan      # show generated C
vulcan bench examples/fib.vulcan      # time repeated runs
vulcan run --safe examples/fib.vulcan # sanitizer build (ASan + UBSan)
vulcan run --target js examples/tour.vulcan   # run on Node.js — same output
vulcan run --target py examples/tour.vulcan   # run on Python 3 — same output
vulcan selftest                       # run the test suite (all 3 backends)
vulcan init myproj                    # scaffold a new project
vulcan package examples/todo.vulcan --platform web   # one-file web app (also: desktop, mobile, linux, windows, macos)
vulcan run examples/todo.vulcan add "buy milk"       # args after the file are forwarded to your program
```

Requires `clang` or `gcc` on your PATH for the default C backend (clang is preferred for faster loops). The `js` and `py` targets need `node` / `python3` installed.

## Making games

The full game stack is built in — timing (`ms_now`, `sleep_ms`), non-blocking keyboard input (`key_ready`, `read_key`, `term_raw`), drawing (`print_at`, `clear_screen`, `set_color`, `cursor_visible`), screen size (`screen_width`, `screen_height`), and randomness (`rand_int`). The canonical game loop:

```vulcan
term_raw()
defer { term_normal() }        // terminal state always restored
cursor_visible(0)
// per frame: drain keys, update state, draw at print_at(x, y, ...), sleep_ms(70)
```

The IDE canvas adds a 256x144 framebuffer with live mouse/keyboard input, and `import render3d` gives you a complete software 3D renderer (see Graphics + physics reference below). Write your own games in pure Vulcan — that's what the stack is for.

## How it works

1. Lexer → parser → AST
2. One of three code generators emits code for the target:
   - **C backend** (default): readable C (see `vulcan emit`), compiled by `clang -O3` to a native binary. No runtime, no GC — arrays and structs are plain C vectors and values, with bounds-checked access.
   - **JS backend**: plain Node.js-compatible JavaScript — arrays are JS arrays, f-strings become template literals, `defer` becomes `try/finally`, integer division truncates like C.
   - **Python backend**: Python 3 source — structs become classes, `impl` methods become real class methods, `defer` becomes `try/finally`.
3. `vulcan selftest` compiles and runs every test case on all three backends and diffs the output — byte-for-byte identical or the build fails.

## Graphics + physics reference

The canvas is a fixed 256x144 framebuffer (colors are `0xRRGGBB` integers). Drawing is identical on every backend — `present()` renders it to the IDE canvas, or to your terminal as truecolor ANSI half-blocks.

| builtin | description |
|---|---|
| `canvas_width()`, `canvas_height()` | 256 and 144 |
| `clear(color)` | fill the canvas |
| `draw_rect(x, y, w, h, color)` | axis-aligned rectangle |
| `draw_circle(x, y, r, color)` | filled circle |
| `draw_line(x1, y1, x2, y2, color)` | Bresenham line |
| `draw_text(x, y, s, color)` | 3x5 pixel font (A-Z, 0-9, punctuation) |
| `present()` | show the frame (IDE canvas / terminal) |
| `mouse_x()`, `mouse_y()`, `mouse_down()` | live mouse (IDE only; -1 / false elsewhere) |
| `key_down(code)` | key held (IDE only; ASCII codes + arrows 37-40) |

The physics engine (`import physics`) follows the copy-in/copy-out discipline that makes one source work on value-semantics (C) and reference-semantics (JS/Python) backends: always re-assign `w = world_step(w, dt)`.

| function | description |
|---|---|
| `world_new(gx, gy)` | create a world with gravity |
| `world_add_circle(w, x, y, r, mass, restitution, friction)` | dynamic circle |
| `world_add_box(w, x, y, hw, hh, mass, restitution, friction)` | dynamic box |
| `world_add_static_box(w, x, y, hw, hh, restitution, friction)` | immovable box |
| `world_add_bounds(w, restitution, friction)` | walls + floor around the canvas |
| `world_add_spring(w, a, b, rest, k, damping)` | damped spring between bodies |
| `world_step(w, dt)` | advance the simulation |
| `world_body_count(w)`, `body_x(w, i)`, `body_y(w, i)`, ... | read state |
| `body_set_pos(w, i, x, y)`, `body_apply_impulse(w, i, ix, iy)` | mutate a body |

## Examples

- `examples/fib.vulcan` — classic recursion
- `examples/tour.vulcan` — a tour of every feature (runs identically on all backends)
- `examples/impl_demo.vulcan` — struct methods (`impl`) across all three backends
- `examples/todo.vulcan` — a CLI todo app: subcommands, file persistence, struct lists with impl methods
- `examples/ffi_demo.vulcan` — calling C libraries (`extern "c"` + `link`)
- `lib/engine.vulcan` — the Vulcan Engine (entities, terrain, player controller, HUD)
- `lib/render3d.vulcan`, `lib/physics.vulcan` — 3D math and 2D rigid-body physics
- `TODO.md` — the roadmap (compiler, engine, tooling, game)
- `bench/` — vulcan-vs-rust benchmarks
