// src/ide.rs — `vulcan ide`: a local web IDE for Vulcan.
//
// Serves a single-page editor + live-canvas runtime on 127.0.0.1. Programs
// are compiled to JavaScript by the compiler in-process and executed inside
// a sandboxed Web Worker; the graphics builtins render to an HTML canvas
// (instead of ANSI) and mouse/keyboard input is wired through a
// SharedArrayBuffer. No network access, no dependencies, std only.
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::panic::AssertUnwindSafe;
use std::path::PathBuf;
use std::thread;

const PHYSICS_LIB: &str = include_str!("../lib/physics.vulcan");

const EXAMPLES: &[(&str, &str)] = &[
    ("hello_canvas.vulcan", include_str!("../examples/hello_canvas.vulcan")),
    ("bounce.vulcan", include_str!("../examples/bounce.vulcan")),
    ("pyramid.vulcan", include_str!("../examples/pyramid.vulcan")),
    ("springs.vulcan", include_str!("../examples/springs.vulcan")),
    ("attract.vulcan", include_str!("../examples/attract.vulcan")),
    ("spin_cube.vulcan", include_str!("../examples/spin_cube.vulcan")),
    ("fortnite.vulcan", include_str!("../examples/fortnite.vulcan")),
];

pub fn serve(port: u16) {
    let ws = provision_workspace();

    let listener = TcpListener::bind(("127.0.0.1", port))
        .unwrap_or_else(|e| panic!("vulcan: cannot bind 127.0.0.1:{port}: {e}"));
    println!("Vulcan IDE: http://127.0.0.1:{port}  (Ctrl+C to stop)");
    println!("  workspace: {}  (your files are saved in .ide/)", ws.display());

    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                let ws = ws.clone();
                thread::spawn(move || {
                    let _ = handle_conn(s, ws);
                });
            }
            Err(_) => continue,
        }
    }
}

// Creates .ide/ in the current directory and provisions .ide/lib/ so that
// `import physics` (and any lib/*.vulcan from the current project) resolves.
fn provision_workspace() -> PathBuf {
    let ws = PathBuf::from(".ide");
    let _ = std::fs::create_dir_all(ws.join("lib"));
    if std::fs::read_dir("lib").is_ok() {
        // a project with a lib/ directory: use its modules
        if let Ok(entries) = std::fs::read_dir("lib") {
            for e in entries.flatten() {
                let p = e.path();
                if p.extension().map(|x| x == "vulcan").unwrap_or(false) {
                    if let Some(name) = p.file_name() {
                        let dest = ws.join("lib").join(name);
                        if !dest.exists() {
                            let _ = std::fs::copy(&p, &dest);
                        }
                    }
                }
            }
        }
    }
    let physics = ws.join("lib").join("physics.vulcan");
    if !physics.exists() {
        let _ = std::fs::write(&physics, PHYSICS_LIB);
    }
    ws
}

fn handle_conn(mut stream: TcpStream, ws: PathBuf) -> std::io::Result<()> {
    let mut reader = BufReader::new(&mut stream);
    let mut request_line = String::new();
    if reader.read_line(&mut request_line).is_err() || request_line.is_empty() {
        return Ok(());
    }
    let mut parts = request_line.trim().split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let target = parts.next().unwrap_or("/").to_string();

    let mut headers: HashMap<String, String> = HashMap::new();
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).is_err() || line.trim().is_empty() {
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            headers.insert(k.trim().to_lowercase(), v.trim().to_string());
        }
    }
    let clen: usize = headers
        .get("content-length")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let mut body = vec![0u8; clen];
    if clen > 0 {
        let _ = reader.read_exact(&mut body);
    }

    let (path, query) = match target.split_once('?') {
        Some((p, q)) => (p.to_string(), q.to_string()),
        None => (target.clone(), String::new()),
    };

    let (status, ctype, out) = route(&method, &path, &query, &headers, &body, &ws);
    // COOP/COEP make the page cross-origin isolated, which is required for
    // SharedArrayBuffer on the main thread (the IDE input channel).
    let resp = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\nCross-Origin-Opener-Policy: same-origin\r\nCross-Origin-Embedder-Policy: require-corp\r\n\r\n",
        out.len()
    );
    stream.write_all(resp.as_bytes())?;
    stream.write_all(&out)?;
    stream.flush()?;
    Ok(())
}

fn route(
    method: &str,
    path: &str,
    query: &str,
    headers: &HashMap<String, String>,
    body: &[u8],
    ws: &PathBuf,
) -> (String, String, Vec<u8>) {
    match (method, path) {
        ("GET", "/") => ("200 OK".into(), "text/html; charset=utf-8".into(), IDE_HTML.as_bytes().to_vec()),
        ("GET", "/api/examples") => {
            let mut items = String::from("[");
            for (i, (name, src)) in EXAMPLES.iter().enumerate() {
                if i > 0 { items.push(','); }
                items.push_str(&format!(
                    "{{\"name\":\"{}\",\"source\":\"{}\"}}",
                    name,
                    json_escape(src)
                ));
            }
            items.push(']');
            ("200 OK".into(), "application/json".into(), items.into_bytes())
        }
        ("GET", "/api/files") => {
            let mut names: Vec<String> = Vec::new();
            if let Ok(entries) = std::fs::read_dir(ws) {
                for e in entries.flatten() {
                    let p = e.path();
                    if p.extension().map(|x| x == "vulcan").unwrap_or(false) {
                        if let Some(n) = p.file_name().and_then(|n| n.to_str()) {
                            names.push(n.to_string());
                        }
                    }
                }
            }
            names.sort();
            let list = names
                .iter()
                .map(|n| format!("\"{}\"", json_escape(n)))
                .collect::<Vec<_>>()
                .join(",");
            ("200 OK".into(), "application/json".into(), format!("{{\"files\":[{list}]}}").into_bytes())
        }
        ("GET", "/api/file") => {
            let name = query_param(query, "name");
            if !valid_name(&name) {
                return ("400 Bad Request".into(), "text/plain".into(), b"bad file name".to_vec());
            }
            match std::fs::read_to_string(ws.join(&name)) {
                Ok(src) => ("200 OK".into(), "text/plain; charset=utf-8".into(), src.into_bytes()),
                Err(_) => ("404 Not Found".into(), "text/plain".into(), b"not found".to_vec()),
            }
        }
        ("POST", "/api/save") => {
            let name = headers
                .get("x-vulcan-name")
                .cloned()
                .unwrap_or_else(|| "main.vulcan".into());
            if !valid_name(&name) {
                return ("400 Bad Request".into(), "text/plain".into(), b"bad file name".to_vec());
            }
            match std::fs::write(ws.join(&name), &body) {
                Ok(_) => ("200 OK".into(), "application/json".into(), b"{\"ok\":true}".to_vec()),
                Err(e) => ("500 Internal Server Error".into(), "application/json".into(),
                    format!("{{\"ok\":false,\"error\":\"{}\"}}", json_escape(&e.to_string())).into_bytes()),
            }
        }
        ("POST", "/api/compile") => {
            let name = headers
                .get("x-vulcan-name")
                .cloned()
                .unwrap_or_else(|| "main.vulcan".into());
            let name = if name.ends_with(".vulcan") { name } else { format!("{name}.vulcan") };
            let src = String::from_utf8_lossy(body).into_owned();
            match compile_source(ws, &name, &src) {
                Ok(js) => (
                    "200 OK".into(),
                    "application/json".into(),
                    format!("{{\"ok\":true,\"js\":\"{}\"}}", json_escape(&js)).into_bytes(),
                ),
                Err(e) => (
                    "200 OK".into(),
                    "application/json".into(),
                    format!("{{\"ok\":false,\"error\":\"{}\"}}", json_escape(&e)).into_bytes(),
                ),
            }
        }
        ("GET", "/favicon.ico") => ("204 No Content".into(), "image/x-icon".into(), Vec::new()),
        _ => ("404 Not Found".into(), "text/plain".into(), b"not found".to_vec()),
    }
}

// Compiles source (written to .ide/<name> so imports resolve) to JavaScript.
fn compile_source(ws: &PathBuf, name: &str, src: &str) -> Result<String, String> {
    let entry = ws.join(name);
    std::fs::write(&entry, src)
        .map_err(|e| format!("cannot write {}: {e}", entry.display()))?;
    let entry = entry.clone();
    let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
        let program = crate::modules::load_program(&entry);
        crate::codegen_js::generate_js(&program)
    }));
    match result {
        Ok(js) => Ok(js),
        Err(e) => {
            let msg = if let Some(s) = e.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = e.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "unknown compile error".to_string()
            };
            Err(msg)
        }
    }
}

fn query_param(query: &str, key: &str) -> String {
    for pair in query.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            if k == key {
                return url_decode(v);
            }
        }
    }
    String::new()
}

fn url_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
                if let Ok(v) = u8::from_str_radix(hex, 16) {
                    out.push(v);
                    i += 3;
                } else {
                    out.push(b'%');
                    i += 1;
                }
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn valid_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 100 || !name.ends_with(".vulcan") {
        return false;
    }
    name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
}

fn json_escape(s: &str) -> String {
    let mut o = String::new();
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o
}

// ----------------------------------------------------------------------------
// The IDE page (single file: markup + styles + editor + worker runtime)
// ----------------------------------------------------------------------------
const IDE_HTML: &str = r####"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Vulcan IDE</title>
<style>
  :root {
    --bg: #0f131a; --panel: #161b24; --panel2: #1b2230; --border: #263044;
    --text: #d7dde6; --dim: #7c8797; --accent: #4fc3f7; --ok: #66bb6a; --err: #ef5350;
  }
  * { box-sizing: border-box; margin: 0; padding: 0; }
  html, body { height: 100%; }
  body {
    background: var(--bg); color: var(--text); font: 14px/1.45 "Segoe UI", system-ui, sans-serif;
    display: grid; grid-template-rows: 46px 1fr; height: 100vh; overflow: hidden;
  }
  header {
    display: flex; align-items: center; gap: 10px; padding: 0 14px;
    background: var(--panel); border-bottom: 1px solid var(--border);
  }
  .logo { font-weight: 700; letter-spacing: 1px; color: var(--accent); font-size: 15px; }
  .logo .up { color: #ffb74d; }
  #fname {
    background: var(--panel2); border: 1px solid var(--border); color: var(--text);
    border-radius: 4px; padding: 5px 8px; width: 190px; font: 13px ui-monospace, monospace;
  }
  button {
    background: var(--panel2); color: var(--text); border: 1px solid var(--border);
    border-radius: 4px; padding: 6px 12px; cursor: pointer; font-size: 13px;
  }
  button:hover { border-color: var(--accent); }
  #run { background: #1b5e20; border-color: #2e7d32; font-weight: 600; }
  #run:hover { background: #2e7d32; }
  #stop { background: #b71c1c; border-color: #c62828; }
  #stop:hover { background: #c62828; }
  #status { margin-left: auto; color: var(--dim); font-size: 12px; }
  #status.err { color: var(--err); }
  main { display: grid; grid-template-columns: 210px 1fr 1fr; min-height: 0; }
  #sidebar {
    background: var(--panel); border-right: 1px solid var(--border); padding: 12px;
    overflow-y: auto; display: flex; flex-direction: column; gap: 4px;
  }
  .side-title {
    color: var(--dim); font-size: 11px; font-weight: 700; letter-spacing: 1px;
    text-transform: uppercase; margin: 10px 0 4px;
  }
  .file-btn {
    text-align: left; background: none; border: none; padding: 4px 8px; border-radius: 4px;
    font: 12px ui-monospace, monospace; color: var(--text);
  }
  .file-btn:hover { background: var(--panel2); color: var(--accent); }
  #editor-pane { display: grid; grid-template-rows: 1fr; min-width: 0; min-height: 0; border-right: 1px solid var(--border); }
  #editor-wrap { display: flex; min-height: 0; height: 100%; background: #12161d; }
  #gutter {
    color: var(--dim); text-align: right; padding: 10px 8px 10px 10px; font: 13px/1.5 ui-monospace, monospace;
    user-select: none; overflow: hidden; min-width: 44px; background: var(--panel);
    border-right: 1px solid var(--border); white-space: pre;
  }
  #editor-stack { position: relative; flex: 1; min-width: 0; }
  #hl, #src {
    position: absolute; inset: 0; font: 13px/1.5 ui-monospace, "Cascadia Mono", Menlo, monospace;
    padding: 10px 12px; white-space: pre; overflow: auto; tab-size: 4;
  }
  #hl { pointer-events: none; overflow: hidden; }
  #src {
    background: transparent; color: transparent; caret-color: #fff; border: none; outline: none;
    resize: none; word-wrap: normal; overflow-wrap: normal;
  }
  #src::selection { background: rgba(79,195,247,.28); color: transparent; }
  .c-kw { color: #ff79c6; } .c-ty { color: #8be9fd; } .c-str { color: #f1fa8c; }
  .c-num { color: #bd93f9; } .c-com { color: #5c6a80; font-style: italic; } .c-bi { color: #50fa7b; }
  #right { display: grid; grid-template-rows: auto 1fr; min-height: 0; min-width: 0; }
  #canvas-bar { background: var(--panel); border-bottom: 1px solid var(--border); padding: 8px 12px; display: flex; align-items: center; gap: 10px; }
  #cv {
    display: block; background: #000; width: 100%; max-width: 512px; aspect-ratio: 16 / 9;
    image-rendering: pixelated; border: 1px solid var(--border); border-radius: 4px;
    margin: 10px auto;
  }
  #fps { color: var(--dim); font-size: 12px; min-width: 64px; }
  #console {
    background: #12161d; border-top: 1px solid var(--border); padding: 10px 12px; overflow-y: auto;
    font: 12px/1.5 ui-monospace, monospace; white-space: pre-wrap; word-break: break-all;
  }
  #console .line-out { color: #cfd8dc; }
  #console .line-err { color: var(--err); }
  #console .line-sys { color: var(--dim); }
</style>
</head>
<body>
<header>
  <span class="logo"><span class="up">&#9650;</span> VULCAN IDE</span>
  <input id="fname" value="main.vulcan" spellcheck="false">
  <button id="new">New</button>
  <button id="save">Save</button>
  <button id="run" title="Ctrl+Enter">&#9654; Run</button>
  <button id="stop" disabled>&#9632; Stop</button>
  <span id="status">ready</span>
</header>
<main>
  <nav id="sidebar">
    <div class="side-title">Examples</div>
    <div id="examples"></div>
    <div class="side-title">My files (.ide/)</div>
    <div id="files"></div>
  </nav>
  <section id="editor-pane">
    <div id="editor-wrap">
      <div id="gutter">1</div>
      <div id="editor-stack">
        <pre id="hl" aria-hidden="true"></pre>
        <textarea id="src" spellcheck="false" wrap="off"></textarea>
      </div>
    </div>
  </section>
  <section id="right">
    <div>
      <canvas id="cv" width="256" height="144"></canvas>
      <div id="canvas-bar"><span id="fps">fps: -</span><span id="runinfo" style="color:var(--dim);font-size:12px"></span></div>
    </div>
    <div id="console"></div>
  </section>
</main>
<script>
"use strict";
const $ = (id) => document.getElementById(id);
const srcEl = $("src"), hlEl = $("hl"), gutterEl = $("gutter");
const consoleEl = $("console"), statusEl = $("status");
const cv = $("cv"), ctx = cv.getContext("2d");

// ---------- syntax highlighting ----------
const KEYWORDS = new Set(["fn","let","struct","impl","enum","if","elif","else","for","while","return","import","pub","break","continue","defer","as","const","true","false"]);
const TYPES = new Set(["i64","f64","bool","string","void"]);
const BUILTINS = new Set(["print","len","sqrt","sin","cos","pow","floor","ceil","abs","min","max","clock","ms_now","sleep_ms","exit","read_key","key_ready","read_line","clear_screen","set_color","print_at","screen_width","screen_height","cursor_visible","canvas_width","canvas_height","clear","draw_rect","draw_circle","draw_line","draw_text","present","mouse_x","mouse_y","mouse_down","key_down","file_read","file_write","file_exists","args","arg_count","to_string","to_int","to_float","rand_int","repeat","find","split","push","pop","shift","contains","join","substr","trim","upper","lower","char_at","insert","remove","last","reverse","sort"]);
const esc = (s) => s.replace(/&/g,"&amp;").replace(/</g,"&lt;").replace(/>/g,"&gt;");
const span = (cls, s) => '<span class="' + cls + '">' + esc(s) + "</span>";
const TOKEN_RE = /(\/\/[^\n]*)|(f?"(?:[^"\\]|\\.)*")|(\b\d+(?:\.\d+)?\b)|([A-Za-z_][A-Za-z0-9_]*)|([\s\S])/g;
function highlight(text) {
  let out = "", m;
  TOKEN_RE.lastIndex = 0;
  while ((m = TOKEN_RE.exec(text)) !== null) {
    if (m[1]) out += span("c-com", m[1]);
    else if (m[2]) out += span("c-str", m[2]);
    else if (m[3]) out += span("c-num", m[3]);
    else if (m[4]) {
      if (KEYWORDS.has(m[4])) out += span("c-kw", m[4]);
      else if (TYPES.has(m[4])) out += span("c-ty", m[4]);
      else if (BUILTINS.has(m[4])) out += span("c-bi", m[4]);
      else out += esc(m[4]);
    } else out += esc(m[5]);
  }
  return out + "\n";
}
function refreshEditor() {
  hlEl.innerHTML = highlight(srcEl.value);
  const lines = srcEl.value.split("\n").length;
  let g = "";
  for (let i = 1; i <= lines; i++) g += i + "\n";
  gutterEl.textContent = g;
}
srcEl.addEventListener("input", refreshEditor);
srcEl.addEventListener("scroll", () => {
  hlEl.scrollTop = srcEl.scrollTop; hlEl.scrollLeft = srcEl.scrollLeft;
  gutterEl.scrollTop = srcEl.scrollTop;
});
srcEl.addEventListener("keydown", (e) => {
  if (e.key === "Tab") {
    e.preventDefault();
    srcEl.setRangeText("    ", srcEl.selectionStart, srcEl.selectionEnd, "end");
    refreshEditor();
  }
  if ((e.ctrlKey || e.metaKey) && e.key === "Enter") { e.preventDefault(); run(); }
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") { e.preventDefault(); save(); }
});

// ---------- console ----------
function logLine(text, cls) {
  const div = document.createElement("div");
  div.className = "line-" + cls;
  div.textContent = text;
  consoleEl.appendChild(div);
  while (consoleEl.childNodes.length > 500) consoleEl.removeChild(consoleEl.firstChild);
  consoleEl.scrollTop = consoleEl.scrollHeight;
}

// ---------- files / examples ----------
async function api(path, opts) {
  const r = await fetch(path, opts);
  if (!r.ok) throw new Error(await r.text());
  return r;
}
async function loadExamples() {
  const r = await api("/api/examples");
  const data = await r.json();
  const box = $("examples");
  for (const ex of data) {
    const b = document.createElement("button");
    b.className = "file-btn"; b.textContent = ex.name;
    b.onclick = () => { $("fname").value = ex.name; srcEl.value = ex.source; refreshEditor(); };
    box.appendChild(b);
  }
}
async function loadFiles() {
  const r = await api("/api/files");
  const data = await r.json();
  const box = $("files");
  box.textContent = "";
  for (const name of data.files) {
    const b = document.createElement("button");
    b.className = "file-btn"; b.textContent = name;
    b.onclick = async () => {
      const f = await api("/api/file?name=" + encodeURIComponent(name));
      $("fname").value = name; srcEl.value = await f.text(); refreshEditor();
    };
    box.appendChild(b);
  }
}
async function save() {
  const name = $("fname").value.trim() || "main.vulcan";
  await api("/api/save", {
    method: "POST",
    headers: { "X-Vulcan-Name": name },
    body: srcEl.value
  });
  statusEl.textContent = "saved " + name; statusEl.className = "";
  loadFiles();
}

// ---------- input (mouse + keyboard via SharedArrayBuffer) ----------
let input;
let inputShared = false;
try {
  input = new Int32Array(new SharedArrayBuffer(4 * 300));
  inputShared = true;
} catch (e) {
  input = new Int32Array(300); // no isolation: program runs, but no live input
}
function canvasPos(e) {
  const r = cv.getBoundingClientRect();
  return [ (e.clientX - r.left) / r.width * 256, (e.clientY - r.top) / r.height * 144 ];
}
cv.addEventListener("mousemove", (e) => {
  const [x, y] = canvasPos(e);
  input[0] = Math.round(x * 100); input[1] = Math.round(y * 100);
});
cv.addEventListener("mousedown", (e) => { input[2] = 1; e.preventDefault(); });
window.addEventListener("mouseup", () => { input[2] = 0; });
cv.addEventListener("mouseleave", () => { input[2] = 0; });
const KEYMAP = { ArrowLeft: 37, ArrowUp: 38, ArrowRight: 39, ArrowDown: 40, Space: 32, Enter: 13, Shift: 16, Control: 17, Alt: 18, Tab: 9, Backspace: 8, Escape: 27, Delete: 46 };
function keyCode(e) {
  if (e.key.length === 1) return e.key.charCodeAt(0);
  return KEYMAP[e.key] || 0;
}
window.addEventListener("keydown", (e) => {
  const c = keyCode(e);
  if (c && running) {
    if (e.key === " " || e.key.startsWith("Arrow")) e.preventDefault();
    input[3 + c] = 1;
  }
});
window.addEventListener("keyup", (e) => {
  const c = keyCode(e);
  if (c) input[3 + c] = 0;
});

// ---------- frame rendering ----------
let frames = 0, lastFps = performance.now(), fpsVal = 0;
function drawFrame(buf, w, h) {
  const view = new Int32Array(buf);
  const img = ctx.createImageData(w, h);
  const px = img.data;
  for (let i = 0; i < w * h; i++) {
    const v = view[i];
    px[i * 4] = (v >> 16) & 255; px[i * 4 + 1] = (v >> 8) & 255;
    px[i * 4 + 2] = v & 255; px[i * 4 + 3] = 255;
  }
  ctx.putImageData(img, 0, 0);
  frames++;
  const now = performance.now();
  if (now - lastFps >= 1000) {
    fpsVal = Math.round(frames * 1000 / (now - lastFps));
    $("fps").textContent = "fps: " + fpsVal;
    frames = 0; lastFps = now;
  }
}

// ---------- worker runtime ----------
// The generated program expects `require("fs")`, `process.stdout.write` and
// `process.exit`; we shim them, then run the compiled code. `__present()`
// detects the worker environment and posts frames instead of printing ANSI.
function workerMain(e) {
  const msg = e.data;
  if (!msg || msg.type !== "run") return;
  if (msg.sab) self.__VULCAN_INPUT = new Int32Array(msg.sab);
  const shim =
    'const process = { argv: ["vulcan"], ' +
    'stdout: { write: (s) => postMessage({ type: "out", s: String(s) }) }, ' +
    'stderr: { write: (s) => postMessage({ type: "err", s: String(s) }) }, ' +
    'exit: (c) => { throw { __vulcan_exit: true, code: c }; } };\n' +
    'const require = (n) => ({ ' +
    'readFileSync: () => { throw new Error("file I/O is not available in the IDE sandbox"); }, ' +
    'writeFileSync: () => { throw new Error("file I/O is not available in the IDE sandbox"); }, ' +
    'existsSync: () => false });';
  try {
    const fn = new Function('"use strict";\n' + shim + "\n" + msg.code);
    fn();
  } catch (err) {
    if (!(err && err.__vulcan_exit)) {
      postMessage({ type: "err", s: String((err && err.stack) || err) });
    }
  }
  postMessage({ type: "done" });
}
let worker = null, running = false;
function makeWorker() {
  const blob = new Blob(["self.onmessage = " + workerMain.toString() + ";"],
                       { type: "application/javascript" });
  const w = new Worker(URL.createObjectURL(blob));
  w.onmessage = (e) => {
    const d = e.data;
    if (d.__vulcan_frame !== undefined) { drawFrame(d.__vulcan_frame, d.w, d.h); return; }
    if (d.type === "out") logLine(d.s, "out");
    else if (d.type === "err") logLine(d.s, "err");
    else if (d.type === "done") stopRun();
  };
  w.onerror = (e) => { logLine("worker error: " + e.message, "err"); stopRun(); };
  return w;
}
function stopRun() {
  running = false;
  $("stop").disabled = true; $("run").disabled = false;
  $("runinfo").textContent = "stopped";
  if (worker) { worker.terminate(); worker = null; }
}
async function run() {
  if (running) return;
  statusEl.textContent = "compiling..."; statusEl.className = "";
  let data;
  try {
    const name = $("fname").value.trim() || "main.vulcan";
    const r = await api("/api/compile", {
      method: "POST",
      headers: { "X-Vulcan-Name": name },
      body: srcEl.value
    });
    data = await r.json();
  } catch (e) {
    statusEl.textContent = "compile failed: " + e.message; statusEl.className = "err";
    return;
  }
  if (!data.ok) {
    statusEl.textContent = "compile error"; statusEl.className = "err";
    logLine(String(data.error), "err");
    return;
  }
  statusEl.textContent = "running"; statusEl.className = "";
  consoleEl.textContent = "";
  ctx.fillStyle = "#000"; ctx.fillRect(0, 0, 256, 144);
  worker = makeWorker();
  running = true;
  $("stop").disabled = false; $("run").disabled = true;
  worker.postMessage({ type: "run", code: data.js, sab: inputShared ? input.buffer : null });
  const t0 = performance.now();
  $("runinfo").textContent = "running";
}
$("run").onclick = run;
$("stop").onclick = stopRun;
$("save").onclick = save;
$("new").onclick = () => {
  $("fname").value = "new" + (Math.floor(Math.random() * 9000) + 1000) + ".vulcan";
  srcEl.value = "fn main() -> void {\n    print(\"hello from \" + \"vulcan\")\n}\n";
  refreshEditor();
};

// boot: load the bouncing-balls example into the editor
(async function boot() {
  try {
    const r = await api("/api/examples");
    const data = await r.json();
    const ex = data.find((x) => x.name === "bounce.vulcan") || data[0];
    if (ex) { $("fname").value = ex.name; srcEl.value = ex.source; }
  } catch (e) { /* offline fallback */ }
  refreshEditor();
  loadExamples();
  loadFiles();
  logLine("Vulcan IDE ready. Ctrl+Enter runs your program.", "sys");
})();
</script>
</body>
</html>
"####;
