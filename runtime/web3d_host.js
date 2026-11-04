// Vulcan web3d host runtime (v0.6.0)
// Provides the gfx_* / hud_* / net_* / snd / file_list builtins to programs compiled
// with `vulcan package <file> --platform web3d`. WebGL2 renderer with a shader-pack
// pipeline: shadow pass -> sky -> opaque -> transparent -> post chain (post0..postN).
"use strict";
(function () {
  const H = {};
  globalThis.__VULCAN_HOST__ = H;
  const $ = (id) => document.getElementById(id);

  // ------------------------------------------------------------------ virtual files
  const vfs = new Map();
  const ASSETS = globalThis.__VULCAN_ASSETS__ || {};
  for (const k in ASSETS) vfs.set(k, ASSETS[k]);
  try {
    for (let i = 0; i < localStorage.length; i++) {
      const k = localStorage.key(i);
      if (k.startsWith("vc:")) vfs.set(k.slice(3), localStorage.getItem(k));
    }
  } catch (e) { /* storage blocked */ }
  function persist(path, contents) {
    if (path.startsWith("save/") || path.startsWith("user/")) {
      try { localStorage.setItem("vc:" + path, contents); } catch (e) { /* quota */ }
    }
  }
  globalThis.require = (m) => {
    if (m === "fs") return {
      readFileSync: (p) => vfs.has(p) ? vfs.get(p) : "",
      writeFileSync: (p, c) => { vfs.set(p, c); persist(p, c); },
      existsSync: (p) => vfs.has(p),
    };
    throw new Error("module '" + m + "' is not available in the browser");
  };
  let stdoutBuf = "";
  globalThis.process = {
    argv: ["vulcan", "app"],
    stdout: { columns: 100, rows: 30, write: (s) => { stdoutBuf += s; let i; while ((i = stdoutBuf.indexOf("\n")) >= 0) { console.log(stdoutBuf.slice(0, i)); stdoutBuf = stdoutBuf.slice(i + 1); } } },
    stderr: { write: (s) => { console.error(s); showError(String(s)); } },
    exit: (c) => { throw new Error("__vulcan_exit " + c); },
  };
  H.file_list = (prefix) => { const out = []; for (const k of vfs.keys()) if (k.startsWith(prefix)) out.push(k); out.sort(); return out; };
  H.host_store = (k, v) => { vfs.set(k, v); persist(k, v); };

  function showError(msg) {
    const el = $("err"); if (!el) return;
    el.style.display = "block"; el.textContent = msg;
  }
  window.addEventListener("error", (e) => showError("error: " + (e.message || e.error)));
  window.addEventListener("unhandledrejection", (e) => { if (String(e.reason).indexOf("__vulcan_exit") < 0) showError("error: " + (e.reason && e.reason.stack || e.reason)); });

  // ------------------------------------------------------------------ math
  const M = {
    ident() { const m = new Float32Array(16); m[0] = m[5] = m[10] = m[15] = 1; return m; },
    mul(a, b) { // a * b (column-major)
      const o = new Float32Array(16);
      for (let c = 0; c < 4; c++) for (let r = 0; r < 4; r++) {
        o[c * 4 + r] = a[r] * b[c * 4] + a[4 + r] * b[c * 4 + 1] + a[8 + r] * b[c * 4 + 2] + a[12 + r] * b[c * 4 + 3];
      }
      return o;
    },
    persp(fovDeg, aspect, n, f) {
      const t = 1 / Math.tan(fovDeg * Math.PI / 360), o = new Float32Array(16);
      o[0] = t / aspect; o[5] = t; o[10] = (f + n) / (n - f); o[11] = -1; o[14] = 2 * f * n / (n - f);
      return o;
    },
    ortho(l, r, b, t, n, f) {
      const o = new Float32Array(16);
      o[0] = 2 / (r - l); o[5] = 2 / (t - b); o[10] = -2 / (f - n);
      o[12] = -(r + l) / (r - l); o[13] = -(t + b) / (t - b); o[14] = -(f + n) / (f - n); o[15] = 1;
      return o;
    },
    lookAt(e, c, up) {
      let zx = e[0] - c[0], zy = e[1] - c[1], zz = e[2] - c[2];
      let l = Math.hypot(zx, zy, zz) || 1; zx /= l; zy /= l; zz /= l;
      let xx = up[1] * zz - up[2] * zy, xy = up[2] * zx - up[0] * zz, xz = up[0] * zy - up[1] * zx;
      l = Math.hypot(xx, xy, xz) || 1; xx /= l; xy /= l; xz /= l;
      const yx = zy * xz - zz * xy, yy = zz * xx - zx * xz, yz = zx * xy - zy * xx;
      const o = new Float32Array(16);
      o[0] = xx; o[1] = yx; o[2] = zx; o[4] = xy; o[5] = yy; o[6] = zy; o[8] = xz; o[9] = yz; o[10] = zz;
      o[12] = -(xx * e[0] + xy * e[1] + xz * e[2]);
      o[13] = -(yx * e[0] + yy * e[1] + yz * e[2]);
      o[14] = -(zx * e[0] + zy * e[1] + zz * e[2]);
      o[15] = 1;
      return o;
    },
    inv(m) {
      const a = m, o = new Float32Array(16);
      const b00 = a[0] * a[5] - a[1] * a[4], b01 = a[0] * a[6] - a[2] * a[4], b02 = a[0] * a[7] - a[3] * a[4];
      const b03 = a[1] * a[6] - a[2] * a[5], b04 = a[1] * a[7] - a[3] * a[5], b05 = a[2] * a[7] - a[3] * a[6];
      const b06 = a[8] * a[13] - a[9] * a[12], b07 = a[8] * a[14] - a[10] * a[12], b08 = a[8] * a[15] - a[11] * a[12];
      const b09 = a[9] * a[14] - a[10] * a[13], b10 = a[9] * a[15] - a[11] * a[13], b11 = a[10] * a[15] - a[11] * a[14];
      let d = b00 * b11 - b01 * b10 + b02 * b09 + b03 * b08 - b04 * b07 + b05 * b06;
      d = d ? 1 / d : 0;
      o[0] = (a[5] * b11 - a[6] * b10 + a[7] * b09) * d; o[1] = (a[2] * b10 - a[1] * b11 - a[3] * b09) * d;
      o[2] = (a[13] * b05 - a[14] * b04 + a[15] * b03) * d; o[3] = (a[10] * b04 - a[9] * b05 - a[11] * b03) * d;
      o[4] = (a[6] * b08 - a[4] * b11 - a[7] * b07) * d; o[5] = (a[0] * b11 - a[2] * b08 + a[3] * b07) * d;
      o[6] = (a[14] * b02 - a[12] * b05 - a[15] * b01) * d; o[7] = (a[8] * b05 - a[10] * b02 + a[11] * b01) * d;
      o[8] = (a[4] * b10 - a[5] * b08 + a[7] * b06) * d; o[9] = (a[1] * b08 - a[0] * b10 - a[3] * b06) * d;
      o[10] = (a[12] * b04 - a[13] * b02 + a[15] * b00) * d; o[11] = (a[9] * b02 - a[8] * b04 - a[11] * b00) * d;
      o[12] = (a[5] * b07 - a[4] * b09 - a[6] * b06) * d; o[13] = (a[0] * b09 - a[1] * b07 + a[2] * b06) * d;
      o[14] = (a[13] * b01 - a[12] * b03 - a[14] * b00) * d; o[15] = (a[8] * b03 - a[9] * b01 + a[10] * b00) * d;
      return o;
    },
    xform(m, x, y, z) {
      const w = m[3] * x + m[7] * y + m[11] * z + m[15];
      return [m[0] * x + m[4] * y + m[8] * z + m[12], m[1] * x + m[5] * y + m[9] * z + m[13], m[2] * x + m[6] * y + m[10] * z + m[14], w];
    },
    planes(m) { // Gribb-Hartmann frustum planes (normalised)
      const p = [];
      const row = (i) => [m[i], m[4 + i], m[8 + i], m[12 + i]];
      const r0 = row(0), r1 = row(1), r2 = row(2), r3 = row(3);
      const add = (a, s) => { const q = [a[0] + s * 0, 0, 0, 0]; return q; };
      const comb = (s, r) => [r3[0] + s * r[0], r3[1] + s * r[1], r3[2] + s * r[2], r3[3] + s * r[3]];
      for (const pl of [comb(1, r0), comb(-1, r0), comb(1, r1), comb(-1, r1), comb(1, r2), comb(-1, r2)]) {
        const l = Math.hypot(pl[0], pl[1], pl[2]) || 1;
        p.push([pl[0] / l, pl[1] / l, pl[2] / l, pl[3] / l]);
      }
      return p;
    },
  };

  // ------------------------------------------------------------------ state
  let gl = null, glc = null, hudc = null, hctx = null;
  let W = 1, Hh = 1, dpr = 1, renderScale = 1, uiK = 1;
  const meshes = new Map();
  const uniformsV = new Map();           // name -> Float32Array (game-set uniforms)
  const progs = new Map();               // slot -> {prog, locs, kind, scale}
  let atlas = null, dummyTex = null, dummyDepth = null;
  let cam = { x: 0, y: 0, z: 0, yaw: 0, pitch: 0, fov: 70 };
  let shadowArea = 70, shadowSize = 2048, shadowRT = null;
  let rts = { key: "", scene: null, passes: new Map() };
  let hudCmds = [];
  let frameT = 0, lastT = 0, hasHalf = false;
  let stats = "";
  let sceneMats = null;                  // last computed matrices

  const FLAG = { };

  // ------------------------------------------------------------------ gfx_open
  H.gfx_open = (title) => {
    document.title = title;
    glc = $("gl"); hudc = $("hud");
    gl = glc.getContext("webgl2", { antialias: false, alpha: false, powerPreference: "high-performance", preserveDrawingBuffer: false });
    if (!gl) { showError("WebGL2 is not available in this browser."); throw new Error("no webgl2"); }
    hasHalf = !!gl.getExtension("EXT_color_buffer_float") || !!gl.getExtension("EXT_color_buffer_half_float");
    gl.getExtension("OES_texture_float_linear");
    hctx = hudc.getContext("2d");
    dummyTex = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D, dummyTex);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, 1, 1, 0, gl.RGBA, gl.UNSIGNED_BYTE, new Uint8Array([255, 255, 255, 255]));
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
    initInput();
    applyQuality(quality);
    resize();
    window.addEventListener("resize", resize);
    lastT = performance.now();
  };

  function resize() {
    dpr = Math.min(window.devicePixelRatio || 1, 1.5);
    const cw = Math.max(2, Math.floor(window.innerWidth)), ch = Math.max(2, Math.floor(window.innerHeight));
    W = Math.floor(cw * dpr); Hh = Math.floor(ch * dpr);
    glc.width = W; glc.height = Hh; glc.style.width = cw + "px"; glc.style.height = ch + "px";
    const hd = Math.min(window.devicePixelRatio || 1, 2);
    hudc.width = Math.floor(cw * hd); hudc.height = Math.floor(ch * hd);
    hudc.style.width = cw + "px"; hudc.style.height = ch + "px";
    uiK = Math.max(0.5, Math.min(1, cw / 560, ch / 470));
    hudc._k = hd; hudc._cw = cw / uiK; hudc._ch = ch / uiK; hudc._rw = cw; hudc._rh = ch;
    layoutTouch();
  }
  H.gfx_size_w = () => hudc ? hudc._cw : window.innerWidth;
  H.gfx_size_h = () => hudc ? hudc._ch : window.innerHeight;
  // Quality: "auto" adapts the render resolution to keep ~60 fps; low/medium/high are fixed.
  let userScale = 1, qScale = 1, quality = "auto", drs = 1, emaDt = 0.016, drsFrames = 0, drsGood = 0, drsOn = true;
  try { quality = localStorage.getItem("vcquality") || "auto"; } catch (e) { /* ignore */ }
  const QBASE = { low: 0.6, medium: 0.8, high: 1.0 };
  function recomputeScale() {
    qScale = quality === "auto" ? drs : QBASE[quality] || 1;
    renderScale = Math.max(0.25, Math.min(2, userScale * qScale));
  }
  function qualityShadowSize() { return quality === "high" || (quality === "auto" && !isTouch) ? 2048 : 1024; }
  function applyQuality(q) {
    quality = q; try { localStorage.setItem("vcquality", q); } catch (e) { /* ignore */ }
    if (q === "auto") drs = isTouch ? 0.8 : 1;
    recomputeScale();
    const ss = qualityShadowSize();
    if (ss !== shadowSize) { shadowSize = ss; if (shadowRT && gl) { gl.deleteTexture(shadowRT.dtex); gl.deleteFramebuffer(shadowRT.fb); } shadowRT = null; rts.key = ""; }
  }
  function drsTick(dt) {
    drsFrames++;
    if (quality !== "auto" || drsFrames < 150 || document.hidden) return;
    emaDt = emaDt * 0.92 + dt * 0.08;
    if (drsFrames % 40 !== 0) return;
    if (emaDt > 0.0245 && drs > 0.5) { drs = Math.max(0.5, drs - 0.1); drsGood = 0; recomputeScale(); }
    else if (emaDt < 0.0185 && drs < 1) { if (++drsGood >= 4) { drs = Math.min(1, drs + 0.1); drsGood = 0; recomputeScale(); } }
    else drsGood = 0;
  }
  H.gfx_scale = (s) => { userScale = Math.max(0.25, Math.min(2, s)); recomputeScale(); };
  H.gfx_shadow_area = (a) => { shadowArea = Math.max(8, a); };
  H.gfx_info = () => { const e = gl.getExtension("WEBGL_debug_renderer_info"); return (e ? gl.getParameter(e.UNMASKED_RENDERER_WEBGL) : gl.getParameter(gl.RENDERER)) + (hasHalf ? " (HDR)" : " (LDR)"); };
  H.gfx_stats = () => stats;
  H.gfx_fullscreen = () => { if (document.fullscreenElement) document.exitFullscreen(); else document.documentElement.requestFullscreen && document.documentElement.requestFullscreen(); };
  H.gfx_chrome = (on) => { const b = $("bar"); if (b) b.style.display = on ? "" : "none"; if (matchUI === !on) return; setTouchPlay(!on); };

  // ------------------------------------------------------------------ shaders
  const UNIFORMS_DECL = `
uniform mat4 uViewProj; uniform mat4 uView; uniform mat4 uProj; uniform mat4 uInvViewProj; uniform mat4 uShadowMat;
uniform vec3 uCamPos; uniform vec2 uResolution; uniform float uTime; uniform float uNear; uniform float uFar;
uniform vec3 uOrigin; uniform float uYaw; uniform int uLayer; uniform vec3 uSunScreen; uniform int uShadowOn; uniform float uShadowTexel;
uniform vec3 uSunDir; uniform vec3 uSunColor; uniform vec3 uSkyTop; uniform vec3 uSkyHorizon; uniform vec3 uAmbient; uniform float uFog; uniform float uDay;
uniform sampler2D uAtlas; uniform sampler2D uScene; uniform sampler2D uDepth; uniform sampler2D uColor; uniform sampler2D uShadow;
uniform sampler2D uPass0; uniform sampler2D uPass1; uniform sampler2D uPass2; uniform sampler2D uPass3;
`;
  const HEAD = "#version 300 es\nprecision highp float;\nprecision highp int;\nprecision highp sampler2D;\n" + UNIFORMS_DECL;
  const HEAD_LINES = HEAD.split("\n").length - 1;
  const VS_ATTR = "layout(location=0) in vec3 aPos; layout(location=1) in vec3 aNormal; layout(location=2) in vec2 aUV; layout(location=3) in vec3 aLight; layout(location=4) in float aMat;\n";
  const FS_OUT = "layout(location=0) out vec4 fragColor;\n";
  const FULL_VS = HEAD + "out vec2 vUV;\nvoid main(){ vec2 p = vec2(float((gl_VertexID<<1)&2), float(gl_VertexID&2)); vUV = p; gl_Position = vec4(p*2.0-1.0, 0.0, 1.0); }\n";
  const BLIT_FS = HEAD + FS_OUT + "in vec2 vUV;\nvoid main(){ vec3 c = texture(uColor, vUV).rgb; c = c/(1.0+c*0.35); fragColor = vec4(pow(c, vec3(1.0/2.2)), 1.0); }\n";
  const names = ["uViewProj", "uView", "uProj", "uInvViewProj", "uShadowMat", "uCamPos", "uResolution", "uTime", "uNear", "uFar", "uOrigin", "uYaw", "uLayer", "uSunScreen", "uShadowOn", "uShadowTexel",
    "uSunDir", "uSunColor", "uSkyTop", "uSkyHorizon", "uAmbient", "uFog", "uDay", "uAtlas", "uScene", "uDepth", "uColor", "uShadow", "uPass0", "uPass1", "uPass2", "uPass3"];
  const UNIT = { uAtlas: 0, uScene: 1, uDepth: 2, uColor: 3, uShadow: 4, uPass0: 5, uPass1: 6, uPass2: 7, uPass3: 8 };

  function compile(type, src) {
    const s = gl.createShader(type);
    gl.shaderSource(s, src); gl.compileShader(s);
    if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) {
      const log = gl.getShaderInfoLog(s) || "compile error";
      gl.deleteShader(s);
      return { err: log };
    }
    return { s };
  }
  function buildProgram(vsrc, fsrc, extra) {
    const vs = compile(gl.VERTEX_SHADER, vsrc); if (vs.err) return { err: "vertex: " + vs.err };
    const fs = compile(gl.FRAGMENT_SHADER, fsrc); if (fs.err) return { err: "fragment: " + fs.err };
    const p = gl.createProgram();
    gl.attachShader(p, vs.s); gl.attachShader(p, fs.s); gl.linkProgram(p);
    gl.deleteShader(vs.s); gl.deleteShader(fs.s);
    if (!gl.getProgramParameter(p, gl.LINK_STATUS)) return { err: "link: " + gl.getProgramInfoLog(p) };
    const locs = {};
    for (const n of names) { const l = gl.getUniformLocation(p, n); if (l) locs[n] = l; }
    return { prog: p, locs };
  }
  H.gfx_shader_clear = () => { for (const v of progs.values()) gl.deleteProgram(v.prog); progs.clear(); rts.key = ""; };
  H.gfx_shader = (slot, vs, fs) => {
    const m = /^(scene|shadow|sky|post(\d+))(?:@([\d.]+))?$/.exec(slot);
    if (!m) return "unknown shader slot '" + slot + "' (scene, shadow, sky, post0..postN[@scale])";
    const kind = m[1].startsWith("post") ? "post" : m[1];
    const key = m[1];
    const scale = m[3] ? parseFloat(m[3]) : 1;
    let vsrc, fsrc;
    if (kind === "scene" || kind === "shadow") {
      vsrc = HEAD + VS_ATTR + vs; fsrc = HEAD + FS_OUT + fs;
    } else {
      vsrc = vs && vs.trim() ? HEAD + vs : FULL_VS;
      fsrc = HEAD + FS_OUT + "in vec2 vUV;\n" + fs;
    }
    const r = buildProgram(vsrc, fsrc);
    if (r.err) return slot + ": " + r.err.replace(/ERROR: 0:(\d+)/g, (_, n) => "line " + (parseInt(n) - HEAD_LINES - (kind === "scene" || kind === "shadow" ? 1 : 0)));
    if (progs.has(key)) gl.deleteProgram(progs.get(key).prog);
    progs.set(key, { prog: r.prog, locs: r.locs, kind, scale, order: kind === "post" ? parseInt(m[2]) : 0 });
    rts.key = "";
    return "";
  };

  // ------------------------------------------------------------------ uniforms
  H.gfx_uniform = (name, vals) => { uniformsV.set(name, Float32Array.from(vals)); };
  H.gfx_camera = (x, y, z, yaw, pitch, fov) => { cam = { x, y, z, yaw, pitch, fov }; };

  function setU(P, name, v) {
    const l = P.locs[name]; if (!l) return;
    switch (v.length) {
      case 1: gl.uniform1f(l, v[0]); break;
      case 2: gl.uniform2fv(l, v); break;
      case 3: gl.uniform3fv(l, v); break;
      case 4: gl.uniform4fv(l, v); break;
    }
  }
  function applyCommon(P, mats, extra) {
    for (const [n, v] of uniformsV) setU(P, n, v);
    const L = P.locs;
    if (L.uViewProj) gl.uniformMatrix4fv(L.uViewProj, false, mats.viewProj);
    if (L.uView) gl.uniformMatrix4fv(L.uView, false, mats.view);
    if (L.uProj) gl.uniformMatrix4fv(L.uProj, false, mats.proj);
    if (L.uInvViewProj) gl.uniformMatrix4fv(L.uInvViewProj, false, mats.invViewProj);
    if (L.uShadowMat) gl.uniformMatrix4fv(L.uShadowMat, false, mats.shadowMat);
    if (L.uCamPos) gl.uniform3f(L.uCamPos, cam.x, cam.y, cam.z);
    if (L.uTime) gl.uniform1f(L.uTime, frameT);
    if (L.uNear) gl.uniform1f(L.uNear, 0.1);
    if (L.uFar) gl.uniform1f(L.uFar, 600);
    if (L.uSunScreen) gl.uniform3fv(L.uSunScreen, mats.sunScreen);
    if (L.uShadowOn) gl.uniform1i(L.uShadowOn, mats.shadowOn ? 1 : 0);
    if (L.uShadowTexel) gl.uniform1f(L.uShadowTexel, 1 / shadowSize);
    if (L.uResolution) gl.uniform2f(L.uResolution, extra.w, extra.h);
    for (const s in UNIT) if (L[s]) gl.uniform1i(L[s], UNIT[s]);
  }

  // ------------------------------------------------------------------ textures / render targets
  H.gfx_texture = (w, h, rgba) => {
    if (atlas) gl.deleteTexture(atlas);
    atlas = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D, atlas);
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, w, h, 0, gl.RGBA, gl.UNSIGNED_BYTE, Uint8Array.from(rgba));
    gl.generateMipmap(gl.TEXTURE_2D);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST_MIPMAP_LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
  };
  function makeColorRT(w, h, depth) {
    const fb = gl.createFramebuffer();
    gl.bindFramebuffer(gl.FRAMEBUFFER, fb);
    const tex = gl.createTexture();
    gl.bindTexture(gl.TEXTURE_2D, tex);
    if (hasHalf) gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA16F, w, h, 0, gl.RGBA, gl.HALF_FLOAT, null);
    else gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA8, w, h, 0, gl.RGBA, gl.UNSIGNED_BYTE, null);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, tex, 0);
    let dtex = null;
    if (depth) {
      dtex = gl.createTexture();
      gl.bindTexture(gl.TEXTURE_2D, dtex);
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.DEPTH_COMPONENT24, w, h, 0, gl.DEPTH_COMPONENT, gl.UNSIGNED_INT, null);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
      gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.DEPTH_ATTACHMENT, gl.TEXTURE_2D, dtex, 0);
    }
    return { fb, tex, dtex, w, h };
  }
  function ensureTargets() {
    const sw = Math.max(2, Math.floor(W * renderScale)), sh = Math.max(2, Math.floor(Hh * renderScale));
    const key = sw + "x" + sh + "|" + [...progs.keys()].sort().join(",") + "|" + [...progs.values()].map((p) => p.scale).join(",");
    if (rts.key === key) return;
    // free old
    const free = (t) => { if (!t) return; gl.deleteFramebuffer(t.fb); gl.deleteTexture(t.tex); if (t.dtex) gl.deleteTexture(t.dtex); };
    free(rts.scene); for (const t of rts.passes.values()) free(t);
    rts = { key, scene: makeColorRT(sw, sh, true), passes: new Map() };
    for (const [k, p] of progs) if (p.kind === "post") {
      rts.passes.set(k, makeColorRT(Math.max(2, Math.floor(sw * p.scale)), Math.max(2, Math.floor(sh * p.scale)), false));
    }
    if (progs.has("shadow") && !shadowRT) {
      const fb = gl.createFramebuffer();
      gl.bindFramebuffer(gl.FRAMEBUFFER, fb);
      const dt = gl.createTexture();
      gl.bindTexture(gl.TEXTURE_2D, dt);
      gl.texImage2D(gl.TEXTURE_2D, 0, gl.DEPTH_COMPONENT24, shadowSize, shadowSize, 0, gl.DEPTH_COMPONENT, gl.UNSIGNED_INT, null);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
      gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.DEPTH_ATTACHMENT, gl.TEXTURE_2D, dt, 0);
      gl.drawBuffers([gl.NONE]); gl.readBuffer(gl.NONE);
      shadowRT = { fb, dtex: dt, w: shadowSize, h: shadowSize };
    }
  }

  // ------------------------------------------------------------------ meshes
  const STRIDE = 12;
  H.gfx_mesh_set = (id, layer, data) => {
    let m = meshes.get(id);
    if (!data.length) { if (m) H.gfx_mesh_free(id); return; }
    if (!m) {
      const vao = gl.createVertexArray(), vbo = gl.createBuffer();
      gl.bindVertexArray(vao); gl.bindBuffer(gl.ARRAY_BUFFER, vbo);
      const f = 4, s = STRIDE * f;
      gl.enableVertexAttribArray(0); gl.vertexAttribPointer(0, 3, gl.FLOAT, false, s, 0);
      gl.enableVertexAttribArray(1); gl.vertexAttribPointer(1, 3, gl.FLOAT, false, s, 3 * f);
      gl.enableVertexAttribArray(2); gl.vertexAttribPointer(2, 2, gl.FLOAT, false, s, 6 * f);
      gl.enableVertexAttribArray(3); gl.vertexAttribPointer(3, 3, gl.FLOAT, false, s, 8 * f);
      gl.enableVertexAttribArray(4); gl.vertexAttribPointer(4, 1, gl.FLOAT, false, s, 11 * f);
      m = { vao, vbo, count: 0, layer, ox: 0, oy: 0, oz: 0, yaw: 0, bb: [0, 0, 0, 0, 0, 0], r: 0 };
      meshes.set(id, m);
    }
    m.layer = layer;
    const arr = new Float32Array(data);
    gl.bindVertexArray(m.vao); gl.bindBuffer(gl.ARRAY_BUFFER, m.vbo);
    gl.bufferData(gl.ARRAY_BUFFER, arr, gl.DYNAMIC_DRAW);
    m.count = (arr.length / STRIDE) | 0;
    let x0 = 1e9, y0 = 1e9, z0 = 1e9, x1 = -1e9, y1 = -1e9, z1 = -1e9;
    for (let i = 0; i < arr.length; i += STRIDE) {
      const x = arr[i], y = arr[i + 1], z = arr[i + 2];
      if (x < x0) x0 = x; if (x > x1) x1 = x; if (y < y0) y0 = y; if (y > y1) y1 = y; if (z < z0) z0 = z; if (z > z1) z1 = z;
    }
    m.bb = [x0, y0, z0, x1, y1, z1];
    m.r = Math.hypot(Math.max(Math.abs(x0), Math.abs(x1)), Math.max(Math.abs(y0), Math.abs(y1)), Math.max(Math.abs(z0), Math.abs(z1)));
  };
  H.gfx_mesh_free = (id) => { const m = meshes.get(id); if (!m) return; gl.deleteVertexArray(m.vao); gl.deleteBuffer(m.vbo); meshes.delete(id); };
  H.gfx_mesh_at = (id, x, y, z) => { const m = meshes.get(id); if (m) { m.ox = x; m.oy = y; m.oz = z; } };
  H.gfx_mesh_yaw = (id, yaw) => { const m = meshes.get(id); if (m) m.yaw = yaw; };

  function visible(m, planes) {
    let b = m.bb;
    let x0, y0, z0, x1, y1, z1;
    if (m.yaw !== 0) { x0 = m.ox - m.r; x1 = m.ox + m.r; y0 = m.oy - m.r; y1 = m.oy + m.r; z0 = m.oz - m.r; z1 = m.oz + m.r; }
    else { x0 = m.ox + b[0]; y0 = m.oy + b[1]; z0 = m.oz + b[2]; x1 = m.ox + b[3]; y1 = m.oy + b[4]; z1 = m.oz + b[5]; }
    for (const p of planes) {
      const px = p[0] >= 0 ? x1 : x0, py = p[1] >= 0 ? y1 : y0, pz = p[2] >= 0 ? z1 : z0;
      if (p[0] * px + p[1] * py + p[2] * pz + p[3] < 0) return false;
    }
    return true;
  }

  let drawCalls = 0, tris = 0;
  function drawMeshes(P, layer, planes, filter) {
    for (const m of meshes.values()) {
      if (m.layer !== layer || !m.count) continue;
      if (planes && !visible(m, planes)) continue;
      if (filter && !filter(m)) continue;
      if (P.locs.uOrigin) gl.uniform3f(P.locs.uOrigin, m.ox, m.oy, m.oz);
      if (P.locs.uYaw) gl.uniform1f(P.locs.uYaw, m.yaw);
      gl.bindVertexArray(m.vao);
      gl.drawArrays(gl.TRIANGLES, 0, m.count);
      drawCalls++; tris += m.count / 3;
    }
  }

  // ------------------------------------------------------------------ frame render
  function bindTex(unit, tex) { gl.activeTexture(gl.TEXTURE0 + unit); gl.bindTexture(gl.TEXTURE_2D, tex || dummyTex); }

  function renderFrame() {
    if (!gl) return;
    ensureTargets();
    drawCalls = 0; tris = 0;
    const sw = rts.scene.w, sh = rts.scene.h;
    // camera
    const cp = Math.cos(cam.pitch), sp = Math.sin(cam.pitch);
    const fwd = [-Math.sin(cam.yaw) * cp, sp, -Math.cos(cam.yaw) * cp];
    const eye = [cam.x, cam.y, cam.z];
    const view = M.lookAt(eye, [eye[0] + fwd[0], eye[1] + fwd[1], eye[2] + fwd[2]], [0, 1, 0]);
    const proj = M.persp(cam.fov, W / Hh, 0.1, 600);
    const viewProj = M.mul(proj, view);
    const mats = { view, proj, viewProj, invViewProj: M.inv(viewProj), shadowMat: M.ident(), sunScreen: new Float32Array([0.5, 0.5, 0]), shadowOn: false };
    const sd = uniformsV.get("uSunDir") || [0.3, 0.8, 0.5];
    const sl = Math.hypot(sd[0], sd[1], sd[2]) || 1;
    const sun = [sd[0] / sl, sd[1] / sl, sd[2] / sl];
    // sun on screen
    const sc = M.xform(viewProj, eye[0] + sun[0] * 500, eye[1] + sun[1] * 500, eye[2] + sun[2] * 500);
    if (sc[3] > 0.01) mats.sunScreen = new Float32Array([(sc[0] / sc[3]) * 0.5 + 0.5, (sc[1] / sc[3]) * 0.5 + 0.5, 1]);
    const planes = M.planes(viewProj);

    gl.enable(gl.DEPTH_TEST); gl.depthFunc(gl.LEQUAL);
    gl.disable(gl.BLEND);

    // ---- shadow pass
    const shP = progs.get("shadow");
    if (shP && shadowRT && sun[1] > 0.02) {
      const texel = (shadowArea * 2) / shadowSize;
      const cx = Math.round(cam.x / texel) * texel, cy = Math.round(cam.y / texel) * texel, cz = Math.round(cam.z / texel) * texel;
      const lv = M.lookAt([cx + sun[0] * 200, cy + sun[1] * 200, cz + sun[2] * 200], [cx, cy, cz], Math.abs(sun[1]) > 0.98 ? [0, 0, 1] : [0, 1, 0]);
      const lp = M.ortho(-shadowArea, shadowArea, -shadowArea, shadowArea, 1, 420);
      mats.shadowMat = M.mul(lp, lv);
      mats.shadowOn = true;
      gl.bindFramebuffer(gl.FRAMEBUFFER, shadowRT.fb);
      gl.viewport(0, 0, shadowSize, shadowSize);
      gl.clearDepth(1); gl.clear(gl.DEPTH_BUFFER_BIT);
      gl.colorMask(false, false, false, false);
      gl.useProgram(shP.prog);
      applyCommon(shP, { ...mats, viewProj: mats.shadowMat }, { w: shadowSize, h: shadowSize });
      for (let u = 0; u < 9; u++) bindTex(u, null);
      bindTex(0, atlas);
      gl.enable(gl.POLYGON_OFFSET_FILL); gl.polygonOffset(2, 4);
      gl.disable(gl.CULL_FACE);
      const ext = shadowArea + 20;
      drawMeshes(shP, 0, null, (m) => Math.abs(m.ox + (m.bb[0] + m.bb[3]) / 2 - cx) < ext + 12 && Math.abs(m.oz + (m.bb[2] + m.bb[5]) / 2 - cz) < ext + 12);
      gl.disable(gl.POLYGON_OFFSET_FILL);
      gl.colorMask(true, true, true, true);
    }

    // ---- scene pass
    gl.bindFramebuffer(gl.FRAMEBUFFER, rts.scene.fb);
    gl.viewport(0, 0, sw, sh);
    const sk = uniformsV.get("uSkyHorizon") || [0.5, 0.7, 1];
    gl.clearColor(sk[0], sk[1], sk[2], 1); gl.clearDepth(1);
    gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
    const skyP = progs.get("sky");
    if (skyP) {
      gl.useProgram(skyP.prog);
      applyCommon(skyP, mats, { w: sw, h: sh });
      for (let u = 0; u < 9; u++) bindTex(u, null);
      gl.disable(gl.DEPTH_TEST); gl.depthMask(false);
      gl.bindVertexArray(emptyVAO()); gl.drawArrays(gl.TRIANGLES, 0, 3);
      gl.enable(gl.DEPTH_TEST); gl.depthMask(true);
    }
    const scP = progs.get("scene");
    if (scP) {
      gl.useProgram(scP.prog);
      applyCommon(scP, mats, { w: sw, h: sh });
      for (let u = 0; u < 9; u++) bindTex(u, null);
      bindTex(0, atlas);
      if (mats.shadowOn) bindTex(4, shadowRT.dtex);
      gl.enable(gl.CULL_FACE); gl.cullFace(gl.BACK); gl.frontFace(gl.CCW);
      if (scP.locs.uLayer) gl.uniform1i(scP.locs.uLayer, 0);
      drawMeshes(scP, 0, planes);
      gl.disable(gl.CULL_FACE);
      gl.enable(gl.BLEND); gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA); gl.depthMask(false);
      if (scP.locs.uLayer) gl.uniform1i(scP.locs.uLayer, 1);
      drawMeshes(scP, 1, planes);
      gl.depthMask(true); gl.disable(gl.BLEND);
    }

    // ---- post chain
    const post = [...progs.entries()].filter(([, p]) => p.kind === "post").sort((a, b) => a[1].order - b[1].order);
    gl.disable(gl.DEPTH_TEST); gl.disable(gl.CULL_FACE);
    let prev = rts.scene.tex;
    const outs = [];
    const emptyV = emptyVAO();
    const runPass = (P, target, w, h) => {
      gl.bindFramebuffer(gl.FRAMEBUFFER, target ? target.fb : null);
      gl.viewport(0, 0, w, h);
      gl.useProgram(P.prog);
      applyCommon(P, mats, { w, h });
      for (let u = 0; u < 9; u++) bindTex(u, null);
      bindTex(1, rts.scene.tex); bindTex(2, rts.scene.dtex); bindTex(3, prev);
      if (mats.shadowOn) bindTex(4, shadowRT.dtex);
      for (let i = 0; i < 4 && i < outs.length; i++) bindTex(5 + i, outs[i]);
      gl.bindVertexArray(emptyV); gl.drawArrays(gl.TRIANGLES, 0, 3);
    };
    if (!post.length) {
      if (!H._blit) H._blit = buildProgram(FULL_VS, BLIT_FS);
      if (H._blit.prog) runPass({ prog: H._blit.prog, locs: H._blit.locs }, null, W, Hh);
    } else {
      post.forEach(([k, P], idx) => {
        const last = idx === post.length - 1;
        const rt = last ? null : rts.passes.get(k);
        runPass(P, rt, last ? W : rt.w, last ? Hh : rt.h);
        if (!last) { prev = rt.tex; outs.push(rt.tex); }
      });
    }
    gl.bindFramebuffer(gl.FRAMEBUFFER, null);
    stats = `draws ${drawCalls} · tris ${Math.round(tris)} · meshes ${meshes.size}`;
    sceneMats = mats;
  }
  let _evao = null;
  function emptyVAO() { if (!_evao) _evao = gl.createVertexArray(); return _evao; }

  H.gfx_project = (x, y, z) => {
    if (!sceneMats) return [-1, -1, -1];
    const c = M.xform(sceneMats.viewProj, x, y, z);
    if (c[3] <= 0.05) return [-1, -1, -1];
    return [(c[0] / c[3] * 0.5 + 0.5) * hudc._cw, (1 - (c[1] / c[3] * 0.5 + 0.5)) * hudc._ch, c[3]];
  };

  // ------------------------------------------------------------------ HUD
  H.hud_rect = (x, y, w, h, r, g, b, a) => { hudCmds.push([0, x, y, w, h, r, g, b, a]); };
  H.hud_text = (x, y, size, r, g, b, a, s) => { hudCmds.push([1, x, y, size, r, g, b, a, s]); };
  const FONT = "'Press Start 2P','Courier New',ui-monospace,Menlo,Consolas,monospace";
  H.hud_text_w = (size, s) => { if (!hctx) return s.length * size * 0.6; hctx.font = "bold " + size + "px " + FONT; return hctx.measureText(s).width; };
  const rgba = (r, g, b, a) => `rgba(${(r * 255) | 0},${(g * 255) | 0},${(b * 255) | 0},${a})`;
  function flushHud() {
    const k = hudc._k;
    hctx.setTransform(k * uiK, 0, 0, k * uiK, 0, 0);
    hctx.clearRect(0, 0, hudc._cw, hudc._ch);
    hctx.textBaseline = "top";
    for (const c of hudCmds) {
      if (c[0] === 0) { hctx.fillStyle = rgba(c[5], c[6], c[7], c[8]); hctx.fillRect(c[1], c[2], c[3], c[4]); }
      else {
        const size = c[3];
        hctx.font = "bold " + size + "px " + FONT;
        hctx.fillStyle = rgba(0, 0, 0, c[7] * 0.55); hctx.fillText(c[8], c[1] + Math.max(1, size / 9), c[2] + Math.max(1, size / 9));
        hctx.fillStyle = rgba(c[4], c[5], c[6], c[7]); hctx.fillText(c[8], c[1], c[2]);
      }
    }
    hudCmds = [];
  }

  // ------------------------------------------------------------------ frame loop
  let tResumed = 0, jsMs = 0, jsMax = 0, rdMs = 0, jsWin = [];
  window.__vcPerf = () => { const a = jsWin.slice().sort((x, y) => x - y); return { n: a.length, median: a[a.length >> 1], p95: a[Math.floor(a.length * 0.95)], max: a[a.length - 1], scale: renderScale, q: quality }; };
  H.gfx_frame = () => new Promise((resolve) => {
    const t0 = performance.now();
    jsMs = t0 - tResumed;
    renderFrame();
    const t1 = performance.now(); rdMs = t1 - t0;
    flushHud();
    jsWin.push(jsMs); if (jsWin.length > 90) jsWin.shift();
    if ((drsFrames & 15) === 0) jsMax = Math.max.apply(null, jsWin);
    stats += ` · js ${jsMs.toFixed(1)}ms (max ${jsMax.toFixed(0)}) · gl ${rdMs.toFixed(1)}ms · res ${Math.round(renderScale * 100)}%`;
    keysPressed.clear(); clicks[0] = clicks[1] = clicks[2] = 0;
    requestAnimationFrame((t) => {
      const dt = Math.min(0.1, Math.max(0.0005, (t - lastT) / 1000));
      lastT = t; frameT += dt;
      document.body.classList.toggle("locked", !!document.pointerLockElement || (isTouch && wantLock));
      drsTick(dt);
      tResumed = performance.now();
      resolve(dt);
    });
  });

  // ------------------------------------------------------------------ input
  const keys = new Set(), keysPressed = new Set();
  const clicks = [0, 0, 0], held = [false, false, false];
  let mdx = 0, mdy = 0, mx = 0, my = 0, wheel = 0, wantLock = false;
  function keyCode(e) {
    const c = e.code;
    if (c.startsWith("Key")) return c.charCodeAt(3);
    if (c.startsWith("Digit")) return c.charCodeAt(5);
    switch (c) {
      case "Space": return 32; case "ShiftLeft": case "ShiftRight": return 16;
      case "ControlLeft": case "ControlRight": return 17; case "Escape": return 27;
      case "Tab": return 9; case "Enter": return 13; case "Backspace": return 8;
      case "ArrowLeft": return 37; case "ArrowUp": return 38; case "ArrowRight": return 39; case "ArrowDown": return 40;
      case "Minus": return 189; case "Equal": return 187; case "Comma": return 188; case "Period": return 190; case "Backquote": return 192;
    }
    const f = /^F(\d+)$/.exec(c); if (f) return 111 + parseInt(f[1]);
    return 0;
  }

  // ------------------------------------------------------------------ touch controls
  // Virtual stick (move), look pad, action buttons. Every button can be moved and resized
  // in the layout editor (Options > Customize touch controls); settings persist in localStorage.
  const UA_MOBILE = /Android|iPhone|iPad|iPod|Mobile|HarmonyOS/i.test((navigator && navigator.userAgent) || "");
  let isTouch = false, touchBound = false, editing = false, selId = null, matchUI = false;
  let touchEl = null, stickEl = null, knobEl = null, menuArm = 0;
  const tpt = new Map();                 // pointerId -> {role, ...}
  const TS_DEF = { size: 1, opacity: 0.8, sens: 1, lefty: false, float: true, sprint: true, labels: true, hints: true };
  let TS = Object.assign({}, TS_DEF, { pos: {} });
  try { const o = JSON.parse(localStorage.getItem("vctouch") || "{}"); TS = Object.assign({}, TS_DEF, o, { pos: (o && o.pos) || {} }); } catch (e) { /* storage blocked */ }
  function saveTS() { try { localStorage.setItem("vctouch", JSON.stringify(TS)); } catch (e) { /* ignore */ } }
  const TB = {
    hit:   { id: "tb-hit",   w: 88, h: 88, a: "br", x: 64,  y: 68,  name: "HIT" },
    jump:  { id: "tb-jump",  w: 68, h: 68, a: "br", x: 64,  y: 160, name: "JUMP" },
    place: { id: "tb-place", w: 62, h: 62, a: "br", x: 155, y: 89,  name: "PLACE" },
    fx:    { id: "tb-fx",    w: 52, h: 34, a: "tr", x: 154, y: 25,  name: "FX" },
    inv:   { id: "tb-inv",   w: 56, h: 34, a: "tr", x: 94,  y: 25,  name: "Blocks" },
    menu:  { id: "tb-menu",  w: 52, h: 34, a: "tr", x: 34,  y: 25,  name: "Menu" },
    prev:  { id: "tb-prev",  w: 42, h: 42, a: "hl", name: "<" },
    next:  { id: "tb-next",  w: 42, h: 42, a: "hr", name: ">" },
    stick: { id: "stickhint", w: 100, h: 100, a: "bl", x: 90, y: 90, name: "move" },
  };
  const clampN = (v, a, b) => Math.max(a, Math.min(b, v));
  function insets() {
    let p = $("sa-probe");
    if (!p) { p = document.createElement("div"); p.id = "sa-probe"; p.style.cssText = "position:fixed;left:0;top:0;width:0;height:0;visibility:hidden;padding:env(safe-area-inset-top) env(safe-area-inset-right) env(safe-area-inset-bottom) env(safe-area-inset-left)"; document.body.appendChild(p); }
    const c = getComputedStyle(p);
    return { t: parseFloat(c.paddingTop) || 0, r: parseFloat(c.paddingRight) || 0, b: parseFloat(c.paddingBottom) || 0, l: parseFloat(c.paddingLeft) || 0 };
  }
  function defCenter(s) {
    const Wn = window.innerWidth, Hn = window.innerHeight, I = insets();
    let cx = 0, cy = 0;
    switch (s.a) {
      case "br": cx = Wn - I.r - s.x; cy = Hn - I.b - s.y; break;
      case "tr": cx = Wn - I.r - s.x; cy = I.t + s.y; break;
      case "bl": cx = I.l + s.x;      cy = Hn - I.b - s.y; break;
      case "hl": cx = Wn / 2 - (266 * uiK + 27); cy = Hn - I.b - (18 * uiK + 21); break;
      default:   cx = Wn / 2 + (266 * uiK + 27); cy = Hn - I.b - (18 * uiK + 21);
    }
    if (TS.lefty && (s.a === "br" || s.a === "tr" || s.a === "bl")) cx = Wn - cx;
    return [cx, cy];
  }
  function btnScale(k) { const o = TS.pos[k]; return TS.size * ((o && o.s) || 1); }
  function applyTouchLayout() {
    if (!touchEl) return;
    const Wn = window.innerWidth, Hn = window.innerHeight;
    for (const k in TB) {
      const s = TB[k], el = $(s.id); if (!el) continue;
      const o = TS.pos[k], sc = btnScale(k);
      let [cx, cy] = defCenter(s);
      if (o && o.x != null) { cx = o.x * Wn; cy = o.y * Hn; }
      const w = s.w * sc, h = s.h * sc;
      cx = clampN(cx, w / 2, Wn - w / 2); cy = clampN(cy, h / 2, Hn - h / 2);
      const st = el.style;
      st.left = (cx - w / 2) + "px"; st.top = (cy - h / 2) + "px"; st.right = "auto"; st.bottom = "auto";
      st.width = w + "px"; st.height = h + "px"; st.fontSize = Math.max(9, 13 * sc) + "px";
      el.dataset.cx = cx; el.dataset.cy = cy;
    }
    touchEl.style.setProperty("--to", TS.opacity);
    touchEl.classList.toggle("nolabels", !TS.labels);
    touchEl.classList.toggle("nohints", !TS.hints);
    if (stickEl) stickEl.style.transform = "translate(-0px,-0px) scale(" + TS.size + ")";
  }
  const STICK_R = 56;
  function setStick(dx, dy) {
    const R = STICK_R * TS.size;
    const m = Math.hypot(dx, dy) || 1, r = Math.min(1, m / R);
    const nx = dx / m * r, ny = dy / m * r;
    const set = (k, on) => { if (on) keys.add(k); else keys.delete(k); };
    set(87, ny < -0.28); set(83, ny > 0.28); set(65, nx < -0.28); set(68, nx > 0.28);
    set(17, TS.sprint && ny < -0.9);                      // push all the way forward to sprint
    if (knobEl) knobEl.style.transform = "translate(" + (nx * STICK_R) + "px," + (ny * STICK_R) + "px)";
  }
  function clearStick() { for (const k of [87, 83, 65, 68, 17]) keys.delete(k); if (knobEl) knobEl.style.transform = ""; if (stickEl) stickEl.style.display = "none"; }
  function setTouchPlay(on, keepOverlay) {
    if (!keepOverlay) { matchUI = !!on; document.body.classList.toggle("touchplay", !!(isTouch && (on || editing))); }
    if (!on) { clearStick(); tpt.clear(); held[0] = held[1] = false; keys.delete(32); }
  }
  function layoutTouch() { if (touchEl) { touchEl.style.setProperty("--uik", uiK); applyTouchLayout(); } }
  function setTouch(on) {
    if (quality === "auto" && drsFrames < 150) drs = on ? 0.8 : 1;
    if (isTouch === on) return;
    isTouch = on;
    document.body.classList.toggle("touch", on);
    if (on) bindTouch();
    if (!on) { clearStick(); tpt.clear(); document.body.classList.remove("touchplay"); }
    else document.body.classList.toggle("touchplay", matchUI || editing);
    applyTouchLayout();
  }
  function bindBtn(id, down, up) {
    const el = $(id); if (!el) return;
    el.addEventListener("pointerdown", (e) => { if (editing) return; e.preventDefault(); e.stopPropagation(); audioInit(); try { el.setPointerCapture(e.pointerId); } catch (x) {} el.classList.add("on"); down(); });
    const end = (e) => { if (editing) return; e.preventDefault(); el.classList.remove("on"); if (up) up(); };
    el.addEventListener("pointerup", end); el.addEventListener("pointercancel", end);
  }
  function bindTouch() {
    if (touchBound) return; touchBound = true;
    touchEl = $("touch"); stickEl = $("stick"); knobEl = $("knob");
    glc.addEventListener("pointerdown", (e) => {
      if (e.pointerType === "mouse" || !isTouch || editing) return;
      e.preventDefault(); audioInit();
      try { glc.setPointerCapture(e.pointerId); } catch (x) {}
      if (!wantLock) {                                    // menu / palette: behave like a mouse
        mx = e.clientX / uiK; my = e.clientY / uiK; clicks[0]++; held[0] = true;
        tpt.set(e.pointerId, { role: "ui" }); return;
      }
      const Wn = window.innerWidth;
      const inZone = TS.lefty ? e.clientX > Wn * 0.58 : e.clientX < Wn * 0.42;
      const hasStick = [...tpt.values()].some((p) => p.role === "stick");
      if (inZone && !hasStick) {
        let sx = e.clientX, sy = e.clientY;
        if (!TS.float) { const h = $("stickhint"); sx = parseFloat(h.dataset.cx); sy = parseFloat(h.dataset.cy); }
        tpt.set(e.pointerId, { role: "stick", sx, sy });
        stickEl.style.display = "block"; stickEl.style.left = sx + "px"; stickEl.style.top = sy + "px";
        setStick(e.clientX - sx, e.clientY - sy);
      } else tpt.set(e.pointerId, { role: "look", lx: e.clientX, ly: e.clientY });
    });
    glc.addEventListener("pointermove", (e) => {
      const p = tpt.get(e.pointerId); if (!p) return;
      e.preventDefault();
      if (p.role === "ui") { mx = e.clientX / uiK; my = e.clientY / uiK; }
      else if (p.role === "stick") setStick(e.clientX - p.sx, e.clientY - p.sy);
      else { mdx += (e.clientX - p.lx) * 1.9 * TS.sens; mdy += (e.clientY - p.ly) * 1.9 * TS.sens; p.lx = e.clientX; p.ly = e.clientY; }
    });
    const up = (e) => {
      const p = tpt.get(e.pointerId); if (!p) return;
      tpt.delete(e.pointerId);
      if (p.role === "ui") held[0] = false;
      else if (p.role === "stick") clearStick();
    };
    glc.addEventListener("pointerup", up); glc.addEventListener("pointercancel", up);
    bindBtn("tb-jump", () => keys.add(32), () => keys.delete(32));
    bindBtn("tb-hit", () => { clicks[0]++; held[0] = true; }, () => { held[0] = false; });
    bindBtn("tb-place", () => { clicks[1]++; }, null);
    bindBtn("tb-prev", () => { wheel -= 1; }, null);
    bindBtn("tb-next", () => { wheel += 1; }, null);
    bindBtn("tb-inv", () => { keysPressed.add(69); }, null);
    bindBtn("tb-fx", () => { keysPressed.add(117); }, null);
    bindBtn("tb-menu", () => {
      const now = performance.now(), mb = $("tb-menu");
      if (now < menuArm) { menuArm = 0; mb.textContent = "Menu"; keysPressed.add(81); }
      else { menuArm = now + 2200; mb.textContent = "Sure?"; setTimeout(() => { if (performance.now() >= menuArm) mb.textContent = "Menu"; }, 2300); }
    }, null);
    bindEditor();
    applyTouchLayout();
  }

  // ---- layout editor
  function selectBtn(k) {
    selId = k;
    for (const n in TB) { const el = $(TB[n].id); if (el) el.classList.toggle("sel", n === k); }
    const lab = $("te-sel"), sl = $("te-selsize");
    if (lab) lab.textContent = k ? TB[k].name : "none";
    if (sl) { sl.disabled = !k; sl.value = k ? ((TS.pos[k] && TS.pos[k].s) || 1) : 1; }
  }
  function bindEditor() {
    let drag = null;
    touchEl.addEventListener("pointerdown", (e) => {
      if (!editing) return;
      const el = e.target.closest && e.target.closest(".tb, #stickhint"); if (!el) return;
      let k = null; for (const n in TB) if (TB[n].id === el.id) k = n;
      if (!k) return;
      e.preventDefault(); e.stopPropagation();
      selectBtn(k);
      drag = { k, id: e.pointerId, ox: e.clientX - parseFloat(el.dataset.cx), oy: e.clientY - parseFloat(el.dataset.cy) };
      try { el.setPointerCapture(e.pointerId); } catch (x) {}
    }, true);
    touchEl.addEventListener("pointermove", (e) => {
      if (!editing || !drag || e.pointerId !== drag.id) return;
      e.preventDefault(); e.stopPropagation();
      const Wn = window.innerWidth, Hn = window.innerHeight;
      const o = TS.pos[drag.k] || (TS.pos[drag.k] = {});
      o.x = clampN((e.clientX - drag.ox) / Wn, 0.02, 0.98); o.y = clampN((e.clientY - drag.oy) / Hn, 0.03, 0.97);
      applyTouchLayout();
    }, true);
    const end = (e) => { if (!editing || !drag || e.pointerId !== drag.id) return; drag = null; saveTS(); };
    touchEl.addEventListener("pointerup", end, true); touchEl.addEventListener("pointercancel", end, true);
    const bind = (id, fn) => { const el = $(id); if (el) el.addEventListener("input", () => { fn(el); applyTouchLayout(); saveTS(); }); };
    bind("te-size", (el) => { TS.size = parseFloat(el.value); });
    bind("te-opacity", (el) => { TS.opacity = parseFloat(el.value); });
    bind("te-sens", (el) => { TS.sens = parseFloat(el.value); });
    bind("te-selsize", (el) => { if (!selId) return; const o = TS.pos[selId] || (TS.pos[selId] = {}); o.s = parseFloat(el.value); });
    for (const [id, key] of [["te-lefty", "lefty"], ["te-float", "float"], ["te-sprint", "sprint"], ["te-labels", "labels"], ["te-hints", "hints"]]) {
      const el = $(id); if (el) el.addEventListener("change", () => { TS[key] = el.checked; if (key === "lefty") { for (const n in TS.pos) { delete TS.pos[n].x; delete TS.pos[n].y; } } applyTouchLayout(); saveTS(); });
    }
    const reset = $("te-reset"); if (reset) reset.addEventListener("click", () => { TS = Object.assign({}, TS_DEF, { pos: {} }); saveTS(); syncEditor(); applyTouchLayout(); });
    const done = $("te-done"); if (done) done.addEventListener("click", endEdit);
    const fold = $("te-fold"); if (fold) fold.addEventListener("click", () => { const b = $("te-body"); const h = b.style.display === "none"; b.style.display = h ? "block" : "none"; fold.textContent = h ? "Hide" : "Show"; });
  }
  function syncEditor() {
    const set = (id, v) => { const el = $(id); if (el) el.value = v; };
    set("te-size", TS.size); set("te-opacity", TS.opacity); set("te-sens", TS.sens);
    for (const [id, key] of [["te-lefty", "lefty"], ["te-float", "float"], ["te-sprint", "sprint"], ["te-labels", "labels"], ["te-hints", "hints"]]) { const el = $(id); if (el) el.checked = !!TS[key]; }
    selectBtn(selId);
  }
  function startEdit() {
    if (!touchBound) { document.body.classList.add("touch"); bindTouch(); }
    editing = true; clearStick(); tpt.clear();
    document.body.classList.add("touchplay", "touchedit");
    $("tedit").style.display = "block";
    for (const k in TB) { const el = $(TB[k].id); if (el) el.dataset.label = TB[k].name; }
    syncEditor(); applyTouchLayout();
  }
  function endEdit() {
    editing = false;
    document.body.classList.remove("touchedit");
    document.body.classList.toggle("touchplay", isTouch && matchUI);
    $("tedit").style.display = "none";
    if (!isTouch) { document.body.classList.remove("touch"); }
    selectBtn(null);
  }
  H.gfx_touch = () => isTouch;
  window.__vulcanBack = () => {             // Android back button / browser hook; returns true if it was handled
    if (editing) { endEdit(); return true; }
    const hp = $("help"), op = $("opts");
    if (hp && hp.classList.contains("open")) { hp.classList.remove("open"); try { localStorage.setItem("vchelp", "1"); } catch (e) { /* ignore */ } return true; }
    if (op && op.classList.contains("open")) { op.classList.remove("open"); return true; }
    if (matchUI) { keysPressed.add(81); return true; }
    keysPressed.add(27);
    return false;
  };

  function initInput() {
    setTouch((UA_MOBILE || ((navigator.maxTouchPoints > 0) && window.matchMedia && window.matchMedia("(pointer: coarse)").matches)));
    window.addEventListener("pointerdown", (e) => {
      if (e.pointerType === "touch" && !isTouch) setTouch(true);
      else if (e.pointerType === "mouse" && isTouch && !UA_MOBILE && !editing) setTouch(false);
    }, true);
    window.addEventListener("keydown", (e) => {
      if (e.target && (e.target.tagName === "TEXTAREA" || e.target.tagName === "INPUT")) return;
      const k = keyCode(e); if (!k) return;
      if (!keys.has(k)) keysPressed.add(k);
      keys.add(k);
      if (k === 9 || k === 32 || (k >= 37 && k <= 40) || k === 8 || (k >= 112 && k <= 120)) e.preventDefault();
      audioInit();
    });
    window.addEventListener("keyup", (e) => { const k = keyCode(e); keys.delete(k); });
    window.addEventListener("blur", () => { keys.clear(); held[0] = held[1] = held[2] = false; });
    glc.addEventListener("mousedown", (e) => {
      if (isTouch && e.sourceCapabilities && e.sourceCapabilities.firesTouchEvents) return;
      mx = e.clientX / uiK; my = e.clientY / uiK;
      audioInit();
      if (wantLock && !document.pointerLockElement) { glc.requestPointerLock && glc.requestPointerLock(); }
      if (e.button < 3) { clicks[e.button]++; held[e.button] = true; }
    });
    window.addEventListener("mouseup", (e) => { if (e.button < 3) held[e.button] = false; });
    window.addEventListener("pointermove", (e) => {
      if (e.pointerType === "touch") return;
      mx = e.clientX / uiK; my = e.clientY / uiK;
      if (document.pointerLockElement) { mdx += e.movementX; mdy += e.movementY; }
    });
    window.addEventListener("wheel", (e) => { wheel += Math.sign(e.deltaY); }, { passive: true });
    window.addEventListener("contextmenu", (e) => { if (e.target === glc) e.preventDefault(); });
  }
  H.gfx_key = (c) => keys.has(c);
  H.gfx_key_pressed = (c) => keysPressed.has(c);
  H.gfx_mouse_dx = () => { const v = mdx; mdx = 0; return v; };
  H.gfx_mouse_dy = () => { const v = mdy; mdy = 0; return v; };
  H.gfx_mouse_x = () => mx;
  H.gfx_mouse_y = () => my;
  H.gfx_click = (b) => clicks[b] || 0;
  H.gfx_mouse_held = (b) => !!held[b];
  H.gfx_wheel = () => { const v = wheel; wheel = 0; return v; };
  H.gfx_locked = () => !!document.pointerLockElement || (isTouch && wantLock);
  H.gfx_set_lock = (on) => {
    wantLock = on;
    if (!on) setTouchPlay(false, true);
    if (on && !isTouch && !document.pointerLockElement) { try { const p = glc.requestPointerLock(); if (p && p.catch) p.catch(() => {}); } catch (e) { /* needs gesture */ } }
    if (!on && document.pointerLockElement) document.exitPointerLock();
  };

  // ------------------------------------------------------------------ sound (tiny WebAudio synth)
  let actx = null, master = null;
  function audioInit() {
    if (actx) { if (actx.state === "suspended") actx.resume(); return; }
    try { actx = new (window.AudioContext || window.webkitAudioContext)(); master = actx.createGain(); master.gain.value = 0.5; master.connect(actx.destination); } catch (e) { /* no audio */ }
  }
  let noiseBuf = null;
  function noise() {
    if (!noiseBuf) { noiseBuf = actx.createBuffer(1, actx.sampleRate, actx.sampleRate); const d = noiseBuf.getChannelData(0); for (let i = 0; i < d.length; i++) d[i] = Math.random() * 2 - 1; }
    const s = actx.createBufferSource(); s.buffer = noiseBuf; return s;
  }
  function env(g, t, a, d, peak) { g.gain.setValueAtTime(0.0001, t); g.gain.exponentialRampToValueAtTime(Math.max(0.0002, peak), t + a); g.gain.exponentialRampToValueAtTime(0.0001, t + a + d); }
  H.snd = (kind, vol) => {
    if (!actx || actx.state !== "running") return;
    const t = actx.currentTime, v = Math.max(0, Math.min(1, vol));
    const tone = (type, f0, f1, dur, peak) => { const o = actx.createOscillator(), g = actx.createGain(); o.type = type; o.frequency.setValueAtTime(f0, t); o.frequency.exponentialRampToValueAtTime(Math.max(20, f1), t + dur); env(g, t, 0.005, dur, peak * v); o.connect(g); g.connect(master); o.start(t); o.stop(t + dur + 0.05); };
    const burst = (freq, q, dur, peak, type) => { const s = noise(), f = actx.createBiquadFilter(), g = actx.createGain(); f.type = type || "bandpass"; f.frequency.value = freq; f.Q.value = q; env(g, t, 0.003, dur, peak * v); s.connect(f); f.connect(g); g.connect(master); s.start(t, Math.random() * 0.5, dur + 0.05); };
    switch (kind) {
      case 0: burst(700, 1.2, 0.14, 0.8); tone("triangle", 200, 90, 0.09, 0.4); break;
      case 1: tone("sine", 160, 70, 0.1, 0.8); burst(500, 2, 0.05, 0.3); break;
      case 2: tone("sawtooth", 420, 180, 0.1, 0.5); burst(2000, 1, 0.06, 0.5); break;
      case 3: tone("square", 160, 70, 0.22, 0.45); break;
      case 4: burst(1200, 3, 0.03, 0.18); break;
      case 5: tone("sawtooth", 330, 40, 0.8, 0.5); break;
      case 6: tone("triangle", 520, 520, 0.12, 0.5); setTimeout(() => { if (actx) { const t2 = actx.currentTime, o = actx.createOscillator(), g = actx.createGain(); o.type = "triangle"; o.frequency.value = 780; env(g, t2, 0.005, 0.18, 0.5 * v); o.connect(g); g.connect(master); o.start(t2); o.stop(t2 + 0.25); } }, 110); break;
      case 7: tone("square", 700, 900, 0.05, 0.25); break;
      case 8: burst(900, 0.6, 0.25, 0.7, "lowpass"); break;
      case 9: burst(180, 0.5, 0.5, 1.0, "lowpass"); tone("sine", 90, 30, 0.4, 0.8); break;
    }
  };

  // ------------------------------------------------------------------ networking (1v1)
  const net = { state: 0, queue: [], mode: "", pc: null, dcRel: null, dcFast: null, bc: null, me: Math.random().toString(36).slice(2), ping: null };
  H.net_state = () => net.state;
  H.net_recv = () => net.queue.length ? net.queue.shift() : "";
  H.net_send = (msg) => {
    if (net.state !== 2) return;
    if (net.mode === "local") { net.bc.postMessage({ from: net.me, m: msg }); return; }
    const fast = msg.charCodeAt(0) === 112; // 'p' = position update (unordered, lossy ok)
    const ch = fast && net.dcFast && net.dcFast.readyState === "open" ? net.dcFast : net.dcRel;
    if (ch && ch.readyState === "open") ch.send(msg);
  };
  H.net_stop = () => {
    if (net.ping) clearInterval(net.ping);
    if (net.bc) net.bc.close();
    if (net.pc) net.pc.close();
    net.state = 0; net.mode = ""; net.queue = []; net.bc = net.pc = net.dcRel = net.dcFast = null;
    panel(false);
  };
  async function enc(obj) {
    const json = JSON.stringify(obj);
    const bytes = new TextEncoder().encode(json);
    if (typeof CompressionStream !== "undefined") {
      const cs = new CompressionStream("deflate-raw"); const w = cs.writable.getWriter(); w.write(bytes); w.close();
      const buf = new Uint8Array(await new Response(cs.readable).arrayBuffer());
      return "z1." + b64(buf);
    }
    return "p1." + b64(bytes);
  }
  async function dec(code) {
    code = code.trim();
    const kind = code.slice(0, 3), raw = unb64(code.slice(3));
    let bytes = raw;
    if (kind === "z1.") {
      const ds = new DecompressionStream("deflate-raw"); const w = ds.writable.getWriter(); w.write(raw); w.close();
      bytes = new Uint8Array(await new Response(ds.readable).arrayBuffer());
    }
    return JSON.parse(new TextDecoder().decode(bytes));
  }
  const b64 = (u8) => { let s = ""; for (const b of u8) s += String.fromCharCode(b); return btoa(s).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, ""); };
  const unb64 = (s) => { s = s.replace(/-/g, "+").replace(/_/g, "/"); while (s.length % 4) s += "="; const r = atob(s); const u = new Uint8Array(r.length); for (let i = 0; i < r.length; i++) u[i] = r.charCodeAt(i); return u; };

  function gather(pc) {
    return new Promise((res) => {
      if (pc.iceGatheringState === "complete") return res();
      const done = () => { if (pc.iceGatheringState === "complete") { pc.removeEventListener("icegatheringstatechange", done); res(); } };
      pc.addEventListener("icegatheringstatechange", done);
      setTimeout(res, 5000);
    });
  }
  function wireChannel(dc, fast) {
    dc.onmessage = (e) => { net.queue.push(String(e.data)); };
    dc.onopen = () => { if (!fast) { net.state = 2; setPanelMsg("Connected."); setTimeout(() => panel(false), 800); } };
    dc.onclose = () => { if (!fast && net.state === 2) net.state = 3; };
  }
  function newPC() {
    const pc = new RTCPeerConnection({ iceServers: [{ urls: "stun:stun.l.google.com:19302" }, { urls: "stun:stun1.l.google.com:19302" }] });
    pc.onconnectionstatechange = () => { if (pc.connectionState === "failed" || pc.connectionState === "closed" || pc.connectionState === "disconnected") { if (net.state === 2) net.state = 3; setPanelMsg("Connection " + pc.connectionState + "."); } };
    return pc;
  }
  H.net_start = (mode) => {
    H.net_stop();
    net.mode = mode; net.state = 1;
    if (mode === "local") {
      net.bc = new BroadcastChannel("vulcancraft-1v1");
      net.bc.onmessage = (e) => {
        const d = e.data; if (!d || d.from === net.me) return;
        if (d.ping) { if (net.state !== 2) { net.state = 2; net.bc.postMessage({ from: net.me, ping: 1 }); } return; }
        if (net.state === 2) net.queue.push(d.m);
      };
      net.ping = setInterval(() => { if (net.state === 1) net.bc.postMessage({ from: net.me, ping: 1 }); }, 400);
      return;
    }
    panel(true, mode);
    if (mode === "host") hostFlow(); else joinFlow();
  };
  async function hostFlow() {
    const pc = net.pc = newPC();
    net.dcRel = pc.createDataChannel("rel"); wireChannel(net.dcRel, false);
    net.dcFast = pc.createDataChannel("fast", { ordered: false, maxRetransmits: 0 }); wireChannel(net.dcFast, true);
    await pc.setLocalDescription(await pc.createOffer());
    setPanelMsg("Gathering network candidates...");
    await gather(pc);
    $("vc-out").value = await enc(pc.localDescription);
    setPanelMsg("1) Send the invite code to your opponent. 2) Paste their reply code below and press Connect.");
    $("vc-go").onclick = async () => {
      try { await pc.setRemoteDescription(await dec($("vc-in").value)); setPanelMsg("Connecting..."); } catch (e) { setPanelMsg("Bad reply code: " + e); }
    };
  }
  async function joinFlow() {
    const pc = net.pc = newPC();
    pc.ondatachannel = (e) => { if (e.channel.label === "rel") { net.dcRel = e.channel; wireChannel(e.channel, false); } else { net.dcFast = e.channel; wireChannel(e.channel, true); } };
    setPanelMsg("Paste the invite code from the host and press Join.");
    $("vc-go").onclick = async () => {
      try {
        await pc.setRemoteDescription(await dec($("vc-in").value));
        await pc.setLocalDescription(await pc.createAnswer());
        setPanelMsg("Gathering network candidates...");
        await gather(pc);
        $("vc-out").value = await enc(pc.localDescription);
        setPanelMsg("Send this reply code back to the host. The game starts when they press Connect.");
      } catch (e) { setPanelMsg("Bad invite code: " + e); }
    };
  }
  function setPanelMsg(m) { const el = $("vc-msg"); if (el) el.textContent = m; }
  function panel(show, mode) {
    const p = $("netpanel"); if (!p) return;
    p.style.display = show ? "block" : "none";
    if (show) {
      $("vc-title").textContent = mode === "host" ? "Host a 1v1 match" : "Join a 1v1 match";
      $("vc-go").textContent = mode === "host" ? "Connect" : "Join";
      $("vc-in").value = ""; $("vc-out").value = "";
      $("vc-in-l").textContent = mode === "host" ? "Opponent's reply code" : "Host's invite code";
      $("vc-out-l").textContent = mode === "host" ? "Your invite code" : "Your reply code";
    }
  }
  H.net_ui = (show) => { const p = $("netpanel"); if (p && net.state !== 2) p.style.display = show && net.mode && net.mode !== "local" ? "block" : "none"; };

  // ------------------------------------------------------------------ user file import (mod packs / shader packs)
  window.addEventListener("DOMContentLoaded", () => {
    const btn = $("importbtn"), inp = $("importfile");
    if (btn && inp) {
      btn.onclick = () => { closeOpts(); inp.click(); };
      inp.onchange = async () => {
        let n = 0;
        for (const f of inp.files) {
          const txt = await f.text();
          const path = "user/" + f.name.replace(/[^A-Za-z0-9_.-]/g, "_");
          vfs.set(path, txt); persist(path, txt); n++;
        }
        $("importmsg").textContent = "Imported " + n + " file(s). Open the menu to pick them.";
        setTimeout(() => { $("importmsg").textContent = ""; }, 5000);
        inp.value = "";
      };
    }
    // ---- options dropdown, help, quality, fullscreen, copy
    const opts = $("opts"), optbtn = $("optbtn");
    const closeOpts = () => { if (opts) opts.classList.remove("open"); };
    if (optbtn) optbtn.onclick = (e) => { e.stopPropagation(); opts.classList.toggle("open"); };
    document.addEventListener("pointerdown", (e) => { if (opts && !e.target.closest("#barwrap")) closeOpts(); }, true);
    const HELP = {
      "Quick start": `<h4>Quick start</h4><ul>
        <li>Pick a mode in the main menu: <b>Practice</b> (vs bot), <b>1v1 Online</b> (vs a friend) or <b>Sandbox</b>.</li>
        <li>Pick a <b>modpack</b> (Classic is the default) and a <b>shader</b>. Phones start on the fast Vanilla shader; switch to Lumina or Cinematic if your device is fast.</li>
        <li>Win a duel by defeating the other player. Mine blocks, build cover, and use special blocks from mods.</li>
        <li>Open <b>Options</b> (top right) any time in menus for Quality, imports and this help.</li></ul>`,
      "Touch": `<h4>Touch controls</h4><ul>
        <li><b>Move:</b> touch and drag on the left side of the screen. Push the stick fully forward to sprint.</li>
        <li><b>Look:</b> drag on the right side of the screen.</li>
        <li><b>HIT</b> mines or attacks (hold to keep mining). <b>PLACE</b> puts a block. <b>JUMP</b> jumps (hold to swim or fly up).</li>
        <li><b>&lt; &gt;</b> change the hotbar slot. <b>Blocks</b> opens the block palette. <b>FX</b> switches to the next shader. <b>Menu</b> asks for a second tap to leave the match.</li>
        <li><b>Customize:</b> Options &gt; Customize touch controls. Drag any button, change sizes, opacity, look speed, left-handed mode, floating stick and auto-sprint. Settings are saved on the device.</li></ul>`,
      "Keyboard and mouse": `<h4>Keyboard and mouse</h4><ul>
        <li><kbd>W A S D</kbd> move, <kbd>Space</kbd> jump, <kbd>Shift</kbd> sneak or descend, <kbd>Ctrl</kbd> sprint.</li>
        <li>Mouse: left click mine or attack, right click place, wheel or <kbd>1</kbd>-<kbd>9</kbd> hotbar.</li>
        <li><kbd>E</kbd> block palette, <kbd>Q</kbd> leave match, <kbd>Esc</kbd> release mouse, <kbd>F3</kbd> stats, <kbd>F6</kbd> next shader.</li></ul>`,
      "Mods and shaders": `<h4>Mods, modpacks and shaders</h4><ul>
        <li>Built in: mods Core, Crystals, Colors, Moon, Hardcore; modpacks Classic, Moon Duel, Gem Caves, Builder Arena, Hardcore; shaders Lumina, Vanilla, Retro, Cinematic.</li>
        <li>Import your own <code>.vmod</code> (mod), <code>.modpack</code> (list of mods) or <code>.vshader</code> (shader pack) files with Options &gt; Import. They appear in the menu straight away and are kept on this device.</li>
        <li>Options &gt; Clear imports removes them again.</li></ul>`,
      "1v1 online": `<h4>1v1 online</h4><ul>
        <li>No server needed: it is a direct peer-to-peer link.</li>
        <li>One player picks <b>1v1 Online &gt; Host</b>, copies their code and sends it to the friend (chat, text).</li>
        <li>The friend picks <b>Join</b>, pastes the code, then copies their reply code back to the host. The host pastes it and presses Connect.</li>
        <li>Both players need the same modpack. Use Cancel on the waiting screen to go back.</li></ul>`,
    };
    const helpEl = $("help"), tabsEl = $("help-tabs"), bodyEl = $("help-body");
    function showTab(name) {
      bodyEl.innerHTML = HELP[name];
      for (const b of tabsEl.children) b.classList.toggle("on", b.textContent === name);
    }
    if (helpEl) {
      for (const name of Object.keys(HELP)) { const b = document.createElement("button"); b.textContent = name; b.onclick = () => showTab(name); tabsEl.appendChild(b); }
      const openHelp = (tab) => { closeOpts(); showTab(tab || "Quick start"); helpEl.classList.add("open"); };
      const closeHelp = () => { helpEl.classList.remove("open"); try { localStorage.setItem("vchelp", "1"); } catch (e) { /* ignore */ } };
      $("helpbtn").onclick = () => openHelp(isTouch ? "Touch" : "Quick start");
      $("help-close").onclick = closeHelp;
      helpEl.addEventListener("pointerdown", (e) => { e.stopPropagation(); if (e.target === helpEl) closeHelp(); });
      window.addEventListener("keydown", (e) => { if (!helpEl.classList.contains("open")) return; e.stopImmediatePropagation(); if (e.key === "Escape") closeHelp(); }, true);
      window.addEventListener("keyup", (e) => { if (helpEl.classList.contains("open")) e.stopImmediatePropagation(); }, true);
      let seen = false; try { seen = !!localStorage.getItem("vchelp"); } catch (e) { /* ignore */ }
      if (!seen) setTimeout(() => openHelp(isTouch ? "Touch" : "Quick start"), 600);
    }
    const cz = $("customizebtn"); if (cz) cz.onclick = () => { closeOpts(); startEdit(); };
    const qs = $("qualsel"); if (qs) { qs.value = quality; qs.onchange = () => applyQuality(qs.value); }
    const fs = $("fsbtn");
    if (fs) fs.onclick = () => {
      closeOpts();
      const el = document.documentElement;
      try {
        if (document.fullscreenElement) document.exitFullscreen();
        else if (el.requestFullscreen) el.requestFullscreen({ navigationUI: "hide" }).catch(() => {});
        else if (el.webkitRequestFullscreen) el.webkitRequestFullscreen();
      } catch (e) { /* ignore */ }
    };
    const cp = $("vc-copy");
    if (cp) cp.onclick = () => {
      const t = $("vc-out"); const txt = t.value;
      const fallback = () => { t.removeAttribute("readonly"); t.focus(); t.select(); try { document.execCommand("copy"); } catch (e) { /* ignore */ } t.setAttribute("readonly", ""); };
      if (navigator.clipboard && window.isSecureContext) navigator.clipboard.writeText(txt).catch(fallback); else fallback();
      cp.textContent = "Copied"; setTimeout(() => { cp.textContent = "Copy my code"; }, 1500);
    };
    const clr = $("clearbtn");
    if (clr) clr.onclick = () => {
      for (const k of [...vfs.keys()]) if (k.startsWith("user/")) { vfs.delete(k); try { localStorage.removeItem("vc:" + k); } catch (e) {} }
      $("importmsg").textContent = "Removed imported files.";
      setTimeout(() => { $("importmsg").textContent = ""; }, 3000);
    };
  });
})();
