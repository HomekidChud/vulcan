// Packaging: ship a Vulcan program anywhere.
//  - web:     single self-contained HTML file (terminal emulator + web worker)
//  - desktop: Electron project wrapping the web bundle (macOS/Windows/Linux)
//  - mobile:  Capacitor project wrapping the web bundle (iOS/Android)
//  - linux/windows/macos: native C cross-build scripts
pub fn package(entry: &str, platform: &str) {
    let program = crate::modules::load_program(std::path::Path::new(entry));
    let js = crate::codegen_js::generate_js(&program);
    let stem = entry.trim_end_matches(".vulcan");
    let (dir, files): (String, Vec<(String, String)>) = match platform {
        "web" => web_bundle(stem, &js),
        "desktop" => desktop_bundle(stem, &js),
        "mobile" => mobile_bundle(stem, &js),
        "linux" | "windows" | "macos" => {
            let c_src = crate::codegen::generate(&program);
            native_bundle(stem, platform, &c_src)
        }
        "android" | "ios" => {
            eprintln!("vulcan: for {platform} use `--platform mobile` (Capacitor wraps the web build)");
            std::process::exit(1);
        }
        _ => {
            eprintln!("vulcan: unknown platform `{platform}` (web, desktop, mobile, linux, windows, macos)");
            std::process::exit(1);
        }
    };
    let _ = std::fs::create_dir_all(&dir);
    for (name, content) in files {
        let path = format!("{dir}/{name}");
        std::fs::write(&path, content).unwrap();
        if name == "build.sh" {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = std::fs::metadata(&path).unwrap().permissions();
                perms.set_mode(0o755);
                let _ = std::fs::set_permissions(&path, perms);
            }
        }
    }
    println!("packaged for {platform}: {dir}/");
}

fn web_bundle(stem: &str, js: &str) -> (String, Vec<(String, String)>) {
    // strip the `main()` invocation — the worker bootstrap runs it after wiring I/O
    let body = match js.split_once("// ---- main ----") {
        Some((b, _)) => b.to_string(),
        None => js.to_string(),
    };
    let worker_src = WORKER_SHIM.to_string() + "\n" + &body + "\n" + WORKER_MAIN;
    let html = HTML_TEMPLATE
        .replace("__TITLE__", &file_stem(stem))
        .replace("__WORKER_JSON__", &json_escape(&worker_src));
    let dir = format!("package-{}-web", file_stem(stem));
    (dir, vec![("index.html".into(), html)])
}

fn desktop_bundle(stem: &str, js: &str) -> (String, Vec<(String, String)>) {
    let (webdir, webfiles) = web_bundle(stem, js);
    let name = file_stem(stem);
    let dir = format!("package-{name}-desktop");
    let mut files: Vec<(String, String)> = webfiles;
    files.push(("package.json".into(), ELECTRON_PKG.replace("__NAME__", &name)));
    files.push(("main.js".into(), ELECTRON_MAIN.to_string()));
    files.push(("README.md".into(), ELECTRON_README.into()));
    let _ = webdir;
    (dir, files)
}

fn mobile_bundle(stem: &str, js: &str) -> (String, Vec<(String, String)>) {
    let (webdir, webfiles) = web_bundle(stem, js);
    let name = file_stem(stem);
    let dir = format!("package-{name}-mobile");
    let mut files: Vec<(String, String)> = webfiles;
    files.push(("package.json".into(), CAP_PKG.replace("__NAME__", &name)));
    files.push(("capacitor.config.json".into(), CAP_CONFIG.replace("__NAME__", &name)));
    files.push(("README.md".into(), CAP_README.into()));
    let _ = webdir;
    (dir, files)
}

fn native_bundle(stem: &str, platform: &str, c_src: &str) -> (String, Vec<(String, String)>) {
    let name = file_stem(stem);
    let dir = format!("package-{name}-{platform}");
    let (script, readme): (String, String) = match platform {
        "linux" => (NATIVE_LINUX.replace("__NAME__", &name), NATIVE_LINUX_README.replace("__NAME__", &name)),
        "windows" => (NATIVE_WINDOWS.replace("__NAME__", &name), NATIVE_WINDOWS_README.replace("__NAME__", &name)),
        "macos" => (NATIVE_MACOS.replace("__NAME__", &name), NATIVE_MACOS_README.replace("__NAME__", &name)),
        _ => unreachable!(),
    };
    let files = vec![
        ("main.vulcan.c".into(), c_src.to_string()),
        ("build.sh".into(), format!("#!/bin/sh\n{script}")),
        ("README.md".into(), readme.to_string()),
    ];
    (dir, files)
}

fn file_stem(path: &str) -> String {
    path.rsplit('/').next().unwrap_or("app").to_string()
}

fn json_escape(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

const WORKER_SHIM: &str = r#"// Vulcan web worker shim: stdin/stdout/terminal/files for the browser
let __vkeys = [];
let __vfs = new Map();
onmessage = (e) => {
    const d = e.data || {};
    if (d.init) { for (const k in d.fs) __vfs.set(k, d.fs[k]); runMain(); }
    if (d.keys) __vkeys.push(...d.keys);
};
const process = {
    argv: ["vulcan", "app"],
    stdout: { columns: 80, rows: 24, write: (s) => postMessage({ out: s }) },
    exit: (c) => { postMessage({ exit: c }); throw new Error("__vulcan_exit"); }
};
const require = (m) => {
    if (m === "fs") return {
        readFileSync: (p) => __vfs.has(p) ? __vfs.get(p) : "",
        writeFileSync: (p, c) => { __vfs.set(p, c); postMessage({ fswrite: { path: p, contents: c } }); },
        existsSync: (p) => __vfs.has(p)
    };
    throw new Error("module '" + m + "' is not available in the browser");
};
function localStorageShim() { throw new Error("no localStorage in worker"); }
"#;

const WORKER_MAIN: &str = r#"
// ---- worker bootstrap: rebind I/O to the browser, then run ----
__out = (s) => postMessage({ out: s });
__key_ready = () => __vkeys.length > 0;
__read_key = () => { const k = __vkeys.shift(); return k === undefined ? 0 : k; };
__input = () => {
    let line = "";
    for (;;) {
        while (__vkeys.length === 0) { /* wait for a key */ }
        const k = __vkeys.shift();
        if (k === 10 || k === 13) break;
        if (k >= 32 && k < 127) line += String.fromCharCode(k);
    }
    return line;
};
__sleep = (ms) => { const t = Date.now() + ms; while (Date.now() < t) { /* spin */ } };
__ms_now = () => Date.now();
__rand = () => Math.floor(Math.random() * 2147483647);
__rand_int = (lo, hi) => lo + Math.floor(Math.random() * (hi - lo + 1));
__file_read = (p) => __vfs.has(p) ? __vfs.get(p) : "";
__file_write = (p, c) => { __vfs.set(p, c); postMessage({ fswrite: { path: p, contents: c } }); };
__file_exists = (p) => __vfs.has(p);
__screen_w = () => 80;
__screen_h = () => 24;
function runMain() {
    // main() is async (the JS backend awaits builtin calls), so wait for
    // the promise to settle before reporting exit — otherwise the page
    // terminates the worker before the first frame renders
    Promise.resolve(main()).then(
        () => postMessage({ exit: 0 }),
        (e) => {
            if (String(e).indexOf("__vulcan_exit") === -1) postMessage({ error: String(e) });
        }
    );
}
"#;

const HTML_TEMPLATE: &str = r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>__TITLE__ — Vulcan app</title>
<style>
    :root { color-scheme: dark; }
    * { box-sizing: border-box; }
    body {
        margin: 0; min-height: 100vh; display: flex; flex-direction: column;
        background: #0c0d10; color: #d6d8de;
        font-family: ui-monospace, "SF Mono", Menlo, Consolas, monospace;
    }
    header {
        padding: 10px 16px; font-size: 13px; color: #8b8f9a;
        border-bottom: 1px solid #23252c; display: flex; justify-content: space-between;
    }
    header b { color: #cdd1d9; }
    header a { color: #8b8f9a; }
    main { flex: 1; display: flex; align-items: flex-start; justify-content: center; padding: 12px; }
    #term {
        background: #101116; border: 1px solid #23252c; border-radius: 8px;
        padding: 12px 14px; width: min(720px, 100%); overflow: hidden;
    }
    #screen { margin: 0; font-size: 14px; line-height: 1.25; white-space: pre; }
    #vscreen {
        display: none; background: #000; border: 1px solid #23252c; border-radius: 8px;
        width: min(720px, 100%); image-rendering: pixelated; image-rendering: crisp-edges;
    }
    #status { padding: 8px 16px; font-size: 12px; color: #6c707a; border-top: 1px solid #23252c; }
</style>
</head>
<body>
<header><span><b>__TITLE__</b> — built with <a href="https://github.com/HomekidChud/vulcan">Vulcan</a></span><span>keys: WASD / arrows · q quits</span></header>
<main><canvas id="vscreen" width="256" height="144"></canvas><div id="term"><pre id="screen"></pre></div></main>
<div id="status">running…</div>
<script>
"use strict";
// ---- tiny ANSI terminal (80x24) ----
const COLS = 80, ROWS = 24;
const PALETTE = ["#17181c", "#c0392b", "#27ae60", "#c9a227", "#2965d8", "#a44bd8", "#1fb6c9", "#cfd3da"];
function xterm256(n) {
    if (n < 16) return PALETTE[n % 8];
    if (n < 232) {
        n -= 16;
        const g = [0, 95, 135, 175, 215, 255];
        return "rgb(" + g[(n / 36) | 0] + "," + g[((n / 6) | 0) % 6] + "," + g[n % 6] + ")";
    }
    const v = 8 + (n - 232) * 10;
    return "rgb(" + v + "," + v + "," + v + ")";
}
function blankRow() { return Array.from({length: COLS}, () => ({ch: " ", fg: -1, bg: -1})); }
const term = {
    grid: [], cx: 0, cy: 0, fg: -1, bg: -1, buf: "",
    reset() { this.grid = Array.from({length: ROWS}, blankRow); this.cx = 0; this.cy = 0; },
    scroll() {
        while (this.cy >= ROWS) { this.grid.shift(); this.grid.push(blankRow()); this.cy--; }
        if (this.cy < 0) this.cy = 0;
    },
    put(ch) {
        this.scroll();
        const x = Math.min(this.cx, COLS - 1);
        this.grid[this.cy][x] = { ch, fg: this.fg, bg: this.bg };
        this.cx = Math.min(this.cx + 1, COLS);
    },
    seq(params, cmd) {
        const ps = params.split(";").map((p) => parseInt(p || "0", 10));
        if (cmd === "H" || cmd === "f") {
            this.cy = Math.max(0, (ps[0] || 1) - 1);
            this.cx = Math.max(0, (ps[1] || 1) - 1);
        } else if (cmd === "J") {
            if ((ps[0] || 0) === 2) this.reset();
        } else if (cmd === "K") {
            for (let x = this.cx; x < COLS; x++) this.grid[this.cy][x] = {ch: " ", fg: -1, bg: -1};
        } else if (cmd === "A") { this.cy = Math.max(0, this.cy - (ps[0] || 1)); }
        else if (cmd === "B") { this.cy += ps[0] || 1; this.scroll(); }
        else if (cmd === "C") { this.cx = Math.min(COLS - 1, this.cx + (ps[0] || 1)); }
        else if (cmd === "D") { this.cx = Math.max(0, this.cx - (ps[0] || 1)); }
        else if (cmd === "m") {
            let i = 0;
            while (i < ps.length) {
                const p = ps[i];
                if (p === 0) { this.fg = -1; this.bg = -1; }
                else if (p >= 30 && p <= 37) this.fg = p - 30;
                else if (p === 38 && ps[i + 1] === 5) { this.fg = ps[i + 2]; i += 2; }
                else if (p === 39) this.fg = -1;
                else if (p >= 40 && p <= 47) this.bg = p - 40;
                else if (p === 48 && ps[i + 1] === 5) { this.bg = ps[i + 2]; i += 2; }
                else if (p === 49) this.bg = -1;
                i++;
            }
        }
    },
    feed(text) {
        this.buf += text;
        let i = 0;
        while (i < this.buf.length) {
            const c = this.buf[i];
            if (c === "\x1b") {
                const m = /^\[([0-9;?]*)([A-Za-z])/.exec(this.buf.slice(i));
                if (!m) break;
                this.seq(m[1], m[2]);
                i += m[0].length;
            } else if (c === "\n") { this.cy++; this.cx = 0; this.scroll(); i++; }
            else if (c === "\r") { this.cx = 0; i++; }
            else { this.put(c); i++; }
        }
        this.buf = this.buf.slice(i);
    },
    render() {
        const esc = (t) => t.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
        let html = "";
        for (let y = 0; y < ROWS; y++) {
            let open = false;
            let curFg = -2, curBg = -2;
            for (let x = 0; x < COLS; x++) {
                const cell = this.grid[y][x];
                if (cell.fg !== curFg || cell.bg !== curBg) {
                    if (open) html += "</span>";
                    let style = "";
                    if (cell.fg >= 0) style += "color:" + xterm256(cell.fg % 256) + ";";
                    if (cell.bg >= 0) style += "background:" + xterm256(cell.bg % 256) + ";";
                    html += "<span style='" + style + "'>";
                    open = true; curFg = cell.fg; curBg = cell.bg;
                }
                html += esc(cell.ch);
            }
            if (open) html += "</span>";
            html += "\n";
        }
        document.getElementById("screen").innerHTML = html;
    }
};
term.reset();
// ---- worker wiring ----
const WORKER_SRC = "__WORKER_JSON__";
const worker = new Worker(URL.createObjectURL(new Blob([WORKER_SRC], {type: "application/javascript"})));
// in-memory file storage (the app's file_write/file_read work per session)
const savedFs = {};
worker.onmessage = (e) => {
    const d = e.data || {};
    if (d.out !== undefined) { term.feed(d.out); term.render(); }
    if (d.__vulcan_frame) {
        // full-resolution canvas frame from present() — draw it on #vscreen
        const cv = document.getElementById("vscreen");
        const termEl = document.getElementById("term");
        cv.style.display = "block";
        termEl.style.display = "none";
        const ctx = cv.getContext("2d");
        if (cv.width !== d.w || cv.height !== d.h) { cv.width = d.w; cv.height = d.h; }
        const img = ctx.createImageData(d.w, d.h);
        const px = new Uint32Array(d.__vulcan_frame);
        for (let i = 0; i < px.length; i++) {
            const v = px[i] | 0;
            img.data[i * 4] = (v >> 16) & 255;
            img.data[i * 4 + 1] = (v >> 8) & 255;
            img.data[i * 4 + 2] = v & 255;
            img.data[i * 4 + 3] = 255;
        }
        ctx.putImageData(img, 0, 0);
    }
    if (d.fswrite) { savedFs[d.fswrite.path] = d.fswrite.contents; }
    if (d.exit !== undefined) { document.getElementById("status").textContent = "program exited with code " + d.exit; worker.terminate(); }
    if (d.error) { document.getElementById("status").textContent = "error: " + d.error; }
};
const KEYMAP = { Enter: 10, Backspace: 127, Escape: 27, ArrowUp: 119, ArrowDown: 115, ArrowLeft: 97, ArrowRight: 100 };
document.addEventListener("keydown", (e) => {
    let code = null;
    if (e.key.length === 1) code = e.key.charCodeAt(0);
    else if (KEYMAP[e.key] !== undefined) code = KEYMAP[e.key];
    if (code !== null) { e.preventDefault(); worker.postMessage({ keys: [code] }); }
});
worker.postMessage({ init: true, fs: savedFs });
</script>
</body>
</html>
"##;

const ELECTRON_PKG: &str = r#"{
  "name": "__NAME__",
  "version": "1.0.0",
  "main": "main.js",
  "scripts": { "start": "electron ." },
  "devDependencies": { "electron": "^33" }
}
"#;

const ELECTRON_MAIN: &str = r#"const { app, BrowserWindow } = require("electron");
app.whenReady().then(() => {
  const win = new BrowserWindow({
    width: 780, height: 560,
    title: "__NAME__",
    webPreferences: { contextIsolation: true }
  });
  win.setMenuBarVisibility(false);
  win.loadFile("index.html");
});
app.on("window-all-closed", () => app.quit());
"#;

const ELECTRON_README: &str = r#"# Desktop app (Electron)

Your Vulcan program as a desktop app for macOS, Windows and Linux.

    npm install
    npm start

Build installers (produces .dmg / .exe / AppImage):

    npx electron-packager . __NAME__ --platform=mac --arch=arm64,x64
    npx electron-packager . __NAME__ --platform=win32
    npx electron-packager . __NAME__ --platform=linux
"#;

const CAP_PKG: &str = r#"{
  "name": "__NAME__",
  "version": "1.0.0",
  "dependencies": {
    "@capacitor/core": "^6",
    "@capacitor/cli": "^6"
  }
}
"#;

const CAP_CONFIG: &str = r#"{
  "appId": "com.vulcan.__NAME__",
  "appName": "__NAME__",
  "webDir": "."
}
"#;

const CAP_README: &str = r#"# Mobile app (Capacitor — iOS + Android)

Your Vulcan program as a native mobile app. The web build (index.html) runs
inside a native WebView shell.

    npm install
    npx cap add ios       # requires Xcode (macOS)
    npx cap add android   # requires Android Studio
    npx cap sync
    npx cap open ios      # or: npx cap open android

Keyboard note: on-device keyboards appear when the app needs input; for games
that read raw keys, tap the screen area first so it gets focus.
"#;

const NATIVE_LINUX: &str = r#"set -e
CC=${CC:-cc}
NAME=__NAME__
$CC -O3 -fstack-protector-strong -D_FORTIFY_SOURCE=2 -fPIE -pie -Wl,-z,relro -Wl,-z,now \
    -Wall -Wno-unused-variable -Wno-unused-function \
    -o "$NAME" main.vulcan.c -lm
echo "built ./$NAME (native Linux binary, -O3 + hardening)""#;

const NATIVE_LINUX_README: &str = r#"# Native Linux build

    ./build.sh

Compiles the bundled C (generated by the Vulcan C backend) with -O3 and
full hardening (canaries, PIE/ASLR, RELRO, FORTIFY). Works with gcc or clang.
"#;

const NATIVE_WINDOWS: &str = r#"set -e
CC=clang
TARGET=x86_64-w64-windows-gnu
if ! command -v $CC >/dev/null; then echo "clang is required (brew install llvm / apt install clang)"; exit 1; fi
if ! command -v x86_64-w64-windows-gnu-gcc >/dev/null 2>&1; then
  echo "MinGW sysroot missing: brew install mingw-w64  (or apt install mingw-w64)"
  exit 1
fi
$CC -target $TARGET -O3 -fstack-protector-strong -D_FORTIFY_SOURCE=2 \
    -Wall -Wno-unused-variable -Wno-unused-function \
    -o __NAME__.exe main.vulcan.c
echo "built __NAME__.exe - a native Windows binary (cross-compiled, no Windows needed)""#;

const NATIVE_WINDOWS_README: &str = r#"# Native Windows build

Cross-compiles the C backend to a native Windows .exe:

    ./build.sh

Requires: LLVM/clang and mingw-w64 (`brew install mingw-w64` on macOS,
`apt install mingw-w64 clang` on Linux). No Windows machine needed.

For best-effort Windows console colors and input the generated C uses only
standard APIs, so the .exe runs on any Windows 10+.
"#;

const NATIVE_MACOS: &str = r#"set -e
if ! command -v xcrun >/dev/null; then
  echo "This script must run ON a Mac (needs Xcode + the macOS SDK)."
  echo "On other machines, cross-compile from the web/desktop package instead."
  exit 1
fi
CC=clang
ARCHS=${CFLAGS:--arch arm64 -arch x86_64}
$CC $ARCHS -O3 -fstack-protector-strong -D_FORTIFY_SOURCE=2 -fPIE -pie \
    -Wall -Wno-unused-variable -Wno-unused-function \
    -o __NAME__ main.vulcan.c -lm
echo "built ./__NAME__ (universal macOS binary: arm64 + x86_64)""#;

const NATIVE_MACOS_README: &str = r#"# Native macOS build

    ./build.sh yourfile.vulcan

Compiles through the C backend on a Mac with the Apple clang toolchain.
For a universal (arm64 + x86_64) binary:

    CFLAGS="-arch arm64 -arch x86_64" ./build.sh yourfile.vulcan

Notarization for distribution outside the App Store:

    codesign --sign "Developer ID Application: YOU" binary
    xcrun notarytool submit binary.zip --apple-id ... --wait
"#;
