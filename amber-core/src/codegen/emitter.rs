// amber-core/src/codegen/emitter.rs
use std::fs::File;
use std::io::{Write, BufWriter};
use super::bytecode::OpCode;
use crate::ast::{Expr, Op};
use crate::ast::Stmt;
use crate::semant::{SymbolTable, ClassInfo};
use std::collections::HashMap;

pub struct Emitter {
    pub code: Vec<u8>,
    pub constants: Vec<String>,
    pub calls_to_patch: Vec<(usize, String)>, // (Bytecode Index, Function Name)
    pub current_class: Option<String>,
}

impl Emitter {
    pub fn new() -> Self { Self { code: Vec::new(), constants: Vec::new(), calls_to_patch: Vec::new(), current_class: None } }

    pub fn emit_byte(&mut self, b: u8) { self.code.push(b); }
    pub fn emit_int(&mut self, val: i32) {
        self.code.extend_from_slice(&val.to_le_bytes());
    }
    pub fn emit_float(&mut self, val: f32) {
        self.code.extend_from_slice(&val.to_le_bytes());
    }

    pub fn emit_expr(&mut self, expr: &Expr, symbols: &mut SymbolTable) {
        match expr {
            Expr::Integer(val) => {
                self.emit_byte(OpCode::Push.into());
                self.emit_int(*val);
            }
            Expr::Float(val) => {
                self.emit_byte(OpCode::PushFloat.into());
                self.emit_float(*val as f32); // Cast f64 to f32 for now as VM expects 4 bytes
            }
            Expr::Char(val) => {
                self.emit_byte(OpCode::PushChar.into());
                self.emit_int(*val as i32); // UTF-32
            }
            Expr::Boolean(val) => {
                self.emit_byte(OpCode::PushBool.into());
                self.emit_byte(if *val { 1 } else { 0 });
            }
            Expr::StringLiteral(s) => {
                // Deduplicate or just push
                let index = if let Some(idx) = self.constants.iter().position(|c| c == s) {
                    idx
                } else {
                    self.constants.push(s.clone());
                    self.constants.len() - 1
                };
                
                self.emit_byte(OpCode::LoadConst.into());
                self.emit_int(index as i32);
            }
            Expr::NewArray(size) => {
                self.emit_expr(size, symbols);
                self.emit_byte(OpCode::NewArray.into());
            }
            Expr::NewList => {
                self.emit_byte(OpCode::NewList.into());
            }
            Expr::ListGet(list_expr, index_expr) => {
                self.emit_expr(list_expr, symbols);
                self.emit_expr(index_expr, symbols);
                self.emit_byte(OpCode::ListGet.into());
            }
            Expr::ListSize(list_expr) => {
                self.emit_expr(list_expr, symbols);
                self.emit_byte(OpCode::ListSize.into());
            }
            Expr::NewInstance(class_name, args) => {
                 // 1. Find the class
                let class_info = symbols.classes.get(class_name)
                    .expect(&format!("Undefined class: {}", class_name));
                
                // 2. Emit OP_NEW_INSTANCE
                self.emit_byte(OpCode::NewInstance.into());
                
                // 3. Emit Class ID (Hash of name for now, or just 0 placeholder) and Field Count
                // For simplicity in v0.3, we pass Field Count directly so VM knows how much to alloc.
                // We can use the constant pool index of the class name as the ID.
                let name_idx = self.add_constant(class_name.clone());
                self.emit_int(name_idx as i32);
                self.emit_int(class_info.fields.len() as i32);

                let init_name = format!("{}_init", class_name);
                if symbols.functions.contains_key(&init_name) {
                    for arg in args {
                        self.emit_expr(arg, symbols);
                    }
                    self.emit_byte(OpCode::Call.into());
                    self.calls_to_patch.push((self.code.len(), init_name));
                    self.emit_int(0);
                    self.emit_byte((args.len() + 1) as u8); // +1 for 'this'
                }
            }
            Expr::GetField(obj_expr, field_name) => {
                // Check for static field access (ClassName.field_name)
                if let Expr::Variable(var_name) = &**obj_expr {
                    if symbols.classes.contains_key(var_name) && !symbols.locals.contains_key(var_name) && !symbols.variables.contains_key(var_name) {
                        if let Some(class_info) = symbols.classes.get(var_name) {
                            if let Some(global_idx) = class_info.static_fields.get(field_name) {
                                self.emit_byte(OpCode::LoadGlobal.into());
                                self.emit_int(*global_idx as i32);
                                return;
                            } else {
                                panic!("Static field '{}' not found in class '{}'", field_name, var_name);
                            }
                        }
                    }
                }

                self.emit_expr(obj_expr, symbols); // Push object ref
                
                // Hack: Find field index by looking at all classes (since we don't track types yet)
                let mut field_idx = None;
                
                // Sort classes to ensure deterministic compilation
                let mut classes: Vec<_> = symbols.classes.values().collect();
                classes.sort_by_key(|c| &c.name);

                // FIXME: no type tracking
                for cls in classes {
                    if let Some((idx, vis)) = cls.fields.get(field_name) {
                        if *vis == crate::ast::Visibility::Private && self.current_class.as_deref() != Some(cls.name.as_str()) {
                            panic!("Cannot access private field '{}' of class '{}'", field_name, cls.name);
                        }
                        field_idx = Some(*idx);
                        break;
                    }
                }
                let idx = field_idx.expect(&format!("Field '{}' not found in any known class", field_name));
                
                self.emit_byte(OpCode::GetField.into());
                self.emit_int(idx as i32);
            }
            Expr::MethodCall(obj, method_name, args) => {
                let mut is_static_call = false;
                if let Expr::Variable(var_name) = &**obj {
                    if symbols.classes.contains_key(var_name) && !symbols.locals.contains_key(var_name) && !symbols.variables.contains_key(var_name) {
                        is_static_call = true;
                    }
                }

                if !is_static_call {
                    self.emit_expr(obj, symbols); // 1. Push Object (this)
                }

                for arg in args {
                    self.emit_expr(arg, symbols); // 2. Push Args
                }

                // Find which class has this method (walk parent chain for inheritance)
                let mut found_class = None;
                // Sort classes to ensure deterministic compilation in case of name collisions
                let mut classes: Vec<_> = symbols.classes.values().collect();
                classes.sort_by_key(|c| &c.name);

                for cls in &classes {
                    if cls.methods.iter().any(|ms| ms.name == *method_name) {
                        found_class = Some(cls.name.clone());
                        break;
                    }
                }

                // If not found directly, walk parent chains
                if found_class.is_none() {
                    for cls in &classes {
                        let mut current = Some(cls.name.clone());
                        while let Some(ref cname) = current {
                            if let Some(ci) = symbols.classes.get(cname) {
                                if ci.methods.iter().any(|ms| ms.name == *method_name) {
                                    found_class = Some(ci.name.clone());
                                    break;
                                }
                                current = ci.parent.clone();
                            } else {
                                break;
                            }
                        }
                        if found_class.is_some() { break; }
                    }
                }

                let class_name = found_class.expect(&format!("Method '{}' not found in any known class", method_name));
                
                // Resolve overloaded method by matching arg count
                let class_info = symbols.classes.get(&class_name)
                    .expect(&format!("Class '{}' not found", class_name));
                let matching: Vec<_> = class_info.methods.iter()
                    .filter(|ms| ms.name == *method_name && ms.param_types.len() == args.len() && ms.is_static == is_static_call)
                    .collect();
                let method_sig = if matching.len() == 1 {
                    &matching[0]
                } else if matching.is_empty() {
                    panic!("No matching overload for method '{}' with {} args (static: {})", method_name, args.len(), is_static_call)
                } else {
                    // Multiple matches with same arg count — pick first (type matching not implemented yet)
                    &matching[0]
                };

                if method_sig.visibility == crate::ast::Visibility::Private && self.current_class.as_deref() != Some(class_name.as_str()) {
                    panic!("Cannot access private method '{}' of class '{}'", method_name, class_name);
                }

                let full_name = method_sig.mangled_name.clone();

                self.emit_byte(OpCode::Call.into());
                self.calls_to_patch.push((self.code.len(), full_name));
                self.emit_int(0);
                self.emit_byte((args.len() + if is_static_call { 0 } else { 1 }) as u8); // +1 for 'this' if not static
            }
            Expr::ArrayAccess(name, index) => {
                // Load array ref
                if let Some(idx) = symbols.locals.get(name) {
                    self.emit_byte(OpCode::LoadLocal.into());
                    self.emit_int(*idx as i32);
                } else {
                    let idx = symbols.variables.get(name).expect("Undefined variable");
                    self.emit_byte(OpCode::LoadGlobal.into());
                    self.emit_int(*idx as i32);
                }
                self.emit_expr(index, symbols); // Load index
                self.emit_byte(OpCode::LoadArray.into());
            }
            Expr::Variable(name) => {
                if let Some(index) = symbols.locals.get(name) {
                    self.emit_byte(OpCode::LoadLocal.into());
                    self.emit_int(*index as i32);
                } else {
                    let index = symbols.variables.get(name)
                        .expect(&format!("Undefined variable: {}", name));
                    self.emit_byte(OpCode::LoadGlobal.into());
                    self.emit_int(*index as i32);
                }
            }
            Expr::Call(name, args) => {
                for arg in args {
                    self.emit_expr(arg, symbols);
                }
                self.emit_byte(OpCode::Call.into());
                
                // Emit placeholder address and record for patching
                self.calls_to_patch.push((self.code.len(), name.clone()));
                self.emit_int(0); 
                self.emit_byte(args.len() as u8);
            }
            Expr::Binary(left, op, right) => {
                self.emit_expr(left, symbols);
                self.emit_expr(right, symbols);
                match op {
                    Op::Add => self.emit_byte(OpCode::Add.into()),
                    Op::Sub => self.emit_byte(OpCode::Sub.into()),
                    Op::Mul => self.emit_byte(OpCode::Mul.into()),
                    Op::Div => self.emit_byte(OpCode::Div.into()),
                    Op::LessThan => self.emit_byte(OpCode::Less.into()),
                    Op::GreaterThan => self.emit_byte(OpCode::Greater.into()),
                }
            }
        }
    }

    // Emits a jump instruction with a placeholder offset. Returns the index of the placeholder.
    fn emit_jump(&mut self, instruction: u8) -> usize {
        self.emit_byte(instruction);
        self.emit_byte(0xFF); // Placeholder (4 bytes)
        self.emit_byte(0xFF);
        self.emit_byte(0xFF);
        self.emit_byte(0xFF);
        self.code.len() - 4
    }

    fn patch_jump(&mut self, offset_index: usize) {
        let jump_dist = (self.code.len() - offset_index - 4) as i32;
        let bytes = jump_dist.to_le_bytes();
        for i in 0..4 {
            self.code[offset_index + i] = bytes[i];
        }
    }

    fn add_constant(&mut self, s: String) -> usize {
        if let Some(idx) = self.constants.iter().position(|c| *c == s) {
            idx
        } else {
            self.constants.push(s);
            self.constants.len() - 1
        }
    }

    pub fn finalize(&mut self, symbols: &SymbolTable) {
        for (index, name) in &self.calls_to_patch {
            let func_info = symbols.functions.get(name)
                .expect(&format!("Undefined function: {}", name));
            
            let bytes = (func_info.address as i32).to_le_bytes();
            for i in 0..4 {
                self.code[index + i] = bytes[i];
            }
        }
    }

    pub fn emit_stmt(&mut self, stmt: &Stmt, symbols: &mut SymbolTable) {
        match stmt {
            Stmt::VarDecl(name, _type, expr) => {
                self.emit_expr(expr, symbols); // Push value
                
                // Assign index
                let index = symbols.next_var_index;
                symbols.variables.insert(name.clone(), index);
                symbols.next_var_index += 1;

                self.emit_byte(OpCode::StoreGlobal.into());
                self.emit_int(index as i32);
            }
            Stmt::Assign(name, expr) => {
                self.emit_expr(expr, symbols);
                if let Some(index) = symbols.locals.get(name) {
                    self.emit_byte(OpCode::StoreLocal.into());
                    self.emit_int(*index as i32);
                } else if let Some(index) = symbols.variables.get(name) {
                    self.emit_byte(OpCode::StoreGlobal.into());
                    self.emit_int(*index as i32);
                } else {
                    panic!("Undefined variable: {}", name);
                }
            }
            Stmt::ArraySet(name, index, value) => {
                // Load array ref
                if let Some(idx) = symbols.locals.get(name) {
                    self.emit_byte(OpCode::LoadLocal.into());
                    self.emit_int(*idx as i32);
                } else {
                    let idx = symbols.variables.get(name).expect("Undefined variable");
                    self.emit_byte(OpCode::LoadGlobal.into());
                    self.emit_int(*idx as i32);
                }
                self.emit_expr(index, symbols);
                self.emit_expr(value, symbols);
                self.emit_byte(OpCode::StoreArray.into());
            }
            Stmt::ListAdd(list_expr, value_expr) => {
                self.emit_expr(list_expr, symbols);
                self.emit_expr(value_expr, symbols);
                self.emit_byte(OpCode::ListAdd.into());
            }
            Stmt::ListSet(list_expr, index_expr, value_expr) => {
                self.emit_expr(list_expr, symbols);
                self.emit_expr(index_expr, symbols);
                self.emit_expr(value_expr, symbols);
                self.emit_byte(OpCode::ListSet.into());
            }
            Stmt::Return(expr) => {
                self.emit_expr(expr, symbols);
                self.emit_byte(OpCode::Return.into());
            }
            Stmt::Print(expr) => {
                self.emit_expr(expr, symbols);
                self.emit_byte(OpCode::Print.into());
            }
            Stmt::Block(stmts) => {
                for s in stmts {
                    self.emit_stmt(s, symbols);
                }
            }
            Stmt::If(cond, then_branch, else_branch) => {
                self.emit_expr(cond, symbols);
                
                // Jump to Else if false
                let then_jump = self.emit_jump(OpCode::JumpIfFalse.into());
                
                self.emit_stmt(then_branch, symbols);
                
                let else_jump = self.emit_jump(OpCode::Jump.into());
                
                self.patch_jump(then_jump);
                
                if let Some(else_stmt) = else_branch {
                    self.emit_stmt(else_stmt, symbols);
                }
                
                self.patch_jump(else_jump);
            }
            Stmt::While(cond, body) => {
                let loop_start = self.code.len();
                
                self.emit_expr(cond, symbols);
                let exit_jump = self.emit_jump(OpCode::JumpIfFalse.into());
                
                self.emit_stmt(body, symbols);
                self.emit_byte(OpCode::Jump.into());
                
                let offset = (loop_start as i32) - (self.code.len() as i32) - 4;
                self.emit_int(offset);
                
                self.patch_jump(exit_jump);
            }
            Stmt::Expression(expr) => {
                self.emit_expr(expr, symbols);
                // An expression used as a statement should have its result popped.
                self.emit_byte(OpCode::Pop.into());
            }
            Stmt::Function(name, params, body, _, _is_static) => {
                // 1. Jump over the function body so it doesn't execute linearly
                let jump_over = self.emit_jump(OpCode::Jump.into());

                // 2. Record function entry point
                let entry_point = self.code.len() as u32;
                if let Some(info) = symbols.functions.get_mut(name) {
                    info.address = entry_point;
                }

                // Setup locals for emission
                let old_locals = symbols.locals.clone();
                let old_local_index = symbols.next_local_index;
                symbols.locals.clear();
                symbols.next_local_index = 0;

                for (param_name, _) in params {
                    symbols.locals.insert(param_name.clone(), symbols.next_local_index);
                    symbols.next_local_index += 1;
                }

                // 3. Emit Body
                for s in body {
                    self.emit_stmt(s, symbols);
                }
                
                self.emit_byte(OpCode::Return.into()); // Implicit return
                self.patch_jump(jump_over);

                // Restore locals
                symbols.locals = old_locals;
                symbols.next_local_index = old_local_index;
            }
            Stmt::Class(name, parent, fields, methods, implements) => {
                // Build field map, separating instance and static fields
                let mut field_map = HashMap::new();
                let mut static_field_map = HashMap::new();

                // If there's a parent, inherit its fields first
                let mut instance_idx = 0u32;
                if let Some(parent_name) = parent {
                    if let Some(parent_info) = symbols.classes.get(parent_name).cloned() {
                        // Copy parent instance fields
                        for (fname, (idx, vis)) in &parent_info.fields {
                            field_map.insert(fname.clone(), (*idx, vis.clone()));
                            if *idx >= instance_idx { instance_idx = *idx + 1; }
                        }
                        // Copy parent static fields
                        for (fname, idx) in &parent_info.static_fields {
                            static_field_map.insert(fname.clone(), *idx);
                        }
                    } else {
                        panic!("Parent class '{}' not found for class '{}'", parent_name, name);
                    }
                }

                for (f, _, vis, is_static) in fields.iter() {
                    if *is_static {
                        // Static fields are stored as globals
                        let global_idx = symbols.next_var_index;
                        symbols.next_var_index += 1;
                        static_field_map.insert(f.clone(), global_idx);
                    } else {
                        field_map.insert(f.clone(), (instance_idx, vis.clone()));
                        instance_idx += 1;
                    }
                }
                
                let mut method_sigs: Vec<crate::semant::MethodSignature> = Vec::new();
                let mut static_method_names = Vec::new();
                for m in methods {
                    if let Stmt::Function(fname, params, _, vis, is_static) = m {
                        let short_name = fname.strip_prefix(&format!("{}_", name)).unwrap_or(fname);
                        // Extract base method name (strip type suffix for display)
                        let base_name = short_name.split('_').next().unwrap_or(short_name).to_string();
                        let param_types: Vec<crate::ast::Type> = params.iter()
                            .filter(|(n, _)| n != "this")
                            .map(|(_, t)| t.clone())
                            .collect();
                        method_sigs.push(crate::semant::MethodSignature {
                            name: base_name.clone(),
                            param_types,
                            visibility: vis.clone(),
                            is_static: *is_static,
                            mangled_name: fname.clone(),
                        });
                        if *is_static {
                            static_method_names.push(short_name.to_string());
                        }
                    }
                }

                // Verify interface implementations
                for iface_name in implements {
                    if let Some(iface_info) = symbols.interfaces.get(iface_name) {
                        for (method_name, _, _) in &iface_info.method_signatures {
                            if !method_sigs.iter().any(|ms| ms.name == *method_name) {
                                panic!("Class '{}' does not implement method '{}' required by interface '{}'",
                                    name, method_name, iface_name);
                            }
                        }
                    } else {
                        panic!("Interface '{}' not found", iface_name);
                    }
                }

                symbols.classes.insert(name.clone(), ClassInfo {
                    name: name.clone(),
                    fields: field_map,
                    methods: method_sigs,
                    static_fields: static_field_map,
                    static_methods: static_method_names,
                    parent: parent.clone(),
                });

                // Emit methods
                let old_class = self.current_class.clone();
                self.current_class = Some(name.clone());
                for method in methods {
                    self.emit_stmt(&method, symbols);
                }
                self.current_class = old_class;
            }
            Stmt::FieldSet(obj, field, value) => {
                // Check for static field set (ClassName.field_name = value)
                if let Expr::Variable(var_name) = &**obj {
                    let mut is_static_global_idx = None;
                    if symbols.classes.contains_key(var_name) && !symbols.locals.contains_key(var_name) && !symbols.variables.contains_key(var_name) {
                        if let Some(class_info) = symbols.classes.get(var_name) {
                            if let Some(global_idx) = class_info.static_fields.get(field) {
                                is_static_global_idx = Some(*global_idx);
                            } else {
                                panic!("Static field '{}' not found in class '{}'", field, var_name);
                            }
                        }
                    }
                    if let Some(global_idx) = is_static_global_idx {
                        self.emit_expr(value, symbols); // Push value to assign
                        self.emit_byte(OpCode::StoreGlobal.into());
                        self.emit_int(global_idx as i32);
                        return;
                    }
                }

                self.emit_expr(obj, symbols);   // Push object ref
                self.emit_expr(value, symbols); // Push value to assign
                
                // Resolve field index
                let mut field_idx = None;
                // FIXME: no type tracking
                for cls in symbols.classes.values() {
                    if let Some((idx, vis)) = cls.fields.get(field) {
                        if *vis == crate::ast::Visibility::Private && self.current_class.as_deref() != Some(cls.name.as_str()) {
                            panic!("Cannot access private field '{}' of class '{}'", field, cls.name);
                        }
                        field_idx = Some(*idx);
                        break;
                    }
                }
                let idx = field_idx.expect(&format!("Field '{}' not found in any known class", field));
                
                self.emit_byte(OpCode::SetField.into());
                self.emit_int(idx as i32);
            }
            Stmt::Interface(name, signatures) => {
                // Interfaces are compile-time only — no bytecode emitted
                symbols.interfaces.insert(name.clone(), crate::semant::InterfaceInfo {
                    name: name.clone(),
                    method_signatures: signatures.clone(),
                });
            }
        }
    }

    pub fn write_file(&self, path: &str) -> std::io::Result<()> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);
        writer.write_all(b"AMBR")?; // Magic
        writer.write_all(&1u16.to_le_bytes())?; // Version
        writer.write_all(&0u32.to_le_bytes())?; // Entry point placeholder
        
        // Write Constant Pool
        writer.write_all(&(self.constants.len() as u32).to_le_bytes())?;
        for s in &self.constants {
            writer.write_all(&(s.len() as u32).to_le_bytes())?;
            writer.write_all(s.as_bytes())?;
        }

        writer.write_all(&(self.code.len() as u32).to_le_bytes())?;
        writer.write_all(&self.code)?;
        Ok(())
    }
}
