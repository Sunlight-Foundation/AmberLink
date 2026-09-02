// amber-core/src/codegen/emitter.rs
use std::fs::File;
use std::io::{Write, BufWriter};
use super::bytecode::OpCode;
use crate::ast::{Expr, Op};
use crate::ast::Stmt;
use crate::error::{ErrorList, CompileError};
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
    pub fn emit_u16(&mut self, val: u16) {
        self.code.extend_from_slice(&val.to_le_bytes());
    }
    pub fn emit_int(&mut self, val: i32) {
        self.code.extend_from_slice(&val.to_le_bytes());
    }
    pub fn emit_float(&mut self, val: f32) {
        self.code.extend_from_slice(&val.to_le_bytes());
    }

    // Emits a safe placeholder value (INT 0) after a compile error so the stack
    // stays balanced even though the generated code (discarded on error) is wrong.
    fn emit_error_fallback(&mut self) {
        self.emit_byte(OpCode::Push.into());
        self.emit_int(0);
    }

    pub fn emit_expr(&mut self, expr: &Expr, symbols: &mut SymbolTable, errors: &mut ErrorList) {
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
                let index = if let Some(idx) = self.constants.iter().position(|c| c == s) {
                    idx
                } else {
                    self.constants.push(s.clone());
                    self.constants.len() - 1
                };

                self.emit_byte(OpCode::LoadConst.into());
                self.emit_int(index as i32);
            }
            Expr::Error => {
                self.emit_error_fallback();
            }
            Expr::NewArray(size) => {
                self.emit_expr(size, symbols, errors);
                self.emit_byte(OpCode::NewArray.into());
            }
            Expr::NewList => {
                self.emit_byte(OpCode::NewList.into());
            }
            Expr::ListGet(list_expr, index_expr) => {
                self.emit_expr(list_expr, symbols, errors);
                self.emit_expr(index_expr, symbols, errors);
                self.emit_byte(OpCode::ListGet.into());
            }
            Expr::ListSize(list_expr) => {
                self.emit_expr(list_expr, symbols, errors);
                self.emit_byte(OpCode::ListSize.into());
            }
            Expr::NewInstance(class_name, args) => {
                let class_info = match symbols.classes.get(class_name) {
                    Some(ci) => ci.clone(),
                    None => {
                        errors.push(CompileError::new(0, 0, format!("Undefined class: {}", class_name)));
                        self.emit_error_fallback();
                        return;
                    }
                };

                self.emit_byte(OpCode::NewInstance.into());

                let name_idx = self.add_constant(class_name.clone());
                self.emit_int(name_idx as i32);
                self.emit_int(class_info.fields.len() as i32);

                let init_name = format!("{}_init", class_name);
                if symbols.functions.contains_key(&init_name) {
                    for arg in args {
                        self.emit_expr(arg, symbols, errors);
                    }
                    self.emit_byte(OpCode::Call.into());
                    self.calls_to_patch.push((self.code.len(), init_name));
                    self.emit_int(0);
                    self.emit_byte((args.len() + 1) as u8); // +1 for 'this'
                }
            }
            Expr::GetField(obj_expr, field_name) => {
                if let Expr::Variable(var_name) = &**obj_expr {
                    if symbols.classes.contains_key(var_name) && !symbols.locals.contains_key(var_name) && !symbols.variables.contains_key(var_name) {
                        if let Some(class_info) = symbols.classes.get(var_name) {
                            if let Some((global_idx, _ft)) = class_info.static_fields.get(field_name) {
                                self.emit_byte(OpCode::LoadGlobal.into());
                                self.emit_int(*global_idx as i32);
                                return;
                            } else {
                                errors.push(CompileError::new(0, 0, format!("Static field '{}' not found in class '{}'", field_name, var_name)));
                                self.emit_error_fallback();
                                return;
                            }
                        }
                    }
                }

                self.emit_expr(obj_expr, symbols, errors); // Push object ref

                // Find field index across all classes.
                let mut field_idx = None;

                let mut classes: Vec<_> = symbols.classes.values().collect();
                classes.sort_by_key(|c| &c.name);

                for cls in classes {
                    if let Some((idx, vis, _ft)) = cls.fields.get(field_name) {
                        if *vis == crate::ast::Visibility::Private && self.current_class.as_deref() != Some(cls.name.as_str()) {
                            errors.push(CompileError::new(0, 0, format!("Cannot access private field '{}' of class '{}'", field_name, cls.name)));
                        }
                        field_idx = Some(*idx);
                        break;
                    }
                }
                let idx = match field_idx {
                    Some(idx) => idx,
                    None => {
                        errors.push(CompileError::new(0, 0, format!("Field '{}' not found in any known class", field_name)));
                        self.emit_error_fallback();
                        return;
                    }
                };

                self.emit_byte(OpCode::GetField.into());
                self.emit_int(idx as i32);
            }
            Expr::MethodCall(obj, method_name, args) => {
                // Intercept List built-in methods before class dispatch
                let is_list_var = if let Expr::Variable(var_name) = &**obj {
                    matches!(symbols.variable_types.get(var_name), Some(crate::ast::Type::List))
                } else { false };

                if is_list_var {
                    match method_name.as_str() {
                        "add" if args.len() == 1 => {
                            self.emit_expr(obj, symbols, errors);
                            self.emit_expr(&args[0], symbols, errors);
                            self.emit_byte(OpCode::ListAdd.into());
                            return;
                        }
                        "set" if args.len() == 2 => {
                            self.emit_expr(obj, symbols, errors);
                            self.emit_expr(&args[0], symbols, errors);
                            self.emit_expr(&args[1], symbols, errors);
                            self.emit_byte(OpCode::ListSet.into());
                            return;
                        }
                        "get" if args.len() == 1 => {
                            self.emit_expr(obj, symbols, errors);
                            self.emit_expr(&args[0], symbols, errors);
                            self.emit_byte(OpCode::ListGet.into());
                            return;
                        }
                        "size" if args.is_empty() => {
                            self.emit_expr(obj, symbols, errors);
                            self.emit_byte(OpCode::ListSize.into());
                            return;
                        }
                        _ => {}
                    }
                }

                let mut is_static_call = false;
                if let Expr::Variable(var_name) = &**obj {
                    if symbols.classes.contains_key(var_name) && !symbols.locals.contains_key(var_name) && !symbols.variables.contains_key(var_name) {
                        is_static_call = true;
                    }
                }

                if !is_static_call {
                    self.emit_expr(obj, symbols, errors); // 1. Push Object (this)
                }

                for arg in args {
                    self.emit_expr(arg, symbols, errors); // 2. Push Args
                }

                // Find which class has this method (walk parent chain for inheritance)
                let mut found_class = None;
                let mut classes: Vec<_> = symbols.classes.values().collect();
                classes.sort_by_key(|c| &c.name);

                for cls in &classes {
                    if cls.methods.iter().any(|ms| ms.name == *method_name) {
                        found_class = Some(cls.name.clone());
                        break;
                    }
                }

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

                let class_name = match found_class {
                    Some(c) => c,
                    None => {
                        errors.push(CompileError::new(0, 0, format!("Method '{}' not found in any known class", method_name)));
                        self.emit_error_fallback();
                        return;
                    }
                };

                // Resolve overloaded method by matching arg count
                let class_info = match symbols.classes.get(&class_name) {
                    Some(ci) => ci,
                    None => {
                        errors.push(CompileError::new(0, 0, format!("Class '{}' not found", class_name)));
                        self.emit_error_fallback();
                        return;
                    }
                };
                let matching: Vec<_> = class_info.methods.iter()
                    .filter(|ms| ms.name == *method_name && ms.param_types.len() == args.len() && ms.is_static == is_static_call)
                    .collect();
                let method_sig = if matching.len() == 1 {
                    &matching[0]
                } else if matching.is_empty() {
                    errors.push(CompileError::new(0, 0, format!("No matching overload for method '{}' with {} args (static: {})", method_name, args.len(), is_static_call)));
                    self.emit_error_fallback();
                    return;
                } else {
                    // Multiple matches with same arg count — pick first (type matching not implemented yet)
                    &matching[0]
                };

                if method_sig.visibility == crate::ast::Visibility::Private && self.current_class.as_deref() != Some(class_name.as_str()) {
                    errors.push(CompileError::new(0, 0, format!("Cannot access private method '{}' of class '{}'", method_name, class_name)));
                }

                let full_name = method_sig.mangled_name.clone();

                self.emit_byte(OpCode::Call.into());
                self.calls_to_patch.push((self.code.len(), full_name));
                self.emit_int(0);
                self.emit_byte((args.len() + if is_static_call { 0 } else { 1 }) as u8); // +1 for 'this' if not static
            }
            Expr::ArrayAccess(name, index) => {
                if let Some(idx) = symbols.locals.get(name) {
                    self.emit_byte(OpCode::LoadLocal.into());
                    self.emit_int(*idx as i32);
                } else if let Some(idx) = symbols.variables.get(name) {
                    self.emit_byte(OpCode::LoadGlobal.into());
                    self.emit_int(*idx as i32);
                } else {
                    errors.push(CompileError::new(0, 0, format!("Undefined variable: {}", name)));
                    self.emit_error_fallback();
                    return;
                }
                self.emit_expr(index, symbols, errors); // Load index
                self.emit_byte(OpCode::LoadArray.into());
            }
            Expr::Variable(name) => {
                if let Some(index) = symbols.locals.get(name) {
                    self.emit_byte(OpCode::LoadLocal.into());
                    self.emit_int(*index as i32);
                } else if let Some(index) = symbols.variables.get(name) {
                    self.emit_byte(OpCode::LoadGlobal.into());
                    self.emit_int(*index as i32);
                } else {
                    errors.push(CompileError::new(0, 0, format!("Undefined variable: {}", name)));
                    self.emit_error_fallback();
                }
            }
            Expr::Call(name, args) => {
                // Native functions are emitted as OP_CALL_NATIVE with a 2-byte ID.
                if let Some(native) = symbols.natives.get(name) {
                    let native_id = native.id;
                    let arity = native.param_types.len();
                    if arity != args.len() {
                        errors.push(CompileError::new(0, 0, format!("Native function '{}' expects {} arguments, got {}", name, arity, args.len())));
                        self.emit_error_fallback();
                        return;
                    }
                    for arg in args {
                        self.emit_expr(arg, symbols, errors);
                    }
                    self.emit_byte(OpCode::CallNative.into());
                    self.emit_u16(native_id);
                    return;
                }

                for arg in args {
                    self.emit_expr(arg, symbols, errors);
                }
                self.emit_byte(OpCode::Call.into());

                self.calls_to_patch.push((self.code.len(), name.clone()));
                self.emit_int(0);
                self.emit_byte(args.len() as u8);
            }
            Expr::Binary(left, op, right) => {
                // Type check: both operands should be compatible.
                let lt = symbols.infer_type(left);
                let rt = symbols.infer_type(right);
                if let (Some(ref l), Some(ref r)) = (&lt, &rt) {
                    if !symbols.types_compatible(l, r) {
                        errors.push(CompileError::new(0, 0, format!("Type mismatch: cannot apply {:?} to {:?} and {:?}", op, l, r)));
                    }
                }
                self.emit_expr(left, symbols, errors);
                self.emit_expr(right, symbols, errors);
                match op {
                    Op::Add => self.emit_byte(OpCode::Add.into()),
                    Op::Sub => self.emit_byte(OpCode::Sub.into()),
                    Op::Mul => self.emit_byte(OpCode::Mul.into()),
                    Op::Div => self.emit_byte(OpCode::Div.into()),
                    Op::LessThan => self.emit_byte(OpCode::Less.into()),
                    Op::GreaterThan => self.emit_byte(OpCode::Greater.into()),
                    Op::Equals => self.emit_byte(OpCode::Equal.into()),
                    Op::NotEquals => self.emit_byte(OpCode::NotEqual.into()),
                    Op::LessEquals => self.emit_byte(OpCode::LessEqual.into()),
                    Op::GreaterEquals => self.emit_byte(OpCode::GreaterEqual.into()),
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

    pub fn finalize(&mut self, symbols: &SymbolTable, errors: &mut ErrorList) {
        for (index, name) in &self.calls_to_patch {
            let func_info = match symbols.functions.get(name) {
                Some(f) => f,
                None => {
                    errors.push(CompileError::new(0, 0, format!("Undefined function: {}", name)));
                    continue;
                }
            };

            let bytes = (func_info.address as i32).to_le_bytes();
            for i in 0..4 {
                self.code[index + i] = bytes[i];
            }
        }
    }

    pub fn emit_stmt(&mut self, stmt: &Stmt, symbols: &mut SymbolTable, errors: &mut ErrorList) {
        match stmt {
            Stmt::VarDecl(name, decl_type, expr) => {
                // Type check: if declared type is given, verify initializer matches
                if let Some(ref expected) = decl_type {
                    if let Some(actual) = symbols.infer_type(expr) {
                        if !symbols.types_compatible(expected, &actual) {
                            errors.push(CompileError::new(0, 0, format!("Type mismatch: cannot assign {:?} to variable '{}' of type {:?}", actual, name, expected)));
                        }
                    }
                }

                self.emit_expr(expr, symbols, errors);

                // Track declared type for List method dispatch and type inference
                let actual_type = if let Some(t) = decl_type {
                    t.clone()
                } else {
                    symbols.infer_type(expr).unwrap_or(crate::ast::Type::Unknown)
                };
                symbols.variable_types.insert(name.clone(), actual_type);

                let index = symbols.next_var_index;
                symbols.variables.insert(name.clone(), index);
                symbols.next_var_index += 1;

                self.emit_byte(OpCode::StoreGlobal.into());
                self.emit_int(index as i32);
            }
            Stmt::Assign(name, expr) => {
                self.emit_expr(expr, symbols, errors);
                if let Some(index) = symbols.locals.get(name) {
                    self.emit_byte(OpCode::StoreLocal.into());
                    self.emit_int(*index as i32);
                } else if let Some(index) = symbols.variables.get(name) {
                    self.emit_byte(OpCode::StoreGlobal.into());
                    self.emit_int(*index as i32);
                } else {
                    errors.push(CompileError::new(0, 0, format!("Undefined variable: {}", name)));
                }
            }
            Stmt::ArraySet(name, index, value) => {
                if let Some(idx) = symbols.locals.get(name) {
                    self.emit_byte(OpCode::LoadLocal.into());
                    self.emit_int(*idx as i32);
                } else if let Some(idx) = symbols.variables.get(name) {
                    self.emit_byte(OpCode::LoadGlobal.into());
                    self.emit_int(*idx as i32);
                } else {
                    errors.push(CompileError::new(0, 0, format!("Undefined variable: {}", name)));
                    self.emit_error_fallback();
                }
                self.emit_expr(index, symbols, errors);
                self.emit_expr(value, symbols, errors);
                self.emit_byte(OpCode::StoreArray.into());
            }
            Stmt::ListAdd(list_expr, value_expr) => {
                self.emit_expr(list_expr, symbols, errors);
                self.emit_expr(value_expr, symbols, errors);
                self.emit_byte(OpCode::ListAdd.into());
            }
            Stmt::ListSet(list_expr, index_expr, value_expr) => {
                self.emit_expr(list_expr, symbols, errors);
                self.emit_expr(index_expr, symbols, errors);
                self.emit_expr(value_expr, symbols, errors);
                self.emit_byte(OpCode::ListSet.into());
            }
            Stmt::Return(expr) => {
                self.emit_expr(expr, symbols, errors);
                self.emit_byte(OpCode::Return.into());
            }
            Stmt::Print(expr) => {
                self.emit_expr(expr, symbols, errors);
                self.emit_byte(OpCode::Print.into());
            }
            Stmt::Block(stmts) => {
                for s in stmts {
                    self.emit_stmt(s, symbols, errors);
                }
            }
            Stmt::If(cond, then_branch, else_branch) => {
                self.emit_expr(cond, symbols, errors);

                // Jump to Else if false
                let then_jump = self.emit_jump(OpCode::JumpIfFalse.into());

                self.emit_stmt(then_branch, symbols, errors);

                let else_jump = self.emit_jump(OpCode::Jump.into());

                self.patch_jump(then_jump);

                if let Some(else_stmt) = else_branch {
                    self.emit_stmt(else_stmt, symbols, errors);
                }

                self.patch_jump(else_jump);
            }
            Stmt::While(cond, body) => {
                let loop_start = self.code.len();

                self.emit_expr(cond, symbols, errors);
                let exit_jump = self.emit_jump(OpCode::JumpIfFalse.into());

                self.emit_stmt(body, symbols, errors);
                self.emit_byte(OpCode::Jump.into());

                let offset = (loop_start as i32) - (self.code.len() as i32) - 4;
                self.emit_int(offset);

                self.patch_jump(exit_jump);
            }
            Stmt::Expression(expr) => {
                // Check if this is a void List operation (add/set) — these push nothing, skip POP.
                let is_void_list_op = if let Expr::MethodCall(obj, method_name, args) = expr {
                    if let Expr::Variable(var_name) = &**obj {
                        if matches!(symbols.variable_types.get(var_name), Some(crate::ast::Type::List)) {
                            matches!(
                                (method_name.as_str(), args.len()),
                                ("add", 1) | ("set", 2)
                            )
                        } else { false }
                    } else { false }
                } else { false };

                self.emit_expr(expr, symbols, errors);
                if !is_void_list_op {
                    self.emit_byte(OpCode::Pop.into());
                }
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
                    self.emit_stmt(s, symbols, errors);
                }

                self.emit_byte(OpCode::Return.into()); // Implicit return
                self.patch_jump(jump_over);

                // Restore locals
                symbols.locals = old_locals;
                symbols.next_local_index = old_local_index;
            }
            Stmt::Class(name, parent, fields, methods, implements) => {
                let mut field_map = HashMap::new();
                let mut static_field_map = HashMap::new();

                let mut instance_idx = 0u32;
                if let Some(parent_name) = parent {
                    if let Some(parent_info) = symbols.classes.get(parent_name).cloned() {
                        for (fname, (idx, vis, ft)) in &parent_info.fields {
                            field_map.insert(fname.clone(), (*idx, vis.clone(), ft.clone()));
                            if *idx >= instance_idx { instance_idx = *idx + 1; }
                        }
                        for (fname, (idx, ft)) in &parent_info.static_fields {
                            static_field_map.insert(fname.clone(), (*idx, ft.clone()));
                        }
                    } else {
                        errors.push(CompileError::new(0, 0, format!("Parent class '{}' not found for class '{}'", parent_name, name)));
                    }
                }

                for (f, ftype, vis, is_static) in fields.iter() {
                    if *is_static {
                        let global_idx = symbols.next_var_index;
                        symbols.next_var_index += 1;
                        static_field_map.insert(f.clone(), (global_idx, ftype.clone()));
                    } else {
                        field_map.insert(f.clone(), (instance_idx, vis.clone(), ftype.clone()));
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
                                errors.push(CompileError::new(0, 0, format!("Class '{}' does not implement method '{}' required by interface '{}'", name, method_name, iface_name)));
                            }
                        }
                    } else {
                        errors.push(CompileError::new(0, 0, format!("Interface '{}' not found", iface_name)));
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
                    self.emit_stmt(&method, symbols, errors);
                }
                self.current_class = old_class;
            }
            Stmt::FieldSet(obj, field, value) => {
                if let Expr::Variable(var_name) = &**obj {
                    let mut is_static_global_idx = None;
                    if symbols.classes.contains_key(var_name) && !symbols.locals.contains_key(var_name) && !symbols.variables.contains_key(var_name) {
                        if let Some(class_info) = symbols.classes.get(var_name) {
                            if let Some((global_idx, _ft)) = class_info.static_fields.get(field) {
                                is_static_global_idx = Some(*global_idx);
                            } else {
                                errors.push(CompileError::new(0, 0, format!("Static field '{}' not found in class '{}'", field, var_name)));
                            }
                        }
                    }
                    if let Some(global_idx) = is_static_global_idx {
                        self.emit_expr(value, symbols, errors);
                        self.emit_byte(OpCode::StoreGlobal.into());
                        self.emit_int(global_idx as i32);
                        return;
                    }
                }

                self.emit_expr(obj, symbols, errors);
                self.emit_expr(value, symbols, errors);

                let mut field_idx = None;
                for cls in symbols.classes.values() {
                    if let Some((idx, vis, _ft)) = cls.fields.get(field) {
                        if *vis == crate::ast::Visibility::Private && self.current_class.as_deref() != Some(cls.name.as_str()) {
                            errors.push(CompileError::new(0, 0, format!("Cannot access private field '{}' of class '{}'", field, cls.name)));
                        }
                        field_idx = Some(*idx);
                        break;
                    }
                }
                let idx = match field_idx {
                    Some(idx) => idx,
                    None => {
                        errors.push(CompileError::new(0, 0, format!("Field '{}' not found in any known class", field)));
                        return;
                    }
                };

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
            Stmt::Error => {
                // Shouldn't reach emission with parse errors; nothing to emit.
            }
            Stmt::Import(_) => {
                // Imports are resolved by the compiler driver (main.rs) before
                // emission and produce no bytecode of their own.
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
