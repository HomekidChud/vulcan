// Diagnostics — errors with file, line, column and a rendered source excerpt.
//
// Phase 1 step 2 of the mega-upgrade. Today the compiler aborts on the first
// error via panic!; this module is the target format every error will move
// to. `vulcan check` already reports in this format; the codegens' panic!
// calls convert progressively as the semantic pass grows.
// ---- did-you-mean suggestions ----
// Levenshtein edit distance over identifiers (the same idea rustc uses:
// suggest a similar name only when it is plausibly a typo of the input).
fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (n, m) = (a.len(), b.len());
    if n == 0 { return m; }
    if m == 0 { return n; }
    let mut prev: Vec<usize> = (0..=m).collect();
    let mut cur = vec![0usize; m + 1];
    for i in 1..=n {
        cur[0] = i;
        for j in 1..=m {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[m]
}

// Best candidate within rustc's confidence rule: only suggest when the edit
// distance is small relative to the name (a wrong suggestion is worse than
// none). Returns the suggestion WITHOUT backticks.
pub fn did_you_mean(input: &str, candidates: &[String]) -> Option<String> {
    let mut best: Option<(usize, String)> = None;
    for c in candidates {
        if c == input { continue; }
        let d = edit_distance(input, c);
        // confidence: distance must be <= len/3 (min 1)
        let limit = (c.len() / 3).max(1);
        if d <= limit {
            if best.as_ref().map_or(true, |(bd, _)| d < *bd) {
                best = Some((d, c.clone()));
            }
        }
    }
    best.map(|(_, c)| c)
}

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
