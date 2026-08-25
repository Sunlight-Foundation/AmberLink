// amber-core/src/semant.rs
use std::collections::HashMap;

pub struct FunctionInfo {
    #[allow(dead_code)]
    pub name: String,
    pub address: u32, // Where it exists in the bytecode
}

#[derive(Clone)]
pub struct ClassInfo {
    pub name: String,
    pub fields: HashMap<String, (u32, crate::ast::Visibility)>, // Field Name -> (Index, Visibility)
    pub methods: HashMap<String, crate::ast::Visibility>, // Method name -> Visibility
}

pub struct SymbolTable {
    pub functions: HashMap<String, FunctionInfo>,
    pub classes: HashMap<String, ClassInfo>,
    pub variables: HashMap<String, u32>, // Maps "x" -> 0 (Global Index)
    pub locals: HashMap<String, u32>,    // Maps "n" -> 0 (Local Index relative to FP)
    pub next_var_index: u32,
    pub next_local_index: u32,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self { 
            functions: HashMap::new(),
            classes: HashMap::new(),
            variables: HashMap::new(),
            locals: HashMap::new(),
            next_var_index: 0,
            next_local_index: 0,
        }
    }
}
