// ---------------------------------------------------------------------------
// `vulcan apk` — build an installable Android APK entirely on-device (Termux)
//
// Gradle + the Android SDK build-tools are x86_64-only, so they can't run on
// a phone. Termux ships native aarch64 builds of the same tools, and this
// pipeline drives them directly:
//
//   aapt package  -> unsigned APK (manifest + res + assets + R.java)
//   ecj           -> compile MainActivity.java + R.java against android.jar
//   d8 (or dx)    -> classes.dex
//   aapt add     -> classes.dex into the APK
//   zipalign      -> 4-byte alignment
//   apksigner     -> sign with an auto-generated debug keystore
//
// Everything is also written to <dir>/build.sh so the build can be re-run
// without invoking vulcan again.
// ---------------------------------------------------------------------------

pub fn apk(entry: &str) {
    let program = crate::modules::load_program(std::path::Path::new(entry));
    let js = crate::codegen_js::generate_js(&program);
    let stem = entry.trim_end_matches(".vulcan");
    let name = file_stem(stem);
    let pkg_id = format!("com.vulcan.{}", name.to_lowercase().chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>());

    let dir = format!("package-{name}-apk");
    let (_wd, webfiles) = web_bundle(stem, &js);

    let mut files: Vec<(String, String)> = vec![];
    for (n, c) in webfiles {
        files.push((format!("assets/{n}"), c));
    }
    files.push(("AndroidManifest.xml".into(), APK_MANIFEST.replace("__PKG__", &pkg_id).replace("__NAME__", &name)));
    files.push(("src/com/vulcan/app/MainActivity.java".into(), APK_MAIN_ACTIVITY.to_string()));
    files.push(("res/values/strings.xml".into(), APK_STRINGS.replace("__NAME__", &name)));
    files.push(("build.sh".into(), APK_BUILD_SH.replace("__NAME__", &name)));
    files.push(("README.md".into(), APK_README.replace("__NAME__", &name)));

    let _ = std::fs::create_dir_all(&dir);
    for (rel, content) in files {
        let path = format!("{dir}/{rel}");
        if let Some(parent) = std::path::Path::new(&path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(&path, content).unwrap();
    }
    // launcher icon (embedded 192x192 PNG)
    let _ = std::fs::create_dir_all(format!("{dir}/res/mipmap-xxhdpi"));
    std::fs::write(format!("{dir}/res/mipmap-xxhdpi/ic_launcher.png"), include_bytes!("apk_icon.png")).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let p = format!("{dir}/build.sh");
        let mut perms = std::fs::metadata(&p).unwrap().permissions();
        perms.set_mode(0o755);
        let _ = std::fs::set_permissions(p, perms);
    }

    println!("project ready: {dir}/");
    if let Err(e) = apk_build(&dir, &name) {
        eprintln!("\nvulcan: {e}");
        eprintln!("  the project is complete in {dir}/ — install any missing tools and re-run:");
        eprintln!("    bash {dir}/build.sh");
        std::process::exit(1);
    }
}

fn sh(cmd: &mut std::process::Command, what: &str) -> Result<(), String> {
    println!("  {what}");
    let out = cmd.output().map_err(|e| format!("could not run `{}`: {e}", cmd.get_program().to_string_lossy()))?;
    if !out.status.success() {
        let log = if out.stderr.is_empty() { out.stdout } else { out.stderr };
        return Err(format!("step `{}` failed:\n{}", what, String::from_utf8_lossy(&log)));
    }
    Ok(())
}

fn find_tool(names: &[&str]) -> Option<std::path::PathBuf> {
    let path = std::env::var("PATH").unwrap_or_default();
    for dir in path.split(':') {
        for n in names {
            let p = std::path::Path::new(dir).join(n);
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}

// locate a platform android.jar for ecj/d8 (Termux ships one)
fn find_android_jar() -> Option<String> {
    if let Ok(p) = std::env::var("ANDROID_JAR") {
        if std::path::Path::new(&p).is_file() {
            return Some(p);
        }
    }
    let prefix = std::env::var("PREFIX").unwrap_or_else(|_| "/data/data/com.termux/files/usr".into());
    let candidates = [
        format!("{prefix}/share/java/android.jar"),
        format!("{prefix}/usr/share/java/android.jar"),
        "/usr/share/java/android.jar".into(),
        format!("{}/android-sdk/platforms/android.jar", std::env::var("HOME").unwrap_or_default()),
        "android.jar".into(),
    ];
    for c in candidates {
        if std::path::Path::new(&c).is_file() {
            return Some(c);
        }
    }
    None
}

fn apk_build(dir: &str, name: &str) -> Result<(), String> {
    let d = dir;
    println!("building APK (Termux-native pipeline)...");

    // ---- tool checks with friendly errors ----
    let aapt = find_tool(&["aapt", "aapt2"]).ok_or_else(|| "aapt not found — install it: pkg install aapt".to_string())?;
    let is_aapt2 = aapt.file_name().map(|f| f == "aapt2").unwrap_or(false);
    let aj = find_android_jar().ok_or_else(|| {
        "android.jar not found — install the platform jar: pkg install aapt (ships android.jar in $PREFIX/share/java), or set ANDROID_JAR=/path/android.jar".to_string()
    })?;
    let ecj = find_tool(&["ecj", "javac"]).ok_or_else(|| "ecj not found — install it: pkg install ecj".to_string())?;
    let dexer = find_tool(&["d8", "dx"]).ok_or_else(|| "d8 not found — install it: pkg install d8".to_string())?;

    // ---- clean slate ----
    let _ = std::fs::remove_dir_all(format!("{d}/build"));
    std::fs::create_dir_all(format!("{d}/build")).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(format!("{d}/build/gen")).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(format!("{d}/build/obj")).map_err(|e| e.to_string())?;

    // ---- 1. resources + manifest -> base APK + R.java ----
    if !is_aapt2 {
        sh(
            std::process::Command::new(&aapt)
                .current_dir(d)
                .args(["package", "-f",
                    "-M", "AndroidManifest.xml",
                    "-S", "res",
                    "-A", "assets",
                    "-I", &aj,
                    "-J", "build/gen",
                    "-F", "build/app-unsigned.apk"]),
            "aapt: packaging manifest + resources + assets",
        )?;
    } else {
        // aapt2: compile res then link
        sh(
            std::process::Command::new(&aapt)
                .current_dir(d)
                .args(["compile", "--dir", "res", "-o", "build/res.zip"]),
            "aapt2: compiling resources",
        )?;
        sh(
            std::process::Command::new(&aapt)
                .current_dir(d)
                .args(["link", "-o", "build/app-unsigned.apk",
                    "--manifest", "AndroidManifest.xml",
                    "--java", "build/gen",
                    "-A", "assets",
                    "-I", &aj,
                    "build/res.zip"]),
            "aapt2: linking manifest + resources + assets",
        )?;
    }

    // ---- 2. compile Java (MainActivity + R) ----
    let mut jc = std::process::Command::new(&ecj);
    jc.current_dir(d)
        .arg("-d").arg("build/obj")
        .arg("-classpath").arg(&aj)
        .arg("build/gen")
        .arg("src");
    if ecj.file_name().map(|f| f == "ecj").unwrap_or(false) {
        jc.arg("-8"); // ecj wants a source level
    }
    sh(&mut jc, "ecj: compiling MainActivity + R.java")?;

    // ---- 3. dex ----
    let mut dex = std::process::Command::new(&dexer);
    dex.current_dir(d);
    if dexer.file_name().map(|f| f == "d8").unwrap_or(false) {
        dex.args(["--release", "--lib", &aj, "--output", "build", "build/obj"]);
    } else {
        dex.args(["--dex", "--output=build/classes.dex", "build/obj"]);
    }
    sh(&mut dex, "d8: dexing classes")?;
    // d8 writes build/classes.dex; dx already wrote it
    if !std::path::Path::new(&format!("{d}/build/classes.dex")).is_file() {
        return Err("d8 did not produce classes.dex".into());
    }

    // ---- 4. add classes.dex into the APK (must run inside build/) ----
    sh(
        std::process::Command::new(&aapt)
            .current_dir(format!("{d}/build"))
            .args(["add", "app-unsigned.apk", "classes.dex"]),
        "aapt: adding classes.dex",
    )?;

    // ---- 5. zipalign ----
    let zipalign = find_tool(&["zipalign"]).ok_or_else(|| "zipalign not found — install it: pkg install zipalign".to_string())?;
    sh(
        std::process::Command::new(&zipalign)
            .current_dir(d)
            .args(["-f", "4", "build/app-unsigned.apk", "build/app-aligned.apk"]),
        "zipalign",
    )?;

    // ---- 6. debug keystore (auto-generate once) ----
    let ks = format!("{d}/debug.keystore");
    if !std::path::Path::new(&ks).is_file() {
        let keytool = find_tool(&["keytool"]).ok_or_else(|| "keytool not found — install it: pkg install openjdk-17".to_string())?;
        sh(
            std::process::Command::new(&keytool)
                .args(["-genkeypair", "-keystore", &ks,
                    "-alias", "androiddebugkey",
                    "-storepass", "android", "-keypass", "android",
                    "-keyalg", "RSA", "-keysize", "2048", "-validity", "10000",
                    "-dname", "CN=Android Debug,O=Android,C=US"]),
            "keytool: generating debug keystore (first run only)",
        )?;
    }

    // ---- 7. sign ----
    let apksigner = find_tool(&["apksigner"]).ok_or_else(|| "apksigner not found — install it: pkg install apksigner".to_string())?;
    let out_apk = format!("{d}/{name}.apk");
    sh(
        std::process::Command::new(&apksigner)
            .current_dir(d)
            .args(["sign",
                "--ks", "debug.keystore",
                "--ks-key-alias", "androiddebugkey",
                "--ks-pass", "pass:android",
                "--key-pass", "pass:android",
                "--out", &out_apk,
                "build/app-aligned.apk"]),
        "apksigner: signing the APK",
    )?;

    let cwd = std::env::current_dir().map(|c| c.display().to_string()).unwrap_or_default();
    println!("\nAPK built: {cwd}/{out_apk}");
    println!("install it with:");
    println!("  pkg install android-tools   # if you don't have adb");
    println!("  adb install {cwd}/{out_apk}");
    println!("or copy it to shared storage and tap it:");
    println!("  cp {cwd}/{out_apk} ~/storage/downloads/   # after termux-setup-storage");
    println!("(Settings > install unknown apps > allow Termux, if prompted)");
    Ok(())
}

const APK_MANIFEST: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android"
    package="__PKG__"
    android:versionCode="1"
    android:versionName="1.0">

    <uses-sdk android:minSdkVersion="21" android:targetSdkVersion="33"/>

    <application
        android:label="@string/app_name"
        android:icon="@mipmap/ic_launcher"
        android:hardwareAccelerated="true"
        android:theme="@android:style/Theme.NoTitleBar.Fullscreen">

        <activity
            android:name="com.vulcan.app.MainActivity"
            android:exported="true"
            android:screenOrientation="landscape"
            android:configChanges="orientation|screenSize|keyboardHidden|screenLayout">
            <intent-filter>
                <action android:name="android.intent.action.MAIN"/>
                <category android:name="android.intent.category.LAUNCHER"/>
            </intent-filter>
        </activity>
    </application>
</manifest>
"#;

const APK_MAIN_ACTIVITY: &str = r#"package com.vulcan.app;

// Vulcan's Android shell: a fullscreen WebView written by the toolchain.
// No Gradle, no Capacitor — just the platform WebView and the game bundle.
import android.app.Activity;
import android.os.Bundle;
import android.view.View;
import android.view.WindowManager;
import android.webkit.WebView;
import android.webkit.WebSettings;

public class MainActivity extends Activity {
    private WebView web;

    @Override
    protected void onCreate(Bundle b) {
        super.onCreate(b);
        // keep the screen on while playing
        getWindow().addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);
        web = new WebView(this);
        setContentView(web);
        WebSettings s = web.getSettings();
        s.setJavaScriptEnabled(true);
        s.setDomStorageEnabled(true);
        s.setAllowFileAccess(true);
        s.setMediaPlaybackRequiresUserGesture(false);
        hideBars();
        web.loadUrl("file:///android_asset/index.html");
    }

    private void hideBars() {
        getWindow().getDecorView().setSystemUiVisibility(
            View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY
            | View.SYSTEM_UI_FLAG_FULLSCREEN
            | View.SYSTEM_UI_FLAG_HIDE_NAVIGATION
            | View.SYSTEM_UI_FLAG_LAYOUT_STABLE
            | View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN
            | View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION);
    }

    @Override
    public void onWindowFocusChanged(boolean f) {
        super.onWindowFocusChanged(f);
        if (f) hideBars();
    }

    @Override
    protected void onPause()  { super.onPause();  if (web != null) web.onPause(); }
    @Override
    protected void onResume() { super.onResume(); if (web != null) web.onResume(); }
    @Override
    public void onBackPressed() { /* let the game handle it */ }
}
"#;

const APK_STRINGS: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<resources>
    <string name="app_name">__NAME__</string>
</resources>
"#;

const APK_BUILD_SH: &str = r#"#!/bin/sh
# builds <name>.apk using Termux's native Android tools (no Gradle needed)
set -e
cd "$(dirname "$0")"

AJ="${ANDROID_JAR:-$PREFIX/share/java/android.jar}"
AAPT="$(command -v aapt || command -v aapt2)"
ECJ="$(command -v ecj || command -v javac)"
D8="$(command -v d8 || command -v dx)"

rm -rf build
mkdir -p build/gen build/obj

if command -v aapt >/dev/null; then
    aapt package -f -M AndroidManifest.xml -S res -A assets \
        -I "$AJ" -J build/gen -F build/app-unsigned.apk
else
    aapt2 compile --dir res -o build/res.zip
    aapt2 link -o build/app-unsigned.apk --manifest AndroidManifest.xml \
        --java build/gen -A assets -I "$AJ" build/res.zip
fi

if [ "$(basename "$ECJ")" = ecj ]; then
    "$ECJ" -8 -d build/obj -classpath "$AJ" build/gen src
else
    "$ECJ" -d build/obj -classpath "$AJ" build/gen src
fi

if [ "$(basename "$D8")" = d8 ]; then
    d8 --release --lib "$AJ" --output build build/obj
else
    dx --dex --output=build/classes.dex build/obj
fi

(cd build && "$AAPT" add app-unsigned.apk classes.dex)
zipalign -f 4 build/app-unsigned.apk build/app-aligned.apk

if [ ! -f debug.keystore ]; then
    keytool -genkeypair -keystore debug.keystore -alias androiddebugkey \
        -storepass android -keypass android -keyalg RSA -keysize 2048 \
        -validity 10000 -dname "CN=Android Debug,O=Android,C=US"
fi

apksigner sign --ks debug.keystore --ks-key-alias androiddebugkey \
    --ks-pass pass:android --key-pass pass:android \
    --out __NAME__.apk build/app-aligned.apk

echo "built: __NAME__.apk"
"#;

const APK_README: &str = r#"# __NAME__ (Android APK)

A Vulcan program packaged as an installable Android app: a fullscreen
WebView running the self-contained web build of the game.

## One-time setup in Termux

    pkg update && pkg upgrade
    pkg install aapt ecj d8 zipalign apksigner openjdk-17

## Build

    bash build.sh

or, from the vulcan repo:

    ./target/release/vulcan apk src-file.vulcan

## Install on this phone

    pkg install android-tools        # once, for adb
    adb install __NAME__.apk

or without adb:

    termux-setup-storage              # once
    cp __NAME__.apk ~/storage/downloads/
    # open Downloads in your file manager and tap the APK
    # (allow "install unknown apps" from Termux if prompted)

The APK is signed with the auto-generated `debug.keystore` in this folder —
fine for personal use and sideloading; generate your own keystore before
publishing to a store.
"#;

// Packaging: ship a Vulcan program anywhere — all of it OURS, no Electron,
// no Capacitor, no third-party scaffolding:
//  - web:     single self-contained HTML file (terminal emulator + web worker)
//  - desktop: native webview shell in plain C (macOS WKWebView / Linux
//             WebKitGTK; Windows uses Edge in --app mode)
//  - android: complete Gradle project + our own WebView activity
//  - ios:     complete Xcode project + our own WKWebView delegate
//  - mobile:  android + ios together
//  - linux/windows/macos: native C cross-build scripts
pub fn package(entry: &str, platform: &str) {
    let program = crate::modules::load_program(std::path::Path::new(entry));
    let js = crate::codegen_js::generate_js(&program);
    let stem = entry.trim_end_matches(".vulcan");
    let (dir, files): (String, Vec<(String, String)>) = match platform {
        "web" => web_bundle(stem, &js),
        "desktop" => desktop_bundle(stem, &js),
        "mobile" => mobile_bundle(stem, &js),
        "android" => android_bundle(stem, &js),
        "ios" => ios_bundle(stem, &js),
        "linux" | "windows" | "macos" => {
            let c_src = crate::codegen::generate(&program);
            native_bundle(stem, platform, &c_src)
        }
        _ => {
            eprintln!("vulcan: unknown platform `{platform}` (web, desktop, mobile, android, ios, linux, windows, macos)");
            std::process::exit(1);
        }
    };
    let _ = std::fs::create_dir_all(&dir);
    for (name, content) in files {
        let path = format!("{dir}/{name}");
        if let Some(parent) = std::path::Path::new(&path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
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

// Desktop: OUR OWN native webview shell — a single C file that opens the
// OS webview (macOS WKWebView / Linux WebKitGTK) on the bundled index.html.
// No Electron, no npm, no third-party runtime.
fn desktop_bundle(stem: &str, js: &str) -> (String, Vec<(String, String)>) {
    let (_webdir, webfiles) = web_bundle(stem, js);
    let name = file_stem(stem);
    let dir = format!("package-{name}-desktop");
    let mut files: Vec<(String, String)> = webfiles;
    files.push(("shell.c".into(), DESKTOP_SHELL.replace("__NAME__", &name)));
    files.push(("build.sh".into(), DESKTOP_BUILD_SH.replace("__NAME__", &name)));
    files.push(("run-windows.cmd".into(), DESKTOP_WIN_CMD.replace("__NAME__", &name)));
    files.push(("README.md".into(), DESKTOP_README.replace("__NAME__", &name)));
    (dir, files)
}

// Mobile: OUR OWN native projects, generated from scratch — a complete
// Android Studio / Gradle project and a complete Xcode project, each with a
// ~40-line WebView shell written by us. No Capacitor, no CocoaPods, no npm.
fn android_bundle(stem: &str, js: &str) -> (String, Vec<(String, String)>) {
    let (_webdir, webfiles) = web_bundle(stem, js);
    let name = file_stem(stem);
    let dir = format!("package-{name}-android");
    let mut files: Vec<(String, String)> = vec![];
    for (n, c) in webfiles { files.push((format!("app/src/main/assets/{n}"), c)); }
    files.push(("settings.gradle".into(), AND_SETTINGS.replace("__NAME__", &name)));
    files.push(("build.gradle".into(), AND_ROOT_GRADLE.into()));
    files.push(("gradle.properties".into(), AND_GRADLE_PROPS.into()));
    files.push(("app/build.gradle".into(), AND_APP_GRADLE.replace("__NAME__", &name)));
    files.push(("app/src/main/AndroidManifest.xml".into(), AND_MANIFEST.replace("__NAME__", &name)));
    files.push(("app/src/main/java/com/vulcan/app/MainActivity.java".into(), AND_MAIN_ACTIVITY.replace("__NAME__", &name)));
    files.push(("app/src/main/res/values/strings.xml".into(), AND_STRINGS.replace("__NAME__", &name)));
    files.push(("README.md".into(), AND_README.replace("__NAME__", &name)));
    (dir, files)
}

fn ios_bundle(stem: &str, js: &str) -> (String, Vec<(String, String)>) {
    let (_webdir, webfiles) = web_bundle(stem, js);
    let name = file_stem(stem);
    let dir = format!("package-{name}-ios");
    let mut files: Vec<(String, String)> = vec![];
    for (n, c) in webfiles { files.push((format!("VulcanApp/Resources/{n}"), c)); }
    files.push(("VulcanApp.xcodeproj/project.pbxproj".into(), IOS_PBXPROJ.replace("__NAME__", &name)));
    files.push(("VulcanApp/Info.plist".into(), IOS_PLIST.replace("__NAME__", &name)));
    files.push(("VulcanApp/AppDelegate.swift".into(), IOS_APPDELEGATE.replace("__NAME__", &name)));
    files.push(("README.md".into(), IOS_README.replace("__NAME__", &name)));
    (dir, files)
}

// `mobile` = both native projects in one folder
fn mobile_bundle(stem: &str, js: &str) -> (String, Vec<(String, String)>) {
    let name = file_stem(stem);
    let (adir, afiles) = android_bundle(stem, js);
    let (idir, ifiles) = ios_bundle(stem, js);
    let _ = (adir, idir);
    let dir = format!("package-{name}-mobile");
    let mut files: Vec<(String, String)> = vec![];
    for (n, c) in afiles { files.push((format!("android/{n}"), c)); }
    for (n, c) in ifiles { files.push((format!("ios/{n}"), c)); }
    files.push(("README.md".into(), MOBILE_README.replace("__NAME__", &name)));
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
let __held = new Set();
let __vfs = new Map();
let __touches = [];
onmessage = (e) => {
    const d = e.data || {};
    if (d.init) {
        for (const k in d.fs) __vfs.set(k, d.fs[k]);
        globalThis.__VULCAN_TOUCH = !!d.touch;
        runMain();
    }
    if (d.keys) { for (const k of d.keys) { __vkeys.push(k); __held.add(k); } }
    if (d.keysup) { for (const k of d.keysup) __held.delete(k); }
    if (d.mouse) { __mdx += d.mouse[0]; __mdy += d.mouse[1]; }
    if (d.touches) { __touches = d.touches; }
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
__key_down = (c) => __held.has(c);
__read_key = () => { const k = __vkeys.shift(); return k === undefined ? 0 : k; };
// IMPORTANT: __sleep stays the DEFAULT promise-based version (setTimeout).
// The worker event loop must stay alive between frames so onmessage can
// deliver key/mouse input — a spin-wait would block it forever and kill
// all input (this exact bug shipped in 1.8).
__input = () => new Promise((resolve) => {
    let line = "";
    const consume = () => {
        while (__vkeys.length > 0) {
            const k = __vkeys.shift();
            if (k === 10 || k === 13) { resolve(line); return; }
            if (k >= 32 && k < 127) line += String.fromCharCode(k);
        }
        setTimeout(consume, 25);
    };
    consume();
});
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
        touch-action: none;
    }
    #touchui { display: none; }
    .tbtn {
        position: fixed; bottom: 24px; width: 74px; height: 74px; border-radius: 50%;
        background: rgba(120, 130, 150, 0.35); border: 2px solid rgba(200, 210, 230, 0.5);
        color: #e8edf2; font-size: 13px; font-weight: bold; text-align: center;
        line-height: 70px; user-select: none; -webkit-user-select: none; z-index: 10;
    }
    #firebtn { right: 20px; }
    #jumpbtn { right: 108px; bottom: 110px; width: 60px; height: 60px; line-height: 56px; }
    .sbtn {
        position: fixed; top: 12px; right: 12px; width: 52px; height: 34px; border-radius: 8px;
        background: rgba(120, 130, 150, 0.35); border: 2px solid rgba(200, 210, 230, 0.5);
        color: #e8edf2; font-size: 12px; font-weight: bold; text-align: center;
        line-height: 30px; user-select: none; -webkit-user-select: none; z-index: 10;
    }
    #joy {
        position: fixed; bottom: 20px; left: 20px; width: 96px; height: 96px;
        border-radius: 50%; background: rgba(120, 130, 150, 0.18);
        border: 2px solid rgba(200, 210, 230, 0.35); z-index: 10;
        pointer-events: none;
    }
    #knob {
        position: absolute; left: 26px; top: 26px; width: 44px; height: 44px;
        border-radius: 50%; background: rgba(200, 210, 230, 0.45);
        pointer-events: none;
    }
    #status { padding: 8px 16px; font-size: 12px; color: #6c707a; border-top: 1px solid #23252c; }
</style>
</head>
<body>
<header><span><b>__TITLE__</b> — built with <a href="https://github.com/HomekidChud/vulcan">Vulcan</a></span><span>WASD move · F fire · R restart · drag to look</span></header>
<main><canvas id="vscreen" width="512" height="288"></canvas><div id="term"><pre id="screen"></pre></div></main>
<div id="touchui"><div id="joy"><div id="knob"></div></div><div class="tbtn" id="firebtn">FIRE</div><div class="tbtn" id="jumpbtn">JUMP</div><div class="sbtn" id="rstbtn">RST</div></div>
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
let customTouch = false; // set true when the game calls touch_custom()
worker.onmessage = (e) => {
    const d = e.data || {};
    if (d.out !== undefined) { term.feed(d.out); term.render(); }
    if (d.__vulcan_frame) {
        // full-resolution canvas frame from present() — draw it on #vscreen
        const cv = document.getElementById("vscreen");
        const termEl = document.getElementById("term");
        cv.style.display = "block";
        termEl.style.display = "none";
        if (d.lw) logW = d.lw;
        if (d.lh) logH = d.lh;
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
    if (d.touchui === 0) {
        // the game drew its own touch controls — hide the default UI and give
        // it the whole screen (raw touch points are forwarded either way)
        customTouch = true;
        const tui = document.getElementById("touchui");
        if (tui) tui.style.display = "none";
    }
    if (d.exit !== undefined) { document.getElementById("status").textContent = "program exited with code " + d.exit; worker.terminate(); }
    if (d.error) { document.getElementById("status").textContent = "error: " + d.error; }
};
// key codes are normalized to LOWERCASE character codes so they match the
// engine's KEY_* constants everywhere (web, IDE, native terminal).
const KEYMAP = { Enter: 13, Backspace: 127, Escape: 27, Tab: 9, ArrowUp: 119, ArrowDown: 115, ArrowLeft: 97, ArrowRight: 100 };
function normKey(e) {
    if (e.key.length === 1) return e.key.toLowerCase().charCodeAt(0);
    return KEYMAP[e.key] !== undefined ? KEYMAP[e.key] : null;
}
document.addEventListener("keydown", (e) => {
    const code = normKey(e);
    if (code !== null && !e.repeat) { e.preventDefault(); worker.postMessage({ keys: [code] }); }
});
document.addEventListener("keyup", (e) => {
    const code = normKey(e);
    if (code !== null) worker.postMessage({ keysup: [code] });
});
// mouse look: drag on the canvas to look around (like an orbit control)
const vscreen = document.getElementById("vscreen");
let __mdown = false;
vscreen.addEventListener("mousedown", () => { __mdown = true; });
window.addEventListener("mouseup", () => { __mdown = false; });
document.addEventListener("mousemove", (e) => {
    if (__mdown) worker.postMessage({ mouse: [e.movementX || 0, e.movementY || 0] });
});
// touch input: raw touch points are ALWAYS forwarded to the game in logical
// canvas coords via touch_count()/touch_x()/touch_y(). The logical size is
// whatever the game set with canvas_resize() (default 256x144).
// A game that wants full control calls touch_custom() — the default UI then
// disappears and every touch belongs to the game (right-side drag = look).
// Without touch_custom() the default joystick + FIRE/JUMP/RST UI stays on.
let logW = 256, logH = 144;
const isTouch = "ontouchstart" in window || navigator.maxTouchPoints > 0;
const touchUI = document.getElementById("touchui");
if (isTouch) touchUI.style.display = "block";
const K = { w: 119, a: 97, s: 115, d: 100, f: 102, space: 32 }; // lowercase codes, same as engine KEY_*
let joyId = null, joyX0 = 0, joyY0 = 0, joyKeys = [];
let lookId = null, lookX = 0, lookY = 0;
function sendTouches(e) {
    const r = vscreen.getBoundingClientRect();
    const pts = [];
    for (const t of e.touches) {
        pts.push([
            Math.round((t.clientX - r.left) / r.width * (logW - 1)),
            Math.round((t.clientY - r.top) / r.height * (logH - 1))
        ]);
    }
    worker.postMessage({ touches: pts });
}
function joyUpdate(dx, dy) {
    const want = [];
    if (dy < -18) want.push(K.w);
    if (dy > 18) want.push(K.s);
    if (dx < -18) want.push(K.a);
    if (dx > 18) want.push(K.d);
    for (const k of want) if (!joyKeys.includes(k)) worker.postMessage({ keys: [k] });
    for (const k of joyKeys) if (!want.includes(k)) worker.postMessage({ keysup: [k] });
    joyKeys = want;
    const knob = document.getElementById("knob");
    knob.style.transform = "translate(" + Math.max(-34, Math.min(34, dx)) + "px," + Math.max(-34, Math.min(34, dy)) + "px)";
}
vscreen.addEventListener("touchstart", (e) => {
    e.preventDefault();
    for (const t of e.changedTouches) {
        const r = vscreen.getBoundingClientRect();
        if (!customTouch && t.clientX - r.left < r.width * 0.45 && joyId === null) {
            joyId = t.identifier; joyX0 = t.clientX; joyY0 = t.clientY;
            const joy = document.getElementById("joy");
            joy.style.left = (t.clientX - 48) + "px";
            joy.style.top = (t.clientY - 48) + "px";
            joy.style.bottom = "auto";
        } else if (lookId === null) {
            lookId = t.identifier; lookX = t.clientX; lookY = t.clientY;
        }
    }
    sendTouches(e);
}, { passive: false });
vscreen.addEventListener("touchmove", (e) => {
    e.preventDefault();
    for (const t of e.changedTouches) {
        if (t.identifier === joyId) joyUpdate(t.clientX - joyX0, t.clientY - joyY0);
        if (t.identifier === lookId) {
            worker.postMessage({ mouse: [t.clientX - lookX, t.clientY - lookY] });
            lookX = t.clientX; lookY = t.clientY;
        }
    }
    sendTouches(e);
}, { passive: false });
vscreen.addEventListener("touchend", (e) => {
    for (const t of e.changedTouches) {
        if (t.identifier === joyId) {
            joyId = null; joyUpdate(0, 0);
            const joy = document.getElementById("joy");
            joy.style.left = "20px"; joy.style.top = "auto"; joy.style.bottom = "20px";
        }
        if (t.identifier === lookId) lookId = null;
    }
    sendTouches(e);
});
function holdBtn(el, code) {
    el.addEventListener("touchstart", (e) => { e.preventDefault(); worker.postMessage({ keys: [code] }); }, { passive: false });
    el.addEventListener("touchend", (e) => { e.preventDefault(); worker.postMessage({ keysup: [code] }); }, { passive: false });
    el.addEventListener("touchcancel", () => worker.postMessage({ keysup: [code] }));
}
holdBtn(document.getElementById("firebtn"), K.f);
holdBtn(document.getElementById("jumpbtn"), K.space);
// restart (R) — one-shot, safe to tap any time
const rstBtn = document.getElementById("rstbtn");
rstBtn.addEventListener("touchstart", (e) => {
    e.preventDefault();
    worker.postMessage({ keys: [114], keysup: [114] });
}, { passive: false });
worker.postMessage({ init: true, fs: savedFs, touch: isTouch });
</script>
</body>
</html>
"##;

// ---------------------------------------------------------------------------
// Desktop: our own native webview shell (no Electron)
// ---------------------------------------------------------------------------

const DESKTOP_SHELL: &str = r#"// Vulcan desktop shell — opens the OS webview on the bundled index.html.
// Written and shipped by the Vulcan toolchain itself. No Electron, no npm.
//   macOS:  AppKit + WKWebView   (clang shell.c -o __NAME__ -framework AppKit -framework WebKit)
//   Linux:  WebKitGTK            (cc shell.c -o __NAME__ $(pkg-config --cflags --libs webkit2gtk-4.1))
#ifdef __APPLE__
#import <AppKit/AppKit.h>
#import <WebKit/WebKit.h>
int main(void) {
    @autoreleasepool {
        NSRect r = NSMakeRect(0, 0, 800, 600);
        NSWindow *w = [[NSWindow alloc] initWithContentRect:r
            styleMask:NSWindowStyleMaskTitled | NSWindowStyleMaskClosable |
                     NSWindowStyleMaskResizable
            backing:NSBackingStoreBuffered defer:NO];
        [w setTitle:@"__NAME__"];
        WKWebView *web = [[WKWebView alloc] initWithFrame:r];
        [web loadRequest:[NSURLRequest requestWithURL:
            [NSURL fileURLWithPath:[[NSBundle mainBundle] resourcePath]
                stringByAppendingPathComponent:@"index.html"] isDirectory:NO]]];
        [w setContentView:web];
        [w makeKeyAndOrderFront:nil];
        [[NSApplication sharedApplication] run];
    }
    return 0;
}
#else
#include <gtk/gtk.h>
#include <webkit2/webkit2.h>
int main(int argc, char **argv) {
    gtk_init(&argc, &argv);
    GtkWidget *win = gtk_window_new(GTK_WINDOW_TOPLEVEL);
    gtk_window_set_title(GTK_WINDOW(win), "__NAME__");
    gtk_window_set_default_size(GTK_WINDOW(win), 800, 600);
    g_signal_connect(win, "destroy", G_CALLBACK(gtk_main_quit), NULL);
    WebKitWebView *web = WEBKIT_WEB_VIEW(webkit_web_view_new());
    char path[4096];
    realpath("index.html", path);
    webkit_web_view_load_uri(web, g_filename_to_uri(path, NULL, NULL));
    gtk_container_add(GTK_CONTAINER(win), GTK_WIDGET(web));
    gtk_widget_show_all(win);
    gtk_main();
    return 0;
}
#endif
"#;

const DESKTOP_BUILD_SH: &str = r#"set -e
NAME=__NAME__
case "$(uname -s)" in
  Darwin)
    clang -Wall -o "$NAME" shell.c -framework AppKit -framework WebKit
    echo "built ./$NAME (macOS WKWebView shell)"
    ;;
  Linux)
    if ! pkg-config --exists webkit2gtk-4.1 2>/dev/null; then
      echo "WebkitGTK 4.1 dev files required: sudo apt install libwebkit2gtk-4.1-dev (or dnf install webkit2gtk4.1-devel)"
      exit 1
    fi
    cc -Wall -o "$NAME" shell.c $(pkg-config --cflags --libs webkit2gtk-4.1)
    echo "built ./$NAME (Linux WebKitGTK shell)"
    ;;
  *)
    echo "On Windows use run-windows.cmd instead (Edge --app window)."
    exit 1
    ;;
esac
./"$NAME" &"#;

const DESKTOP_WIN_CMD: &str = r#"@echo off
rem Vulcan desktop shell for Windows: Edge (WebView2) in app mode.
rem No install, no build — Edge ships with Windows 10/11.
start msedge --app="%~dp0index.html" --window-size=800,600
echo Launched __NAME__ in an Edge app window (WebView2)."#;

const DESKTOP_README: &str = r#"# Desktop app (Vulcan's own native shell)

Your Vulcan program as a desktop app — a ~60-line C webview shell that the
Vulcan toolchain generates itself. No Electron, no npm, no bundler.

## macOS

    ./build.sh          # builds + launches (needs Xcode command line tools)

## Linux

    sudo apt install libwebkit2gtk-4.1-dev   # one-time dependency
    ./build.sh

## Windows

    double-click run-windows.cmd
    (opens the app in an Edge WebView2 app window — zero dependencies)

The shell opens `index.html` (the same self-contained web bundle) in the
operating system's own webview, so the app is small and starts instantly.
"#;

// ---------------------------------------------------------------------------
// Android: complete Gradle project, our own WebView activity (no Capacitor)
// ---------------------------------------------------------------------------

const AND_SETTINGS: &str = r#"pluginManagement {
    repositories { google(); mavenCentral(); gradlePluginPortal() }
}
dependencyResolutionManagement {
    repositories { google(); mavenCentral() }
}
rootProject.name = "__NAME__"
include ":app"
"#;

const AND_ROOT_GRADLE: &str = r#"// generated by the Vulcan toolchain
plugins {
    id "com.android.application" version "8.5.2" apply false
}
"#;

const AND_GRADLE_PROPS: &str = r#"org.gradle.jvmargs=-Xmx2048m
android.useAndroidX=true
android.nonTransitiveRClass=true
"#;

const AND_APP_GRADLE: &str = r#"plugins { id "com.android.application" }
android {
    namespace "com.vulcan.app"
    compileSdk 34
    defaultConfig {
        applicationId "com.vulcan.app.__NAME__"
        minSdk 24
        targetSdk 34
        versionCode 1
        versionName "1.0"
    }
    buildTypes {
        release {
            minifyEnabled false
        }
    }
}
"#;

const AND_MANIFEST: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<manifest xmlns:android="http://schemas.android.com/apk/res/android">
    <application
        android:label="@string/app_name"
        android:icon="@android:drawable/sym_def_app_icon"
        android:hardwareAccelerated="true">
        <activity
            android:name=".MainActivity"
            android:exported="true"
            android:configChanges="orientation|screenSize|keyboardHidden">
            <intent-filter>
                <action android:name="android.intent.action.MAIN" />
                <category android:name="android.intent.category.LAUNCHER" />
            </intent-filter>
        </activity>
    </application>
</manifest>
"#;

const AND_MAIN_ACTIVITY: &str = r#"package com.vulcan.app;

// Vulcan's own Android shell — a WebView activity written by the toolchain.
// No Capacitor, no plugins, no webview dependency: just the platform WebView.
import android.app.Activity;
import android.os.Bundle;
import android.webkit.WebView;
import android.webkit.WebSettings;

public class MainActivity extends Activity {
    private WebView web;

    @Override
    protected void onCreate(Bundle b) {
        super.onCreate(b);
        web = new WebView(this);
        setContentView(web);
        WebSettings s = web.getSettings();
        s.setJavaScriptEnabled(true);
        s.setDomStorageEnabled(true);
        s.setAllowFileAccess(true);
        // the game runs in a web worker; keep it running when the screen dims
        WebView.setWebContentsDebuggingEnabled(false);
        web.loadUrl("file:///android_asset/index.html");
    }

    @Override
    protected void onPause()   { super.onPause();   if (web != null) web.onPause(); }
    @Override
    protected void onResume()  { super.onResume();  if (web != null) web.onResume(); }
    @Override
    public void onBackPressed() { /* let the game handle it */ }
}
"#;

const AND_STRINGS: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<resources>
    <string name="app_name">__NAME__</string>
</resources>
"#;

const AND_README: &str = r#"# Android app (Vulcan's own Gradle project)

A complete Android Studio project generated by the Vulcan toolchain — no
Capacitor, no Cordova, no npm. The app is a ~40-line WebView activity we
generate, loading the self-contained web build from app/src/main/assets/.

## Build the APK

    # on a machine with Android Studio (or just the Android SDK + Gradle):
    cd __NAME__-android   # this folder
    gradle assembleDebug  # or: ./gradlew assembleDebug if you add a wrapper

The APK lands in app/build/outputs/apk/debug/app-debug.apk — install it with:

    adb install app/build/outputs/apk/debug/app-debug.apk

## Release / Play Store APK

    gradle assembleRelease   # needs a signing config in app/build.gradle

The generated project uses only the Android platform WebView (System WebView
/ Chrome), so the APK is tiny (~100 KB + your game).
"#;

// ---------------------------------------------------------------------------
// iOS: complete Xcode project, our own WKWebView delegate (no CocoaPods)
// ---------------------------------------------------------------------------

const IOS_PBXPROJ: &str = r#"// !SOURCE: generated by the Vulcan toolchain
// !$*UTF8*$!
{
	archiveVersion = 1;
	objectVersion = 56;
	classes = {};
	objects = {
/* Begin PBXFileReference section */
		1000001 /* AppDelegate.swift */ = {isa = PBXFileReference; fileEncoding = 4; lastKnownFileType = sourcecode.swift; path = AppDelegate.swift; sourceTree = "<group>"; };
		1000002 /* index.html */ = {isa = PBXFileReference; fileEncoding = 4; lastKnownFileType = text.html; path = index.html; sourceTree = "<group>"; };
		1000003 /* Info.plist */ = {isa = PBXFileReference; fileEncoding = 4; lastKnownFileType = text.plist.xml; path = Info.plist; sourceTree = "<group>"; };
		1000004 /* __NAME__.app */ = {isa = PBXFileReference; explicitFileType = wrapper.application; includeInIndex = 0; path = "__NAME__.app"; sourceTree = BUILT_PRODUCTS_DIR; };
/* End PBXFileReference section */
/* Begin PBXBuildFile section */
		2000001 /* AppDelegate.swift in Sources */ = {isa = PBXBuildFile; fileRef = 1000001; };
		2000002 /* index.html in Resources */ = {isa = PBXBuildFile; fileRef = 1000002; };
/* End PBXBuildFile section */
/* Begin PBXGroup section */
		3000001 /* VulcanApp */ = {isa = PBXGroup; children = (1000001, 1000002, 1000003); path = VulcanApp; sourceTree = "<group>"; };
		3000002 /* main */ = {isa = PBXGroup; children = (3000001); sourceTree = "<group>"; };
		3000003 /* Products */ = {isa = PBXGroup; children = (1000004); name = Products; sourceTree = "<group>"; };
		3000004 /* root */ = {isa = PBXGroup; children = (3000002, 3000003); sourceTree = "<group>"; };
/* End PBXGroup section */
/* Begin PBXNativeTarget section */
		4000001 /* __NAME__ */ = {isa = PBXNativeTarget; buildConfigurationList = 5000001; buildPhases = (4000002, 4000003, 4000004); buildRules = (); dependencies = (); name = "__NAME__"; productName = "__NAME__"; productReference = 1000004; productType = "com.apple.product-type.application"; };
		4000002 /* Sources */ = {isa = PBXSourcesBuildPhase; buildActionMask = 2147483647; files = (2000001); runOnlyForDeploymentPostprocessing = 0; };
		4000003 /* Frameworks */ = {isa = PBXFrameworksBuildPhase; buildActionMask = 2147483647; files = (); runOnlyForDeploymentPostprocessing = 0; };
		4000004 /* Resources */ = {isa = PBXResourcesBuildPhase; buildActionMask = 2147483647; files = (2000002); runOnlyForDeploymentPostprocessing = 0; };
/* End PBXNativeTarget section */
/* Begin PBXProject section */
		6000001 /* Project object */ = {isa = PBXProject; attributes = {BuildIndependentTargetsInParallel = 1; LastUpgradeCheck = 1540; }; buildConfigurationList = 5000002; compatibilityVersion = "Xcode 14.0"; developmentRegion = en; hasScannedForEncodings = 0; knownRegions = (en, Base); mainGroup = 3000004; productRefGroup = 3000003; projectDirPath = ""; projectRoot = ""; targets = (4000001); };
/* End PBXProject section */
/* Begin XCConfigurationList section */
		5000001 /* Build configuration list for PBXNativeTarget "__NAME__" */ = {isa = XCConfigurationList; buildConfigurations = (5000003); defaultConfigurationIsVisible = 0; defaultConfigurationName = Release; };
		5000002 /* Build configuration list for PBXProject "__NAME__" */ = {isa = XCConfigurationList; buildConfigurations = (5000004); defaultConfigurationIsVisible = 0; defaultConfigurationName = Release; };
/* End XCConfigurationList section */
/* Begin XCBuildConfiguration section */
		5000003 /* Release */ = {isa = XCBuildConfiguration; buildSettings = {ASSETCATALOG_COMPILER_APPICON_NAME = AppIcon; CODE_SIGN_STYLE = Automatic; CURRENT_PROJECT_VERSION = 1; GENERATE_INFOPLIST_FILE = NO; INFOPLIST_FILE = VulcanApp/Info.plist; IPHONEOS_DEPLOYMENT_TARGET = 15.0; LD_RUNPATH_SEARCH_PATHS = "$(inherited) @executable_path/Frameworks"; PRODUCT_BUNDLE_IDENTIFIER = com.vulcan.__NAME__; PRODUCT_NAME = "$(TARGET_NAME)"; SWIFT_VERSION = 5.0; TARGETED_DEVICE_FAMILY = "1,2"; }; name = Release; };
		5000004 /* Release */ = {isa = XCBuildConfiguration; buildSettings = {ALWAYS_SEARCH_USER_PATHS = NO; CLANG_ENABLE_MODULES = YES; IPHONEOS_DEPLOYMENT_TARGET = 15.0; SDKROOT = iphoneos; SWIFT_OPTIMIZATION_LEVEL = "-Owholemodule"; }; name = Release; };
/* End XCBuildConfiguration section */
	};
	rootObject = 6000001;
}
"#;

const IOS_PLIST: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key><string>en</string>
    <key>CFBundleDisplayName</key><string>__NAME__</string>
    <key>CFBundleExecutable</key><string>$(EXECUTABLE_NAME)</string>
    <key>CFBundleIdentifier</key><string>$(PRODUCT_BUNDLE_IDENTIFIER)</string>
    <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
    <key>CFBundleName</key><string>$(PRODUCT_NAME)</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleShortVersionString</key><string>1.0</string>
    <key>CFBundleVersion</key><string>1</string>
    <key>LSRequiresIPhoneOS</key><true/>
    <key>UIRequiresFullScreen</key><true/>
    <key>UISupportedInterfaceOrientations</key>
    <array>
        <string>UIInterfaceOrientationPortrait</string>
        <string>UIInterfaceOrientationLandscapeLeft</string>
        <string>UIInterfaceOrientationLandscapeRight</string>
    </array>
</dict>
</plist>
"#;

const IOS_APPDELEGATE: &str = r#"import UIKit
import WebKit

// Vulcan's own iOS shell — a WKWebView delegate written by the toolchain.
// No CocoaPods, no SwiftPM, no Capacitor: just WebKit (a system framework).
@main
class AppDelegate: UIResponder, UIApplicationDelegate {
    var window: UIWindow?

    func application(_ application: UIApplication,
                     didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?) -> Bool {
        window = UIWindow(frame: UIScreen.main.bounds)
        let web = WKWebView(frame: window!.bounds)
        web.autoresizingMask = [.flexibleWidth, .flexibleHeight]
        web.scrollView.isScrollEnabled = false
        if let url = Bundle.main.url(forResource: "index", withExtension: "html") {
            web.loadFileURL(url, allowingReadAccessTo: url.deletingLastPathComponent())
        }
        window?.rootViewController = UIViewController()
        window?.rootViewController?.view = web
        window?.makeKeyAndVisible()
        return true
    }
}
"#;

const IOS_README: &str = r#"# iOS app (Vulcan's own Xcode project)

A complete Xcode project generated by the Vulcan toolchain — no CocoaPods,
no Capacitor, no npm. The app is a ~25-line WKWebView delegate we generate,
loading the self-contained web build from VulcanApp/Resources/.

## Build & run

    open VulcanApp.xcodeproj     # on a Mac with Xcode
    # select an iPhone simulator (or a connected device) and press Cmd+R

## App Store IPA

    Product > Archive, then distribute through Xcode's organizer.
    (needs an Apple Developer account for device + store builds)

WKWebView is a system framework, so the app has zero third-party code.
"#;

const MOBILE_README: &str = r#"# Mobile apps (Vulcan's own packaging)

Generated by the Vulcan toolchain itself — no Capacitor, no Cordova, no npm:

- `android/` — a complete Gradle project with our own WebView activity.
  Build the APK on any machine with the Android SDK: `cd android && gradle assembleDebug`
- `ios/` — a complete Xcode project with our own WKWebView delegate.
  Open `ios/VulcanApp.xcodeproj` on a Mac and press Cmd+R.

Both shells run the same self-contained `index.html` (the web build), so
your game behaves identically to the browser version, including touch
controls. See the README inside each folder for details.
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
