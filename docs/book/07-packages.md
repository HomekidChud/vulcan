# Chapter 7: Packages and projects

In this chapter you'll learn how imports resolve, how to structure a project with `vulcan.toml`, and how to add and publish packages through the registry. Your todo app can finally use someone else's code.

## `import` and where files are found

`import math` loads another Vulcan file and brings its public names into scope:

```vulcan
import math

fn main() {
    print(square(6.0))
    print(clamp(150.0, 0.0, 100.0))
    print(PI)
}
```

Output:

```
36
100
3.14159
```

The resolver looks for the module in several places, in order:

1. next to the importing file (`./math.vulcan`),
2. in `src/` and `lib/` directories next to it,
3. in `deps/<name>/src/lib.vulcan`, `deps/<name>/lib.vulcan`, or `deps/<name>/src/main.vulcan` (where `vulcan add` puts dependencies).

So a project layout like this just works with `import math`:

```
myapp/
  vulcan.toml
  src/main.vulcan
  lib/math.vulcan
```

Imports are transitive: an imported file's own imports are loaded too. Everything flattens into one program.

## Projects: `vulcan init`

```bash
vulcan init myapp
cd myapp
vulcan run        # builds to build/myapp and runs it
vulcan test       # runs every test_* function
vulcan build      # produces build/myapp
```

`vulcan init` creates:

```
myapp/
  vulcan.toml      # [package] name + version
  src/main.vulcan  # a working starter program
```

## Dependencies: `vulcan add`

```bash
vulcan add git@github.com:user/mathlib.git   # from git
vulcan add ../mathlib                        # from a local path
vulcan add mathlib                           # bare name: resolved via the registry
```

`vulcan add` vendors the dependency into `deps/` and records it in `vulcan.lock`. Related commands:

| command | what it does |
|---|---|
| `vulcan remove <name>` | drop a dependency |
| `vulcan update` | re-fetch every dependency |
| `vulcan search <query>` | find packages in the registry |
| `vulcan publish` | push this project to the registry (needs `gh`) |

The registry resolves bare names — `vulcan add mathlib` finds the package so you don't need a URL. Publishing requires the GitHub CLI (`gh`) because the registry is built on GitHub.

## Publishing your own package

Put your library in a project with a `vulcan.toml` and run `vulcan publish` from the project root. Anyone can then `vulcan add yourpkg`. Mark the functions and constants you intend as public API with `pub` — it's convention (and what `vulcan doc` prints), not an access barrier; today any top-level name is importable.

## What ships in `lib/`

The compiler's own repository doubles as a package source. These libraries are pure Vulcan and importable anywhere:

- `lib/math.vulcan` — `PI`, `square`, `clamp`
- `lib/strbuf.vulcan` — string builder (`[string]` parts, one `join` at the end)
- `lib/http.vulcan` — HTTP/1.1 client (`http_get` and friends) on the socket layer
- `lib/physics.vulcan` — 2D rigid bodies
- `lib/render3d.vulcan` — Vec3, camera, software 3D renderer
- `lib/engine.vulcan` — entities, terrain, player controller, HUD

Where to go next: [Chapter 8: Backends and FFI](08-backends.md) — what the three targets are and how to call C.
