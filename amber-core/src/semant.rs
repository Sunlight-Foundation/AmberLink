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

pub struct SymbolTable {
    pub functions: HashMap<String, FunctionInfo>,
    pub classes: HashMap<String, ClassInfo>,
    pub interfaces: HashMap<String, InterfaceInfo>,
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
            variables: HashMap::new(),
            locals: HashMap::new(),
            variable_types: HashMap::new(),
            next_var_index: 0,
            next_local_index: 0,
        }
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
            Expr::Variable(name) => self.get_var_type(name),
            Expr::Binary(left, op, right) => {
                let lt = self.infer_type(left)?;
                let rt = self.infer_type(right)?;
                match op {
                    Op::Add | Op::Sub | Op::Mul | Op::Div => {
                        if lt == rt { Some(lt) } else { None }
                    }
                    Op::LessThan | Op::GreaterThan | Op::Equals | Op::NotEquals
                    | Op::LessEquals | Op::GreaterEquals => Some(Type::Bool),
                }
            }
            Expr::ListGet(list_expr, _) => {
                let _ = self.infer_type(list_expr)?;
                Some(Type::List) // TODO: track inner type
            }
            Expr::ListSize(_) => Some(Type::Int),
            Expr::Call(name, _) => {
                self.functions.get(name).map(|f| f.return_type.clone())
            }
            _ => None,
        }
    }

    pub fn types_compatible(&self, expected: &Type, actual: &Type) -> bool {
        if expected == actual { return true; }
        // Allow int -> float promotion
        if matches!(expected, Type::Float) && matches!(actual, Type::Int) { return true; }
        // Class types are compatible if same name (no inheritance check yet)
        false
    }
}
