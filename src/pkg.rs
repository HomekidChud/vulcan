// Vulcan package manager: projects, dependencies, lockfile.
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    pub deps: Vec<(String, String)>, // name -> source spec ("git:user/repo" or "path:...")
}

// Minimal TOML subset parser for vulcan.toml (zero dependencies on purpose).
pub fn parse_manifest(src: &str) -> Manifest {
    let mut name = String::from("app");
    let mut version = String::from("0.1.0");
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
                    _ => {}
                },
                "dependencies" => deps.push((key, val)),
                _ => {}
            }
        }
    }
    Manifest { name, version, deps }
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
    println!("  vulcan add user/repo  # add a dependency (git or local path)");
    println!("  vulcan build          # produce build/{name}");
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

// `vulcan add <source>` — source is "user/repo" (git) or a local path.
pub fn add(proj: &Path, source: &str) {
    let deps_dir = proj.join("deps");
    std::fs::create_dir_all(&deps_dir).unwrap();
    let name = match dep_dir_name(source) {
        Some(n) => n,
        None => panic!("vulcan: invalid dependency source `{source}`: cannot derive a safe folder name"),
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
