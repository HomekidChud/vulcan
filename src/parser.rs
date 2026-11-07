// Parser + AST for Vulcan.
use crate::lexer::Tok;

#[derive(Debug, Clone, PartialEq)]
pub enum Ty {
    I64, F64, Bool, Str, Void,
    Array(Box<Ty>),
    Struct(String),
}

#[derive(Debug, Clone)]
pub enum Item {
    Fn(FnDecl),
    Struct(StructDecl),
    Enum(EnumDecl),
    Const(ConstDecl),
    Import(String),
    Impl(ImplDecl),
    Extern(ExternDecl),
    Link(String),
}

// `extern "c" { fn sigs }` — foreign function declarations (FFI).
// `abi` is "c" (C ABI: real prototypes on the native backend, host shims
// elsewhere) or "host" (functions provided by the embedding host).
#[derive(Debug, Clone)]
pub struct ExternDecl {
    pub abi: String,
    pub fns: Vec<FnDecl>, // signatures only — bodies must be empty
}

#[derive(Debug, Clone)]
pub struct ImplDecl {
    pub struct_name: String,
    pub fns: Vec<FnDecl>,
}

#[derive(Debug, Clone)]
pub struct FnDecl {
    pub line: usize, // 1-based source line of the `fn` keyword
    pub name: String,
    pub params: Vec<(String, Ty)>,
    pub ret: Ty,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub struct StructDecl {
    pub line: usize, // 1-based source line of the `struct` keyword
    pub name: String,
    pub fields: Vec<(String, Ty)>,
}

#[derive(Debug, Clone)]
pub struct EnumDecl {
    pub line: usize, // 1-based source line of the `enum` keyword
    pub name: String,
    pub variants: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ConstDecl {
    pub line: usize, // 1-based source line of the `const` keyword
    pub name: String,
    pub ty: Option<Ty>,
    pub value: ConstVal,
}

#[derive(Debug, Clone)]
pub enum ConstVal { Int(i64), Float(f64), Str(String), Bool(bool) }

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pats: Vec<Expr>,
    pub guard: Option<Expr>, // `when p if cond` — arm only matches when cond holds
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Let { name: String, ty: Option<Ty>, init: Expr },
    Assign { target: Expr, value: Expr },
    CompoundAssign { target: Expr, op: String, value: Expr },
    IncDec { target: Expr, delta: i64 },
    If { cond: Expr, then: Vec<Stmt>, els: Option<Vec<Stmt>> },
    While { cond: Expr, body: Vec<Stmt> },
    Loop { body: Vec<Stmt> },
    For { var: String, from: Expr, to: Expr, step: Option<i64>, inclusive: bool, body: Vec<Stmt> },
    ForIn { var: String, arr: Expr, body: Vec<Stmt> },
    Match { scrutinee: Expr, arms: Vec<MatchArm>, default: Option<Vec<Stmt>> },
    Defer(Vec<Stmt>),
    Assert { cond: Expr, msg: Option<Expr>, line: usize },
    Return(Option<Expr>),
    Break,
    Continue,
    Expr(Expr),
}

#[derive(Debug, Clone)]
pub enum IPart {
    Lit(String),
    E(Expr),
}

#[derive(Debug, Clone)]
pub enum Expr {
    Int(i64), Float(f64), Str(String), Bool(bool), Char(i64),
    Ident(String),
    BinOp(Box<Expr>, String, Box<Expr>),
    Call { callee: String, args: Vec<Expr> },
    Method { recv: Box<Expr>, name: String, args: Vec<Expr> },
    Field(Box<Expr>, String),
    Index(Box<Expr>, Box<Expr>),
    ArrayLit(Vec<Expr>),
    StructLit { name: String, fields: Vec<(String, Expr)> },
    Interp(Vec<IPart>),          // f"sum is {x}"
    As(Box<Expr>, Ty),           // x as f64
}

pub struct Parser {
    toks: Vec<(Tok, usize)>,
    pos: usize,
    fn_line: usize, // line of the most recent `fn` keyword (decl line stamp)
}

pub fn parse_program(toks: &[(Tok, usize)]) -> Vec<Item> {
    let mut p = Parser { toks: toks.to_vec(), pos: 0, fn_line: 0 };
    let mut items = Vec::new();
    while p.peek() != &Tok::EOF {
        items.push(p.parse_item());
    }
    items
}

impl Parser {
    fn peek(&self) -> &Tok { &self.toks[self.pos].0 }
    fn peek_line(&self) -> usize { self.toks[self.pos].1 }
    fn next(&mut self) -> Tok { let t = self.toks[self.pos].0.clone(); self.pos += 1; t }
    fn err(&self, msg: String) -> ! {
        panic!("vulcan: line {}: {}", self.peek_line(), msg);
    }
    fn expect(&mut self, t: Tok, what: &str) {
        if self.peek() != &t {
            let got = self.peek().clone();
            self.err(format!("expected {what} ({t:?}), got {got:?}"));
        }
        self.pos += 1;
    }
    fn eat(&mut self, t: &Tok) -> bool {
        if self.peek() == t { self.pos += 1; true } else { false }
    }
    fn ident(&mut self, what: &str) -> String {
        match self.next() {
            Tok::Ident(s) => s,
            other => self.err(format!("expected {what}, got {other:?}")),
        }
    }

    fn parse_ty(&mut self) -> Ty {
        match self.next() {
            Tok::Ident(s) => match s.as_str() {
                "i64" => Ty::I64, "f64" => Ty::F64, "bool" => Ty::Bool,
                "string" => Ty::Str, "void" => Ty::Void,
                _ => Ty::Struct(s),
            },
            Tok::LBracket => {
                let inner = self.parse_ty();
                self.expect(Tok::RBracket, "] in array type");
                Ty::Array(Box::new(inner))
            }
            other => self.err(format!("expected type, got {other:?}")),
        }
    }

    fn parse_item(&mut self) -> Item {
        let _ = self.eat(&Tok::KwPub); // visibility is permissive in the single-compilation-unit model
        if self.eat(&Tok::KwImport) {
            let spec = match self.next() {
                Tok::Ident(s) => s,
                Tok::Str(s) => s,
                other => self.err(format!("expected module name after import, got {other:?}")),
            };
            return Item::Import(spec);
        }
        if self.peek() == &Tok::KwConst {
            let line = self.peek_line();
            self.next();
            let name = self.ident("const name");
            let ty = if self.eat(&Tok::Colon) {
                Some(self.parse_ty())
            } else {
                None
            };
            self.expect(Tok::Eq, "= in const");
            let value = match self.next() {
                Tok::Int(v) => ConstVal::Int(v),
                Tok::Float(v) => ConstVal::Float(v),
                Tok::Str(s) => ConstVal::Str(s),
                Tok::KwTrue => ConstVal::Bool(true),
                Tok::KwFalse => ConstVal::Bool(false),
                other => self.err(format!("const value must be a literal, got {other:?}")),
            };
            return Item::Const(ConstDecl { line, name, ty, value });
        }
        if self.peek() == &Tok::KwEnum {
            let line = self.peek_line();
            self.next();
            let name = self.ident("enum name");
            self.expect(Tok::LBrace, "{ to open enum");
            let mut variants = Vec::new();
            while !self.eat(&Tok::RBrace) {
                variants.push(self.ident("enum variant"));
                self.eat(&Tok::Comma);
            }
            return Item::Enum(EnumDecl { line, name, variants });
        }
        if self.peek() == &Tok::KwStruct {
            let line = self.peek_line();
            self.next();
            let name = self.ident("struct name");
            self.expect(Tok::LBrace, "{");
            let mut fields = Vec::new();
            while !self.eat(&Tok::RBrace) {
                let fname = self.ident("field name");
                self.expect(Tok::Colon, ": after field name");
                let fty = self.parse_ty();
                fields.push((fname, fty));
                self.eat(&Tok::Comma);
            }
            return Item::Struct(StructDecl { line, name, fields });
        }
        if self.eat(&Tok::KwExtern) {
            // extern "c" { fn sig; fn sig; }   (abi defaults to "c")
            let abi = if let Tok::Str(s) = self.peek() {
                let s = s.clone();
                self.next();
                s
            } else {
                "c".to_string()
            };
            self.expect(Tok::LBrace, "{ to open extern block");
            let mut fns = Vec::new();
            while !self.eat(&Tok::RBrace) {
                self.fn_line = self.peek_line();
                self.expect(Tok::KwFn, "fn signature in extern block");
                fns.push(self.parse_fn_sig());
            }
            return Item::Extern(ExternDecl { abi, fns });
        }
        // `link "curl"` — pass -l<name> to the C linker (native backend)
        if let Tok::Ident(name) = self.peek() {
            if name.as_str() == "link" {
                self.next();
                let lib = match self.next() {
                    Tok::Str(s) => s,
                    other => self.err(format!("expected library name string after link, got {other:?}")),
                };
                self.eat(&Tok::Semi);
                return Item::Link(lib);
            }
        }
        if self.eat(&Tok::KwImpl) {
            let struct_name = self.ident("impl struct name");
            self.expect(Tok::LBrace, "{ to open impl");
            let mut fns = Vec::new();
            while !self.eat(&Tok::RBrace) {
                self.fn_line = self.peek_line();
                self.expect(Tok::KwFn, "fn in impl block");
                fns.push(self.parse_fn());
            }
            return Item::Impl(ImplDecl { struct_name, fns });
        }
        self.fn_line = self.peek_line(); // line of the `fn` keyword (peek is KwFn here)
        self.expect(Tok::KwFn, "fn (or struct/enum/const/import) at top level");
        self.parse_fn_as_item()
    }

    fn parse_fn_as_item(&mut self) -> Item {
        let f = self.parse_fn();
        Item::Fn(f)
    }

    fn parse_fn(&mut self) -> FnDecl {
        let line = self.fn_line;
        self.fn_line = 0;
        let name = self.ident("fn name");
        self.expect(Tok::LParen, "(");
        let mut params = Vec::new();
        while !self.eat(&Tok::RParen) {
            let pname = self.ident("param name");
            // `self` in impl methods takes no type annotation
            if pname == "self" && self.peek() != &Tok::Colon {
                params.push((pname, Ty::Struct(String::new())));
                self.eat(&Tok::Comma);
                continue;
            }
            self.expect(Tok::Colon, ": after param");
            let pty = self.parse_ty();
            params.push((pname, pty));
            self.eat(&Tok::Comma);
        }
        let ret = if self.eat(&Tok::Arrow) {
            self.parse_ty()
        } else {
            Ty::Void
        };
        self.expect(Tok::LBrace, "{ to open fn body");
        let body = self.parse_block();
        FnDecl { line, name, params, ret, body }
    }

    // signature-only fn (inside extern blocks): `fn name(params) -> ret;`
    fn parse_fn_sig(&mut self) -> FnDecl {
        let line = self.fn_line;
        self.fn_line = 0;
        let name = self.ident("extern fn name");
        self.expect(Tok::LParen, "(");
        let mut params = Vec::new();
        while !self.eat(&Tok::RParen) {
            let pname = self.ident("param name");
            self.expect(Tok::Colon, ": after param name");
            let pty = self.parse_ty();
            params.push((pname, pty));
            self.eat(&Tok::Comma);
        }
        let ret = if self.eat(&Tok::Arrow) {
            self.parse_ty()
        } else {
            Ty::Void
        };
        self.eat(&Tok::Semi);
        FnDecl { line, name, params, ret, body: Vec::new() }
    }

    fn parse_block(&mut self) -> Vec<Stmt> {
        let mut stmts = Vec::new();
        while !self.eat(&Tok::RBrace) {
            if self.peek() == &Tok::EOF { self.err("unexpected end of file inside block (missing })".into()); }
            stmts.push(self.parse_stmt());
        }
        stmts
    }

    fn parse_stmt(&mut self) -> Stmt {
        match self.peek().clone() {
            Tok::KwLet => {
                self.next();
                let name = self.ident("let name");
                let ty = if self.eat(&Tok::Colon) { Some(self.parse_ty()) } else { None };
                self.expect(Tok::Eq, "= in let");
                let init = self.parse_expr();
                self.eat(&Tok::Semi);
                Stmt::Let { name, ty, init }
            }
            Tok::KwIf => self.parse_if(),
            Tok::KwWhile => {
                self.next();
                let cond = self.parse_expr();
                self.expect(Tok::LBrace, "{ after while");
                let body = self.parse_block();
                Stmt::While { cond, body }
            }
            Tok::KwLoop => {
                self.next();
                self.expect(Tok::LBrace, "{ after loop");
                let body = self.parse_block();
                Stmt::Loop { body }
            }
            Tok::KwFor => {
                self.next();
                let var = self.ident("loop var");
                self.expect(Tok::KwIn, "in");
                let subject = self.parse_expr();
                if self.peek() == &Tok::DotDot || self.peek() == &Tok::DotDotEq {
                    let inclusive = self.next() == Tok::DotDotEq;
                    let to = self.parse_expr();
                    let step = if self.eat(&Tok::KwBy) {
                        Some(match self.next() {
                            Tok::Minus => match self.next() {
                                Tok::Int(v) => -v,
                                other => self.err(format!("step after `by` must be an integer literal, got {other:?}")),
                            },
                            Tok::Int(v) => v,
                            other => self.err(format!("step after `by` must be an integer literal, got {other:?}")),
                        })
                    } else { None };
                    self.expect(Tok::LBrace, "{ after for");
                    let body = self.parse_block();
                    Stmt::For { var, from: subject, to, step, inclusive, body }
                } else {
                    self.expect(Tok::LBrace, "{ after `for x in <array>`");
                    let body = self.parse_block();
                    Stmt::ForIn { var, arr: subject, body }
                }
            }
            Tok::KwMatch => {
                self.next();
                let scrutinee = self.parse_expr();
                self.expect(Tok::LBrace, "{ to open match");
                let mut arms: Vec<MatchArm> = Vec::new();
                let mut default = None;
                while !self.eat(&Tok::RBrace) {
                    if self.peek() == &Tok::KwElse {
                        self.next();
                        self.expect(Tok::LBrace, "{ after else in match");
                        default = Some(self.parse_block());
                    } else {
                        self.expect(Tok::KwWhen, "when (or else) in match");
                        let mut pats = vec![self.parse_expr()];
                        while self.eat(&Tok::Comma) { pats.push(self.parse_expr()); }
                        let guard = if self.peek() == &Tok::KwIf {
                            self.next();
                            Some(self.parse_expr())
                        } else { None };
                        self.expect(Tok::LBrace, "{ after match arm patterns");
                        let body = self.parse_block();
                        arms.push(MatchArm { pats, guard, body });
                    }
                }
                if arms.is_empty() { self.err("match needs at least one `when` arm".into()); }
                Stmt::Match { scrutinee, arms, default }
            }
            Tok::KwDefer => {
                self.next();
                self.expect(Tok::LBrace, "{ after defer");
                Stmt::Defer(self.parse_block())
            }
            Tok::KwAssert => {
                let line = self.peek_line();
                self.next();
                self.expect(Tok::LParen, "( after assert");
                let cond = self.parse_expr();
                let msg = if self.eat(&Tok::Comma) { Some(self.parse_expr()) } else { None };
                self.expect(Tok::RParen, ") to close assert");
                self.eat(&Tok::Semi);
                Stmt::Assert { cond, msg, line }
            }
            Tok::KwReturn => {
                self.next();
                if self.eat(&Tok::Semi) { return Stmt::Return(None); }
                // bare `return` before a closing brace (no semicolon, no value)
                if matches!(self.peek(), Tok::RBrace | Tok::EOF) { return Stmt::Return(None); }
                let e = self.parse_expr();
                self.eat(&Tok::Semi);
                Stmt::Return(Some(e))
            }
            Tok::KwBreak => { self.next(); self.eat(&Tok::Semi); Stmt::Break }
            Tok::KwContinue => { self.next(); self.eat(&Tok::Semi); Stmt::Continue }
            _ => {
                let e = self.parse_expr();
                if self.eat(&Tok::Eq) {
                    let value = self.parse_expr();
                    self.eat(&Tok::Semi);
                    return Stmt::Assign { target: e, value };
                }
                if let Some(op) = self.compound_op() {
                    let value = self.parse_expr();
                    self.eat(&Tok::Semi);
                    return Stmt::CompoundAssign { target: e, op, value };
                }
                if self.eat(&Tok::PlusPlus) { self.eat(&Tok::Semi); return Stmt::IncDec { target: e, delta: 1 }; }
                if self.eat(&Tok::MinusMinus) { self.eat(&Tok::Semi); return Stmt::IncDec { target: e, delta: -1 }; }
                self.eat(&Tok::Semi);
                Stmt::Expr(e)
            }
        }
    }

    fn compound_op(&mut self) -> Option<String> {
        let op = match self.peek() {
            Tok::PlusEq => "+=", Tok::MinusEq => "-=",
            Tok::StarEq => "*=", Tok::SlashEq => "/=", Tok::PercentEq => "%=",
            Tok::AmpEq => "&=", Tok::PipeEq => "|=", Tok::CaretEq => "^=",
            Tok::ShlEq => "<<=", Tok::ShrEq => ">>=",
            _ => return None,
        };
        self.pos += 1;
        Some(op.into())
    }

    fn parse_if(&mut self) -> Stmt {
        self.expect(Tok::KwIf, "if");
        let cond = self.parse_expr();
        self.expect(Tok::LBrace, "{ after if");
        let then = self.parse_block();
        let els = if self.eat(&Tok::KwElse) {
            if self.peek() == &Tok::KwIf {
                Some(vec![self.parse_if()])
            } else if self.peek() == &Tok::KwElif {
                Some(vec![self.parse_elif()])
            } else {
                self.expect(Tok::LBrace, "{ after else");
                Some(self.parse_block())
            }
        } else if self.peek() == &Tok::KwElif {
            Some(vec![self.parse_elif()])
        } else { None };
        Stmt::If { cond, then, els }
    }

    // `elif` continuing an if-chain (Python-style; an `else` before it is optional).
    fn parse_elif(&mut self) -> Stmt {
        self.expect(Tok::KwElif, "elif");
        let cond = self.parse_expr();
        self.expect(Tok::LBrace, "{ after elif");
        let then = self.parse_block();
        let els = if self.eat(&Tok::KwElse) {
            if self.peek() == &Tok::KwIf {
                Some(vec![self.parse_if()])
            } else if self.peek() == &Tok::KwElif {
                Some(vec![self.parse_elif()])
            } else {
                self.expect(Tok::LBrace, "{ after else");
                Some(self.parse_block())
            }
        } else if self.peek() == &Tok::KwElif {
            Some(vec![self.parse_elif()])
        } else { None };
        Stmt::If { cond, then, els }
    }

    fn parse_expr(&mut self) -> Expr {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Expr {
        let mut lhs = self.parse_and();
        while self.eat(&Tok::OrOr) {
            let rhs = self.parse_and();
            lhs = Expr::BinOp(Box::new(lhs), "||".into(), Box::new(rhs));
        }
        lhs
    }
    fn parse_and(&mut self) -> Expr {
        let mut lhs = self.parse_cmp();
        while self.eat(&Tok::AndAnd) {
            let rhs = self.parse_cmp();
            lhs = Expr::BinOp(Box::new(lhs), "&&".into(), Box::new(rhs));
        }
        lhs
    }
    fn parse_cmp(&mut self) -> Expr {
        let mut lhs = self.parse_bitor();
        loop {
            let op = match self.peek() {
                Tok::EqEq => "==", Tok::Ne => "!=",
                Tok::Lt => "<", Tok::Gt => ">",
                Tok::Le => "<=", Tok::Ge => ">=",
                _ => break,
            };
            self.next();
            let rhs = self.parse_bitor();
            lhs = Expr::BinOp(Box::new(lhs), op.into(), Box::new(rhs));
        }
        lhs
    }
    fn parse_bitor(&mut self) -> Expr {
        let mut lhs = self.parse_bitxor();
        while self.peek() == &Tok::Pipe {
            self.next();
            let rhs = self.parse_bitxor();
            lhs = Expr::BinOp(Box::new(lhs), "|".into(), Box::new(rhs));
        }
        lhs
    }
    fn parse_bitxor(&mut self) -> Expr {
        let mut lhs = self.parse_bitand();
        while self.peek() == &Tok::Caret {
            self.next();
            let rhs = self.parse_bitand();
            lhs = Expr::BinOp(Box::new(lhs), "^".into(), Box::new(rhs));
        }
        lhs
    }
    fn parse_bitand(&mut self) -> Expr {
        let mut lhs = self.parse_shift();
        while self.peek() == &Tok::Amp {
            self.next();
            let rhs = self.parse_shift();
            lhs = Expr::BinOp(Box::new(lhs), "&".into(), Box::new(rhs));
        }
        lhs
    }
    fn parse_shift(&mut self) -> Expr {
        let mut lhs = self.parse_add();
        loop {
            let op = match self.peek() {
                Tok::Shl => "<<", Tok::Shr => ">>",
                _ => break,
            };
            self.next();
            let rhs = self.parse_add();
            lhs = Expr::BinOp(Box::new(lhs), op.into(), Box::new(rhs));
        }
        lhs
    }
    fn parse_add(&mut self) -> Expr {
        let mut lhs = self.parse_mul();
        loop {
            let op = match self.peek() {
                Tok::Plus => "+", Tok::Minus => "-",
                _ => break,
            };
            self.next();
            let rhs = self.parse_mul();
            lhs = Expr::BinOp(Box::new(lhs), op.into(), Box::new(rhs));
        }
        lhs
    }
    fn parse_mul(&mut self) -> Expr {
        let mut lhs = self.parse_unary();
        loop {
            let op = match self.peek() {
                Tok::Star => "*", Tok::Slash => "/", Tok::Percent => "%",
                _ => break,
            };
            self.next();
            let rhs = self.parse_unary();
            lhs = Expr::BinOp(Box::new(lhs), op.into(), Box::new(rhs));
        }
        lhs
    }
    fn parse_unary(&mut self) -> Expr {
        if self.eat(&Tok::Not) {
            let e = self.parse_unary();
            return Expr::BinOp(Box::new(Expr::Bool(false)), "!".into(), Box::new(e));
        }
        if self.eat(&Tok::Minus) {
            let e = self.parse_unary();
            return Expr::BinOp(Box::new(Expr::Int(0)), "-u".into(), Box::new(e));
        }
        if self.eat(&Tok::Tilde) {
            let e = self.parse_unary();
            return Expr::BinOp(Box::new(Expr::Int(0)), "~".into(), Box::new(e));
        }
        let e = self.parse_postfix();
        if self.eat(&Tok::KwAs) {
            let ty = self.parse_ty();
            return Expr::As(Box::new(e), ty);
        }
        e
    }
    fn parse_postfix(&mut self) -> Expr {
        let mut e = self.parse_atom();
        loop {
            if self.eat(&Tok::Dot) || (self.peek() == &Tok::ColonColon) {
                if self.peek() == &Tok::ColonColon { self.pos += 1; }
                let name = self.ident("field or method");
                if self.eat(&Tok::LParen) {
                    let mut args = Vec::new();
                    while !self.eat(&Tok::RParen) {
                        args.push(self.parse_expr());
                        self.eat(&Tok::Comma);
                    }
                    e = Expr::Method { recv: Box::new(e), name, args };
                } else {
                    e = Expr::Field(Box::new(e), name);
                }
            } else if self.eat(&Tok::LBracket) {
                let idx = self.parse_expr();
                self.expect(Tok::RBracket, "] after index");
                e = Expr::Index(Box::new(e), Box::new(idx));
            } else {
                break;
            }
        }
        e
    }
    fn parse_atom(&mut self) -> Expr {
        match self.next() {
            Tok::Int(v) => Expr::Int(v),
            Tok::Char(v) => Expr::Char(v),
            Tok::Float(v) => Expr::Float(v),
            Tok::Str(s) => Expr::Str(s),
            Tok::FStr(raw) => self.parse_interp(&raw),
            Tok::KwTrue => Expr::Bool(true),
            Tok::KwFalse => Expr::Bool(false),
            Tok::LParen => {
                let e = self.parse_expr();
                self.expect(Tok::RParen, ")");
                e
            }
            Tok::LBracket => {
                let mut items = Vec::new();
                if !self.eat(&Tok::RBracket) {
                    let mut closed = false;
                    loop {
                        items.push(self.parse_expr());
                        if !self.eat(&Tok::Comma) { break; }
                        if self.eat(&Tok::RBracket) { closed = true; break; } // trailing comma allowed
                    }
                    if !closed { self.expect(Tok::RBracket, "] after array literal"); }
                }
                Expr::ArrayLit(items)
            }
            Tok::Ident(name) => {
                if self.eat(&Tok::LParen) {
                    let mut args = Vec::new();
                    while !self.eat(&Tok::RParen) {
                        args.push(self.parse_expr());
                        self.eat(&Tok::Comma);
                    }
                    return Expr::Call { callee: name, args };
                }
                if self.peek() == &Tok::LBrace {
                    // struct literal  Name { field: val, ... }
                    // Only capitalized names are treated as struct literals, and only when
                    // the brace is followed by `field:` (or `}`) — this keeps
                    // `for y in 0 .. H { ... }` / `match C { ... }` unambiguous.
                    let looks_like_literal = self.toks.get(self.pos + 1).map(|t| match &t.0 {
                        Tok::RBrace => true,
                        Tok::Ident(_) => matches!(self.toks.get(self.pos + 2).map(|t| &t.0), Some(Tok::Colon)),
                        _ => false,
                    }).unwrap_or(false);
                    if name.chars().next().map(|c| c.is_uppercase()) != Some(true) || !looks_like_literal {
                        return Expr::Ident(name);
                    }
                    self.pos += 1;
                    let mut fields = Vec::new();
                    while !self.eat(&Tok::RBrace) {
                        let fname = self.ident("struct field");
                        self.expect(Tok::Colon, ": in struct literal");
                        fields.push((fname, self.parse_expr()));
                        self.eat(&Tok::Comma);
                    }
                    return Expr::StructLit { name, fields };
                }
                Expr::Ident(name)
            }
            other => self.err(format!("unexpected token {other:?}")),
        }
    }

    // f-string: split raw content into literal / {expr} parts.
    fn parse_interp(&mut self, raw: &str) -> Expr {
        let chars: Vec<char> = raw.chars().collect();
        let mut parts: Vec<IPart> = Vec::new();
        let mut lit = String::new();
        let mut i = 0;
        while i < chars.len() {
            if chars[i] == '{' {
                if i + 1 < chars.len() && chars[i + 1] == '{' {
                    lit.push('{');
                    i += 2;
                    continue;
                }
                let mut depth = 0i64;
                let mut j = i;
                while j < chars.len() {
                    if chars[j] == '{' { depth += 1; }
                    else if chars[j] == '}' {
                        depth -= 1;
                        if depth == 0 { break; }
                    }
                    j += 1;
                }
                if j >= chars.len() { self.err("unbalanced {{ in f-string".into()); }
                let inner: String = chars[i + 1..j].iter().collect();
                if inner.trim().is_empty() { self.err("empty {} in f-string (use a positional print, or put an expression inside: {x})".into()); }
                if !lit.is_empty() { parts.push(IPart::Lit(std::mem::take(&mut lit))); }
                let toks = crate::lexer::lex(&inner);
                let mut p = Parser { toks, pos: 0, fn_line: 0 };
                let e = p.parse_expr();
                if p.peek() != &Tok::EOF { p.err(format!("unexpected token after f-string expression")); }
                parts.push(IPart::E(e));
                i = j + 1;
            } else {
                lit.push(chars[i]);
                i += 1;
            }
        }
        if !lit.is_empty() { parts.push(IPart::Lit(lit)); }
        Expr::Interp(parts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;

    fn parse(src: &str) -> Vec<Item> {
        parse_program(&lex(src))
    }

    #[test]
    fn parses_a_function_with_params() {
        let p = parse("fn add(a: i64, b: i64) -> i64 { return a + b }");
        match &p[0] {
            Item::Fn(f) => {
                assert_eq!(f.name, "add");
                assert_eq!(f.params.len(), 2);
                assert_eq!(f.params[0], ("a".into(), Ty::I64));
                assert_eq!(f.ret, Ty::I64);
            }
            other => panic!("expected Fn item, got {other:?}"),
        }
    }

    #[test]
    fn parses_struct_enum_const() {
        let p = parse("struct P { x: i64, y: i64 } enum C { Red, Green, Blue } const N: i64 = 16");
        assert!(matches!(&p[0], Item::Struct(s) if s.name == "P" && s.fields.len() == 2));
        assert!(matches!(&p[1], Item::Enum(e) if e.variants == vec!["Red", "Green", "Blue"]));
        assert!(matches!(&p[2], Item::Const(c) if c.name == "N"));
    }

    #[test]
    fn parses_array_and_struct_types() {
        let p = parse("fn f(xs: [i64], m: Mat) -> [[f64]] { return [[1.0]] }");
        match &p[0] {
            Item::Fn(f) => {
                assert_eq!(f.params[0].1, Ty::Array(Box::new(Ty::I64)));
                assert_eq!(f.params[1].1, Ty::Struct("Mat".into()));
                assert_eq!(f.ret, Ty::Array(Box::new(Ty::Array(Box::new(Ty::F64)))));
            }
            other => panic!("expected Fn item, got {other:?}"),
        }
    }

    #[test]
    fn parses_match_and_when() {
        let src = "fn d(x: i64) -> i64 { match x { when 1 { return 10 } else { return 20 } } }";
        let p = parse(src);
        assert!(matches!(&p[0], Item::Fn(_))); // full match statement parses without panic
    }

    #[test]
    fn parses_extern_and_link() {
        let p = parse("extern \"c\" { fn abs(x: i64) -> i64 } link \"m\"");
        assert!(matches!(&p[0], Item::Extern(_)));
        assert!(matches!(&p[1], Item::Link(l) if l == "m"));
    }

    #[test]
    fn parses_fstring_and_defer() {
        let src = "fn main() -> void { let n = 3 print(f\"n is {n}\") defer { print(\"bye\") } }";
        let p = parse(src);
        assert!(matches!(&p[0], Item::Fn(_)));
    }

    // Every example and stdlib file must parse without panicking — this is the
    // parser's regression net until real diagnostics land (Phase 1).
    #[test]
    fn parses_all_examples_and_libs() {
        let mut checked = 0;
        for dir in ["examples", "lib"] {
            let entries = std::fs::read_dir(dir).unwrap_or_else(|e| panic!("cannot read {dir}: {e}"));
            for entry in entries {
                let path = entry.unwrap().path();
                if path.extension().and_then(|e| e.to_str()) != Some("vulcan") { continue; }
                let src = std::fs::read_to_string(&path)
                    .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
                let items = parse_program(&lex(&src));
                assert!(!items.is_empty(), "{} parsed to zero items", path.display());
                checked += 1;
            }
        }
        assert!(checked >= 10, "expected at least 10 vulcan files, saw {checked}");
    }
}
