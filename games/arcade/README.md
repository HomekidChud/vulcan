# Vulcan Arcade

Four small 3D games written in Vulcan on the web3d platform. They share one flat-colour palette atlas, one tiny shader pack and a handful of box-mesh helpers (`src/gx.vulcan`). Each game runs on desktop (mouse and keyboard) and on phones (tap and drag).

| Game | Goal | Touch | Keyboard |
| --- | --- | --- | --- |
| Sky Runner | Endless three-lane runner. Dodge barriers, jump low ones, grab coins. | Tap left or right third to change lane, tap the middle to jump | A / D or arrows, Space |
| Tower Guard | Tower defense. Build Arrow and Cannon towers beside the path, survive the waves. | Tap a tile to build, tap the buttons to pick a tower | 1 / 2 pick tower, mouse to build |
| Star Dodge | Space flyer. The ship fires automatically, shoot or dodge the asteroids. | Touch and drag to steer | WASD / arrows, or mouse |
| Stack Tower | Tap to drop the sliding slab, overhangs get cut off. | Tap | Space or click |

Best scores are stored on the device. Press Escape (Android back button) to return to the menu.

## Build

```bash
cargo build --release
./target/release/vulcan package games/arcade/src/main.vulcan --platform web3d --assets games/arcade/assets --name Arcade
./target/release/vulcan package games/arcade/src/main.vulcan --platform android --assets games/arcade/assets --name VulcanArcade
```

The first command writes a single `index.html`, the second an installable debug-signed APK.
