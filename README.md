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

## Vulcan 0.6.1 — Android, custom touch controls, faster

- **Android APK**: `vulcan package game.vulcan --platform android [--assets dir] [--name Name]` writes a Gradle-free Android project (WebView shell, landscape, immersive, file picker for imports, Back button hook) and, if an Android SDK (`ANDROID_HOME`) and JDK are found, builds a debug-signed `.apk` with aapt2, javac, d8, zipalign and apksigner in about two seconds. Without the SDK it leaves `build.sh` for you to run. `--platform mobile` and `desktop` now wrap 3D programs correctly too (they used to wrap the terminal runtime, which is why a 3D game looked broken on a phone). iOS still needs Xcode on a Mac.
- **Editable touch controls**: Options > Customize touch controls lets you drag every button, resize all or one, change opacity and look speed, and toggle left-handed layout, floating stick, auto-sprint, labels and stick hint. Settings persist per device. Touch is detected on the first touch, not only at load.
- **Built-in help and Options menu**: first-run How to play (touch, keyboard, mods, 1v1) and a single Options dropdown (help, customize, quality, import, clear, fullscreen).
- **Quality and speed**: Auto quality adapts render resolution to hold about 60 fps (Low, Medium and High are fixed presets with smaller shadow maps). F3 shows JS time, GL submit time and render resolution. Packaging itself takes milliseconds.

## Vulcan 0.6.0 — what's new

- **A 3D web platform**: `vulcan package game.vulcan --platform web3d [--assets dir]` emits a single `index.html` with a WebGL2 host runtime (shadow pass, sky pass, opaque and transparent layers, user-defined post-processing chain), keyboard/mouse/pointer-lock input, a 2D HUD layer, sound, a persistent virtual filesystem and WebRTC/BroadcastChannel networking. The program itself is compiled to JavaScript. `--assets` embeds a folder of files that the program reads with `file_read` / `file_list`.
- **Host builtins** (web3d only, JS backend only; the C and Python backends now report a clear error): `gfx_open`, `gfx_frame` (suspends until the next animation frame and returns the frame time), `gfx_shader`, `gfx_mesh_set` / `gfx_mesh_free` / `gfx_mesh_at` / `gfx_mesh_yaw`, `gfx_camera`, `gfx_uniform`, `gfx_texture`, `gfx_project`, `gfx_key` / `gfx_key_pressed`, `gfx_mouse_*`, `gfx_click`, `gfx_wheel`, `gfx_set_lock`, `gfx_chrome`, `gfx_touch`, `hud_rect` / `hud_text` / `hud_text_w`, `net_start` / `net_send` / `net_recv` / `net_state` / `net_ui` / `net_stop`, `snd`, `file_list`, `host_store`.
- **Async frame functions**: any function that (transitively) calls `gfx_frame()` becomes `async` in the generated JS, and its callers `await` it automatically. Write the game loop as plain `loop { dt = gfx_frame() ... }`.
- **Compiler fixes**: int32 fast paths for bit operators, `hash3`, extra math builtins (`atan2`, `tan`, `exp`, `log`, `asin`, `acos`, `atan`, `round`, `sign`, `trunc`), and empty `[]` literals now take their type from the surrounding struct field, argument or assignment.
- **Touch support**: the web3d host auto-detects touch devices, scales the HUD to small screens, and provides virtual stick, look, and button controls mapped onto the same `gfx_key` / `gfx_click` / `gfx_mouse_*` builtins. `gfx_touch()` lets a program adapt its defaults.
- **Vulcancraft**: a full 1v1 voxel game written in Vulcan lives in `games/vulcancraft/` with mod packs and shader packs (see its README).

```bash
vulcan package games/vulcancraft/src/main.vulcan --platform web3d --assets games/vulcancraft/assets
# open package-main-web3d/index.html in any WebGL2 browser
```

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
- **Three example games** in pure Vulcan, covering three genres:
  - `examples/snake.vulcan` — arcade (grid, growth, collision)
  - `examples/pong.vulcan` — real-time physics (ball angles, AI opponent, frame loop)
  - `examples/tictactoe.vulcan` — turn-based strategy (full minimax AI — it cannot be beaten)
- **A first-person 3D engine** — `examples/raycaster.vulcan` is a Wolfenstein-style raycaster written in pure Vulcan: one ray per screen column, distance-falloff wall heights, fisheye correction, 256-color depth shading, collision detection, live minimap, ~30fps. This is the same core technique that powered Wolfenstein 3D.
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

Run the tour: `vulcan run examples/tour.vulcan`. Play the game: `vulcan run examples/snake.vulcan`.

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
vulcan package examples/pong.vulcan --platform web       # single index.html: ANSI terminal emulator + web-worker runtime
vulcan package game.vulcan --platform web3d --assets assets   # single index.html: WebGL2 host, shaders, input, HUD, networking
vulcan package examples/pong.vulcan --platform desktop   # Electron scaffold (npm install; packages .dmg/.exe/AppImage)
vulcan package examples/pong.vulcan --platform mobile    # Capacitor scaffold (npx cap add ios / android)
vulcan package examples/pong.vulcan --platform linux     # embedded C source + ./build.sh (cc -O3 + hardening)
vulcan package examples/pong.vulcan --platform windows   # mingw-w64 cross-compile build.sh
vulcan package examples/pong.vulcan --platform macos      # universal (arm64 + x86_64) build.sh
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
vulcan run examples/fib.vulcan        # compile + execute (native, via C)
vulcan build examples/fib.vulcan     # emit C + native binary
vulcan emit examples/fib.vulcan      # show generated C
vulcan bench examples/fib.vulcan      # time repeated runs
vulcan run --safe examples/fib.vulcan # sanitizer build (ASan + UBSan)
vulcan run --target js examples/tour.vulcan   # run on Node.js — same output
vulcan run --target py examples/tour.vulcan   # run on Python 3 — same output
vulcan selftest                       # run the test suite (all 3 backends)
vulcan init myproj                    # scaffold a new project
vulcan package examples/pong.vulcan --platform web   # one-file web app (also: desktop, mobile, linux, windows, macos)
vulcan run examples/todo.vulcan add "buy milk"       # args after the file are forwarded to your program
```

Requires `clang` or `gcc` on your PATH for the default C backend (clang is preferred for faster loops). The `js` and `py` targets need `node` / `python3` installed.

## Making games

The full game stack is built in — timing (`ms_now`, `sleep_ms`), non-blocking keyboard input (`key_ready`, `read_key`, `term_raw`), drawing (`print_at`, `clear_screen`, `set_color`, `cursor_visible`), screen size (`screen_width`, `screen_height`), and randomness (`rand_int`). See `examples/pong.vulcan` for the canonical game loop:

```vulcan
term_raw()
defer { term_normal() }        // terminal state always restored
cursor_visible(0)
// per frame: drain keys, update state, draw at print_at(x, y, ...), sleep_ms(70)
```

Three complete games ship in `examples/`: Snake (arcade), Pong (real-time physics + AI), and Tic-Tac-Toe (unbeatable minimax AI).

## How it works

1. Lexer → parser → AST
2. One of three code generators emits code for the target:
   - **C backend** (default): readable C (see `vulcan emit`), compiled by `clang -O3` to a native binary. No runtime, no GC — arrays and structs are plain C vectors and values, with bounds-checked access.
   - **JS backend**: plain Node.js-compatible JavaScript — arrays are JS arrays, f-strings become template literals, `defer` becomes `try/finally`, integer division truncates like C.
   - **Python backend**: Python 3 source — structs become classes, `impl` methods become real class methods, `defer` becomes `try/finally`.
3. `vulcan selftest` compiles and runs every test case on all three backends and diffs the output — byte-for-byte identical or the build fails.

## Examples

- `examples/fib.vulcan` — classic recursion
- `examples/tour.vulcan` — a tour of every feature (runs identically on all backends)
- `examples/impl_demo.vulcan` — struct methods (`impl`) across all three backends
- `examples/todo.vulcan` — a CLI todo app (not a game): subcommands, file persistence, struct lists with impl methods
- `examples/snake.vulcan` — Snake, written in pure Vulcan (WASD keys; uses `term_raw` + `defer` for terminal cleanup)
- `bench/` — vulcan-vs-rust benchmarks
