// amber-core/src/codegen/ir.rs
// Amberlink IR — the structured backend interface.
//
// The emitter lowers AST directly to bytes today. This module models that same
// program as structured data: one IrInstr per operation with decoded operands.
// It is the interface future backends (LLVM, AOT — see MISC roadmap) consume
// instead of raw bytes; the bytecode backend is its first consumer.
//
// Guarantees:
// - decode() covers every opcode the emitter can produce; unknown bytes error.
// - Jump targets are resolved to absolute byte offsets (the VM executes jumps
//   relative to the end of the operand: ip += 4; ip += offset).
// - encode() is the exact inverse of decode(): re-encoding a decoded program
//   reproduces the input bytes, so decoder drift is detectable (round-trip).

use super::bytecode::OpCode;

#[derive(Debug, Clone, PartialEq)]
pub enum IrInstr {
    Halt,
    Jump { target: usize },
    JumpIfFalse { target: usize },
    Push(i32),
    StoreGlobal(i32),
    LoadGlobal(i32),
    StoreLocal(i32),
    LoadLocal(i32),
    LoadConst(i32),
    NewArray,
    StoreArray,
    LoadArray,
    PushFloat(f32),
    PushBool(bool),
    PushChar(i32),
    Add,
    Sub,
    Mul,
    Div,
    Less,
    Greater,
    Equal,
    NotEqual,
    LessEqual,
    GreaterEqual,
    Call { addr: i32, argc: u8 },
    Spawn { addr: i32, argc: u8 },
    Return,
    CallNative(u16),
    NewInstance { name_idx: i32, field_count: i32 },
    GetField(i32),
    SetField(i32),
    NewList,
    ListAdd,
    ListGet,
    ListSet,
    ListSize,
    Pop,
    Print,
}

/// A decoded program: (byte offset, instruction) pairs in emission order.
pub type IrProgram = Vec<(usize, IrInstr)>;

fn read_i32(code: &[u8], pos: &mut usize) -> Result<i32, String> {
    if *pos + 4 > code.len() {
        return Err(format!("truncated i32 operand at offset {}", pos));
    }
    let mut b = [0u8; 4];
    b.copy_from_slice(&code[*pos..*pos + 4]);
    *pos += 4;
    Ok(i32::from_le_bytes(b))
}

fn read_f32(code: &[u8], pos: &mut usize) -> Result<f32, String> {
    if *pos + 4 > code.len() {
        return Err(format!("truncated f32 operand at offset {}", pos));
    }
    let mut b = [0u8; 4];
    b.copy_from_slice(&code[*pos..*pos + 4]);
    *pos += 4;
    Ok(f32::from_le_bytes(b))
}

fn read_u16(code: &[u8], pos: &mut usize) -> Result<u16, String> {
    if *pos + 2 > code.len() {
        return Err(format!("truncated u16 operand at offset {}", pos));
    }
    let mut b = [0u8; 2];
    b.copy_from_slice(&code[*pos..*pos + 2]);
    *pos += 2;
    Ok(u16::from_le_bytes(b))
}

/// Decodes raw bytecode into structured IR, resolving jump targets.
pub fn decode(code: &[u8]) -> Result<IrProgram, String> {
    let mut pos = 0usize;
    let mut out: IrProgram = Vec::new();
    while pos < code.len() {
        let off = pos;
        let op = code[pos];
        pos += 1;
        // Match on raw values: OpCode has no TryFrom<u8>, and unknown bytes must error.
        let instr = match op {
            x if x == OpCode::Halt as u8 => IrInstr::Halt,
            x if x == OpCode::Jump as u8 => {
                let rel = read_i32(code, &mut pos)?;
                IrInstr::Jump { target: (pos as i32 + rel) as usize }
            }
            x if x == OpCode::JumpIfFalse as u8 => {
                let rel = read_i32(code, &mut pos)?;
                IrInstr::JumpIfFalse { target: (pos as i32 + rel) as usize }
            }
            x if x == OpCode::Push as u8 => IrInstr::Push(read_i32(code, &mut pos)?),
            x if x == OpCode::StoreGlobal as u8 => IrInstr::StoreGlobal(read_i32(code, &mut pos)?),
            x if x == OpCode::LoadGlobal as u8 => IrInstr::LoadGlobal(read_i32(code, &mut pos)?),
            x if x == OpCode::StoreLocal as u8 => IrInstr::StoreLocal(read_i32(code, &mut pos)?),
            x if x == OpCode::LoadLocal as u8 => IrInstr::LoadLocal(read_i32(code, &mut pos)?),
            x if x == OpCode::LoadConst as u8 => IrInstr::LoadConst(read_i32(code, &mut pos)?),
            x if x == OpCode::NewArray as u8 => IrInstr::NewArray,
            x if x == OpCode::StoreArray as u8 => IrInstr::StoreArray,
            x if x == OpCode::LoadArray as u8 => IrInstr::LoadArray,
            x if x == OpCode::PushFloat as u8 => IrInstr::PushFloat(read_f32(code, &mut pos)?),
            x if x == OpCode::PushBool as u8 => {
                if pos >= code.len() {
                    return Err(format!("truncated u8 operand at offset {}", pos));
                }
                let v = code[pos] != 0;
                pos += 1;
                IrInstr::PushBool(v)
            }
            x if x == OpCode::PushChar as u8 => IrInstr::PushChar(read_i32(code, &mut pos)?),
            x if x == OpCode::Add as u8 => IrInstr::Add,
            x if x == OpCode::Sub as u8 => IrInstr::Sub,
            x if x == OpCode::Mul as u8 => IrInstr::Mul,
            x if x == OpCode::Div as u8 => IrInstr::Div,
            x if x == OpCode::Less as u8 => IrInstr::Less,
            x if x == OpCode::Greater as u8 => IrInstr::Greater,
            x if x == OpCode::Equal as u8 => IrInstr::Equal,
            x if x == OpCode::NotEqual as u8 => IrInstr::NotEqual,
            x if x == OpCode::LessEqual as u8 => IrInstr::LessEqual,
            x if x == OpCode::GreaterEqual as u8 => IrInstr::GreaterEqual,
            x if x == OpCode::Call as u8 => {
                let addr = read_i32(code, &mut pos)?;
                if pos >= code.len() {
                    return Err(format!("truncated u8 operand at offset {}", pos));
                }
                let argc = code[pos];
                pos += 1;
                IrInstr::Call { addr, argc }
            }
            x if x == OpCode::Spawn as u8 => {
                let addr = read_i32(code, &mut pos)?;
                if pos >= code.len() {
                    return Err(format!("truncated u8 operand at offset {}", pos));
                }
                let argc = code[pos];
                pos += 1;
                IrInstr::Spawn { addr, argc }
            }
            x if x == OpCode::Return as u8 => IrInstr::Return,
            x if x == OpCode::CallNative as u8 => IrInstr::CallNative(read_u16(code, &mut pos)?),
            x if x == OpCode::NewInstance as u8 => {
                let name_idx = read_i32(code, &mut pos)?;
                let field_count = read_i32(code, &mut pos)?;
                IrInstr::NewInstance { name_idx, field_count }
            }
            x if x == OpCode::GetField as u8 => IrInstr::GetField(read_i32(code, &mut pos)?),
            x if x == OpCode::SetField as u8 => IrInstr::SetField(read_i32(code, &mut pos)?),
            x if x == OpCode::NewList as u8 => IrInstr::NewList,
            x if x == OpCode::ListAdd as u8 => IrInstr::ListAdd,
            x if x == OpCode::ListGet as u8 => IrInstr::ListGet,
            x if x == OpCode::ListSet as u8 => IrInstr::ListSet,
            x if x == OpCode::ListSize as u8 => IrInstr::ListSize,
            x if x == OpCode::Pop as u8 => IrInstr::Pop,
            x if x == OpCode::Print as u8 => IrInstr::Print,
            other => return Err(format!("unknown opcode 0x{:02X} at offset {}", other, off)),
        };
        out.push((off, instr));
    }
    Ok(out)
}

fn push_i32(out: &mut Vec<u8>, v: i32) {
    out.extend_from_slice(&v.to_le_bytes());
}

/// Re-encodes IR to bytes. Must reproduce decode()'s input exactly (round-trip).
pub fn encode(prog: &IrProgram) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    for (off, instr) in prog {
        match instr {
            IrInstr::Halt => out.push(OpCode::Halt.into()),
            IrInstr::Jump { target } => {
                out.push(OpCode::Jump.into());
                // Stored relative to the end of this operand, like the emitter.
                push_i32(&mut out, *target as i32 - (*off + 5) as i32);
            }
            IrInstr::JumpIfFalse { target } => {
                out.push(OpCode::JumpIfFalse.into());
                push_i32(&mut out, *target as i32 - (*off + 5) as i32);
            }
            IrInstr::Push(v) => { out.push(OpCode::Push.into()); push_i32(&mut out, *v); }
            IrInstr::StoreGlobal(v) => { out.push(OpCode::StoreGlobal.into()); push_i32(&mut out, *v); }
            IrInstr::LoadGlobal(v) => { out.push(OpCode::LoadGlobal.into()); push_i32(&mut out, *v); }
            IrInstr::StoreLocal(v) => { out.push(OpCode::StoreLocal.into()); push_i32(&mut out, *v); }
            IrInstr::LoadLocal(v) => { out.push(OpCode::LoadLocal.into()); push_i32(&mut out, *v); }
            IrInstr::LoadConst(v) => { out.push(OpCode::LoadConst.into()); push_i32(&mut out, *v); }
            IrInstr::NewArray => out.push(OpCode::NewArray.into()),
            IrInstr::StoreArray => out.push(OpCode::StoreArray.into()),
            IrInstr::LoadArray => out.push(OpCode::LoadArray.into()),
            IrInstr::PushFloat(v) => { out.push(OpCode::PushFloat.into()); out.extend_from_slice(&v.to_le_bytes()); }
            IrInstr::PushBool(v) => { out.push(OpCode::PushBool.into()); out.push(u8::from(*v)); }
            IrInstr::PushChar(v) => { out.push(OpCode::PushChar.into()); push_i32(&mut out, *v); }
            IrInstr::Add => out.push(OpCode::Add.into()),
            IrInstr::Sub => out.push(OpCode::Sub.into()),
            IrInstr::Mul => out.push(OpCode::Mul.into()),
            IrInstr::Div => out.push(OpCode::Div.into()),
            IrInstr::Less => out.push(OpCode::Less.into()),
            IrInstr::Greater => out.push(OpCode::Greater.into()),
            IrInstr::Equal => out.push(OpCode::Equal.into()),
            IrInstr::NotEqual => out.push(OpCode::NotEqual.into()),
            IrInstr::LessEqual => out.push(OpCode::LessEqual.into()),
            IrInstr::GreaterEqual => out.push(OpCode::GreaterEqual.into()),
            IrInstr::Call { addr, argc } => { out.push(OpCode::Call.into()); push_i32(&mut out, *addr); out.push(*argc); }
            IrInstr::Spawn { addr, argc } => { out.push(OpCode::Spawn.into()); push_i32(&mut out, *addr); out.push(*argc); }
            IrInstr::Return => out.push(OpCode::Return.into()),
            IrInstr::CallNative(id) => { out.push(OpCode::CallNative.into()); out.extend_from_slice(&id.to_le_bytes()); }
            IrInstr::NewInstance { name_idx, field_count } => {
                out.push(OpCode::NewInstance.into());
                push_i32(&mut out, *name_idx);
                push_i32(&mut out, *field_count);
            }
            IrInstr::GetField(v) => { out.push(OpCode::GetField.into()); push_i32(&mut out, *v); }
            IrInstr::SetField(v) => { out.push(OpCode::SetField.into()); push_i32(&mut out, *v); }
            IrInstr::NewList => out.push(OpCode::NewList.into()),
            IrInstr::ListAdd => out.push(OpCode::ListAdd.into()),
            IrInstr::ListGet => out.push(OpCode::ListGet.into()),
            IrInstr::ListSet => out.push(OpCode::ListSet.into()),
            IrInstr::ListSize => out.push(OpCode::ListSize.into()),
            IrInstr::Pop => out.push(OpCode::Pop.into()),
            IrInstr::Print => out.push(OpCode::Print.into()),
        }
    }
    out
}

/// Formats one instruction. LoadConst shows the pooled string when available.
pub fn format_instr(instr: &IrInstr, constants: &[String]) -> String {
    match instr {
        IrInstr::Halt => "halt".into(),
        IrInstr::Jump { target } => format!("jump @{}", target),
        IrInstr::JumpIfFalse { target } => format!("jump_if_false @{}", target),
        IrInstr::Push(v) => format!("push {}", v),
        IrInstr::StoreGlobal(v) => format!("store_global {}", v),
        IrInstr::LoadGlobal(v) => format!("load_global {}", v),
        IrInstr::StoreLocal(v) => format!("store_local {}", v),
        IrInstr::LoadLocal(v) => format!("load_local {}", v),
        IrInstr::LoadConst(v) => match constants.get(*v as usize) {
            Some(s) => format!("load_const {} ; {:?}", v, s),
            None => format!("load_const {}", v),
        },
        IrInstr::NewArray => "new_array".into(),
        IrInstr::StoreArray => "store_array".into(),
        IrInstr::LoadArray => "load_array".into(),
        IrInstr::PushFloat(v) => format!("push_float {}", v),
        IrInstr::PushBool(v) => format!("push_bool {}", v),
        IrInstr::PushChar(v) => format!("push_char {}", v),
        IrInstr::Add => "add".into(),
        IrInstr::Sub => "sub".into(),
        IrInstr::Mul => "mul".into(),
        IrInstr::Div => "div".into(),
        IrInstr::Less => "less".into(),
        IrInstr::Greater => "greater".into(),
        IrInstr::Equal => "equal".into(),
        IrInstr::NotEqual => "not_equal".into(),
        IrInstr::LessEqual => "less_equal".into(),
        IrInstr::GreaterEqual => "greater_equal".into(),
        IrInstr::Call { addr, argc } => format!("call @{} argc={}", addr, argc),
        IrInstr::Spawn { addr, argc } => format!("spawn @{} argc={}", addr, argc),
        IrInstr::Return => "return".into(),
        IrInstr::CallNative(id) => format!("call_native {}", id),
        IrInstr::NewInstance { name_idx, field_count } => match constants.get(*name_idx as usize) {
            Some(s) => format!("new_instance {:?} fields={}", s, field_count),
            None => format!("new_instance {} fields={}", name_idx, field_count),
        },
        IrInstr::GetField(v) => format!("get_field {}", v),
        IrInstr::SetField(v) => format!("set_field {}", v),
        IrInstr::NewList => "new_list".into(),
        IrInstr::ListAdd => "list_add".into(),
        IrInstr::ListGet => "list_get".into(),
        IrInstr::ListSet => "list_set".into(),
        IrInstr::ListSize => "list_size".into(),
        IrInstr::Pop => "pop".into(),
        IrInstr::Print => "print".into(),
    }
}

/// Formats a whole program as `offset: instr` lines.
pub fn format_program(prog: &IrProgram, constants: &[String]) -> String {
    let mut out = String::new();
    for (off, instr) in prog {
        out.push_str(&format!("{:04}: {}\n", off, format_instr(instr, constants)));
    }
    out
}
