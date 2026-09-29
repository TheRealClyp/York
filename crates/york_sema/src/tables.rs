use std::collections::HashMap;
use york_ir::Ty;

/// Information about a collected struct.
#[derive(Debug, Clone)]
pub struct StructInfo {
    pub name: String,
    pub fields: Vec<(String, Ty)>,
}

/// Information about a collected enum.
#[derive(Debug, Clone)]
pub struct EnumInfo {
    pub name: String,
    pub variants: Vec<(String, Vec<Ty>)>,
}

/// Information about a collected function.
#[derive(Debug, Clone)]
pub struct FnInfo {
    pub name: String,
    pub params: Vec<Ty>,
    pub return_ty: Ty,
    pub is_static: bool,
}

/// Global symbol tables collected during the first pass.
#[derive(Debug, Clone)]
pub struct Tables {
    pub structs: HashMap<String, StructInfo>,
    pub enums: HashMap<String, EnumInfo>,
    pub functions: HashMap<String, FnInfo>,
}

impl Tables {
    pub fn new() -> Self {
        Tables {
            structs: HashMap::new(),
            enums: HashMap::new(),
            functions: HashMap::new(),
        }
    }
}

/// A lexical scope for local variable bindings.
#[derive(Debug, Clone)]
pub struct Scope {
    bindings: HashMap<String, Ty>,
    parent: Option<Box<Scope>>,
}

impl Scope {
    pub fn new() -> Self {
        Scope { bindings: HashMap::new(), parent: None }
    }

    pub fn child(&self) -> Self {
        Scope { bindings: HashMap::new(), parent: Some(Box::new(self.clone())) }
    }

    pub fn define(&mut self, name: String, ty: Ty) {
        self.bindings.insert(name, ty);
    }

    pub fn lookup(&self, name: &str) -> Option<Ty> {
        self.bindings.get(name).cloned()
            .or_else(|| self.parent.as_ref().and_then(|p| p.lookup(name)))
    }
}
