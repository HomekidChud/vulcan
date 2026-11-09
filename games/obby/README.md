# Vulcan Obby

A Roblox-style obstacle course written 100% in Vulcan, running on the web3d
platform (WebGL2). It packages to a single `index.html` you can open in any
modern browser — no install, no server.

```bash
cargo build --release
./target/release/vulcan package games/obby/src/main.vulcan --platform web3d --assets games/obby/assets
# open package-main-web3d/index.html
```

## The game

You are a classic blocky avatar in third person. Cross five checkpoints of
classic obby hazards to reach the gold finish pad:

- gap jumps and rising platforms
- a lava hop with stepping tiles
- a moving platform ride across a chasm
- a spinning kill bar you have to time and jump over
- an elevator up to the high route
- narrow beams with a long fall below
- a final parkour climb to the finish

Checkpoints save your progress — fall off or touch a hazard and you respawn at
the last one. Coins are scattered along the route, the run is timed, and your
best time persists on the device.

## Controls

| input | action |
|---|---|
| WASD / arrows | move |
| Space (hold) | jump / hop |
| Shift | sprint |
| R | respawn |
| Esc | menu |

Touch devices get on-screen arrow and JUMP buttons automatically.

The game ships a `game.json` — its help tabs, first-run help, title and touch
defaults are its own, not the host's Vulcancraft-flavoured defaults (see
`games/README.md`).

## How it's built

Everything — the avatar mesh, the AABB physics (gravity, jumping, per-axis
collision resolution, riding moving platforms), the rotating-bar hit test, the
HUD, the menu, the win screen, best-time persistence — is plain Vulcan: arrays
of f64 for vertex data, parallel arrays for course colliders, one struct for
the player. The only host calls are the standard web3d platform builtins
(`gfx_*`, `hud_*`, `host_store`), the same surface the Arcade games and
Vulcancraft use.
