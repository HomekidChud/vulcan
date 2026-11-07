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
use crate::parser::{ConstDecl, ConstVal, Item, Ty};
use std::collections::HashMap;

pub struct Tables {
    pub structs: HashMap<String, Vec<(String, Ty)>>,
    pub enums: HashMap<String, Vec<String>>,
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
        Tables { structs, enums }
    }

    pub fn is_enum(&self, name: &str) -> bool { self.enums.contains_key(name) }

    // Enums are represented as i64 at runtime (the variant index) in every
    // backend; `norm` maps an enum struct type down to i64 for type checks.
    pub fn norm(&self, t: &Ty) -> Ty {
        match t {
            Ty::Struct(n) if self.is_enum(n) => Ty::I64,
            Ty::Array(inner) => Ty::Array(Box::new(self.norm(inner))),
            other => other.clone(),
        }
    }

    // The type of `recv.field`. Falls back to F64 (the historic behavior:
    // an unknown field is a numeric method receiver, e.g. `1.0.sin()`),
    // pending the real semantic pass which will make this an error.
    pub fn field_ty(&self, recv_ty: &Ty, fname: &str) -> Ty {
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
                    if seen.insert(v, ()).is_some() {
                        diags.push(Diag::new(e.line, 1,
                            format!("duplicate variant `{v}` in enum `{}`", e.name)));
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
