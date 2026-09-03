// amber-vm/include/bytecode.hpp
#pragma once
#include <cstdint>

enum OpCode : uint8_t {
    // --- Control Flow ---
    OP_HALT           = 0x00, // Stop execution
    OP_JUMP           = 0x01, // Unconditional jump by a 4-byte signed offset
    OP_JUMP_IF_FALSE  = 0x02, // Pop a value; jump if it's 0

    // --- Constants & Variables ---
    OP_PUSH           = 0x10, // Push a 4-byte integer onto the stack
    OP_STORE_GLOBAL   = 0x11, // Pop a value and store it in a global variable slot (by 4-byte index)
    OP_LOAD_GLOBAL    = 0x12, // Load a global variable (by 4-byte index) onto the stack
    OP_STORE_LOCAL    = 0x13, // Pop a value and store it in a local slot (FP + index)
    OP_LOAD_LOCAL     = 0x14, // Load a local variable (FP + index) onto the stack
    OP_LOAD_CONST     = 0x15, // Load a constant from the pool (by 4-byte index)
    OP_NEW_ARRAY      = 0x16, // Pop size, push array reference
    OP_STORE_ARRAY    = 0x17, // Pop value, Pop index, Pop array ref, Store
    OP_LOAD_ARRAY     = 0x18, // Pop index, Pop array ref, Push value

    // New Opcodes for Basic Types
    OP_PUSH_FLOAT     = 0x19, // Push a 4-byte float
    OP_PUSH_BOOL      = 0x1A, // Push a 1-byte bool (0 or 1)
    OP_PUSH_CHAR      = 0x1B, // Push a 4-byte char (UTF-32/int)

    // --- Arithmetic & Logic ---
    OP_ADD            = 0x20,
    OP_SUB            = 0x21,
    OP_MUL            = 0x22,
    OP_DIV            = 0x23,
    OP_LESS           = 0x24, // Pop b, Pop a, Push (a < b)
    OP_GREATER        = 0x25, // Pop b, Pop a, Push (a > b)
    OP_EQUAL          = 0x26, // Pop b, Pop a, Push (a == b)
    OP_NOT_EQUAL      = 0x27, // Pop b, Pop a, Push (a != b)
    OP_LESS_EQUAL     = 0x28, // Pop b, Pop a, Push (a <= b)
    OP_GREATER_EQUAL  = 0x29, // Pop b, Pop a, Push (a >= b)

    // --- Object-Oriented ---
    OP_NEW_INSTANCE   = 0x40, // Operand: Class ID (u32). Push instance ref.
    OP_GET_FIELD      = 0x41, // Operand: Field Index (u32). Pop ref, Push value.
    OP_SET_FIELD      = 0x42, // Operand: Field Index (u32). Pop value, Pop ref.

    // --- Collections (List) ---
    OP_NEW_LIST       = 0x50, // Push list reference
    OP_LIST_ADD       = 0x51, // Pop value, Pop list ref, Add value to list
    OP_LIST_GET       = 0x52, // Pop index, Pop list ref, Push value
    OP_LIST_SET       = 0x53, // Pop value, Pop index, Pop list ref, Set value
    OP_LIST_SIZE      = 0x54, // Pop list ref, Push size

    // --- Functions & Calls ---
    OP_CALL           = 0x30, // Call function at 4-byte address
    OP_RETURN         = 0x31, // Return from function
    OP_CALL_NATIVE    = 0x32, // Operand: Native ID (u16). Pop args, call C++ native, push result.
    OP_SPAWN          = 0x33, // Start a thread at 4-byte address (argc u8). Pop args, push handle.

    // --- Utilities ---
    OP_POP            = 0x80, // Pop the top value from the stack and discard it
    OP_PRINT          = 0x81, // Pop the top value and print it to the console
};
