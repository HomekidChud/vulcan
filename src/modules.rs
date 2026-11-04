// Module loader — resolves `import` statements across files.
use crate::parser::{self, Item};
use std::path::{Path, PathBuf};

// Loads `entry` and transitively expands all imports into a single program.
pub fn load_program(entry: &Path) -> Vec<Item> {
    let mut visited: Vec<PathBuf> = Vec::new();
    load_file(entry, &mut visited)
}

fn load_file(path: &Path, visited: &mut Vec<PathBuf>) -> Vec<Item> {
    let canon = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if visited.iter().any(|p| *p == canon) {
        return Vec::new(); // already expanded (also breaks import cycles)
    }
    visited.push(canon);
    let src = std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("vulcan: cannot read {}: {e}", path.display()));
    let toks = crate::lexer::lex(&src);
    let items = parser::parse_program(&toks);
    let mut out = Vec::new();
    for item in items {
        match item {
            Item::Import(spec) => {
                let target = resolve(path, &spec);
                out.extend(load_file(&target, visited));
            }
            other => out.push(other),
        }
    }
    out
}

// `import foo`          -> foo.vulcan, src/foo.vulcan, lib/foo.vulcan,
//                          deps/foo/src/lib.vulcan, deps/foo/lib.vulcan, deps/foo/src/main.vulcan
// `import "dir/x.vulcan"` -> the exact file, relative to the importing file
fn resolve(from: &Path, spec: &str) -> PathBuf {
    let dir = from.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or_else(|| Path::new("."));
    let mut candidates: Vec<PathBuf> = Vec::new();
    if spec.ends_with(".vulcan") {
        candidates.push(dir.join(spec));
    } else {
        candidates.push(dir.join(format!("{spec}.vulcan")));
        candidates.push(dir.join("src").join(format!("{spec}.vulcan")));
        candidates.push(dir.join("lib").join(format!("{spec}.vulcan")));
        candidates.push(dir.join("deps").join(spec).join("src").join("lib.vulcan"));
        candidates.push(dir.join("deps").join(spec).join("lib.vulcan"));
        candidates.push(dir.join("deps").join(spec).join("src").join("main.vulcan"));
        // project-root variants (e.g. importing from src/main.vulcan)
        if let Some(root) = dir.parent() {
            candidates.push(root.join("src").join(format!("{spec}.vulcan")));
            candidates.push(root.join("lib").join(format!("{spec}.vulcan")));
            candidates.push(root.join("deps").join(spec).join("src").join("lib.vulcan"));
            candidates.push(root.join("deps").join(spec).join("lib.vulcan"));
            candidates.push(root.join("deps").join(spec).join("src").join("main.vulcan"));
        }
    }
    for c in candidates {
        if c.exists() {
            return c;
        }
    }
    panic!("vulcan: cannot resolve import `{spec}` (searched relative to {})", from.display());
}
