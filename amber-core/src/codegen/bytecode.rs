// amber-core/src/codegen/bytecode.rs

#[repr(u8)]
pub enum OpCode {
    // --- Control Flow ---
    Halt = 0x00,
    Jump = 0x01,
    JumpIfFalse = 0x02,

    // --- Constants & Variables ---
    Push = 0x10,
    StoreGlobal = 0x11,
    LoadGlobal = 0x12,
    StoreLocal = 0x13,
    LoadLocal = 0x14,
    LoadConst = 0x15,
    NewArray = 0x16,
    StoreArray = 0x17,
    LoadArray = 0x18,

    // --- Basic Types ---
    PushFloat = 0x19,
    PushBool = 0x1A,
    PushChar = 0x1B,

    // --- Arithmetic & Logic ---
    Add = 0x20,
    Sub = 0x21,
    Mul = 0x22,
    Div = 0x23,
    Less = 0x24,
    Greater = 0x25,

    // --- Object-Oriented ---
    NewInstance = 0x40,
    GetField = 0x41,
    SetField = 0x42,

    // --- Functions & Calls ---
    Call = 0x30,
    Return = 0x31,

    // --- Utilities ---
    Pop = 0x80,
    Print = 0x81,
}

impl From<OpCode> for u8 {
    fn from(op: OpCode) -> u8 { op as u8 }
}