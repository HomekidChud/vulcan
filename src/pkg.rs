// Vulcan package manager: projects, dependencies, lockfile.
use std::path::{Path, PathBuf};
use std::process::Command;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[derive(Debug, Clone)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub deps: Vec<(String, String)>, // name -> source spec ("git:user/repo" or "path:...")
}

// Minimal TOML subset parser for vulcan.toml (zero dependencies on purpose).
pub fn parse_manifest(src: &str) -> Manifest {
    let mut name = String::from("app");
    let mut version = String::from("0.1.0");
    let mut description = String::new();
    let mut deps = Vec::new();
    let mut section = String::new();
    for raw in src.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        if line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len() - 1].trim().to_string();
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            let key = k.trim().to_string();
            let val = v.trim().trim_matches('"').to_string();
            match section.as_str() {
                "package" => match key.as_str() {
                    "name" => name = val,
                    "version" => version = val,
                    "description" => description = val,
                    _ => {}
                },
                "dependencies" => deps.push((key, val)),
                _ => {}
            }
        }
    }
    Manifest { name, version, description, deps }
}

pub fn read_manifest(dir: &Path) -> Manifest {
    let p = dir.join("vulcan.toml");
    let src = std::fs::read_to_string(&p)
        .unwrap_or_else(|e| panic!("vulcan: cannot read {}: {e}", p.display()));
    parse_manifest(&src)
}

pub fn write_manifest(dir: &Path, m: &Manifest) {
    let mut out = String::new();
    out.push_str("[package]\n");
    out.push_str(&format!("name = \"{}\"\n", m.name));
    out.push_str(&format!("version = \"{}\"\n", m.version));
    if !m.description.is_empty() {
        out.push_str(&format!("description = \"{}\"\n", m.description));
    }
    if !m.deps.is_empty() {
        out.push_str("\n[dependencies]\n");
        for (k, v) in &m.deps {
            out.push_str(&format!("{k} = \"{v}\"\n"));
        }
    }
    std::fs::write(dir.join("vulcan.toml"), out).unwrap();
}

pub fn update_lockfile(proj: &Path, entries: &[(String, String, String)]) {
    let mut out = String::from("# vulcan lockfile — auto-generated, do not edit\n");
    for (name, source, commit) in entries {
        out.push_str(&format!("[[dep]]\nname = \"{name}\"\nsource = \"{source}\"\ncommit = \"{commit}\"\n\n"));
    }
    std::fs::write(proj.join("vulcan.lock"), out).unwrap();
}

// Walk up from `start` looking for a directory containing vulcan.toml.
pub fn find_project(start: &Path) -> Option<PathBuf> {
    let mut cur = start.to_path_buf();
    loop {
        if cur.join("vulcan.toml").exists() {
            return Some(cur);
        }
        match cur.parent() {
            Some(p) => cur = p.to_path_buf(),
            None => return None,
        }
    }
}

// `vulcan init [name]`
pub fn init(name: &str) {
    let root = Path::new(name);
    let src_dir = root.join("src");
    std::fs::create_dir_all(&src_dir).expect("cannot create project dirs");
    write_manifest(root, &Manifest {
        name: name.to_string(),
        version: "0.1.0".into(),
        description: String::new(),
        deps: vec![],
    });
    std::fs::write(root.join(".gitignore"), "/build\n*.vbin\n*.v.c\n").unwrap();
    std::fs::write(src_dir.join("main.vulcan"), MAIN_TEMPLATE).unwrap();
    println!("created Vulcan project `{name}`");
    println!("  {name}/vulcan.toml");
    println!("  {name}/src/main.vulcan");
    println!("\nnext steps:");
    println!("  cd {name}");
    println!("  vulcan run            # compile + execute");
    println!("  vulcan search <q>     # find packages in the registry");
    println!("  vulcan add <name>     # add a registry or git dependency");
    println!("  vulcan build          # produce build/{name}");
    println!("  vulcan publish        # push this project to the registry");
}

const MAIN_TEMPLATE: &str = r#"// Welcome to Vulcan!
const GREETING: string = "world"

fn main() -> void {
    print(f"hello, {GREETING}!")
    let xs = [1, 2, 3, 4, 5]
    let total = 0
    for x in xs {
        total += x * x
    }
    print(f"sum of squares: {total}")
    match total {
        when 55 { print("that is 5 triangular numbers squared") }
        else { print("math still works") }
    }
}

fn test_sum_of_squares() -> void {
    assert(1 * 1 + 2 * 2 + 3 * 3 + 4 * 4 + 5 * 5 == 55, "sum of squares")
    print("test_sum_of_squares passed")
}
"#;

// The on-disk folder name for a dependency source. STRICTLY validated:
// dependency sources are untrusted input, and `vulcan add` deletes
// `deps/<name>` before installing, so a name like `..` (from a source ending
// in `/..`) must never get through — it could delete outside deps/.
fn dep_dir_name(source: &str) -> Option<String> {
    let name = source.rsplit('/').next().unwrap_or("").trim_end_matches(".git");
    if name.is_empty() { return None; }
    // only plain file-name characters; no dots-only, no leading dot/dash,
    // no separators or parent refs can survive this
    let ok = name.len() <= 64
        && !name.starts_with('.')
        && !name.starts_with('-')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
        && name != ".";
    if ok { Some(name.to_string()) } else { None }
}

// Remove `deps/<name>` only when the canonicalized target really is inside
// the canonicalized deps/ directory (defense in depth on top of the name
// validation above).
fn safe_remove_dep(deps_dir: &Path, name: &str) {
    let target = deps_dir.join(name);
    let canon_deps = deps_dir.canonicalize().ok();
    let canon_target = target.canonicalize().ok();
    match (canon_deps, canon_target) {
        (Some(d), Some(t)) if t.starts_with(&d) && t != d => {
            let _ = std::fs::remove_dir_all(&t);
        }
        // target doesn't exist yet — fine, nothing to clean
        (Some(_), None) => {}
        _ => panic!("vulcan: refusing to remove {} (outside deps/)", target.display()),
    }
}

// `vulcan add <source>` — source is a registry name, "user/repo" (git) or a
// local path. `name_override` pins the dependency name (used by the registry
// resolution and `vulcan update`, so `import <name>` matches the manifest key).
pub fn add(proj: &Path, source: &str) {
    add_named(proj, source, None);
}

pub fn add_named(proj: &Path, source: &str, name_override: Option<&str>) {
    // A bare name (no '/', not a local path, not a URL) resolves through the
    // package registry: `vulcan add snake` -> the registry's user/repo.
    let source_owned;
    let source = if !source.contains('/') && !source.contains("://") && !Path::new(source).exists() {
        match registry_index() {
            Some(index) => {
                match index.iter().find(|(name, _, _, _)| name == source) {
                    Some((_, repo, _, _)) => { source_owned = repo.clone(); &source_owned }
                    None => {
                        eprintln!("vulcan: `{source}` is not in the package registry");
                        eprintln!("       search with:  vulcan search {source}");
                        std::process::exit(1);
                    }
                }
            }
            None => {
                eprintln!("vulcan: `{source}` is not a local path, and the registry is unreachable");
                eprintln!("       (add packages as user/repo, or check your connection)");
                std::process::exit(1);
            }
        }
    } else {
        source
    };
    let deps_dir = proj.join("deps");
    std::fs::create_dir_all(&deps_dir).unwrap();
    // the dependency name: the override (registry name / manifest key) when
    // given and valid, otherwise the last path segment of the source
    let name = match name_override {
        Some(n) if dep_dir_name(n).is_some() => dep_dir_name(n).unwrap(),
        _ => match dep_dir_name(source) {
            Some(n) => n,
            None => panic!("vulcan: invalid dependency source `{source}`: cannot derive a safe folder name"),
        },
    };
    let target = deps_dir.join(&name);
    safe_remove_dep(&deps_dir, &name);
    let (spec, commit);
    if source.contains("://") || (!Path::new(source).exists() && source.contains('/')) {
        // git dependency: user/repo
        println!("fetching {source} ...");
        let url = if source.contains("://") { source.to_string() } else { format!("https://github.com/{source}") };
        let ok = Command::new("git")
            // `--` ends option parsing so a URL starting with `-` is treated
            // as a value, not a git option
            .args(["clone", "--depth", "1", "--", &url, &target.to_string_lossy()])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !ok {
            panic!("vulcan: failed to clone {url}");
        }
        commit = git_head(&target).unwrap_or_else(|| "unknown".into());
        spec = format!("git:{source}");
    } else {
        // local path dependency: copy into deps/<name>
        let src = Path::new(source);
        if !src.exists() { panic!("vulcan: no such dependency source: {source}"); }
        copy_dir(src, &target);
        spec = format!("path:deps/{name}");
        commit = "local".into();
    }
    let dep_name = name;
    let mut m = read_manifest(proj);
    m.deps.retain(|(k, _)| *k != dep_name);
    m.deps.push((dep_name.clone(), spec.clone()));
    write_manifest(proj, &m);
    let mut entries: Vec<(String, String, String)> = m.deps.iter()
        .map(|(k, v)| (k.clone(), v.clone(), String::new()))
        .collect();
    if let Some(e) = entries.iter_mut().find(|e| e.0 == dep_name) {
        e.2 = commit.clone();
    }
    update_lockfile(proj, &entries);
    println!("added dependency `{dep_name}` ({spec})");
    println!("import it in code with:  import {dep_name}");
}

fn git_head(dir: &Path) -> Option<String> {
    let out = Command::new("git")
        .args(["-C", &dir.to_string_lossy(), "rev-parse", "HEAD"])
        .output()
        .ok()?;
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

// ---------------------------------------------------------------------------
// The package registry — a plain-text index in a public GitHub repo
// (HomekidChud/vulcan-registry), one line per package:
//   name|user/repo|description|version
// Reads go through raw.githubusercontent.com (no auth); writes (publish) go
// through the `gh` CLI, which is already authenticated.
// ---------------------------------------------------------------------------

pub const REGISTRY_REPO: &str = "HomekidChud/vulcan-registry";
const REGISTRY_INDEX_URL: &str = "https://raw.githubusercontent.com/HomekidChud/vulcan-registry/main/index";

fn run_cmd(cmd: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(cmd).args(args).output().ok()?;
    if !out.status.success() { return None; }
    Some(String::from_utf8_lossy(&out.stdout).to_string())
}

/// Fetch a URL's body with curl (or wget). Returns None when neither tool
/// exists or the request fails — callers print their own diagnostics.
pub fn http_get(url: &str) -> Option<String> {
    if let Some(s) = run_cmd("curl", &["-fsSL", "--", url]) { return Some(s); }
    if let Some(s) = run_cmd("wget", &["-qO-", "--", url]) { return Some(s); }
    None
}

/// Download a URL to a file with curl (or wget).
pub fn http_download(url: &str, dest: &Path) -> bool {
    let d = dest.to_string_lossy();
    let ok = Command::new("curl").args(["-fsSL", "-o", &d, "--", url]).status()
        .map(|s| s.success()).unwrap_or(false)
        || Command::new("wget").args(["-q", "--", url, "-O", &d]).status()
            .map(|s| s.success()).unwrap_or(false);
    ok && dest.exists()
}

/// The parsed registry index (name, repo, description, version) per line.
pub fn registry_index() -> Option<Vec<(String, String, String, String)>> {
    let body = http_get(REGISTRY_INDEX_URL)?;
    let mut out = Vec::new();
    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 2 {
            out.push((
                parts[0].to_string(),
                parts[1].to_string(),
                parts.get(2).unwrap_or(&"").to_string(),
                parts.get(3).unwrap_or(&"0.0.0").to_string(),
            ));
        }
    }
    Some(out)
}

/// `vulcan search <query>` — list registry packages matching name/description.
pub fn search(query: &str) {
    let index = registry_index().unwrap_or_else(|| {
        eprintln!("vulcan: could not fetch the package index ({REGISTRY_INDEX_URL})");
        eprintln!("         check your connection — curl or wget is required");
        std::process::exit(1);
    });
    let q = query.to_lowercase();
    let hits: Vec<_> = index.iter().filter(|(name, _, desc, _)| {
        q.is_empty() || name.to_lowercase().contains(&q) || desc.to_lowercase().contains(&q)
    }).collect();
    if hits.is_empty() {
        println!("no packages matching `{query}` (registry has {} packages)", index.len());
        return;
    }
    println!("{:<20} {:<24} {}", "PACKAGE", "VERSION", "DESCRIPTION");
    for (name, _, desc, ver) in hits {
        println!("{:<20} {:<24} {}", name, ver, desc);
    }
    println!("\nadd one with:  vulcan add <name>");
}

/// `vulcan remove <name>` — drop a dependency from the manifest, lockfile and deps/.
pub fn remove(proj: &Path, name: &str) {
    if dep_dir_name(name).is_none() {
        panic!("vulcan: invalid dependency name `{name}`");
    }
    let mut m = read_manifest(proj);
    let before = m.deps.len();
    m.deps.retain(|(k, _)| k != name);
    if m.deps.len() == before {
        eprintln!("vulcan: `{name}` is not a dependency of this project");
        std::process::exit(1);
    }
    write_manifest(proj, &m);
    let entries: Vec<(String, String, String)> = m.deps.iter()
        .map(|(k, v)| (k.clone(), v.clone(), String::new()))
        .collect();
    update_lockfile(proj, &entries);
    safe_remove_dep(&proj.join("deps"), name);
    println!("removed dependency `{name}`");
}

/// `vulcan update` — re-fetch every dependency at its manifest spec.
pub fn update_all(proj: &Path) {
    let m = read_manifest(proj);
    if m.deps.is_empty() {
        println!("no dependencies — add one with `vulcan add <name>`");
        return;
    }
    for (name, spec) in &m.deps {
        let source = spec.strip_prefix("git:").or_else(|| spec.strip_prefix("path:")).unwrap_or(name);
        println!("updating {name} ({source}) ...");
        // pin the dependency name to the manifest key
        add_named(proj, source, Some(name));
    }
}

/// `vulcan publish` — register this project in the package registry.
/// Requires: a vulcan.toml, a clean-enough git repo, a GitHub remote, and the
/// `gh` CLI (authenticated) to write to the registry repo.
pub fn publish(proj: &Path) {
    let m = read_manifest(proj);
    if m.name.is_empty() || m.version.is_empty() {
        eprintln!("vulcan: vulcan.toml needs a name and a version before publishing");
        std::process::exit(1);
    }
    // resolve the GitHub remote (owner/repo) — host-agnostic: take the last
    // two path segments of the origin URL (works for github.com, SSH remotes
    // and enterprise mirrors/proxies that keep owner/repo in the path)
    let remote = run_cmd("git", &["-C", &proj.to_string_lossy(), "remote", "get-url", "origin"]);
    let remote = remote.unwrap_or_default();
    let repo = if remote.contains("//") || remote.contains('@') || remote.contains('/') {
        let r = remote.trim().trim_end_matches(".git").to_string();
        if let Some(rest) = r.strip_prefix("git@github.com:") {
            rest.to_string()
        } else {
            let segs: Vec<&str> = r.split('/').filter(|s| !s.is_empty()).collect();
            if segs.len() >= 2 {
                format!("{}/{}", segs[segs.len() - 2], segs[segs.len() - 1])
            } else { String::new() }
        }
    } else {
        String::new()
    };
    if repo.is_empty() || !repo.contains('/') {
        eprintln!("vulcan: publish needs a GitHub remote named `origin` (owner/repo)");
        std::process::exit(1);
    }
    // build the new index line
    let line = format!("{}|{}|{}|{}", m.name, repo, m.description, m.version);
    // fetch the current index + its SHA (gh api: authenticated, works for the
    // registry repo owner and public readers alike)
    let api = format!("repos/{REGISTRY_REPO}/contents/index");
    let meta = run_cmd("gh", &["api", &api, "--jq", ".sha"]).unwrap_or_default();
    let sha = meta.trim();
    if sha.is_empty() {
        eprintln!("vulcan: could not read the registry index — is the `gh` CLI installed and authenticated?");
        std::process::exit(1);
    }
    let current = run_cmd("gh", &["api", &api, "--jq", ".content"]).unwrap_or_default();
    // .content is base64 with newlines
    let b64: String = current.chars().filter(|c| !c.is_whitespace()).collect();
    let decoded = base64_decode(&b64).unwrap_or_default();
    // replace or append this package's line
    let decoded_str = String::from_utf8_lossy(&decoded).to_string();
    let mut lines: Vec<String> = decoded_str.lines().map(|l| l.to_string()).collect();
    lines.retain(|l| !l.starts_with(&format!("{}|", m.name)));
    lines.push(line);
    let new_index = lines.join("\n") + "\n";
    // write it back via the contents API
    let encoded = base64_encode(new_index.as_bytes());
    let msg = format!("publish {} {}", m.name, m.version);
    let body = format!("{{\"message\":\"{msg}\",\"content\":\"{encoded}\",\"sha\":\"{sha}\"}}");
    let tmp = std::env::temp_dir().join(format!("vulcan-publish-{}.json", std::process::id()));
    std::fs::write(&tmp, body).unwrap();
    let out = Command::new("gh").args(["api", "-X", "PUT", &api, "--input", &tmp.to_string_lossy()]).output();
    let _ = std::fs::remove_file(&tmp);
    match out {
        Ok(o) if o.status.success() => {
            println!("published `{}` {} -> github.com/{repo}", m.name, m.version);
            println!("install it anywhere with:  vulcan add {}", m.name);
        }
        _ => {
            eprintln!("vulcan: publish failed — the registry write was rejected");
            eprintln!("         (are you a collaborator on {REGISTRY_REPO}?)");
            std::process::exit(1);
        }
    }
}

/// `vulcan self-update` — replace this binary with the latest release asset.
pub fn self_update() {
    let os = std::env::consts::OS;                 // "linux" | "macos" | ...
    let arch = std::env::consts::ARCH;              // "x86_64" | "aarch64" | ...
    let asset = format!("vulcan-latest-{os}-{arch}.tar.gz");
    let url = format!("https://github.com/HomekidChud/vulcan/releases/latest/download/{asset}");
    let tmp = std::env::temp_dir().join(format!("vulcan-selfupdate-{}.tar.gz", std::process::id()));
    if !http_download(&url, &tmp) {
        eprintln!("vulcan: could not download {asset}");
        eprintln!("         releases: https://github.com/HomekidChud/vulcan/releases");
        std::process::exit(1);
    }
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("vulcan"));
    let dir = tmp.parent().unwrap().join(format!("vulcan-su-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let ok = Command::new("tar").args(["-xzf", &tmp.to_string_lossy(), "-C", &dir.to_string_lossy()])
        .status().map(|s| s.success()).unwrap_or(false);
    let _ = std::fs::remove_file(&tmp);
    if !ok {
        eprintln!("vulcan: failed to extract the release archive");
        std::process::exit(1);
    }
    let new_bin = dir.join("vulcan");
    if !new_bin.exists() {
        eprintln!("vulcan: the archive did not contain a `vulcan` binary");
        std::process::exit(1);
    }
    // write the new binary next to the old one, then swap (rename is atomic
    // on the same filesystem; write-to-temp + rename avoids self-overwrite)
    let staged = exe.with_extension("vulcan-new");
    if std::fs::copy(&new_bin, &staged).is_err() {
        eprintln!("vulcan: could not stage the update (permissions?)");
        std::process::exit(1);
    }
    let _ = std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755));
    if std::fs::rename(&staged, &exe).is_err() {
        eprintln!("vulcan: could not replace {} — replace it manually from {}",
            exe.display(), new_bin.display());
        std::process::exit(1);
    }
    let _ = std::fs::remove_dir_all(&dir);
    println!("updated: {} (restart your shell or re-run `vulcan --version`)", exe.display());
}

// minimal base64 (standard alphabet) — encode/decode without external crates
const B64: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn base64_encode(data: &[u8]) -> String {
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(B64[(n >> 18) as usize & 63] as char);
        out.push(B64[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 { B64[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if chunk.len() > 2 { B64[n as usize & 63] as char } else { '=' });
    }
    out
}

pub fn base64_decode(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut acc = 0u32;
    let mut bits = 0u32;
    for c in s.chars() {
        if c == '=' || c.is_whitespace() { continue; }
        let v = B64.iter().position(|&b| b as char == c)? as u32;
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    Some(out)
}

fn copy_dir(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).unwrap();
    for entry in std::fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            if entry.file_name() == ".git" { continue; }
            copy_dir(&from, &to);
        } else {
            std::fs::copy(&from, &to).unwrap();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Regression: a source ending in `/..` (or `/.`, or empty) must never
    // produce a folder name that escapes deps/ when remove_dir_all runs.
    #[test]
    fn dep_names_are_sanitized() {
        assert_eq!(dep_dir_name("user/repo"), Some("repo".into()));
        assert_eq!(dep_dir_name("https://github.com/user/repo.git"), Some("repo".into()));
        assert_eq!(dep_dir_name("user/my-lib_2"), Some("my-lib_2".into()));
        // traversal / hostile names rejected
        assert_eq!(dep_dir_name("user/repo/.."), None); // last segment is `..`
        assert_eq!(dep_dir_name("user/repo/."), None);
        assert_eq!(dep_dir_name("user/repo/"), None);
        assert_eq!(dep_dir_name(""), None);
        assert_eq!(dep_dir_name("-evil"), None);
        assert_eq!(dep_dir_name(".hidden"), None);
        assert_eq!(dep_dir_name("a/b/c d"), None); // space not allowed
    }

    #[test]
    fn safe_remove_dep_stays_inside_deps() {
        let tmp = std::env::temp_dir().join(format!("vulcan-pkg-test-{}", std::process::id()));
        let deps = tmp.join("deps");
        let outside = tmp.join("keepme.txt");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&deps).unwrap();
        std::fs::write(&outside, "important").unwrap();
        // removing a normal dep works
        std::fs::create_dir_all(deps.join("dep")).unwrap();
        safe_remove_dep(&deps, "dep");
        assert!(!deps.join("dep").exists());
        // the guard itself refuses traversal-shaped names (they are rejected
        // earlier by dep_dir_name; safe_remove_dep must also never touch
        // anything outside deps/)
        let outside_canon = outside.canonicalize().unwrap();
        assert!(outside_canon.exists()); // untouched
        std::fs::remove_dir_all(&tmp).unwrap();
    }
}
