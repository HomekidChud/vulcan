// Shared semantic tables and type helpers.
//
// Phase 1 step 1 of the mega-upgrade: the type helpers that used to be
// copy-pasted into all three backends (field_ty, const_ty, norm, array_elem,
// check_assignable) live here, once. The backends keep their own struct/enum
// maps for code generation but delegate every type QUESTION to this module.
//
// The end state (later in Phase 1) is a full semantic pass that walks the AST
// once, annotates it with types, and hands the typed tree to all backends —
// this module is where that pass will live.
use crate::parser::{ty_name, ConstDecl, ConstVal, EnumVariant, Expr, Item, Stmt, Ty};
use std::collections::HashMap;

pub struct Tables {
    pub structs: HashMap<String, Vec<(String, Ty)>>,
    pub enums: HashMap<String, Vec<EnumVariant>>,
}

impl Tables {
    pub fn from_program(program: &[Item]) -> Tables {
        let mut structs = HashMap::new();
        let mut enums = HashMap::new();
        for item in program {
            match item {
                Item::Struct(s) => { structs.insert(s.name.clone(), s.fields.clone()); }
                Item::Enum(e) => { enums.insert(e.name.clone(), e.variants.clone()); }
                _ => {}
            }
        }
        // register every Option<T> / Result<T, E> mentioned anywhere in the
        // program as a synthetic payload enum (Some/None, Ok/Err)
        let mut insts = Vec::new();
        collect_tys(program, &mut insts);
        for t in insts {
            register_option_result(&mut enums, &t);
        }
        Tables { structs, enums }
    }

    pub fn is_enum(&self, name: &str) -> bool { self.enums.contains_key(name) }

    // does this enum have any variant with a payload? (payload enums are
    // tagged structs at runtime; payload-less enums stay plain i64 tags)
    pub fn is_payload_enum(&self, name: &str) -> bool {
        self.enums.get(name).map(|vs| vs.iter().any(|v| !v.payload.is_empty())).unwrap_or(false)
    }

    // Payload-less enums are represented as i64 at runtime (the variant
    // index) in every backend; `norm` maps those down to i64 for type
    // checks. Payload enums and Option/Result keep their identity.
    pub fn norm(&self, t: &Ty) -> Ty {
        match t {
            Ty::Struct(n) if self.is_enum(n) && !self.is_payload_enum(n) => Ty::I64,
            Ty::Array(inner) => Ty::Array(Box::new(self.norm(inner))),
            other => other.clone(),
        }
    }

    // The type of `recv.field`. Falls back to F64 (the historic behavior:
    // an unknown field is a numeric method receiver, e.g. `1.0.sin()`),
    // pending the real semantic pass which will make this an error.
    pub fn field_ty(&self, recv_ty: &Ty, fname: &str) -> Ty {
        if let Ty::Ref(i) | Ty::RefMut(i) = recv_ty {
            return self.field_ty(i, fname);
        }
        if let Ty::Struct(sname) = recv_ty {
            if let Some(fields) = self.structs.get(sname) {
                if let Some((_, t)) = fields.iter().find(|(n, _)| n == fname) {
                    return t.clone();
                }
            }
        }
        Ty::F64
    }

    // One rule for all backends: types must match, except i64 widens to f64.
    // (JS/Python used to also allow implicit f64->i64; that silently
    // truncated and is gone — use `as i64` for that.)
    pub fn check_assignable(&self, from: &Ty, to: &Ty, what: &str) {
        let from = self.norm(from);
        let to = self.norm(to);
        // numeric kinds convert implicitly among themselves: ints may narrow
        // (with wrap-around at runtime, use `as` to be explicit), ints widen to
        // floats, and f64 may narrow to f32 (rounds to nearest)
        let ok = from == to
            || (from.is_int() && to.is_int())
            || (from.is_int() && to.is_float())
            || (from.is_float() && to.is_float())
            // arrays convert when their elements do
            || (matches!(from, Ty::Array(_)) && matches!(to, Ty::Array(_)) && {
                let fe = self.array_elem(&from);
                let te = self.array_elem(&to);
                fe == te
                    || (fe.is_int() && te.is_int())
                    || (fe.is_int() && te.is_float())
                    || (fe.is_float() && te.is_float())
            });
        if !ok {
            panic!("vulcan: type mismatch in {what}: {from:?} vs {to:?}");
        }
    }

    pub fn array_elem(&self, t: &Ty) -> Ty {
        match t {
            Ty::Array(inner) => (**inner).clone(),
            _ => panic!("vulcan: indexing a non-array value ({t:?})"),
        }
    }
}

// ---- Option/Result instantiation ----
//
// The parser produces Ty::Option(t) / Ty::Result(t, e) wherever those types
// are written. There are no user declarations for them, so from_program walks
// the whole program, collects every Ty it can reach, and registers each
// distinct Option/Result as a synthetic payload enum keyed by its source
// name ("Option<i64>", "Result<Pixel, string>", ...).

fn register_option_result(enums: &mut HashMap<String, Vec<EnumVariant>>, t: &Ty) {
    let (name, variants) = match t {
        Ty::Option(inner) => (ty_name(t), vec![
            EnumVariant { name: "Some".into(), payload: vec![(**inner).clone()] },
            EnumVariant { name: "None".into(), payload: vec![] },
        ]),
        Ty::Result(ok, err) => (ty_name(t), vec![
            EnumVariant { name: "Ok".into(), payload: vec![(**ok).clone()] },
            EnumVariant { name: "Err".into(), payload: vec![(**err).clone()] },
        ]),
        _ => return,
    };
    enums.entry(name).or_insert(variants);
}

// collect every type mentioned in the program (signatures, annotations,
// payloads, cast targets) into `out`
fn collect_tys(program: &[Item], out: &mut Vec<Ty>) {
    for item in program {
        match item {
            Item::Fn(f) => {
                for (_, t) in &f.params { push_ty(out, t); }
                push_ty(out, &f.ret);
                walk_stmts(&f.body, out);
            }
            Item::Extern(x) => for f in &x.fns {
                for (_, t) in &f.params { push_ty(out, t); }
                push_ty(out, &f.ret);
            },
            Item::Struct(s) => for (_, t) in &s.fields { push_ty(out, t); },
            Item::Enum(e) => for v in &e.variants { for t in &v.payload { push_ty(out, t); } },
            Item::Const(c) => if let Some(t) = &c.ty { push_ty(out, t); },
            Item::Impl(i) => for f in &i.fns {
                for (_, t) in &f.params { push_ty(out, t); }
                push_ty(out, &f.ret);
                walk_stmts(&f.body, out);
            },
            _ => {}
        }
    }
}

fn push_ty(out: &mut Vec<Ty>, t: &Ty) {
    out.push(t.clone());
    match t {
        Ty::Array(i) => push_ty(out, i),
        Ty::Option(i) => push_ty(out, i),
        Ty::Result(a, b) => { push_ty(out, a); push_ty(out, b); }
        Ty::Ref(i) | Ty::RefMut(i) => push_ty(out, i),
        _ => {}
    }
}

fn walk_stmts(stmts: &[Stmt], out: &mut Vec<Ty>) {
    for st in stmts {
        match st {
            Stmt::Let { ty, init, .. } => {
                if let Some(t) = ty { push_ty(out, t); }
                walk_expr(init, out);
            }
            Stmt::Assign { target, value } | Stmt::CompoundAssign { target, value, .. } => {
                walk_expr(target, out); walk_expr(value, out);
            }
            Stmt::IncDec { target, .. } => walk_expr(target, out),
            Stmt::If { cond, then, els } => {
                walk_expr(cond, out); walk_stmts(then, out);
                if let Some(e) = els { walk_stmts(e, out); }
            }
            Stmt::While { cond, body } => { walk_expr(cond, out); walk_stmts(body, out); }
            Stmt::Loop { body } => walk_stmts(body, out),
            Stmt::For { from, to, body, .. } => {
                walk_expr(from, out); walk_expr(to, out); walk_stmts(body, out);
            }
            Stmt::ForIn { arr, body, .. } => { walk_expr(arr, out); walk_stmts(body, out); }
            Stmt::Match { scrutinee, arms, default } => {
                walk_expr(scrutinee, out);
                for a in arms {
                    for p in &a.pats { walk_expr(p, out); }
                    if let Some(g) = &a.guard { walk_expr(g, out); }
                    walk_stmts(&a.body, out);
                }
                if let Some(d) = default { walk_stmts(d, out); }
            }
            Stmt::Defer(b) => walk_stmts(b, out),
            Stmt::Assert { cond, msg, .. } => {
                walk_expr(cond, out);
                if let Some(m) = msg { walk_expr(m, out); }
            }
            Stmt::Return(e) => if let Some(e) = e { walk_expr(e, out); },
            Stmt::Break | Stmt::Continue => {}
            Stmt::Expr(e) => walk_expr(e, out),
        }
    }
}

fn walk_expr(e: &Expr, out: &mut Vec<Ty>) {
    match e {
        Expr::BinOp(l, _, r) => { walk_expr(l, out); walk_expr(r, out); }
        Expr::Call { args, .. } | Expr::Method { args, .. } => for a in args { walk_expr(a, out); },
        Expr::Method { recv, .. } | Expr::Field(recv, _) => walk_expr(recv, out),
        Expr::Index(a, b) => { walk_expr(a, out); walk_expr(b, out); }
        Expr::ArrayLit(xs) => for x in xs { walk_expr(x, out); }
        Expr::StructLit { fields, .. } => for (_, v) in fields { walk_expr(v, out); }
        Expr::Interp(parts) => for p in parts {
            if let crate::parser::IPart::E(e) = p { walk_expr(e, out); }
        },
        Expr::As(inner, t) => { walk_expr(inner, out); push_ty(out, t); }
        Expr::Try(inner) => walk_expr(inner, out),
        Expr::Addr { inner, .. } => walk_expr(inner, out),
        // closures are inlined by the iterator methods; they are walked for
        // diagnostics but never evaluated standalone
        Expr::Closure { body, .. } => walk_expr(body, out),
        Expr::Block { stmts, value } => {
            for st in stmts {
                if let Stmt::Expr(e) = st { walk_expr(e, out); }
                // let/if/… initializers are walked by their own arms via
                // walk_stmt when reachable; here we only need expression types
                if let Stmt::Let { init, .. } = st { walk_expr(init, out); }
            }
            walk_expr(value, out);
        }
        Expr::Int(_) | Expr::Float(_) | Expr::Str(_) | Expr::Bool(_) | Expr::Char(_) | Expr::Ident(_) => {}
    }
}

// The type of a `const` declaration: the annotation if given, otherwise the
// type of its literal value.
pub fn const_ty(c: &ConstDecl) -> Ty {
    match &c.ty {
        Some(t) => t.clone(),
        None => match c.value {
            ConstVal::Int(_) => Ty::I64,
            ConstVal::Float(_) => Ty::F64,
            ConstVal::Str(_) => Ty::Str,
            ConstVal::Bool(_) => Ty::Bool,
        },
    }
}

// Early semantic checks over the whole program (the seed of the real
// semantic pass). Today: duplicate definitions, duplicate struct fields,
// duplicate enum variants, unknown impl target. Everything reports a Diag
// with the declaration's line; more checks move here as Phase 1 continues.
pub fn check_program(program: &[Item]) -> Vec<crate::diag::Diag> {
    use crate::diag::Diag;
    let mut diags: Vec<Diag> = Vec::new();
    let mut fns: HashMap<&str, usize> = HashMap::new();
    let mut structs: HashMap<&str, usize> = HashMap::new();
    let mut enums: HashMap<&str, usize> = HashMap::new();
    let mut consts: HashMap<&str, usize> = HashMap::new();

    for item in program {
        match item {
            Item::Fn(f) => {
                if let Some(&prev) = fns.get(f.name.as_str()) {
                    diags.push(Diag::new(f.line, 1,
                        format!("duplicate function `{}` (first defined on line {prev})", f.name)));
                } else {
                    fns.insert(&f.name, f.line);
                }
            }
            Item::Struct(s) => {
                if let Some(&prev) = structs.get(s.name.as_str()) {
                    diags.push(Diag::new(s.line, 1,
                        format!("duplicate struct `{}` (first defined on line {prev})", s.name)));
                } else {
                    structs.insert(&s.name, s.line);
                }
                let mut seen: HashMap<&str, usize> = HashMap::new();
                for (fname, _) in &s.fields {
                    if let Some(&prev) = seen.get(fname.as_str()) {
                        diags.push(Diag::new(s.line, 1,
                            format!("duplicate field `{fname}` in struct `{}` (first on line {prev})", s.name)));
                    } else {
                        seen.insert(fname, s.line);
                    }
                }
            }
            Item::Enum(e) => {
                if let Some(&prev) = enums.get(e.name.as_str()) {
                    diags.push(Diag::new(e.line, 1,
                        format!("duplicate enum `{}` (first defined on line {prev})", e.name)));
                } else {
                    enums.insert(&e.name, e.line);
                }
                let mut seen: HashMap<&str, ()> = HashMap::new();
                for v in &e.variants {
                    if seen.insert(v.name.as_str(), ()).is_some() {
                        diags.push(Diag::new(e.line, 1,
                            format!("duplicate variant `{}` in enum `{}`", v.name, e.name)));
                    }
                }
            }
            Item::Const(c) => {
                if let Some(&prev) = consts.get(c.name.as_str()) {
                    diags.push(Diag::new(c.line, 1,
                        format!("duplicate const `{}` (first defined on line {prev})", c.name)));
                } else {
                    consts.insert(&c.name, c.line);
                }
            }
            Item::Impl(i) => {
                if !structs.contains_key(i.struct_name.as_str()) && !enums.contains_key(i.struct_name.as_str()) {
                    diags.push(Diag::new(0, 0,
                        format!("impl for unknown type `{}`", i.struct_name)));
                }
            }
            _ => {}
        }
    }
    diags
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diag::Diag;
    use crate::lexer::lex;
    use crate::parser::parse_program;

    fn check(src: &str) -> Vec<Diag> {
        check_program(&parse_program(&lex(src)))
    }

    #[test]
    fn clean_program_has_no_diags() {
        assert!(check("fn main() -> void { print(1) }").is_empty());
    }

    #[test]
    fn duplicate_fn_reported_with_lines() {
        let d = check("fn f() -> i64 { return 1 }\nfn f() -> i64 { return 2 }");
        assert_eq!(d.len(), 1);
        assert!(d[0].msg.contains("duplicate function `f`"));
        assert!(d[0].msg.contains("line 1"));
        assert_eq!(d[0].line, 2);
    }

    #[test]
    fn duplicate_fields_and_variants() {
        let d = check("struct S { x: i64, x: f64 }\nenum E { A, A }");
        assert_eq!(d.len(), 2);
        assert!(d[0].msg.contains("duplicate field `x`"));
        assert!(d[1].msg.contains("duplicate variant `A`"));
    }

    #[test]
    fn impl_for_unknown_type() {
        let d = check("impl Q { fn z(self) -> i64 { return 0 } }");
        assert_eq!(d.len(), 1);
        assert!(d[0].msg.contains("unknown type `Q`"));
    }
}
