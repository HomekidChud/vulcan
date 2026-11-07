# Vulcan — TODO / Roadmap

Working notes for future sessions. Everything here is ordered by impact and
effort. Verify with `cargo build --release && VULCAN_CC=gcc ./target/release/vulcan selftest`
after every change — all 24 cases must pass byte-identically on C, JS, and
Python backends before pushing.

## Current state (v1.6)

- Language: structs, enums, impl methods, match, f-strings, defer, FFI
  (`extern "c"` + `link`), nested arrays `[[T]]` (scalar bases in struct
  fields; any base as locals/params/returns), math builtins incl. `atan2`,
  `clamp`, `lerp`, `sign`, method calls on ANY expression receiver
  (`a.add(b).add(b)`, `get_pos(e).add(v)`, `pts[0].add(b)`), and
  forward references between functions (any-order `fn` calls, C backend
  now emits prototypes).
- Backends: C (native, hardened), JS (Node), Python 3 — byte-identical
  output, including method receivers on temporaries (JS/Py clone the
  receiver so mutating methods match C's value semantics).
- Libs: `lib/physics.vulcan` (2D rigid bodies), `lib/render3d.vulcan`
  (Vec3, camera, software 3D), `lib/engine.vulcan` (**Engine v0.3**:
  entities, scene, heightmap terrain + `terrain_sines` procedural
  islands, ray-sphere/ray-AABB + `scene_raycast` line-of-sight,
  `scene_shoot` hitscan, `scene_fire_projectile` with swept-impact
  anti-tunneling test, `bot_think` wander/chase/attack AI with steering,
  player controller, HUD, fixed-timestep loop, meshes with flat shading,
  first-person/orbit/spectator cameras, particles, weapons with
  ammo/reload, save/load level format).
- Tooling: IDE, package manager, `vulcan test`, `vulcan package`
  (web/desktop/mobile/native bundles), selftest (24 cases). `vulcan doc` + GitHub Actions CI.

## Compiler / language (highest impact first)

- [x] ~~Method calls on non-variable receivers~~ — DONE in v1.5 (temp
      materialization on C; `__vclone`/`_vclone` on JS/Py for semantic
      parity). Covered by selftest "method calls on expressions".
- [x] ~~Forward references between functions~~ — DONE in v1.5 (C backend
      emits prototypes before definitions; any-order fn calls work on
      all backends). Covered by selftest case.
- [x] ~~String comparison `==`/`!=`~~ — was already working; stale item.
      (Kept a selftest guard via the string cases.)
- [ ] **Nested arrays of structs as struct fields** — `struct W { grid: [[Cell]] }`
      panics with a clear message today. Needs Vec_Point emitted before
      Vec_Vec_Point in the struct-emission loop (see the panic in
      `generate_inner` field scan). Grids of scalars already work.
- [ ] **Closures / function values** — the biggest language gap. Start with
      non-capturing function pointers (`let f: fn(i64) -> i64 = double`), then
      closures over immutable locals. Needs parser (new Expr arm), all three
      backends, and a selftest case.
- [ ] **Struct copy on assignment for JS/Py** — the deepest semantic gap:
      JS/Py backends share struct objects by reference, so `let muzzle =
      e.pos` then mutating `e` also moves `muzzle` (C copies by value).
      Engine code now defensively copies at choke points (`scene_spawn`,
      `scene_fire_projectile`), but the real fix is deep-copy on struct
      assignment/passing in codegen_js/codegen_py. Big change; must keep
      all 24 selftest cases byte-identical.
- [ ] **Smaller int/float types** — `i32` / `f32` / `u8` for memory-heavy
      games (heightmaps, voxel data). C backend only at first; keep `i64`/`f64`
      the default.
- [ ] **`elif`-style `when` guards in match arms** and destructuring struct
      patterns in match.
- [ ] **Compile-time constant folding** for `const` arithmetic (e.g. grid
      sizes), so `const N: i64 = 16` can size arrays without runtime cost.
- [ ] **Better errors** — replace `panic!` with line-numbered diagnostics
      collected per file instead of aborting the compiler.
- [ ] **Tail calls / deep recursion safety** — C backend overflows the stack
      on deep recursion; consider a stack-depth check or trampoline for
      self-tail-calls.
- [ ] **String builder** — repeated `+` on strings is O(n²); a `StringBuilder`
      type (VecStr-backed) with `push`/`to_string`.
- [ ] **Bitwise ops on enums** — treat enums as flags (`|`/`&`/`^` already
      emit on i64 via `__bor` etc. on JS; make the type checker accept
      enum-typed operands).

## Performance path to "beat C++" (realistic framing)

Vulcan compiles through C, so "beating C++" really means: C-level speed
with a safer, more productive language on top. Items that actually move
the needle:

- [ ] **Optimization passes** — the C backend emits `-O3` builds; add
      dead-store elimination and copy-prop for the `s = scene_step(s, dt)`
      write-back pattern (currently copies whole structs every tick).
- [ ] **Struct-of-arrays option** — `scene.entities` is array-of-structs;
      an SoA variant (`pos_x: [f64]`, ...) would halve cache misses on
      big scenes. Offer as an alternative Scene layout.
- [ ] **Benchmark suite** — `vulcan bench` exists; add standard benchmarks
      (fib, nested loops, entity tick with 1000 entities) to track
      regressions, C vs JS vs Py.
- [ ] **Arena/scratch allocator via FFI** — for entity churn (spawn/reap
      per frame) a bump allocator in C beats Vec push/pop.
- [ ] **SIMD-friendly math** — Vec3 ops as separate x/y/z arrays opens
      auto-vectorization; keep the API, change the layout internally.

## Vulcan Engine (lib/engine.vulcan)

Grow the engine system by system. It is NOT a port of Unreal Engine (Epic's
proprietary product) — it is vulcan's own engine, built in pure vulcan.

- [x] ~~Ray-entity intersection~~ — DONE in v0.2 (`ray_sphere`, `ray_aabb`,
      `scene_raycast` line-of-sight, `scene_shoot` hitscan).
- [x] ~~Projectile entities~~ — DONE in v0.2 (`scene_fire_projectile`,
      swept-segment impact test so fast projectiles can't tunnel; fixed
      double-integration bug where scene_step moved projectiles twice).
- [x] ~~Bot AI states~~ — DONE in v0.2 (`bot_think`: idle/wander/chase/
      attack with `atan2`-style steering, aggro/attack ranges, fire
      cooldown, fires projectiles).
- [x] ~~Terrain generation~~ — DONE in v0.2 (`terrain_sines` sum-of-sines
      island heightmap with edge falloff).
- [x] ~~Mesh component~~ — DONE in v0.3 (`Mesh` struct, `mesh_vert` /
      `mesh_face` builders, `mesh_cube` primitive, `mesh_draw` with yaw
      rotation + flat shading via face normals, backface-agnostic).
- [ ] **Entity-terrain collision beyond rest height** — currently entities
      just rest on the heightmap; add slope sliding and wall push-out
      (sample terrain gradient via finite differences).
- [ ] **Nav-mesh / pathfinding** — A* over a coarse grid derived from
      terrain slope; bots path around water/cliffs instead of walking
      through them.
- [ ] **Audio** — FFI to miniaudio or SDL_mixer via `extern "c"` + `link`
      (native backend only; no-op stubs on JS/Py).
- [x] ~~Level format~~ — DONE in v0.3 (the `scene_save` text format IS the
      level format: terrain grid + entities; `scene_load` rebuilds a Scene).
- [ ] **Raylib/SDL3 binding layer** — `lib/raylib.vulcan` with `extern "c"`
      declarations for windowed GPU rendering, textures, and input at native
      speed. This is the real path to Fortnite-scale visuals.
- [x] ~~Camera modes~~ — DONE in v0.3 (`cam_first_person`, `cam_orbit`,
      `cam_spectator`; chase remains the default scene camera).
- [x] ~~Weapon system~~ — DONE in v0.3 (`Weapon` struct: fire-rate gating,
      ammo magazine, reload timing; `weapon_tick` / `weapon_can_fire` /
      `weapon_fire` / `weapon_reload`).
- [x] ~~Particles~~ — DONE in v0.3 (`Particle` pool on Scene capped at
      `MAX_PARTICLES`, gravity + lifetime, `draw_particles` with fading).
- [x] ~~Save states~~ — DONE in v0.3 (`scene_save` / `scene_load` text
      format; %.6g floats round-trip identically on all backends).
- [ ] **Multiplayer-ready determinism** — fixed timestep already helps;
      document + test that identical input sequences give identical
      scenes across backends (extend selftest with a 1000-tick
      determinism check).

## Tooling / ecosystem

- [x] ~~`vulcan doc`~~ — DONE in v1.6 (`vulcan doc <file|dir>` prints
      structs, fns, impl methods, consts; `vulcan doc lib` documents the
      whole standard library).
- [ ] `vulcan fmt` — canonical formatter (canonical style is already used
      throughout lib/ and examples/).
- [ ] LSP server (hover, go-to-definition) — reuse the lexer/parser crates.
- [ ] Package registry: `vulcan add` currently does git/path only; add a
      central index repo.
- [x] ~~CI~~ — DONE in v1.6 (`.github/workflows/ci.yml`: build + selftest
      on every push/PR to master; watch the Actions tab for the first run).
- [ ] **Playground examples for the IDE** — add engine demos (terrain
      flyover, bot arena) to `src/ide.rs` examples list once the engine
      renders something visual.

## The game (private repo: vulcan-royale)

Honest scope: a Fortnite-STYLE battle royale, not a 1:1 Fortnite replica
(that is impossible and would infringe Epic's IP). Build on the engine:

- [ ] Battle-bus drop + glider descent phase
- [ ] Movement (sprint, crouch, jump — engine player controller is the base)
- [ ] Hitscan weapons (2-3 types) + projectile weapons + ammo (engine
      `scene_shoot` / `scene_fire_projectile` are the building blocks)
- [ ] Building (walls/ramps/floors on a grid, using nested arrays)
- [ ] Storm circle: shrinking safe zone with timed phases + damage
- [ ] Bots with the AI states above (`bot_think` v0.2 is the base),
      kill feed, scoreboard
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
- "Beat C++" in benchmarks: not literally — vulcan IS C underneath. The
  honest claim is C-grade performance with a much smaller, safer language.
  The performance items above (SoA, copy-prop, arenas) close the gap that
  vulcan's current codegen adds on top of C.
