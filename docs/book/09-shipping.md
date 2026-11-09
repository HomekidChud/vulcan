# Chapter 9: Shipping

In this chapter you'll learn to turn one `.vulcan` file into a web app, a desktop app, a mobile app, or a native-OS bundle with a single command. No project setup needed — `vulcan package` works on any single file.

## The one command

```bash
vulcan package examples/todo.vulcan --platform web       # single index.html
vulcan package examples/todo.vulcan --platform desktop   # Electron scaffold (npm install; packages .dmg/.exe/AppImage)
vulcan package examples/todo.vulcan --platform mobile    # Capacitor scaffold (npx cap add ios / android)
vulcan package examples/todo.vulcan --platform linux     # embedded C source + ./build.sh (cc -O3 + hardening)
vulcan package examples/todo.vulcan --platform windows   # mingw-w64 cross-compile build.sh
vulcan package examples/todo.vulcan --platform macos     # universal (arm64 + x86_64) build.sh
vulcan apk examples/todo.vulcan                          # installable Android APK (no Gradle)
```

For the web platform the result is one self-contained directory:

```
$ vulcan package examples/todo.vulcan --platform web
packaged for web: package-todo-web/
```

`package-todo-web/` contains a single `index.html` — your program compiled to JS, running in a Web Worker, rendered through an 80x24 ANSI terminal emulator with full color. Keyboard input, `sleep_ms` pacing, and in-memory `file_read`/`file_write` all work in the browser. Open the file directly or serve it statically; no build step, no npm.

## What each platform gives you

| platform | bundle | what you get |
|---|---|---|
| `web` | single `index.html` | terminal emulator + web-worker runtime, touch input support |
| `desktop` | Electron scaffold | `npm install` then packages `.dmg` / `.exe` / AppImage |
| `mobile` | Capacitor scaffold | `npx cap add ios` / `android` |
| `linux` / `windows` / `macos` | generated C + `build.sh` | one-command native build with hardening |
| `vulcan apk` | installable APK | Termux-native pipeline (aapt + ecj + d8 + apksigner), no Gradle |

The native-OS bundles ship the generated C — the same hardened code `vulcan build` produces — with a `build.sh` that compiles it on the target.

## Your UI, your rules

The web/APK shell draws nothing but your program's terminal output and canvas — no header bar, no default buttons. Whatever appears on screen is there because the program built it:

```vulcan
fn main() {
    if is_touch() {
        ui_joystick(16.0, 76.0, 96.0, 119, 115, 97, 100)   // WASD
        ui_button("FIRE", 102, 82.0, 80.0, 74.0)          // 'f'
    }
    ui_css("body { background: #101020 }")
    ui_title("my game")
}
```

(On a terminal with no touch screen this runs and exits silently — the `ui_*` calls are no-ops where there is no shell.)

The `ui_*` builtins (`ui_button`, `ui_joystick`, `ui_html`, `ui_css`, `ui_title`, `ui_clear`) work on every backend — they are no-ops where there is no shell. For total freedom, drop a `shell.html` next to your `.vulcan` file: it becomes the entire app page. The APK build also honors your own `AndroidManifest.xml` and `icon.png`, so orientation, permissions, and the launcher icon are yours too.

## Touch and canvas

The canvas is a live framebuffer (default 256x144, resizable to 4096x4096 with `canvas_resize`). Every touch point is forwarded in logical canvas coordinates, so games lay out their own controls and hit-test with `touch_x` / `touch_y`. The full graphics API — `clear`, `draw_rect`, `draw_circle`, `draw_line`, `draw_text`, `present`, `mouse_x` — is in the [README graphics reference](https://github.com/HomekidChud/vulcan#graphics--physics-reference).

## A note on honesty

`vulcan package` scaffolds real, standard tooling around your program — Electron, Capacitor, or a C build script. It does not hide that tooling: you'll need `npm` for desktop/mobile targets and a C compiler for native-OS targets, and you can edit every generated file. The scaffold is a starting point, not a black box.

Where to go next: [Chapter 10: Testing](10-testing.md) — keeping what you shipped working.
