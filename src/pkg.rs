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

// `vulcan add <source>` — source is "user/repo" (git) or a local path.
pub fn add(proj: &Path, source: &str) {
    let deps_dir = proj.join("deps");
    std::fs::create_dir_all(&deps_dir).unwrap();
    let (dep_name, spec, commit);
    if source.contains("://") || (!Path::new(source).exists() && source.contains('/')) {
        // git dependency: user/repo
        let name = source.rsplit('/').next().unwrap_or("dep").to_string();
        let target = deps_dir.join(&name);
        if target.exists() {
            std::fs::remove_dir_all(&target).unwrap();
        }
        println!("fetching {source} ...");
        let url = if source.contains("://") { source.to_string() } else { format!("https://github.com/{source}") };
        let ok = Command::new("git")
            .args(["clone", "--depth", "1", &url, &target.to_string_lossy()])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !ok {
            panic!("vulcan: failed to clone {url}");
        }
        commit = git_head(&target).unwrap_or_else(|| "unknown".into());
        dep_name = name;
        spec = format!("git:{source}");
    } else {
        // local path dependency: copy into deps/<name>
        let src = Path::new(source);
        if !src.exists() { panic!("vulcan: no such dependency source: {source}"); }
        let name = src.file_name().map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "dep".into());
        let target = deps_dir.join(&name);
        if target.exists() {
            std::fs::remove_dir_all(&target).unwrap();
        }
        copy_dir(src, &target);
        dep_name = name;
        spec = format!("path:deps/{dep_name}");
        commit = "local".into();
    }
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
