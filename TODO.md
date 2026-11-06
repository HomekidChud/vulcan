# Vulcan — TODO / Roadmap

Working notes for future sessions. Everything here is ordered by impact and
effort. Verify with `cargo build --release && VULCAN_CC=gcc ./target/release/vulcan selftest`
after every change — all 22 cases must pass on C, JS, and Python backends
before pushing.

## Current state (v1.4)

- Language: structs, enums, impl methods, match, f-strings, defer, FFI
  (`extern "c"` + `link`), nested arrays `[[T]]` (scalar bases in struct
  fields; any base as locals/params/returns), math builtins incl. `atan2`,
  `clamp`, `lerp`, `sign`.
- Backends: C (native, hardened), JS (Node), Python 3 — byte-identical output.
- Libs: `lib/physics.vulcan` (2D rigid bodies), `lib/render3d.vulcan` (Vec3,
  camera, software 3D), `lib/engine.vulcan` (Vulcan Engine v0.1: entities,
  scene, heightmap terrain, player controller, HUD, fixed-timestep loop).
- Tooling: IDE, package manager, `vulcan test`, `vulcan package` (web/desktop/
  mobile/native bundles), selftest (22 cases).

## Compiler / language (highest impact first)

- [ ] **Method calls on non-variable receivers** — `get_pos(e).add(v)` and
      `e.pos.add(v)` currently panic on the C backend ("method calls need a
      simple variable receiver"). Materialize the receiver into a temp like
      the nested-array index fix does. Files: `src/codegen.rs` (~line 1320).
- [ ] **Nested arrays of structs as struct fields** — `struct W { grid: [[Cell]] }`
      panics with a clear message today. Needs Vec_Point emitted before
      Vec_Vec_Point in the struct-emission loop (see the panic in
      `generate_inner` field scan). Grids of scalars already work.
- [ ] **Closures / function values** — the biggest language gap. Start with
      non-capturing function pointers (`let f: fn(i64) -> i64 = double`), then
      closures over immutable locals. Needs parser (new Expr arm), all three
      backends, and a selftest case.
- [ ] **String comparison** `==`/`!=` on strings (currently errors; use
      `.find()` workarounds). C: `strcmp`, JS: `===`, Py: `==`.
- [ ] **Smaller int/float types** — `i32` / `f32` / `u8` for memory-heavy
      games (heightmaps, voxel data). C backend only at first; keep `i64`/`f64`
      the default.
- [ ] **`elif`-style `when` guards in match arms** and destructuring struct
      patterns in match.
- [ ] **Compile-time constant folding** for `const` arithmetic (e.g. grid
      sizes), so `const N: i64 = 16` can size arrays without runtime cost.
- [ ] **Better errors** — replace `panic!` with line-numbered diagnostics
      collected per file instead of aborting the compiler.

## Vulcan Engine (lib/engine.vulcan)

Grow the engine system by system. It is NOT a port of Unreal Engine (Epic's
proprietary product) — it is vulcan's own engine, built in pure vulcan.

- [ ] **Mesh component** — vertex arrays `[Vec3]` + face index arrays
      `[[i64]]` (nested arrays now make this possible), draw via `draw_tri`,
      flat shading with `color_shade`.
- [ ] **Ray-entity intersection** — ray-sphere and ray-AABB helpers for
      hitscan shooting and line-of-sight (see old fortnite.vulcan in git
      history, commit before 1.3, for reference implementations).
- [ ] **Projectile entities** — ENTITY_KIND_PROJECTILE with lifetime,
      travel, and impact damage in scene_step.
- [ ] **Bot AI states** — wander / chase / flee / attack as an enum on
      Entity, driven by distance + line-of-sight. Use `atan2` for steering.
- [ ] **Audio** — FFI to miniaudio or SDL_mixer via `extern "c"` + `link`
      (native backend only; no-op stubs on JS/Py).
- [ ] **Level format** — serialize Scene (terrain grid + entity list) to a
      `.json`-ish text file with file_read/file_write; a loader in vulcan.
- [ ] **Terrain generation** — sum-of-sines / value-noise heightmap builder
      (reference: old fortnite.vulcan terrain fn in git history).
- [ ] **Raylib/SDL3 binding layer** — `lib/raylib.vulcan` with `extern "c"`
      declarations for windowed GPU rendering, textures, and input at native
      speed. This is the real path to Fortnite-scale visuals.
- [ ] **Camera modes** — first-person, orbit, and spectator (currently
      third-person chase only).

## Tooling / ecosystem

- [ ] `vulcan doc` — generate a docs page from `pub fn` signatures + comments.
- [ ] `vulcan fmt` — canonical formatter (canonical style is already used
      throughout lib/ and examples/).
- [ ] LSP server (hover, go-to-definition) — reuse the lexer/parser crates.
- [ ] Package registry: `vulcan add` currently does git/path only; add a
      central index repo.
- [ ] CI: GitHub Actions running build + selftest on every push.

## The game (private repo: vulcan-royale)

Honest scope: a Fortnite-STYLE battle royale, not a 1:1 Fortnite replica
(that is impossible and would infringe Epic's IP). Build on the engine:

- [ ] Battle-bus drop + glider descent phase
- [ ] Movement (sprint, crouch, jump — engine player controller is the base)
- [ ] Hitscan weapons (2-3 types) + projectile weapons + ammo
- [ ] Building (walls/ramps/floors on a grid, using nested arrays)
- [ ] Storm circle: shrinking safe zone with timed phases + damage
- [ ] Bots with the AI states above, kill feed, scoreboard
- [ ] Victory/elimination screens, restart loop
- [ ] Package as web bundle (`vulcan package --platform web`) and share
      the single index.html; an actual Android APK needs the Capacitor
      mobile scaffold + Android SDK (not available in the current sandbox).

## Honest expectations (do not overpromise)

- "Better than Python and its ecosystem" is not a reachable milestone —
  Python has decades of libraries. Vulcan's real edge: native speed via C,
  one language across C/JS/Python backends, tiny readable output, and FFI
  into any C library. Compete on that.
- Unreal Engine is millions of lines of C++ built by hundreds of engineers
  over decades. The realistic goal is a capable vulcan-only engine with
  FFI to raylib/SDL for GPU rendering — not an Unreal clone.
