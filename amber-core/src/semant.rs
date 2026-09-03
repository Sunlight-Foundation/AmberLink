// amber-core/src/semant.rs
use std::collections::HashMap;
use crate::ast::{Expr, Type, Op};

pub struct FunctionInfo {
    pub name: String,
    pub address: u32,
    pub return_type: Type,
}

#[derive(Clone, Debug)]
pub struct MethodSignature {
    pub name: String,
    pub param_types: Vec<crate::ast::Type>,
    pub visibility: crate::ast::Visibility,
    pub is_static: bool,
    pub mangled_name: String,
}

#[derive(Clone)]
pub struct ClassInfo {
    pub name: String,
    pub fields: HashMap<String, (u32, crate::ast::Visibility, crate::ast::Type)>,
    pub methods: Vec<MethodSignature>,
    pub static_fields: HashMap<String, (u32, crate::ast::Type)>,
    pub static_methods: Vec<String>,
    pub parent: Option<String>,
}

#[derive(Clone)]
pub struct InterfaceInfo {
    pub name: String,
    pub method_signatures: Vec<(String, crate::ast::Type, Vec<(String, crate::ast::Type)>)>,
}

// A built-in native function implemented in the VM (C++), callable from Amberlink source.
#[derive(Clone)]
pub struct NativeInfo {
    pub name: String,
    pub id: u16,
    pub param_types: Vec<crate::ast::Type>,
    pub return_type: crate::ast::Type,
}

pub struct SymbolTable {
    pub functions: HashMap<String, FunctionInfo>,
    pub classes: HashMap<String, ClassInfo>,
    pub interfaces: HashMap<String, InterfaceInfo>,
    pub natives: HashMap<String, NativeInfo>,
    pub variables: HashMap<String, u32>,
    pub locals: HashMap<String, u32>,
    pub variable_types: HashMap<String, crate::ast::Type>,
    pub next_var_index: u32,
    pub next_local_index: u32,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
            classes: HashMap::new(),
            interfaces: HashMap::new(),
            natives: HashMap::new(),
            variables: HashMap::new(),
            locals: HashMap::new(),
            variable_types: HashMap::new(),
            next_var_index: 0,
            next_local_index: 0,
        }
    }

    // Registers the built-in native functions. IDs must match the registry
    // order in amber-vm/src/natives.cpp (Natives::registry()).
    pub fn init_native_registry(&mut self) {
        use crate::ast::Type;
        let mut register = |name: &str, id: u16, return_type: Type, param_types: Vec<Type>| {
            self.natives.insert(
                name.to_string(),
                NativeInfo { name: name.to_string(), id, param_types, return_type },
            );
        };
        // Order must match natives.cpp registry().
        register("len", 0, Type::Int, vec![Type::String]);
        register("input", 1, Type::String, vec![]);
        register("toString", 2, Type::String, vec![Type::Int]);
        register("toInt", 3, Type::Int, vec![Type::String]);
        register("toFloat", 4, Type::Float, vec![Type::String]);
        register("abs", 5, Type::Float, vec![Type::Float]);
    }

    pub fn get_var_type(&self, name: &str) -> Option<Type> {
        if let Some(t) = self.variable_types.get(name) {
            return Some(t.clone());
        }
        for (_, class_info) in &self.classes {
            if let Some((_, _, t)) = class_info.fields.get(name) {
                return Some(t.clone());
            }
            if let Some((_, t)) = class_info.static_fields.get(name) {
                return Some(t.clone());
            }
        }
        None
    }

    pub fn infer_type(&self, expr: &Expr) -> Option<Type> {
        match expr {
            Expr::Integer(_) => Some(Type::Int),
            Expr::Float(_) => Some(Type::Float),
            Expr::Char(_) => Some(Type::Char),
            Expr::Boolean(_) => Some(Type::Bool),
            Expr::StringLiteral(_) => Some(Type::String),
            Expr::NewList => Some(Type::List),
            Expr::NewArray(_) => Some(Type::Unknown),
            Expr::ArrayAccess(_, _) => Some(Type::Unknown),
            Expr::NewInstance(class_name, _) => Some(Type::Class(class_name.clone())),
            Expr::GetField(obj, field) => {
                let ot = self.infer_type(obj)?;
                if let Type::Class(cname) = ot {
                    if let Some(ci) = self.classes.get(&cname) {
                        if let Some((_, _, ft)) = ci.fields.get(field) {
                            return Some(ft.clone());
                        }
                    }
                }
                Some(Type::Unknown)
            }
            Expr::MethodCall(obj, method, args) => {
                let ot = self.infer_type(obj);
                let cname = match ot {
                    Some(Type::Class(c)) => c,
                    _ => match self.find_class_with_method(method) {
                        Some(ci) => ci.name.clone(),
                        None => return Some(Type::Unknown),
                    },
                };
                if let Some(ci) = self.classes.get(&cname) {
                    for m in &ci.methods {
                        if m.name == *method && m.param_types.len() == args.len() {
                            if let Some(f) = self.functions.get(&m.mangled_name) {
                                return Some(f.return_type.clone());
                            }
                        }
                    }
                }
                Some(Type::Unknown)
            }
            Expr::Variable(name) => self.get_var_type(name),
            Expr::Binary(left, op, right) => {
                let lt = self.infer_type(left)?;
                let rt = self.infer_type(right)?;
                match op {
                    Op::Add | Op::Sub | Op::Mul | Op::Div => {
                        if lt == rt || matches!(lt, Type::Unknown) || matches!(rt, Type::Unknown) {
                            Some(lt)
                        } else {
                            None
                        }
                    }
                    Op::LessThan | Op::GreaterThan | Op::Equals | Op::NotEquals
                    | Op::LessEquals | Op::GreaterEquals => Some(Type::Bool),
                }
            }
            Expr::ListGet(list_expr, _) => {
                let _ = self.infer_type(list_expr)?;
                Some(Type::Unknown) // no element-type tracking yet
            }
            Expr::ListSize(_) => Some(Type::Int),
            Expr::Call(name, _) => {
                if let Some(f) = self.functions.get(name) {
                    Some(f.return_type.clone())
                } else {
                    self.natives.get(name).map(|n| n.return_type.clone())
                }
            }
            Expr::Error => Some(Type::Unknown),
        }
    }

    // Finds the first class containing a method with the given name (searching
    // parent chains), used when we cannot resolve a MethodCall receiver's type.
    fn find_class_with_method(&self, method: &str) -> Option<&ClassInfo> {
        for ci in self.classes.values() {
            if self.class_or_parent_has_method(ci, method) {
                return Some(ci);
            }
        }
        None
    }

    fn class_or_parent_has_method(&self, ci: &ClassInfo, method: &str) -> bool {
        if ci.methods.iter().any(|m| m.name == *method) { return true; }
        match &ci.parent {
            Some(parent) => match self.classes.get(parent) {
                Some(p) => self.class_or_parent_has_method(p, method),
                None => false,
            },
            None => false,
        }
    }

    pub fn types_compatible(&self, expected: &Type, actual: &Type) -> bool {
        if expected == actual { return true; }
        // Allow int -> float promotion
        if matches!(expected, Type::Float) && matches!(actual, Type::Int) { return true; }
        // Unknown matches anything; the real check is deferred.
        if matches!(expected, Type::Unknown) || matches!(actual, Type::Unknown) { return true; }
        // Inheritance: a subclass instance satisfies a parent type.
        if let (Type::Class(exp), Type::Class(act)) = (expected, actual) {
            if self.is_subclass(act, exp) { return true; }
        }
        false
    }

    fn is_subclass(&self, child: &str, ancestor: &str) -> bool {
        let mut cur = Some(child.to_string());
        while let Some(c) = cur {
            if &c == ancestor { return true; }
            cur = self.classes.get(&c).and_then(|ci| ci.parent.clone());
        }
        false
    }
}
