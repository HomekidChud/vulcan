# Vulcan architecture

One page describing the pipeline from source to running program, plus the
engine layers. Written at the start of the mega-upgrade (Phase 0); update it
when the pipeline changes.

## Compiler pipeline

```
source (.vulcan)
  │
  ▼
lexer.rs        lex(src) -> Vec<(Tok, line)>      hand-written, line-tagged tokens
  │
  ▼
modules.rs      load_program(entry) -> Vec<Item>   expands `import` transitively
  │             (searches ./, src/, lib/, deps/<name>/...) into ONE flat item list
  ▼
parser.rs       parse_program(toks) -> Vec<Item>   items: Fn/Struct/Enum/Const/Impl/Extern/Import
  │             Ty = I64|F64|Bool|Str|Void|Array(Box<Ty>)|Struct(String)
  │             NO type checking here — parse only; ~39 expect() calls abort on
  │             malformed input
  ▼
codegen.rs      generate() -> C source + link libs    each backend re-infers all
codegen_js.rs   generate_js() -> JS (Node)            types independently; they
codegen_py.rs   generate_py() -> Python 3             duplicate field_ty/const_ty
  │                                            (~114/104/104 panic! calls on
  │                                             unexpected AST shapes)
  ▼
main.rs / cli.rs
  C:      write .v.c, invoke $VULCAN_CC (gcc default, clang ok) -O3, run binary
  JS:     write .v.js, invoke node
  Python: write .v.py, invoke python3
```

There is no shared semantic pass today: name resolution and type inference are
duplicated per backend. Making one semantic pass feed all three backends is
Phase 1's headline goal.

## Runtime model

- No external runtime. The C backend emits runtime helpers (print, string ops,
  array ops, graphics builtins like `vdraw_tri`/`vdraw_rect` and the framebuffer)
  inline into every generated file; JS and Python emit equivalent helpers.
- Byte-identical output across the three backends is a hard invariant, enforced
  by `vulcan selftest` (24 cases today).
- Structs are copied by value in C but shared by reference in JS and Python;
  JS/Py clone method receivers defensively to emulate value semantics. Full
  value semantics is a Phase 1 item.
- FFI: `extern "c"` declarations allow i64/f64/bool/string only; `link "name"`
  adds `-l<name>` at link time.

## Standard library and engine (all pure Vulcan, in lib/)

- `lib/math.vulcan` — small numeric helpers.
- `lib/physics.vulcan` — 2D rigid bodies (integrate, resolve collisions).
- `lib/render3d.vulcan` — Vec3, camera, software 3D: depth-buffered
  `draw_tri`, 512x288 internal framebuffer with 2x2 supersampling resolved to
  256x144, flat shading.
- `lib/engine.vulcan` — Engine v0.4: `Entity` is one fat struct with
  `kind: i64`; the whole `Scene` is passed by value each tick
  (`s = scene_step(s, dt)`), so every tick copies the scene. Includes
  heightmap terrain, raycasts, hitscan/projectile weapons, bot AI (wander/
  chase/attack), player controller, HUD, fixed-timestep loop, cameras (first
  person / orbit / spectator / third person), particles, day/night + fog,
  save/load level format v0.3.

## Tooling around the compiler

- `src/pkg.rs` — package manager: `vulcan init`, `vulcan add <user/repo|path>`,
  a text manifest (`vulcan.lock` style lockfile with commits). Git deps are
  shallow-cloned into `deps/<name>`.
- `src/package.rs` — `vulcan package <platform>`: bundles for web (single-file
  index.html with a worker), desktop (WebView shell), Android (Gradle project),
  iOS.
- `src/ide.rs` — `vulcan ide`: serves a small web IDE (edit + run in browser).
- `src/cli.rs` — selftest harness, `vulcan bench`, `vulcan test`, `vulcan doc`,
  graphics demos.

## Constraints that shape all decisions

- Zero dependencies in Cargo.toml (no crates unless a phase allows it).
- Must build on Linux and in Termux (Android phone is the primary dev machine):
  native GPU / SDL deps must stay optional; the web path must always work.
- All selftest output must stay byte-identical across C, JS and Python.
