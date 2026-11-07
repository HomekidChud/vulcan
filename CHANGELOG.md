# Changelog

Per-version history, newest first. The version number lives in Cargo.toml
(single source of truth, exposed by `vulcan --version`).

## 1.11.1 — VULCAN FRONTIER + codegen fixes

**`examples/frontier.vulcan` — an open-world western adventure game** in the
spirit of RDR/GTA: ride a horse between three bandit camps, clear each boss
for a bounty, spend cash at the general store, watch the full day/night cycle
roll over the valley. Runs natively (`vulcan run examples/frontier.vulcan`)
and as a single-file web build (`vulcan package examples/frontier.vulcan
--platform web`). Rendering is a software 3D pipeline written in the game
itself: near-plane-clipped terrain quads (Sutherland-Hodgman, so the ground
fills the screen with no sky gap), behind-camera cell culling, distance fog
and real frame-time simulation.

- **Fixed (C backend):** `Vec_<T>` typedefs for structs used as array
  elements in function signatures were emitted *after* the function
  prototypes, so any function returning `[Struct]` failed to compile
  (`unknown type name 'Vec_Building'`). Late-discovered typedefs are now
  spliced in ahead of the prototypes block.
- **Fixed (JS backend):** `to_string(bool)` panicked ("needs i64 or f64");
  it now emits `"true"`/`"false"` like the C backend.
- Frontier gameplay: six-shooter with reload, hitscan + tracer, boss/minion
  AI via `bot_think`, medkit drops, mount/dismount horse (E), shop heal (B),
  minimap/compass/health HUD, victory when all three bounties are collected.

## 1.11.0 — self-hosting: the compiler is written in Vulcan

**`self/vulc.vulcan` (~3,100 lines) is a complete lexer, parser, type
checker and C code generator for Vulcan Core — and it compiles itself.**
`vulcan bootstrap` runs the classic stage1 → stage2 → stage3 chain and
verifies the fixpoint (stage2.c and stage3.c byte-identical), the same
scheme rustc uses. The Rust compiler stays in the tree as the bootstrap
seed ("stage 0") and the fast path for everyday development; building the
compiler from its own source no longer requires cargo.

- **`vulcan self <file>`** — compile and run any program with the
  self-hosted compiler.
- **`vulcan bootstrap`** — full chain + fixpoint check + smoke test
  (`self/test/smoke.vulcan` must produce identical output on both
  compilers).
- **`./scripts/self-test.sh`** — every case in `self/test/` is compiled by
  both compilers; outputs must match. Runs in CI on gcc and clang.
- **Arena-AST compiler architecture** with contiguous `blk`/`iargs`/`finits`
  ranges (collect-then-append), copy-in/copy-out `Ctx` convention, and an
  embedded prelude (`scripts/gen-self-prelude.py` regenerates it).
- **`to_string(bool)`** now works on the C backend of the Rust compiler
  too (parity with the self-hosted one).
- **`char_at(s, i)`** builtin added to the C backend.
- Docs: [docs/selfhost.md](docs/selfhost.md).

## Unreleased — language upgrade (Phase 1)

**The compiler core grows a semantic layer, real diagnostics, value
semantics, and the most-requested language features — with all three
backends kept byte-identical.**

- **`vulcan check <file>`** — collect every error in one pass (lexer
  diagnostics, duplicate fn/struct/field/variant/const declarations,
  impl-for-unknown-type, type mismatches) rendered with line, column and
  a caret, instead of aborting on the first panic.
- **Struct value semantics on JS/Python** — struct assignment, argument
  passing and returns now copy (arrays are shared, matching C Vec-header
  copy semantics exactly). The engine's defensive copies were removed.
- **Match guards** — `when pats if cond` on all backends.
- **Compile-time constant folding** for const arithmetic (refuses to fold
  division by zero so the runtime check still fires).
- **String methods** — `s.replace(old, new)` (literal, empty pattern is a
  no-op) and `[string].join(sep)` on all backends.
- **`lib/strbuf.vulcan`** — StrBuf string builder (amortized O(1) push,
  one join at the end).
- **Checked division** — `/`/`%` by zero (int and float, including
  compound forms) aborts with `vulcan: division by zero` on stderr and
  exit 1 on all three backends. Behavior change: float division by zero
  no longer yields inf on C/JS; it is an error (matching Python).
- **Smaller numeric types** — `i8 i16 i32 u8 u16 u32 f32` on all backends:
  locals, params, returns, fields, casts (`x as u8`), arrays; arithmetic
  computes in i64/f64 and wraps at storage; f32 rounds to nearest.
  Semantics recorded in DECISIONS.md.
- **Payload enums** — variants can carry payloads
  (`enum Shape { Circle(f64), Rect(f64, f64), Point }`). Payload enums
  are tagged structs; payload-less enums keep their existing i64-tag
  representation and output. `match` compares the tag and binds
  payloads (`when Shape.Circle(r) { ... }`), including in guards.
- **`Option<T>` / `Result<T, E>`** — `Option.Some/None`,
  `Result.Ok/Err` construction, matching, and value-semantics copying on
  all backends. `Option.None` / `Result.Ok` / `Result.Err` require a
  type hint (let annotation or function return type).
- **`?` propagation** — `expr?` on an Option/Result in a function that
  returns the same family propagates None/Err and yields the payload.
- Not yet supported (gated with clear errors): arrays of payload enums /
  Option / Result, printing payload enums directly, `==` on payload
  enums. Noted in TODO.md.
- **Reference parameters** — `&T` / `&mut T` function parameters with
  `&expr` / `&mut expr` call arguments on all backends. Reads and writes
  through the reference see the caller's value (`p->x` on C, shared
  objects on JS/Py); refs passed to value params are dereferenced and
  copied; `match` on a reference matches the pointee. Gated with clear
  errors: non-struct references, returning references, `let r = &mut x`,
  reassigning a reference.
- Selftest grew 24 → 31 cases; every case runs on C (gcc), JS and Python
  and must be byte-identical.

## 1.10

**VULCAN OBBY: a complete Roblox-style obstacle course, and a real
third-person camera.**

- **`examples/obby.vulcan`** — a full game built with the engine: an
  obstacle course in the classic Roblox style, informed by published
  Roblox references. It has:
  - the classic **R6 "noob" avatar** with the canonical colors —
    Bright yellow head/arms (245,205,48), Bright blue torso
    (13,105,172), Br. yellowish green legs (164,189,71) — with walk
    animation (arm/leg swing) and body-yaw facing
  - **Roblox physics numbers**: WalkSpeed 16, JumpPower 50,
    gravity 196.2 studs/s² — the jump arc covers ~8 studs, exactly
    like the real thing
  - obby conventions: green **checkpoints** every few obstacles,
    red **kill bricks** (touch = OOF + respawn), a sine-tween
    **moving platform**, a vertical **elevator**, a **jump pad**
    that launches you to a high road, a narrow **beam walk**, and a
    gold **finish** platform with a win screen
  - stage/deaths/time HUD, messages (CHECKPOINT / OOF! / BOING!),
    a restart key (R), and mobile touch controls from the standard
    web bundle
- **`cam_third_person` fixed** — it used to build on top of the
  first-person camera helper (with its +1.6 eye offset), which put
  the orbit target off-screen below the bottom of the view. It now
  constructs the camera directly and orbits cleanly.
- **Touch joystick key codes fixed** — the joystick sent uppercase
  codes (87/65/83/68) while the engine expects lowercase (119/97/
  115/100), so mobile WASD never moved. All input paths now agree.
- **Jump buffering** in the obby player controller — a Space tap
  between frames is buffered 0.15s and fires on the next landing,
  so quick taps are never lost.

Play it: `vulcan package examples/obby.vulcan --platform web` and
open the emitted `index.html` (or the `--platform mobile` Gradle /
Xcode projects for phones).

## 1.9

**Input fixed for real, and Vulcan's own packaging — no more Electron or
Capacitor.**

- **The big input fix** — the web runtime's sleep was a busy spin, which
  blocked the worker's event loop, so key/mouse messages from the page were
  never delivered. Every web game's input was silently dead. The runtime now
  yields between frames; held keys, fire, and mouse look all work (verified
  end-to-end in a real browser).
- **Key codes unified** — the engine's `KEY_W` etc. now match what every
  input path actually delivers (lowercase character codes, normalized from
  Shift-held input). Before this, `KEY_W` was 87 but web/IDE/native all
  sent 119, so movement never triggered.
- **Native `key_down()` implemented** — the C backend had a stub that always
  returned false; it now drains pending terminal input (terminal key
  auto-repeat makes held keys work).
- **Touch fixes** — FIRE/JUMP buttons now release (they used to stick held
  forever), a RESTART button appears on touch devices, and the joystick
  spawns wherever your thumb lands.
- **Hold-to-fire** in the arena game (the weapon's own cooldown sets the
  rate), plus arena bounds so you can't walk off the island.
- **`vulcan package` is 100% ours now**:
  - `--platform android` emits a complete Gradle project with our own
    ~40-line WebView activity (no Capacitor, no npm) — `gradle assembleDebug`
    produces the APK on any machine with the Android SDK.
  - `--platform ios` emits a complete Xcode project with our own ~25-line
    WKWebView delegate (no CocoaPods) — open and press Cmd+R.
  - `--platform desktop` emits a ~60-line C webview shell (macOS WKWebView /
    Linux WebKitGTK; Windows uses an Edge app window) instead of Electron.
  - `--platform mobile` emits both the android and ios projects.

## 1.8

**Mobile support and the first real game.** You can now build and play
actual games — on desktop and phones.

- **Held-key fix** — `key_down()` in web builds used to only see fresh
  key presses, which made movement feel dead in the browser. Web
  workers now track held keys via keydown/keyup, so WASD works like a
  native game.
- **Touch controls (every web bundle)** — on touch devices the page
  shows a virtual joystick (left half of the screen), drag-to-look
  (right half), and FIRE / JUMP buttons. The joystick maps to WASD key
  codes, so games need zero extra code for mobile. The web link runs
  directly on phones — no install.
- **`player_move_cam` (Engine v0.5)** — camera-relative WASD + jump for
  first-person games. You aim by setting `s.camera` (e.g.
  `cam_first_person` + mouse look); the engine moves the player.
- **VULCAN ARENA** (`examples/arena.vulcan`) — a complete wave-survival
  FPS built in ~200 lines of pure Vulcan: 5 waves of bots, a hitscan
  rifle with reload and muzzle flash, medkit pickups, minimap,
  compass, HUD, game-over/victory screens. No engine changes were
  needed — proof the library API ships games.
- **Drag-to-look** — mouse look in web builds now works by dragging
  (no pointer lock), which also works on touch.

## 1.7

**Vulcan Engine v0.4** — antialiasing, a living atmosphere, and mouse look.

- **Supersampled antialiasing (all backends)** — every program now renders
  internally at 512x288 and resolves to 256x144 with a 2x2 box filter plus
  a subtle vignette. Edges on lines, circles, triangles, and text are
  smooth; the resolve is byte-identical across the C, JS, and Python
  backends (selftest-guarded).
- **Day/night cycle** — `sky_colors(time)` (dawn/noon/dusk/midnight sky
  gradients) and `sun_light(time)` ambient light; a full cycle is 120
  simulated seconds.
- **Atmosphere** — `fog_mix` distance fog blending toward the sky,
  `sea_color` animated water, `draw_sky_gradient` cheap skybox.
- **Third-person camera + mouse look** — `cam_third_person` boom camera,
  new `mouse_dx()` / `mouse_dy()` builtins; the web bundle uses pointer
  lock so FPS-style mouse look works in the browser.
- **HUD + gameplay** — `hud_compass` heading strip, `scene_spawn_pickup`
  health/ammo pickups (bobbing, on the minimap).
- **Island demo v2** — the example now has a full day/night cycle, fog,
  animated sea, pickups, a compass, and three camera modes (C to cycle).

## 1.6

**Vulcan Engine v0.3** — meshes, cameras, particles, weapons, save files — plus `vulcan doc` and CI.

- **Meshes** — `Mesh` with vertex arrays `[Vec3]` and triangle faces `[[i64]]`:
  ```vulcan
  let cube = mesh_cube(1.0, 0x8040C0)          // 8 verts, 12 tris
  mesh_draw(cube, s.camera, entity_pos, yaw)   // flat-shaded, depth-buffered
  ```
  `mesh_draw` rotates around Y, shades each face by its normal against a fixed
  light, and skips triangles with any vertex behind the camera.
- **Camera modes** — `cam_first_person(eye, yaw, pitch)`,
  `cam_orbit(target, angle, dist, height)`, `cam_spectator(pos, yaw, pitch)`
  alongside the existing third-person chase camera.
- **Particles** — `scene_spawn_particle(s, pos, vel, life, color)` into a capped
  pool (256); particles fly, feel 30% gravity, and `draw_particles(s)` renders
  them as fading 2x2 rectangles.
- **Weapons** — the `Weapon` struct with fire-rate gating, ammo magazine, and
  reload timing:
  ```vulcan
  let w = weapon_new("rifle", 25.0, 0.15, true, 60.0, 5, 2.0)
  if weapon_can_fire(w) { w = weapon_fire(w) }
  w = weapon_tick(w, dt)      // call every tick, same dt as scene_step
  ```
- **Save files / level format** — `scene_save(s, "level.txt")` writes terrain +
  entities to a text file; `scene_load("level.txt")` rebuilds the Scene.
  Floats round-trip identically on all three backends.
- **`vulcan doc <file|dir>`** — print the public API (structs, fns, impl
  methods, consts) of a file or a whole directory: `vulcan doc lib`.
- **CI** — GitHub Actions builds and runs the full selftest on every push.
- Engine selftest now covers all v0.3 systems (24 cases total, byte-identical
  across C/JS/Python).

## 1.5

Method calls anywhere, any-order functions, and the **Vulcan Engine v0.2**.

- **Method calls on any expression** — `a.add(b).add(b)`, `get_pos(e).add(v)`,
  and `pts[0].add(b)` now work on all backends. The C backend materializes the
  receiver into a temp; JS/Python clone the receiver so mutating methods match
  C's value semantics (chained `add`s give identical results everywhere):
  ```vulcan
  let c = a.add(b).add(b)      // chained
  let d = origin().add(a)      // on a call result
  let e = pts[0].add(b)        // on an array element
  ```
- **Forward references between functions** — functions can call functions
  defined later in the file on every backend (the C backend now emits
  prototypes before definitions). `main` can sit at the top of the file.
- **`lib/engine.vulcan` — Vulcan Engine v0.2** — combat and world systems in
  pure vulcan:
  - **Raycasts** — `ray_sphere` / `ray_aabb` (slab method) primitives,
    `scene_raycast` for line-of-sight, `scene_shoot` for hitscan weapons.
  - **Projectiles** — `scene_fire_projectile` with lifetime, straight-line
    travel, and a swept-segment impact test so fast projectiles can't tunnel
    through targets in one step. Projectiles are fully owned by the projectile
    system (no more double integration).
  - **Bot AI** — `bot_think`: idle / wander / chase / attack states per entity
    with aggro + attack ranges, `atan2`-style steering, and return fire with
    a cooldown.
  - **Procedural terrain** — `terrain_sines`: sum-of-sines island heightmap
    with edge falloff (flat at the borders, hills in the middle).
  ```vulcan
  import engine
  fn main() -> void {
      let s = scene_new()
      s = terrain_sines(s, 16, 4.0)
      s = scene_spawn(s, "bot", ENTITY_KIND_BOT, Vec3 { x: 0.0, y: 1.0, z: 8.0 })
      s = bot_think(s, 1.0)          // bot decides to chase/attack
      s = scene_shoot(s, player_id, muzzle, aim_dir, 50.0, 30.0)   // hitscan
      s = scene_fire_projectile(s, player_id, muzzle, aim_dir, 10.0, 25.0, 2.0)
  }
  ```
- **Engine hardening** — `scene_spawn` / `scene_fire_projectile` copy their
  `Vec3` arguments so entities never alias the caller's position objects on
  the reference-semantics JS/Python backends (a real cross-backend bug class,
  now guarded and covered by the selftest).
- Selftest grew to 24 cases (method receivers on expressions, forward
  references, engine v0.2 ray/AI/projectile/terrain coverage — all
  byte-identical across backends).

## 1.4

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

## 1.3

The repo no longer ships pre-bundled demo games — `examples/` is now pure language and tooling
reference code (`tour`, `fib`, `todo`, `impl_demo`, `ffi_demo`), and the IDE ships the same set.
The compiler gained the math builtins that real simulation, steering and aiming code needs:

- **`atan2(y, x)`** — two-argument arctangent, the function bot AI, projectile and camera code
  can't live without. Available as a free function on all three backends (libm `atan2`,
  `Math.atan2`, `math.atan2`), with identical `%.6g` output everywhere.
- **`tan(x)`, `exp(x)`, `log(x)`** — join `sqrt` / `sin` / `cos` / `pow` as free functions on
  every backend.
- New selftest case covering all four — 19 cases, each verified byte-for-byte on C, JS, and Python.

## 1.2

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

## 1.1

Vulcan now has a **software 3D renderer**:

- **`draw_tri` — depth-buffered triangles** — the canvas gained a real 3D pipeline: barycentric rasterization with a per-pixel z-buffer, pixel-center sampling, and winding-insensitive coverage. Draw far triangles first or last — the depth buffer makes the result identical. Works on all three backends, byte-for-byte.
- **`import render3d`** — `lib/render3d.vulcan` gives you `Vec3` math (add/sub/scaled/dot/cross/length/normalized via impl methods), a `Camera` (position + yaw/pitch), `project()` (world point → screen x/y + camera depth + visibility flag), `draw_quad()`, and color helpers (`color_shade`, `color_lerp`). Together with `draw_tri` that's a complete software renderer: perspective projection, flat shading, fog, and hidden-surface removal — in ~130 lines of pure Vulcan.
- `to_string(n)` now works on the JS and Python backends too (it was C-only).
- The Python backend no longer breaks when builtins like `sqrt()` are called inside `impl` methods (Python name-mangled the runtime helpers; they're now mangle-proof).
- The selftest suite now covers all of the above (17 cases, each verified on C, JS, and Python — the animated examples produce identical output on all backends, ANSI frames included).

  ```bash
  vulcan ide                # http://127.0.0.1:7788 — open examples/tour.vulcan, Run
  ```

## 1.0

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

## 0.5.0

- **Package once, run anywhere**: `vulcan package app.vulcan --platform <web|desktop|mobile|linux|windows|macos>` emits a ready-to-ship bundle for that target (see Packaging below).
- **File I/O and CLI args**: `file_read`, `file_write`, `file_exists`, `arg_count()`, `arg(i)` — identical on C, JS, and Python backends (args after the entry file are forwarded to your program).
- **Full string stdlib**: `split`, `starts_with`, `ends_with`, `substring(start, end)`, `find`, plus `to_int(s)` / `to_float(s)` parsing.
- **Full array stdlib**: `remove(index)`, `insert(index, item)` join `push`, `pop`, `shift`, `last`, `contains`.
- **Not just games**: `examples/todo.vulcan` is a complete CLI todo app — file persistence, subcommands, struct lists, impl methods — running identically on all three backends.
- The selftest suite now covers all of the above (14 cases, each verified on C, JS, and Python).

## 0.4.0

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

## 0.3.0

- **`--safe` build mode** — `vulcan run --safe` (or build/bench/test) compiles with AddressSanitizer + UndefinedBehaviorSanitizer: out-of-bounds heap access, use-after-free, and signed integer overflow abort the program at the exact line instead of corrupting memory.
- **Game/terminal stdlib** — everything you need to build any kind of game:
  - timing: `ms_now()` (monotonic ms clock), `sleep_ms(n)` (frame caps)
  - input: `key_ready()`, `read_key()`, `term_raw()` / `term_normal()` (with `defer` cleanup)
  - output: `print_at(x, y, s)`, `clear_screen()`, `cursor_visible(0)`, `set_color(fg, bg)` (8 ANSI colors, `-1` skips)
  - screen: `screen_width()`, `screen_height()` (live terminal size)
  - randomness: `rand_int(lo, hi)`
- **Language**: bare `return` (no semicolon, no value), string indexing (`s[i]` yields the char code), trailing commas in array literals, `.repeat(n)`, array printing, typed empty arrays

## 0.2.0

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


## 1.10.0 (unreleased — Phase 0 of the mega-upgrade)

- Versions reconciled: Cargo.toml is the single source of truth (1.10.0),
  exposed via the new `vulcan --version` / `-V` and the usage banner.
  README per-version history moved here (see sections above).
- Security: `vulcan add` sanitizes dependency folder names and refuses to
  `remove_dir_all` anything outside `deps/` (a source ending in `/..` could
  previously delete project files); `git clone` now receives `--` before the
  URL so a leading-dash URL can't be read as a git option.
- Lexer: hex and binary literals use checked arithmetic — an out-of-range
  literal is an error with the line number instead of a silently wrapped
  value. Decimal literal overflow reports the literal and line instead of a
  raw unwrap panic.
- CLI: missing compilers/interpreters (gcc/clang/node/python3) now print
  clean error messages instead of panicking with a Rust backtrace.
- Tests: new Rust unit tests for the lexer, parser (all examples and stdlib
  files parse) and pkg path safety; CI runs `cargo test` and selftest on
  both gcc and clang.
- Docs: `docs/architecture.md` (pipeline overview) and `docs/baseline.md`
  (Phase 0 baseline: 24/24 selftest, bench numbers, confirmed bugs).
