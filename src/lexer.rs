// Lexer for Vulcan.
#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    Ident(String),
    Int(i64),
    Float(f64),
    Str(String),
    KwFn, KwLet, KwReturn, KwIf, KwElse, KwWhile, KwFor, KwIn, KwStruct, KwTrue, KwFalse, KwBreak, KwContinue,
    Plus, Minus, Star, Slash, Percent,
    Eq, EqEq, Ne, Lt, Gt, Le, Ge, AndAnd, OrOr, Not,
    LParen, RParen, LBrace, RBrace, LBracket, RBracket, Dot, Comma, Colon, ColonColon, Semi, Arrow,
    EOF,
}

pub fn lex(src: &str) -> Vec<Tok> {
    let mut toks = Vec::new();
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0;
    let n = chars.len();
    while i < n {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c == '/' && i + 1 < n && chars[i + 1] == '/' {
            while i < n && chars[i] != '\n' { i += 1; }
            continue;
        }
        if c == '/' && i + 1 < n && chars[i + 1] == '*' {
            i += 2;
            while i + 1 < n && !(chars[i] == '*' && chars[i + 1] == '/') { i += 1; }
            i += 2;
            continue;
        }
        if c.is_ascii_digit() || (c == '.' && i + 1 < n && chars[i + 1].is_ascii_digit()) {
            let mut s = String::new();
            let mut is_float = false;
            while i < n && (chars[i].is_ascii_digit() || chars[i] == '.' || chars[i] == '_') {
                if chars[i] == '.' { is_float = true; }
                if chars[i] != '_' { s.push(chars[i]); }
                i += 1;
            }
            if is_float {
                toks.push(Tok::Float(s.parse().unwrap()));
            } else {
                toks.push(Tok::Int(s.parse().unwrap()));
            }
            continue;
        }
        if c.is_alphabetic() || c == '_' {
            let mut s = String::new();
            while i < n && (chars[i].is_alphanumeric() || chars[i] == '_') {
                s.push(chars[i]);
                i += 1;
            }
            let t = match s.as_str() {
                "fn" => Tok::KwFn, "let" => Tok::KwLet, "return" => Tok::KwReturn,
                "if" => Tok::KwIf, "else" => Tok::KwElse, "while" => Tok::KwWhile,
                "for" => Tok::KwFor, "in" => Tok::KwIn, "struct" => Tok::KwStruct,
                "true" => Tok::KwTrue, "false" => Tok::KwFalse,
                "break" => Tok::KwBreak, "continue" => Tok::KwContinue,
                _ => Tok::Ident(s),
            };
            toks.push(t);
            continue;
        }
        if c == '"' {
            i += 1;
            let mut s = String::new();
            while i < n && chars[i] != '"' {
                if chars[i] == '\\' && i + 1 < n {
                    i += 1;
                    match chars[i] {
                        'n' => s.push('\n'), 't' => s.push('\t'), '"' => s.push('"'),
                        other => { s.push('\\'); s.push(other); }
                    }
                } else {
                    s.push(chars[i]);
                }
                i += 1;
            }
            i += 1;
            toks.push(Tok::Str(s));
            continue;
        }
        let t = match c {
            '+' => Tok::Plus, '*' => Tok::Star, '/' => Tok::Slash, '%' => Tok::Percent,
            '=' => { if i + 1 < n && chars[i + 1] == '=' { i += 1; Tok::EqEq } else { Tok::Eq } }
            '!' => { if i + 1 < n && chars[i + 1] == '=' { i += 1; Tok::Ne } else { Tok::Not } }
            '<' => { if i + 1 < n && chars[i + 1] == '=' { i += 1; Tok::Le } else { Tok::Lt } }
            '>' => { if i + 1 < n && chars[i + 1] == '=' { i += 1; Tok::Ge } else { Tok::Gt } }
            '&' => { if i + 1 < n && chars[i + 1] == '&' { i += 1; Tok::AndAnd } else { panic!("single & not supported"); } }
            '|' => { if i + 1 < n && chars[i + 1] == '|' { i += 1; Tok::OrOr } else { panic!("single | not supported"); } }
            '-' => { if i + 1 < n && chars[i + 1] == '>' { i += 1; Tok::Arrow } else { Tok::Minus } }
            '(' => Tok::LParen, ')' => Tok::RParen, '{' => Tok::LBrace, '}' => Tok::RBrace,
            '[' => Tok::LBracket, ']' => Tok::RBracket,
            '.' => { if i + 1 < n && chars[i + 1] == '.' { i += 1; Tok::ColonColon } else { Tok::Dot } },
            ',' => Tok::Comma,
            ':' => { if i + 1 < n && chars[i + 1] == ':' { i += 1; Tok::ColonColon } else { Tok::Colon } }
            ';' => Tok::Semi,
            _ => unreachable!(),
        };
        let _ = t.clone();
        toks.push(t);
        i += 1;
    }
    toks.push(Tok::EOF);
    toks
}
