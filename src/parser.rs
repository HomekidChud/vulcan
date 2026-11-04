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
}

#[derive(Debug, Clone)]
pub struct FnDecl {
    pub name: String,
    pub params: Vec<(String, Ty)>,
    pub ret: Ty,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub struct StructDecl {
    pub name: String,
    pub fields: Vec<(String, Ty)>,
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Let { name: String, ty: Option<Ty>, init: Expr },
    Assign { target: Expr, value: Expr },
    If { cond: Expr, then: Vec<Stmt>, els: Option<Vec<Stmt>> },
    While { cond: Expr, body: Vec<Stmt> },
    For { var: String, from: Expr, to: Expr, body: Vec<Stmt> },
    Return(Option<Expr>),
    Break,
    Continue,
    Expr(Expr),
}

#[derive(Debug, Clone)]
pub enum Expr {
    Int(i64), Float(f64), Str(String), Bool(bool),
    Ident(String),
    BinOp(Box<Expr>, String, Box<Expr>),
    Call { callee: String, args: Vec<Expr> },
    Method { recv: Box<Expr>, name: String, args: Vec<Expr> },
    Field(Box<Expr>, String),
    Index(Box<Expr>, Box<Expr>),
    ArrayLit(Vec<Expr>),
    StructLit { name: String, fields: Vec<(String, Expr)> },
}

pub struct Parser {
    toks: Vec<Tok>,
    pos: usize,
}

pub fn parse_program(toks: &[Tok]) -> Vec<Item> {
    let mut p = Parser { toks: toks.to_vec(), pos: 0 };
    let mut items = Vec::new();
    while p.peek() != &Tok::EOF {
        items.push(p.parse_item());
    }
    items
}

impl Parser {
    fn peek(&self) -> &Tok { &self.toks[self.pos] }
    fn next(&mut self) -> Tok { let t = self.toks[self.pos].clone(); self.pos += 1; t }
    fn expect(&mut self, t: Tok, what: &str) {
        if self.peek() != &t {
            panic!("parse error: expected {what}, got {:?} (near token #{})", t, self.pos);
        }
        self.pos += 1;
    }
    fn eat(&mut self, t: &Tok) -> bool {
        if self.peek() == t { self.pos += 1; true } else { false }
    }
    fn ident(&mut self, what: &str) -> String {
        match self.next() {
            Tok::Ident(s) => s,
            other => panic!("parse error: expected {what}, got {other:?}"),
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
            other => panic!("parse error: expected type, got {other:?}"),
        }
    }

    fn parse_item(&mut self) -> Item {
        if self.eat(&Tok::KwStruct) {
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
            return Item::Struct(StructDecl { name, fields });
        }
        self.expect(Tok::KwFn, "fn");
        let name = self.ident("fn name");
        self.expect(Tok::LParen, "(");
        let mut params = Vec::new();
        while !self.eat(&Tok::RParen) {
            let pname = self.ident("param name");
            self.expect(Tok::Colon, ": after param");
            let pty = self.parse_ty();
            params.push((pname, pty));
            self.eat(&Tok::Comma);
        }
        self.expect(Tok::Arrow, "'->' before return type");
        let ret = self.parse_ty();
        self.expect(Tok::LBrace, "{ to open fn body");
        let body = self.parse_block();
        Item::Fn(FnDecl { name, params, ret, body })
    }

    fn parse_block(&mut self) -> Vec<Stmt> {
        let mut stmts = Vec::new();
        while !self.eat(&Tok::RBrace) {
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
            Tok::KwIf => {
                self.next();
                let cond = self.parse_expr();
                self.expect(Tok::LBrace, "{ after if");
                let then = self.parse_block();
                let els = if self.eat(&Tok::KwElse) {
                    if self.eat(&Tok::KwIf) {
                        // else if — desugar into nested if
                        let cond2 = self.parse_expr();
                        self.expect(Tok::LBrace, "{ after else if");
                        let then2 = self.parse_block();
                        let els2 = if self.eat(&Tok::KwElse) {
                            self.expect(Tok::LBrace, "{ after else");
                            Some(self.parse_block())
                        } else { None };
                        Some(vec![Stmt::If { cond: cond2, then: then2, els: els2 }])
                    } else {
                        self.expect(Tok::LBrace, "{ after else");
                        Some(self.parse_block())
                    }
                } else { None };
                Stmt::If { cond, then, els }
            }
            Tok::KwWhile => {
                self.next();
                let cond = self.parse_expr();
                self.expect(Tok::LBrace, "{ after while");
                let body = self.parse_block();
                Stmt::While { cond, body }
            }
            Tok::KwFor => {
                self.next();
                let var = self.ident("loop var");
                self.expect(Tok::KwIn, "in");
                let from = self.parse_expr();
                self.expect(Tok::ColonColon, ".. in for");
                let to = self.parse_expr();
                self.expect(Tok::LBrace, "{ after for");
                let body = self.parse_block();
                Stmt::For { var, from, to, body }
            }
            Tok::KwReturn => {
                self.next();
                if self.eat(&Tok::Semi) { return Stmt::Return(None); }
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
                    Stmt::Assign { target: e, value }
                } else {
                    self.eat(&Tok::Semi);
                    Stmt::Expr(e)
                }
            }
        }
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
        let mut lhs = self.parse_add();
        loop {
            let op = match self.peek() {
                Tok::EqEq => "==", Tok::Ne => "!=",
                Tok::Lt => "<", Tok::Gt => ">",
                Tok::Le => "<=", Tok::Ge => ">=",
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
            return Expr::BinOp(Box::new(Expr::Int(0)), "-".into(), Box::new(e));
        }
        self.parse_postfix()
    }
    fn parse_postfix(&mut self) -> Expr {
        let mut e = self.parse_atom();
        loop {
            if self.eat(&Tok::Dot) {
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
            Tok::Float(v) => Expr::Float(v),
            Tok::Str(s) => Expr::Str(s),
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
                    loop {
                        items.push(self.parse_expr());
                        if !self.eat(&Tok::Comma) { break; }
                    }
                    self.expect(Tok::RBracket, "] after array literal");
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
                if self.eat(&Tok::LBrace) {
                    // struct literal  Name { field: val, ... }
                    let mut fields = Vec::new();
                    while !self.eat(&Tok::RBrace) {
                        let fname = self.ident("struct field");
                        self.expect(Tok::Colon, ": in struct literal");
                        fields.push((fname, self.parse_expr()));
                        self.eat(&Tok::Comma);
                    }
                    return Expr::StructLit { name, fields };
                    // trailing comma handled by eat(Comma) + RBrace check below
                }
                Expr::Ident(name)
            }
            other => panic!("parse error: unexpected token {other:?}"),
        }
    }
}
