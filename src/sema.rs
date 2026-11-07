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
        let ok = from == to || (from == Ty::I64 && to == Ty::F64);
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
