// Diagnostics — errors with file, line, column and a rendered source excerpt.
//
// Phase 1 step 2 of the mega-upgrade. Today the compiler aborts on the first
// error via panic!; this module is the target format every error will move
// to. `vulcan check` already reports in this format; the codegens' panic!
// calls convert progressively as the semantic pass grows.
pub struct Diag {
    pub line: usize, // 1-based; 0 = "no position"
    pub col: usize,  // 1-based; 0 = "no position"
    pub msg: String,
}

impl Diag {
    pub fn new(line: usize, col: usize, msg: impl Into<String>) -> Diag {
        Diag { line, col, msg: msg.into() }
    }
}

// Render diagnostics against the offending source: file:line:col + message,
// the source line, and a caret under the column. Lines that do not exist in
// the source (or diags with no position) render without an excerpt.
pub fn render(src: &str, file: &str, diags: &[Diag]) -> String {
    let lines: Vec<&str> = src.lines().collect();
    let mut out = String::new();
    for d in diags {
        out.push_str(&format!("{file}:{}:{}: error: {}\n", d.line, d.col, d.msg));
        if d.line >= 1 && d.line <= lines.len() && d.col >= 1 {
            let text = lines[d.line - 1];
            out.push_str(&format!("    {text}\n"));
            out.push_str(&format!("    {}^\n", " ".repeat(d.col - 1)));
        }
    }
    out
}
