// Packaging: ship a Vulcan program anywhere.
//  - web:     single self-contained HTML file (terminal emulator + web worker)
//  - desktop: Electron project wrapping the web bundle (macOS/Windows/Linux)
//  - mobile:  Capacitor project wrapping the web bundle (iOS/Android)
//  - android: a ready-to-build native Android project (WebView) and, when the SDK is found, a signed .apk
//  - linux/windows/macos: native C cross-build scripts
pub fn package(entry: &str, platform: &str, assets: Option<&str>, name: Option<&str>, fast: Option<bool>) {
    // 3D games default to unchecked array access (no per-access bounds test); `--checked` restores it
    let fast = fast.unwrap_or(matches!(platform, "web3d" | "android"));
    crate::codegen_js::set_unchecked(fast);
    let program = crate::modules::load_program(std::path::Path::new(entry));
    let js = crate::codegen_js::generate_js(&program);
    let stem = entry.trim_end_matches(".vulcan");
    let stem_owned = name.map(|n| n.to_string());
    let stem = stem_owned.as_deref().unwrap_or(stem);
    let is3d = js.contains("__host.gfx_open") || js.contains("H.gfx_open") || js.contains("gfx_open(");
    let (dir, files): (String, Vec<(String, String)>) = match platform {
        "web" => web_bundle(stem, &js),
        "web3d" => web3d_bundle(stem, &js, assets),
        "desktop" => desktop_bundle(stem, &js, is3d, assets),
        "mobile" => mobile_bundle(stem, &js, is3d, assets),
        "android" => android_bundle(stem, &js, assets),
        "linux" | "windows" | "macos" => {
            let c_src = crate::codegen::generate(&program);
            native_bundle(stem, platform, &c_src)
        }
        "ios" => {
            eprintln!("vulcan: iOS apps need Xcode on a Mac; use `--platform mobile` (Capacitor wraps the web build) and run `npx cap add ios` there");
            std::process::exit(1);
        }
        _ => {
            eprintln!("vulcan: unknown platform `{platform}` (web, web3d, android, desktop, mobile, linux, windows, macos)");
            std::process::exit(1);
        }
    };
    let _ = std::fs::create_dir_all(&dir);
    for (name, content) in files {
        let path = format!("{dir}/{name}");
        if let Some(parent) = std::path::Path::new(&path).parent() { let _ = std::fs::create_dir_all(parent); }
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
    if platform == "android" { android_build(&dir); }
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

// web3d: WebGL2 game bundle. One self-contained index.html: the compiled program,
// the host runtime (renderer, input, audio, 1v1 networking) and an embedded assets
// folder (mods, modpacks, shader packs) readable through file_read/file_list.
fn web3d_bundle(stem: &str, js: &str, assets: Option<&str>) -> (String, Vec<(String, String)>) {
    let mut map: Vec<(String, String)> = Vec::new();
    if let Some(root) = assets {
        collect_assets(std::path::Path::new(root), std::path::Path::new(root), &mut map);
    }
    map.sort();
    let mut json = String::from("{");
    for (i, (k, v)) in map.iter().enumerate() {
        if i > 0 { json.push(','); }
        json.push_str(&format!("\"{}\":\"{}\"", json_escape(k), json_escape(v)));
    }
    json.push('}');
    let json = json.replace("</", "<\\/");
    let host = include_str!("../runtime/web3d_host.js").replace("</script", "<\\/script");
    let prog = js.replace("</script", "<\\/script");
    let html = include_str!("../runtime/web3d.html")
        .replace("__TITLE__", &file_stem(stem))
        .replace("__ASSETS_JSON__", &json)
        .replace("__HOST_JS__", &host)
        .replace("__PROGRAM_JS__", &prog);
    (format!("package-{}-web3d", file_stem(stem)), vec![("index.html".into(), html)])
}

fn collect_assets(root: &std::path::Path, dir: &std::path::Path, out: &mut Vec<(String, String)>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() { collect_assets(root, &p, out); continue; }
        if let Ok(text) = std::fs::read_to_string(&p) {
            let rel = p.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
            out.push((rel, text));
        }
    }
}

fn desktop_bundle(stem: &str, js: &str, is3d: bool, assets: Option<&str>) -> (String, Vec<(String, String)>) {
    let (webdir, webfiles) = if is3d { web3d_bundle(stem, js, assets) } else { web_bundle(stem, js) };
    let name = file_stem(stem);
    let dir = format!("package-{name}-desktop");
    let mut files: Vec<(String, String)> = webfiles;
    files.push(("package.json".into(), ELECTRON_PKG.replace("__NAME__", &name)));
    files.push(("main.js".into(), ELECTRON_MAIN.to_string()));
    files.push(("README.md".into(), ELECTRON_README.into()));
    let _ = webdir;
    (dir, files)
}

fn mobile_bundle(stem: &str, js: &str, is3d: bool, assets: Option<&str>) -> (String, Vec<(String, String)>) {
    let (webdir, webfiles) = if is3d { web3d_bundle(stem, js, assets) } else { web_bundle(stem, js) };
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
    try {
        main();
        postMessage({ exit: 0 });
    } catch (e) {
        if (String(e).indexOf("__vulcan_exit") === -1) postMessage({ error: String(e) });
    }
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
    #status { padding: 8px 16px; font-size: 12px; color: #6c707a; border-top: 1px solid #23252c; }
</style>
</head>
<body>
<header><span><b>__TITLE__</b> — built with <a href="https://github.com/HomekidChud/vulcan">Vulcan</a></span><span>keys: WASD / arrows · q quits</span></header>
<main><div id="term"><pre id="screen"></pre></div></main>
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

// ---------------------------------------------------------------- android
// A tiny native shell: one Activity with a full-screen WebView that loads the bundled
// page from assets. No Gradle: build.sh calls aapt2 / javac / d8 / apksigner directly,
// which takes a few seconds.
fn android_bundle(stem: &str, js: &str, assets: Option<&str>) -> (String, Vec<(String, String)>) {
    let name = file_stem(stem);
    let is3d = js.contains("gfx_open(") || js.contains("__host.gfx_open") || js.contains("H.gfx_open");
    let (_d, webfiles) = if is3d { web3d_bundle(stem, js, assets) } else { web_bundle(stem, js) };
    let pkg_id: String = name.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_lowercase();
    let pkg_id = if pkg_id.is_empty() || pkg_id.chars().next().unwrap().is_ascii_digit() { format!("app{pkg_id}") } else { pkg_id };
    let pkg = format!("com.vulcan.{pkg_id}");
    let label = name.replace('&', "and").replace('<', "").replace('>', "");
    let fill = |t: &str| t.replace("__PKG__", &pkg).replace("__LABEL__", &label).replace("__NAME__", &name);
    let mut files: Vec<(String, String)> = Vec::new();
    for (n, c) in webfiles { files.push((format!("assets/{n}"), c)); }
    files.push(("AndroidManifest.xml".into(), fill(ANDROID_MANIFEST)));
    files.push((format!("src/{}/MainActivity.java", pkg.replace('.', "/")), fill(ANDROID_ACTIVITY)));
    files.push(("res/drawable/ic_launcher.xml".into(), ANDROID_ICON.into()));
    files.push(("build.sh".into(), fill(ANDROID_BUILD)));
    files.push(("README.md".into(), fill(ANDROID_README)));
    (format!("package-{name}-android"), files)
}

fn android_tools() -> Option<(String, String)> {
    let home = std::env::var("HOME").unwrap_or_default();
    let sdk = ["ANDROID_HOME", "ANDROID_SDK_ROOT", "VULCAN_ANDROID_SDK"].iter()
        .filter_map(|k| std::env::var(k).ok()).chain(std::iter::once(format!("{home}/sdk/android")))
        .find(|p| std::path::Path::new(&format!("{p}/build-tools")).is_dir() && std::path::Path::new(&format!("{p}/platforms")).is_dir())?;
    let jdk = std::env::var("JAVA_HOME").ok().into_iter().chain(std::iter::once(format!("{home}/sdk/jdk")))
        .find(|p| std::path::Path::new(&format!("{p}/bin/javac")).exists());
    let jdk = match jdk {
        Some(j) => j,
        None => if std::process::Command::new("javac").arg("-version").output().is_ok() { String::new() } else { return None },
    };
    Some((sdk, jdk))
}

fn android_build(dir: &str) {
    let Some((sdk, jdk)) = android_tools() else {
        println!("android SDK/JDK not found. Install the Android SDK (build-tools + a platform) and a JDK, set ANDROID_HOME, then run {dir}/build.sh");
        return;
    };
    let t = std::time::Instant::now();
    let mut cmd = std::process::Command::new("sh");
    cmd.arg("build.sh").current_dir(dir).env("ANDROID_HOME", &sdk);
    if !jdk.is_empty() {
        let path = std::env::var("PATH").unwrap_or_default();
        cmd.env("JAVA_HOME", &jdk).env("PATH", format!("{jdk}/bin:{path}"));
    }
    match cmd.status() {
        Ok(st) if st.success() => println!("built apk in {:.1}s", t.elapsed().as_secs_f64()),
        _ => eprintln!("vulcan: apk build failed (see output above); project is in {dir}/"),
    }
}

const ANDROID_MANIFEST: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android" package="__PKG__" android:versionCode="1" android:versionName="1.0">
    <uses-sdk android:minSdkVersion="24" android:targetSdkVersion="34"/>
    <uses-permission android:name="android.permission.INTERNET"/>
    <uses-permission android:name="android.permission.ACCESS_NETWORK_STATE"/>
    <uses-feature android:glEsVersion="0x00030000" android:required="true"/>
    <application android:label="__LABEL__" android:icon="@drawable/ic_launcher" android:hardwareAccelerated="true" android:allowBackup="true">
        <activity android:name=".MainActivity" android:exported="true"
            android:screenOrientation="sensorLandscape"
            android:configChanges="orientation|screenSize|keyboardHidden|keyboard|navigation|smallestScreenSize|screenLayout|uiMode|density"
            android:theme="@android:style/Theme.DeviceDefault.NoActionBar.Fullscreen">
            <intent-filter>
                <action android:name="android.intent.action.MAIN"/>
                <category android:name="android.intent.category.LAUNCHER"/>
            </intent-filter>
        </activity>
    </application>
</manifest>
"#;

const ANDROID_ACTIVITY: &str = r#"package __PKG__;

import android.app.Activity;
import android.content.Intent;
import android.graphics.Color;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.view.View;
import android.view.WindowManager;
import android.webkit.ValueCallback;
import android.webkit.WebChromeClient;
import android.webkit.WebSettings;
import android.webkit.WebView;
import android.webkit.WebViewClient;

public class MainActivity extends Activity {
    private WebView web;
    private ValueCallback<Uri[]> chooser;
    private static final int REQ_FILES = 7;

    @Override
    protected void onCreate(Bundle b) {
        super.onCreate(b);
        getWindow().addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);
        if (Build.VERSION.SDK_INT >= 28) {
            WindowManager.LayoutParams lp = getWindow().getAttributes();
            lp.layoutInDisplayCutoutMode = WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES;
            getWindow().setAttributes(lp);
        }
        web = new WebView(this);
        web.setBackgroundColor(Color.BLACK);
        web.setOverScrollMode(View.OVER_SCROLL_NEVER);
        WebSettings s = web.getSettings();
        s.setJavaScriptEnabled(true);
        s.setDomStorageEnabled(true);
        s.setAllowFileAccess(true);
        s.setMediaPlaybackRequiresUserGesture(false);
        s.setSupportZoom(false);
        s.setBuiltInZoomControls(false);
        web.setWebViewClient(new WebViewClient());
        web.setWebChromeClient(new WebChromeClient() {
            @Override
            public boolean onShowFileChooser(WebView v, ValueCallback<Uri[]> cb, FileChooserParams params) {
                if (chooser != null) chooser.onReceiveValue(null);
                chooser = cb;
                Intent i = new Intent(Intent.ACTION_GET_CONTENT);
                i.addCategory(Intent.CATEGORY_OPENABLE);
                i.setType("*/*");
                i.putExtra(Intent.EXTRA_ALLOW_MULTIPLE, true);
                try {
                    startActivityForResult(Intent.createChooser(i, "Import mods and shaders"), REQ_FILES);
                } catch (Exception e) {
                    chooser = null;
                    return false;
                }
                return true;
            }
        });
        setContentView(web);
        web.loadUrl("file:///android_asset/index.html");
    }

    @Override
    protected void onActivityResult(int req, int res, Intent data) {
        if (req == REQ_FILES && chooser != null) {
            chooser.onReceiveValue(WebChromeClient.FileChooserParams.parseResult(res, data));
            chooser = null;
        }
        super.onActivityResult(req, res, data);
    }

    private void immersive() {
        getWindow().getDecorView().setSystemUiVisibility(
            View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY | View.SYSTEM_UI_FLAG_FULLSCREEN
            | View.SYSTEM_UI_FLAG_HIDE_NAVIGATION | View.SYSTEM_UI_FLAG_LAYOUT_STABLE
            | View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN | View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION);
    }

    @Override
    public void onWindowFocusChanged(boolean focus) {
        super.onWindowFocusChanged(focus);
        if (focus) immersive();
    }

    @Override
    public void onBackPressed() {
        web.evaluateJavascript("(window.__vulcanBack && window.__vulcanBack()) ? '1' : '0'", new ValueCallback<String>() {
            @Override
            public void onReceiveValue(String v) {
                if (!"\"1\"".equals(v)) finish();
            }
        });
    }

    @Override
    protected void onPause() { super.onPause(); web.onPause(); }

    @Override
    protected void onResume() { super.onResume(); web.onResume(); immersive(); }

    @Override
    protected void onDestroy() { web.destroy(); super.onDestroy(); }
}
"#;

const ANDROID_ICON: &str = r##"<?xml version="1.0" encoding="utf-8"?>
<vector xmlns:android="http://schemas.android.com/apk/res/android" android:width="108dp" android:height="108dp" android:viewportWidth="108" android:viewportHeight="108">
    <path android:fillColor="#1B2236" android:pathData="M0,0h108v108h-108z"/>
    <path android:fillColor="#7FD36B" android:pathData="M54,22 L84,38 L54,54 L24,38 Z"/>
    <path android:fillColor="#8A5A2B" android:pathData="M24,38 L54,54 L54,88 L24,72 Z"/>
    <path android:fillColor="#6B4521" android:pathData="M84,38 L54,54 L54,88 L84,72 Z"/>
    <path android:fillColor="#FFC93C" android:pathData="M54,30 L70,38 L54,46 L38,38 Z"/>
</vector>
"##;

const ANDROID_BUILD: &str = r#"#!/bin/sh
# Builds __NAME__.apk without Gradle: aapt2 -> javac -> d8 -> zipalign -> apksigner.
set -e
cd "$(dirname "$0")"
SDK="${ANDROID_HOME:-${ANDROID_SDK_ROOT:-${VULCAN_ANDROID_SDK:-$HOME/sdk/android}}}"
BT=$(ls -d "$SDK"/build-tools/* | sort -V | tail -1)
PL=$(ls -d "$SDK"/platforms/android-* | sort -V | tail -1)
JAR="$PL/android.jar"
if ! command -v javac >/dev/null 2>&1; then
  if [ -x "$HOME/sdk/jdk/bin/javac" ]; then export JAVA_HOME="$HOME/sdk/jdk" PATH="$HOME/sdk/jdk/bin:$PATH"; else echo "javac not found: install a JDK (11+) or set JAVA_HOME" >&2; exit 1; fi
fi
KS="${VULCAN_KEYSTORE:-$HOME/.vulcan/debug.keystore}"
if [ ! -f "$KS" ]; then
  mkdir -p "$(dirname "$KS")"
  keytool -genkeypair -keystore "$KS" -storepass android -keypass android -alias vulcandebug -keyalg RSA -keysize 2048 -validity 10000 -dname "CN=Vulcan Debug,O=Vulcan,C=US" >/dev/null 2>&1
fi
rm -rf build; mkdir -p build/classes
"$BT/aapt2" compile --dir res -o build/res.zip
"$BT/aapt2" link -o build/base.apk -I "$JAR" --manifest AndroidManifest.xml --min-sdk-version 24 --target-sdk-version 34 -A assets build/res.zip
javac -nowarn -Xlint:-options -source 8 -target 8 -classpath "$JAR" -d build/classes $(find src -name '*.java')
"$BT/d8" --release --min-api 24 --lib "$JAR" --output build $(find build/classes -name '*.class')
( cd build && if command -v zip >/dev/null 2>&1; then zip -q -0 -X base.apk classes.dex; else "$BT/aapt" add base.apk classes.dex >/dev/null; fi )
"$BT/zipalign" -f -p 4 build/base.apk build/aligned.apk
"$BT/apksigner" sign --ks "$KS" --ks-pass pass:android --key-pass pass:android --out __NAME__.apk build/aligned.apk
rm -f __NAME__.apk.idsig
echo "built $(pwd)/__NAME__.apk ($(wc -c < __NAME__.apk) bytes)"
"#;

const ANDROID_README: &str = r#"# __NAME__ for Android

Native shell around a full-screen WebView (WebGL2), generated by `vulcan package --platform android`.

Build (a few seconds, no Gradle):

    ./build.sh          # needs Android build-tools + a platform in $ANDROID_HOME and a JDK 11+

Output: `__NAME__.apk`, signed with a debug key (`~/.vulcan/debug.keystore`). Install it by copying the
file to the phone and opening it (allow "install unknown apps" for your file manager), or with
`adb install __NAME__.apk`.

The package id is `__PKG__`. For the Play Store, sign with your own release key instead of the debug key.
"#;
