# Vulcancraft

A voxel 1v1 duel game written entirely in Vulcan (about 4,000 lines), with mod packs, swappable shader packs and online play. It compiles to one `index.html`.

```bash
cargo build --release
./target/release/vulcan package games/vulcancraft/src/main.vulcan --platform web3d --assets games/vulcancraft/assets
# open package-main-web3d/index.html in a WebGL2 browser (Chrome, Edge, Firefox, Safari 15+)
```

## Modes

- Practice vs Bot: a simple melee bot, offline.
- 1v1 on this device: open the page in two tabs of the same browser; they pair automatically.
- Host / Join online: WebRTC with manual invite codes. The host creates a code, the guest pastes it and sends back an answer code. No server is needed except a public STUN server, so it works on most home networks but not behind strict symmetric NAT.

First to the pack's kill limit wins. The lowest peer id chooses the world seed, so both players see the same terrain; block edits and hits are synced.

## Controls

WASD move, Shift sprint, Space jump (hold to swim), mouse look, left click hit / mine, right click place, middle click pick block, 1-9 or wheel for the hotbar, E block palette, F6 next shader pack, F3 debug, Q (or Esc) back to the menu.

## Mobile and touch

The page detects touch devices and shows on-screen controls (landscape): drag on the left half to move (push the stick all the way forward to sprint), drag on the right half to look, HIT (hold to mine), PLACE, JUMP, the < > buttons to change hotbar slot, Blocks for the palette, FX to cycle shader packs and Menu (tap twice) to leave the match. Touch devices default to the Vanilla shader and a render distance of 4; raise them in the menu if your phone can take it. The UI scales down automatically on small screens, and the Fullscreen button appears where the browser supports it (not on iPhone Safari; use Share, Add to Home Screen there).

## Mod packs

A modpack (`assets/modpacks/*.modpack`) is an ordered list of mods:

```
modpack Moon_Duel
desc Low gravity arena duel.
mod core
mod moon
```

A mod (`assets/mods/*.vmod`) is one command per line, `#` for comments:

```
block <id 1..255> <key> name= kind=opaque|cutout|glass|liquid|plant tex= top= bottom= base=#rrggbb accent=#rrggbb top_base= top_accent= bottom_base= bottom_accent= hardness= light= damage=
override <key> hardness= light= damage=
rule gravity= jump= speed= sprint= reach= damage= health= kills= knockback= fly= daylen= respawn= start_time= swim= fall= mine=
weapon <key> name= damage= reach= cooldown= knockback= blade=#rrggbb hilt=#rrggbb
biome <plains|forest|desert|snow|mountains|beach> top= filler= tree=log,leaves tree_freq= plant= plant_freq=
ore <key> min= max= count= size=
sea level=
cave width= lava=
```

Texture patterns for `tex=`: noise, speckle, bricks, planks, grass_side, grass_top, logside, logtop, leaves, sand, snow_side, glass, water, lava, glow, crystal, cactus, plant, flower, checker, stripes, solid. Use underscores for spaces in names.

Bundled mods: core, crystals, colors, moon, hardcore. Bundled packs: Classic, Moon Duel, Gem Caves, Builder Arena, Hardcore. Use the Import button in the menu to add your own `.vmod`, `.modpack` or `.vshader`; imports persist in the browser. In online play both players must pick the same pack (a content hash is checked in the handshake).

## Shader packs

A `.vshader` file (`assets/shaders/`) holds GLSL ES 3.00 sections:

```
@name Name
@desc One line description
@common       shared GLSL
@scene.vs / @scene.fs   world geometry (layer 0 opaque, layer 1 transparent)
@shadow.vs / @shadow.fs sun shadow map
@sky.fs                 sky background
@post0.fs scale=0.5     post chain: post0..postN, each optional scale of screen size
```

Available uniforms include uViewProj, uInvViewProj, uShadowMat, uCamPos, uResolution, uTime, uSunDir, uSunColor, uSkyTop, uSkyHorizon, uAmbient, uFog, uDay, plus samplers uAtlas, uShadow, uScene, uDepth, uColor (previous pass) and uPass0..3. Scene attributes: aPos, aNormal, aUV, aLight (sky, block, ao), aMat. Output to `fragColor`.

Bundled: Lumina (shadows, SSAO, bloom, god rays, water, clouds, ACES, FXAA), Vanilla, Retro (low-res dithered), Cinematic (film grade and letterbox).

## Source layout

`util`, `tex` (procedural texture atlas), `blocks`, `mods` (parsers), `world` (terrain, caves, ores, streaming), `mesh` (smooth AO and light flood fill), `game`, `player` (physics, combat, bot), `net` (protocol), `render`, `ui`, `shaders`, `main`.
