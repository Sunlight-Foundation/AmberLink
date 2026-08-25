// amber-core/src/semant.rs
use std::collections::HashMap;

pub struct FunctionInfo {
    #[allow(dead_code)]
    pub name: String,
    pub address: u32, // Where it exists in the bytecode
}

#[derive(Clone, Debug)]
pub struct MethodSignature {
    pub name: String,
    pub param_types: Vec<crate::ast::Type>,
    pub visibility: crate::ast::Visibility,
    pub is_static: bool,
    pub mangled_name: String, // e.g. "MyClass_doThing_int_String"
}

#[derive(Clone)]
pub struct ClassInfo {
    pub name: String,
    pub fields: HashMap<String, (u32, crate::ast::Visibility)>, // Field Name -> (Index, Visibility)
    pub methods: Vec<MethodSignature>, // Method name -> Visibility
    pub static_fields: HashMap<String, u32>, // Static field name -> global index
    pub static_methods: Vec<String>, // Static method names
    pub parent: Option<String>, // Parent class name for inheritance
}

#[derive(Clone)]
pub struct InterfaceInfo {
    pub name: String,
    pub method_signatures: Vec<(String, crate::ast::Type, Vec<(String, crate::ast::Type)>)>, // (name, return_type, params)
}

pub struct SymbolTable {
    pub functions: HashMap<String, FunctionInfo>,
    pub classes: HashMap<String, ClassInfo>,
    pub interfaces: HashMap<String, InterfaceInfo>,
    pub variables: HashMap<String, u32>, // Maps "x" -> 0 (Global Index)
    pub locals: HashMap<String, u32>,    // Maps "n" -> 0 (Local Index relative to FP)
    pub variable_types: HashMap<String, crate::ast::Type>, // Maps variable name -> declared type
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
}

