// Lexer for Vulcan — emits tokens tagged with source line numbers.
#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    Ident(String),
    Int(i64),
    Float(f64),
    Str(String),
    FStr(String), // f"..." — raw content; {expr} placeholders are resolved by the parser
    Char(i64),
    KwFn, KwLet, KwReturn, KwIf, KwElse, KwElif, KwWhile, KwFor, KwIn, KwBy, KwStruct,
    KwTrue, KwFalse, KwBreak, KwContinue, KwLoop,
    KwImport, KwPub, KwConst, KwEnum, KwMatch, KwWhen, KwDefer, KwAssert, KwAs, KwImpl, KwExtern,
    Plus, Minus, Star, Slash, Percent,
    PlusEq, MinusEq, StarEq, SlashEq, PercentEq,
    PlusPlus, MinusMinus,
    Amp, Pipe, Caret, Tilde, Shl, Shr, AmpEq, PipeEq, CaretEq, ShlEq, ShrEq,
    Eq, EqEq, Ne, Lt, Gt, Le, Ge, AndAnd, OrOr, Not,
    LParen, RParen, LBrace, RBrace, LBracket, RBracket, Dot, Comma, Colon, ColonColon, Semi, Arrow,
    DotDot, DotDotEq,
    EOF,
}

fn keyword(s: &str) -> Option<Tok> {
    Some(match s {
        "fn" => Tok::KwFn, "let" => Tok::KwLet, "return" => Tok::KwReturn,
        "if" => Tok::KwIf, "else" => Tok::KwElse, "elif" => Tok::KwElif,
        "while" => Tok::KwWhile, "for" => Tok::KwFor, "in" => Tok::KwIn, "by" => Tok::KwBy,
        "struct" => Tok::KwStruct, "true" => Tok::KwTrue, "false" => Tok::KwFalse,
        "break" => Tok::KwBreak, "continue" => Tok::KwContinue, "loop" => Tok::KwLoop,
        "import" => Tok::KwImport, "pub" => Tok::KwPub, "const" => Tok::KwConst,
        "enum" => Tok::KwEnum, "match" => Tok::KwMatch, "when" => Tok::KwWhen,
        "defer" => Tok::KwDefer, "assert" => Tok::KwAssert, "as" => Tok::KwAs, "impl" => Tok::KwImpl, "extern" => Tok::KwExtern,
        _ => return None,
    })
}

fn lex_escape(c: char) -> char {
    match c {
        'n' => '\n', 't' => '\t', 'r' => '\r', '0' => '\0',
        '\\' => '\\', '"' => '"', '\'' => '\'',
        other => other,
    }
}

// Lex with diagnostics: literal errors are collected (with line AND column)
// instead of aborting, so `vulcan check` can report several at once.
pub fn lex_diags(src: &str) -> (Vec<(Tok, usize)>, Vec<crate::diag::Diag>) {
    let mut toks: Vec<(Tok, usize)> = Vec::new();
    let mut diags: Vec<crate::diag::Diag> = Vec::new();
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0;
    let mut line = 1usize;
    let mut line_start = 0usize; // char index where the current line begins
    let n = chars.len();
    let col = |i: usize, line_start: usize| i - line_start + 1;
    while i < n {
        let c = chars[i];
        if c == '\n' { line += 1; line_start = i + 1; i += 1; continue; }
        if c.is_whitespace() { i += 1; continue; }
        // comments
        if c == '/' && i + 1 < n && chars[i + 1] == '/' {
            while i < n && chars[i] != '\n' { i += 1; }
            continue;
        }
        if c == '/' && i + 1 < n && chars[i + 1] == '*' {
            i += 2;
            while i + 1 < n && !(chars[i] == '*' && chars[i + 1] == '/') {
                if chars[i] == '\n' { line += 1; }
                i += 1;
            }
            i += 2;
            continue;
        }
        // char literal 'a'
        if c == '\'' {
            i += 1;
            let v = if i < n && chars[i] == '\\' && i + 1 < n {
                i += 1;
                let e = lex_escape(chars[i]);
                i += 1;
                e
            } else if i < n {
                let ch = chars[i];
                i += 1;
                ch
            } else { ' ' };
            if i < n && chars[i] == '\'' { i += 1; }
            toks.push((Tok::Char(v as i64), line));
            continue;
        }
        // numbers: hex, binary, decimal, float
        if c.is_ascii_digit() {
            let lit_start = i;
            if c == '0' && i + 1 < n && (chars[i + 1] == 'x' || chars[i + 1] == 'X') {
                let lit_start = i;
                i += 2;
                let mut v: i64 = 0;
                let mut bad = false;
                while i < n && chars[i].is_ascii_hexdigit() {
                    // checked arithmetic: a hex literal that does not fit i64
                    // is an error, not a silently wrapped value
                    match v.checked_mul(16).and_then(|v| v.checked_add(chars[i].to_digit(16).unwrap() as i64)) {
                        Some(x) => v = x,
                        None => bad = true,
                    }
                    i += 1;
                }
                if bad {
                    diags.push(crate::diag::Diag::new(line, col(lit_start, line_start),
                        "integer literal out of range for i64"));
                } else {
                    toks.push((Tok::Int(v), line));
                }
                continue;
            }
            if c == '0' && i + 1 < n && (chars[i + 1] == 'b' || chars[i + 1] == 'B') {
                let lit_start = i;
                i += 2;
                let mut v: i64 = 0;
                let mut bad = false;
                while i < n && (chars[i] == '0' || chars[i] == '1') {
                    match v.checked_mul(2).and_then(|v| v.checked_add((chars[i] == '1') as i64)) {
                        Some(x) => v = x,
                        None => bad = true,
                    }
                    i += 1;
                }
                if bad {
                    diags.push(crate::diag::Diag::new(line, col(lit_start, line_start),
                        "integer literal out of range for i64"));
                } else {
                    toks.push((Tok::Int(v), line));
                }
                continue;
            }
            let mut s = String::new();
            let mut is_float = false;
            while i < n && (chars[i].is_ascii_digit() || chars[i] == '_') {
                if chars[i] != '_' { s.push(chars[i]); }
                i += 1;
            }
            // a '.' only continues the number when followed by a digit
            // (so `0..10` ranges still lex correctly)
            if i + 1 < n && chars[i] == '.' && chars[i + 1].is_ascii_digit() {
                is_float = true;
                s.push('.');
                i += 1;
                while i < n && (chars[i].is_ascii_digit() || chars[i] == '_') {
                    if chars[i] != '_' { s.push(chars[i]); }
                    i += 1;
                }
            }
            if is_float {
                match s.parse::<f64>() {
                    Ok(v) => toks.push((Tok::Float(v), line)),
                    Err(_) => diags.push(crate::diag::Diag::new(line, col(lit_start, line_start), format!("bad float literal `{s}`"))),
                }
            } else {
                match s.parse::<i64>() {
                    Ok(v) => toks.push((Tok::Int(v), line)),
                    Err(_) => diags.push(crate::diag::Diag::new(line, col(lit_start, line_start), format!("integer literal `{s}` out of range for i64"))),
                }
            }
            continue;
        }
        // identifiers / keywords / f-strings
        if c.is_alphabetic() || c == '_' {
            let mut s = String::new();
            while i < n && (chars[i].is_alphanumeric() || chars[i] == '_') {
                s.push(chars[i]);
                i += 1;
            }
            if s == "f" && i < n && chars[i] == '"' {
                i += 1;
                let raw = lex_fstr(&chars, &mut i);
                toks.push((Tok::FStr(raw), line));
                continue;
            }
            let t = keyword(&s).unwrap_or(Tok::Ident(s));
            toks.push((t, line));
            continue;
        }
        // strings
        if c == '"' {
            i += 1;
            let mut s = String::new();
            while i < n && chars[i] != '"' {
                if chars[i] == '\\' && i + 1 < n {
                    i += 1;
                    s.push(lex_escape(chars[i]));
                } else {
                    s.push(chars[i]);
                }
                i += 1;
            }
            i += 1;
            toks.push((Tok::Str(s), line));
            continue;
        }
        // operators: longest match first
        let three = if i + 2 < n {
            Some(chars[i].to_string() + &chars[i + 1].to_string() + &chars[i + 2].to_string())
        } else { None };
        let t = if let Some(ref th) = three {
            match th.as_str() {
                "<<=" => Some(Tok::ShlEq), ">>=" => Some(Tok::ShrEq), "..=" => Some(Tok::DotDotEq),
                _ => None,
            }
        } else { None };
        if t.is_some() {
            toks.push((t.unwrap(), line));
            i += 3;
            continue;
        }
        let two = if i + 1 < n { format!("{}{}", chars[i], chars[i + 1]) } else { String::new() };
        let t = match two.as_str() {
            "==" => Some(Tok::EqEq), "!=" => Some(Tok::Ne),
            "<=" => Some(Tok::Le), ">=" => Some(Tok::Ge),
            "&&" => Some(Tok::AndAnd), "||" => Some(Tok::OrOr),
            "+=" => Some(Tok::PlusEq), "-=" => Some(Tok::MinusEq),
            "*=" => Some(Tok::StarEq), "/=" => Some(Tok::SlashEq), "%=" => Some(Tok::PercentEq),
            "++" => Some(Tok::PlusPlus), "--" => Some(Tok::MinusMinus),
            "&=" => Some(Tok::AmpEq), "|=" => Some(Tok::PipeEq), "^=" => Some(Tok::CaretEq),
            "<<" => Some(Tok::Shl), ">>" => Some(Tok::Shr),
            "->" => Some(Tok::Arrow), "::" => Some(Tok::ColonColon),
            ".." => Some(Tok::DotDot),
            _ => None,
        };
        if let Some(t) = t {
            toks.push((t, line));
            i += 2;
            continue;
        }
        let t = match c {
            '+' => Tok::Plus, '-' => Tok::Minus, '*' => Tok::Star, '/' => Tok::Slash,
            '%' => Tok::Percent, '&' => Tok::Amp, '|' => Tok::Pipe, '^' => Tok::Caret,
            '~' => Tok::Tilde, '=' => Tok::Eq, '!' => Tok::Not,
            '<' => Tok::Lt, '>' => Tok::Gt,
            '(' => Tok::LParen, ')' => Tok::RParen, '{' => Tok::LBrace, '}' => Tok::RBrace,
            '[' => Tok::LBracket, ']' => Tok::RBracket,
            '.' => Tok::Dot, ',' => Tok::Comma, ':' => Tok::Colon, ';' => Tok::Semi,
            _ => panic!("vulcan: line {line}: unexpected character `{c}`"),
        };
        toks.push((t, line));
        i += 1;
    }
    toks.push((Tok::EOF, line));
    (toks, diags)
}

// Historic entry point: lex and abort with a rendered diagnostic on the first
// literal error. New code should prefer lex_diags.
pub fn lex(src: &str) -> Vec<(Tok, usize)> {
    let (toks, diags) = lex_diags(src);
    if let Some(d) = diags.first() {
        panic!("vulcan: line {}:{}: {}", d.line, d.col, d.msg);
    }
    toks
}

// f-string body: escapes are processed outside {expr} regions; {expr} copied verbatim.
fn lex_fstr(chars: &[char], i: &mut usize) -> String {
    let mut out = String::new();
    let n = chars.len();
    while *i < n && chars[*i] != '"' {
        if chars[*i] == '{' {
            let mut depth = 0i64;
            while *i < n {
                let ch = chars[*i];
                out.push(ch);
                if ch == '{' { depth += 1; }
                else if ch == '}' {
                    depth -= 1;
                    if depth == 0 { *i += 1; break; }
                }
                *i += 1;
            }
        } else if chars[*i] == '\\' && *i + 1 < n {
            *i += 1;
            out.push(lex_escape(chars[*i]));
            *i += 1;
        } else {
            out.push(chars[*i]);
            *i += 1;
        }
    }
    *i += 1; // closing quote
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integers_decimal() {
        let t = lex("42 0 1_000_000");
        assert_eq!(t[0], (Tok::Int(42), 1));
        assert_eq!(t[1], (Tok::Int(0), 1));
        assert_eq!(t[2], (Tok::Int(1_000_000), 1));
        assert_eq!(*t.last().unwrap(), (Tok::EOF, 1));
    }

    #[test]
    fn integers_hex_and_binary() {
        let t = lex("0xFF 0xdeadBEEF 0b1010 0b0");
        assert_eq!(t[0], (Tok::Int(0xFF), 1));
        assert_eq!(t[1], (Tok::Int(0xdeadBEEF), 1));
        assert_eq!(t[2], (Tok::Int(0b1010), 1));
        assert_eq!(t[3], (Tok::Int(0), 1));
    }

    #[test]
    fn i64_max_literals_ok() {
        let t = lex("0x7FFFFFFFFFFFFFFF 9223372036854775807 0b111111111111111111111111111111111111111111111111111111111111111");
        assert_eq!(t[0], (Tok::Int(i64::MAX), 1));
        assert_eq!(t[1], (Tok::Int(i64::MAX), 1));
        assert_eq!(t[2], (Tok::Int(i64::MAX), 1));
    }

    // Regression: hex/binary literals used to silently wrap on overflow
    // (`v = v * 16 + digit` with no check). They must abort with a clear
    // message instead. (Phase 1 turns this into a real diagnostic.)
    #[test]
    #[should_panic(expected = "out of range for i64")]
    fn hex_overflow_panics() {
        lex("0xFFFFFFFFFFFFFFFFF"); // 17 hex digits
    }

    #[test]
    #[should_panic(expected = "out of range for i64")]
    fn binary_overflow_panics() {
        lex("0b1111111111111111111111111111111111111111111111111111111111111111"); // 65 bits
    }

    #[test]
    #[should_panic(expected = "out of range for i64")]
    fn decimal_overflow_panics() {
        lex("9223372036854775808"); // i64::MAX + 1
    }

    #[test]
    fn floats_and_ranges() {
        let t = lex("3.14 0..10 0..=10");
        assert_eq!(t[0], (Tok::Float(3.14), 1));
        assert_eq!(t[1], (Tok::Int(0), 1));
        assert_eq!(t[2], (Tok::DotDot, 1));
        assert_eq!(t[3], (Tok::Int(10), 1));
        assert_eq!(t[5], (Tok::DotDotEq, 1));
    }

    #[test]
    fn strings_escapes_and_fstrings() {
        let t = lex(r#" "hi\nthere" f"n={n}" "#);
        assert_eq!(t[0], (Tok::Str("hi\nthere".into()), 1));
        assert_eq!(t[1], (Tok::FStr("n={n}".into()), 1));
    }

    #[test]
    fn comments_and_lines() {
        let src = "// line one comment\nlet x = 1 // trailing\n/* block\ncomment */ let y = 2";
        let t = lex(src);
        let idents: Vec<usize> = t.iter().filter(|(k, _)| matches!(k, Tok::Ident(_)))
            .map(|(_, l)| *l).collect();
        assert_eq!(idents, vec![2, 4]); // `let x` on line 2, `let y` on line 4
    }

    #[test]
    fn keywords_and_operators() {
        let t = lex("fn let match when <- << == && ..");
        assert_eq!(t[0], (Tok::KwFn, 1));
        assert_eq!(t[1], (Tok::KwLet, 1));
        assert_eq!(t[2], (Tok::KwMatch, 1));
        assert_eq!(t[3], (Tok::KwWhen, 1));
        assert_eq!(t[4], (Tok::Lt, 1));
        assert_eq!(t[5], (Tok::Minus, 1));
        assert_eq!(t[6], (Tok::Shl, 1));
        assert_eq!(t[7], (Tok::EqEq, 1));
        assert_eq!(t[8], (Tok::AndAnd, 1));
        assert_eq!(t[9], (Tok::DotDot, 1));
    }
}
