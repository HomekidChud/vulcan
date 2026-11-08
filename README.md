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

## What's new

The current release is **1.11** (see `vulcan --version`): **Vulcan compiles
itself**. The compiler now lives in `self/vulc.vulcan` (~3,100 lines of
Vulcan), and `vulcan bootstrap` proves the language builds itself with a
byte-identical fixpoint check — the same stage1 → stage2 → stage3 scheme
rustc uses. The full per-version history lives in
[CHANGELOG.md](CHANGELOG.md).

## Self-hosting

The compiler is written in the language it compiles. `self/vulc.vulcan` is a
complete lexer, parser, type checker and C code generator for Vulcan Core
(structs, impl methods, arrays, strings, control flow, the builtin library),
emitting the same C dialect the Rust compiler emits.

```bash
vulcan bootstrap          # stage1 (Rust-built) -> stage2 -> stage3, byte-identical fixpoint
vulcan self foo.vulcan    # compile + run a program with the self-hosted compiler
./scripts/self-test.sh    # every self/test/*.vulcan must behave identically on both compilers
```

The Rust compiler remains in the tree as the bootstrap "stage 0" (and as the
fast path for everyday development): it compiles `vulc.vulcan` once, and
after that the language is self-sufficient — no cargo needed to build the
compiler from its own source. Details in [docs/selfhost.md](docs/selfhost.md).

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

The IDE canvas is a live framebuffer with mouse/keyboard input, and `import render3d` gives you a complete software 3D renderer (see Graphics + physics reference below). Write your own games in pure Vulcan — that's what the stack is for.

## How it works

1. Lexer → parser → AST
2. One of three code generators emits code for the target:
   - **C backend** (default): readable C (see `vulcan emit`), compiled by `clang -O3` to a native binary. No runtime, no GC — arrays and structs are plain C vectors and values, with bounds-checked access.
   - **JS backend**: plain Node.js-compatible JavaScript — arrays are JS arrays, f-strings become template literals, `defer` becomes `try/finally`, integer division truncates like C.
   - **Python backend**: Python 3 source — structs become classes, `impl` methods become real class methods, `defer` becomes `try/finally`.
3. `vulcan selftest` compiles and runs every test case on all three backends and diffs the output — byte-for-byte identical or the build fails.

## Graphics + physics reference

The canvas defaults to 256x144 but is fully dynamic: call `canvas_resize(w, h)` to make it any size up to 4096x4096 — portrait, ultrawide, square, whatever the game wants (288x512 for a Flappy-Bird-style layout, 384x216 for 16:9 HD, ...). Colors are `0xRRGGBB` integers. Drawing is identical on every backend — `present()` renders it to the IDE canvas, or to your terminal as truecolor ANSI half-blocks.

| builtin | description |
|---|---|
| `canvas_width()`, `canvas_height()` | current logical size (default 256x144) |
| `canvas_resize(w, h)` | set any canvas size, 1x1 to 4096x4096 (clamped); reallocates the framebuffer on all backends, IDE, web and mobile bundles |
| `clear(color)` | fill the canvas |
| `draw_rect(x, y, w, h, color)` | axis-aligned rectangle |
| `draw_circle(x, y, r, color)` | filled circle |
| `draw_line(x1, y1, x2, y2, color)` | Bresenham line |
| `draw_text(x, y, s, color)` | 3x5 pixel font (A-Z, 0-9, punctuation) |
| `present()` | show the frame (IDE canvas / terminal) |
| `mouse_x()`, `mouse_y()`, `mouse_down()` | live mouse (IDE only; -1 / false elsewhere) |
| `key_down(code)` | key held (IDE only; ASCII codes + arrows 37-40) |
| `is_touch()` | true when running on a touch device (web bundle) |
| `touch_count()` | number of fingers currently on the screen |
| `touch_x(i)`, `touch_y(i)` | finger `i` position in logical canvas coords (0-255, 0-143) |
| `touch_custom()` | hide the default touch UI — the game draws its own controls |

**Touch input (web/mobile).** Every touch point is forwarded to the game in
logical canvas coordinates, so a game can lay out any control scheme it
wants (joysticks, wheels, gesture pads) and hit-test with `touch_x` /
`touch_y`. Calling `touch_custom()` hides the default joystick + FIRE/JUMP
overlay and gives the game the whole screen — `examples/liberty.vulcan`
is the reference implementation. Right-side drag still steers the camera.

**3D meshes and character rigs** (`import engine`): `mesh_box(hx, hy, hz,
color)` builds a box with independent half-extents; `mesh_draw_rot(m, cam,
pos, yaw, pitch)` rotates around Y then the yawed X axis (swinging limbs,
tilted props). On top of those sits the humanoid rig: `rig_new(shirt,
pants, skin, hair)` and `rig_draw(rig, cam, pos, yaw, phase, moving)` draw
a ~1.8-unit character with a walk cycle (advance `phase` ~10 rad/s while
walking) — legs and arms swing, the torso bobs. `examples/liberty.vulcan`
uses it for the player and every pedestrian.

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
- `examples/obby.vulcan` — VULCAN OBBY, a Roblox-style obstacle course game (checkpoints, kill bricks, jump pad, moving platforms, R6 avatar)
- `examples/flappy.vulcan` — VULCAN FLAPPY, a Flappy-Bird-style game at the original's true 288x512 portrait resolution (showcases `canvas_resize`)
- `examples/frontier.vulcan` — VULCAN FRONTIER, an open-world western adventure (RDR/GTA-flavored): horse riding, three bandit camps with bosses, six-shooter, general store, day/night cycle; playable natively and as a web build
- `examples/todo.vulcan` — a CLI todo app: subcommands, file persistence, struct lists with impl methods
- `examples/ffi_demo.vulcan` — calling C libraries (`extern "c"` + `link`)
- `lib/engine.vulcan` — the Vulcan Engine (entities, terrain, player controller, HUD)
- `lib/render3d.vulcan`, `lib/physics.vulcan` — 3D math and 2D rigid-body physics
- `TODO.md` — the roadmap (compiler, engine, tooling, game)
- `bench/` — vulcan-vs-rust benchmarks
