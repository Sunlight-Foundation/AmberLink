#include "avm.hpp"
#include "bytecode.hpp"
#include "heap.hpp"
#include "value.hpp"
#include "natives.hpp"
#include "vm.hpp"
#include <iostream>
#include <vector>
#include <stack>
#include <stdexcept>
#include <cstdint>
#include <cstring> // For std::memcpy

// Threaded dispatch via computed gotos (GCC/Clang only).
// MSVC does not support computed gotos — falls back to switch.
// Opcode enum values are sparse so we use a 256-entry table with a
// catch-all default for unknown opcodes.
#if defined(__GNUC__) || defined(__clang__)
    #define USE_COMPUTED_GOTO
#endif

static int run_loop(VMContext& vm, const uint8_t* start_ip, std::vector<Value> args,
                    bool is_main, Value& out_result);

int execute(const std::vector<uint8_t>& bytecode, std::vector<std::string>& constants) {
    if (bytecode.empty()) {
        std::cout << "AVM Warning: No bytecode to execute." << std::endl;
        return 0;
    }

    VMContext vm;
    vm.bytecode = &bytecode;
    vm.constants = &constants;
    std::vector<Value> no_args;
    Value result;
    return run_loop(vm, bytecode.data(), no_args, true, result);
}

// The interpreter loop, shared by the main thread and (in a later slice)
// spawned threads. Per-thread state (stacks, ip) is local; VMContext holds
// what threads share. is_main/out_result are dormant until OP_SPAWN lands;
// they fix the call shape now so the next slice touches no signatures.
static int run_loop(VMContext& vm, const uint8_t* start_ip, std::vector<Value> args,
                    bool is_main, Value& out_result) {
    (void)is_main; (void)out_result;

    std::vector<Value> vm_stack(std::move(args));
    vm_stack.reserve(1024);

    Heap& gc = vm.heap;
    std::vector<Value>& globals = vm.globals;
    std::vector<std::string>& constants = *vm.constants;
    std::vector<const uint8_t*> call_stack;
    std::vector<size_t> fp_stack;
    size_t fp = 0;

    // Reused across native calls (declared at function scope so computed-goto
    // dispatch never jumps across its construction/destruction).
    std::vector<Value> native_args;

    const uint8_t* ip = start_ip;
    const uint8_t* end = vm.bytecode->data() + vm.bytecode->size();

    try {
#ifdef USE_COMPUTED_GOTO
        // 256-entry dispatch table — unused slots point to lbl_UNKNOWN.
        static const void* dispatch_table[256];
        static bool table_init = false;
        if (!table_init) {
            for (int i = 0; i < 256; i++) dispatch_table[i] = &&lbl_UNKNOWN;
            dispatch_table[OP_HALT]           = &&lbl_OP_HALT;
            dispatch_table[OP_JUMP]           = &&lbl_OP_JUMP;
            dispatch_table[OP_JUMP_IF_FALSE]  = &&lbl_OP_JUMP_IF_FALSE;
            dispatch_table[OP_PUSH]           = &&lbl_OP_PUSH;
            dispatch_table[OP_PUSH_FLOAT]     = &&lbl_OP_PUSH_FLOAT;
            dispatch_table[OP_PUSH_BOOL]      = &&lbl_OP_PUSH_BOOL;
            dispatch_table[OP_PUSH_CHAR]      = &&lbl_OP_PUSH_CHAR;
            dispatch_table[OP_LOAD_CONST]     = &&lbl_OP_LOAD_CONST;
            dispatch_table[OP_STORE_GLOBAL]   = &&lbl_OP_STORE_GLOBAL;
            dispatch_table[OP_LOAD_GLOBAL]    = &&lbl_OP_LOAD_GLOBAL;
            dispatch_table[OP_STORE_LOCAL]    = &&lbl_OP_STORE_LOCAL;
            dispatch_table[OP_LOAD_LOCAL]     = &&lbl_OP_LOAD_LOCAL;
            dispatch_table[OP_NEW_ARRAY]      = &&lbl_OP_NEW_ARRAY;
            dispatch_table[OP_STORE_ARRAY]    = &&lbl_OP_STORE_ARRAY;
            dispatch_table[OP_LOAD_ARRAY]     = &&lbl_OP_LOAD_ARRAY;
            dispatch_table[OP_ADD]            = &&lbl_OP_ADD;
            dispatch_table[OP_SUB]            = &&lbl_OP_SUB;
            dispatch_table[OP_MUL]            = &&lbl_OP_MUL;
            dispatch_table[OP_DIV]            = &&lbl_OP_DIV;
            dispatch_table[OP_LESS]           = &&lbl_OP_LESS;
            dispatch_table[OP_GREATER]        = &&lbl_OP_GREATER;
            dispatch_table[OP_EQUAL]          = &&lbl_OP_EQUAL;
            dispatch_table[OP_NOT_EQUAL]      = &&lbl_OP_NOT_EQUAL;
            dispatch_table[OP_LESS_EQUAL]     = &&lbl_OP_LESS_EQUAL;
            dispatch_table[OP_GREATER_EQUAL]  = &&lbl_OP_GREATER_EQUAL;
            dispatch_table[OP_NEW_INSTANCE]   = &&lbl_OP_NEW_INSTANCE;
            dispatch_table[OP_GET_FIELD]      = &&lbl_OP_GET_FIELD;
            dispatch_table[OP_SET_FIELD]      = &&lbl_OP_SET_FIELD;
            dispatch_table[OP_NEW_LIST]       = &&lbl_OP_NEW_LIST;
            dispatch_table[OP_LIST_ADD]       = &&lbl_OP_LIST_ADD;
            dispatch_table[OP_LIST_GET]       = &&lbl_OP_LIST_GET;
            dispatch_table[OP_LIST_SET]       = &&lbl_OP_LIST_SET;
            dispatch_table[OP_LIST_SIZE]      = &&lbl_OP_LIST_SIZE;
            dispatch_table[OP_CALL]           = &&lbl_OP_CALL;
            dispatch_table[OP_RETURN]         = &&lbl_OP_RETURN;
            dispatch_table[OP_CALL_NATIVE]    = &&lbl_OP_CALL_NATIVE;
            dispatch_table[OP_POP]            = &&lbl_OP_POP;
            dispatch_table[OP_PRINT]          = &&lbl_OP_PRINT;
            table_init = true;
        }

        #define DISPATCH() if (ip < end) goto *dispatch_table[*ip++]; else goto lbl_OP_HALT
        DISPATCH();

        lbl_OP_HALT:
            return 0;

        lbl_OP_JUMP: {
            int32_t offset; std::memcpy(&offset, ip, 4); ip += 4; ip += offset;
            DISPATCH();
        }
        lbl_OP_JUMP_IF_FALSE: {
            int32_t offset; std::memcpy(&offset, ip, 4); ip += 4;
            if (vm_stack.empty()) throw std::runtime_error("Stack underflow during JUMP_IF_FALSE.");
            Value condition = vm_stack.back(); vm_stack.pop_back();
            bool is_false = false;
            if (condition.type == ValueType::BOOL) is_false = !condition.as.b;
            else if (condition.type == ValueType::INT) is_false = (condition.as.i == 0);
            else throw std::runtime_error("JUMP_IF_FALSE requires a boolean or integer condition.");
            if (is_false) ip += offset;
            DISPATCH();
        }
        lbl_OP_PUSH: {
            int32_t value; std::memcpy(&value, ip, 4); ip += 4;
            vm_stack.push_back(Value(value));
            DISPATCH();
        }
        lbl_OP_PUSH_FLOAT: {
            float value; std::memcpy(&value, ip, 4); ip += 4;
            vm_stack.push_back(Value(value));
            DISPATCH();
        }
        lbl_OP_PUSH_BOOL: {
            vm_stack.push_back(Value(*ip++ != 0));
            DISPATCH();
        }
        lbl_OP_PUSH_CHAR: {
            int32_t value; std::memcpy(&value, ip, 4); ip += 4;
            vm_stack.push_back(Value((char)value));
            DISPATCH();
        }
        lbl_OP_LOAD_CONST: {
            int32_t index; std::memcpy(&index, ip, 4); ip += 4;
            vm_stack.push_back(Value::make_string(index));
            DISPATCH();
        }
        lbl_OP_STORE_GLOBAL: {
            int32_t index; std::memcpy(&index, ip, 4); ip += 4;
            if (vm_stack.empty()) throw std::runtime_error("Stack underflow during STORE.");
            Value val = vm_stack.back(); vm_stack.pop_back();
            if (index >= (int32_t)globals.size()) globals.resize(index + 1);
            globals[index] = val;
            DISPATCH();
        }
        lbl_OP_LOAD_GLOBAL: {
            int32_t index; std::memcpy(&index, ip, 4); ip += 4;
            if (index < 0 || index >= (int32_t)globals.size()) throw std::runtime_error("Global variable index out of bounds.");
            vm_stack.push_back(globals[index]);
            DISPATCH();
        }
        lbl_OP_STORE_LOCAL: {
            int32_t index; std::memcpy(&index, ip, 4); ip += 4;
            if (vm_stack.empty()) throw std::runtime_error("Stack underflow during STORE_LOCAL.");
            Value val = vm_stack.back(); vm_stack.pop_back();
            if (fp + index >= vm_stack.size()) vm_stack.resize(fp + index + 1);
            vm_stack[fp + index] = val;
            DISPATCH();
        }
        lbl_OP_LOAD_LOCAL: {
            int32_t index; std::memcpy(&index, ip, 4); ip += 4;
            vm_stack.push_back(vm_stack[fp + index]);
            DISPATCH();
        }
        lbl_OP_NEW_ARRAY: {
            if (vm_stack.empty()) throw std::runtime_error("Stack underflow during NEW_ARRAY.");
            Value size_val = vm_stack.back(); vm_stack.pop_back();
            if (size_val.type != ValueType::INT) throw std::runtime_error("Array size must be an integer.");
            ArrayObject* arr = new ArrayObject(size_val.as.i);
            vm_stack.push_back(Value::make_obj(gc.register_object(arr)));
            DISPATCH();
        }
        lbl_OP_STORE_ARRAY: {
            if (vm_stack.size() < 3) throw std::runtime_error("Stack underflow during STORE_ARRAY.");
            Value val = vm_stack.back(); vm_stack.pop_back();
            Value idx_val = vm_stack.back(); vm_stack.pop_back();
            Value ref = vm_stack.back(); vm_stack.pop_back();
            if (ref.type != ValueType::OBJ_REF) throw std::runtime_error("Reference is not an object.");
            if (idx_val.type != ValueType::INT) throw std::runtime_error("Array index must be an integer.");
            ArrayObject* arr = dynamic_cast<ArrayObject*>(gc.objects[ref.as.obj_ref]);
            if (!arr) throw std::runtime_error("Reference is not an array.");
            if (idx_val.as.i < 0 || idx_val.as.i >= (int32_t)arr->data.size()) throw std::runtime_error("Array index out of bounds.");
            arr->data[idx_val.as.i] = val;
            DISPATCH();
        }
        lbl_OP_LOAD_ARRAY: {
            if (vm_stack.size() < 2) throw std::runtime_error("Stack underflow during LOAD_ARRAY.");
            Value idx_val = vm_stack.back(); vm_stack.pop_back();
            Value ref = vm_stack.back(); vm_stack.pop_back();
            if (ref.type != ValueType::OBJ_REF) throw std::runtime_error("Reference is not an object.");
            if (idx_val.type != ValueType::INT) throw std::runtime_error("Array index must be an integer.");
            ArrayObject* arr = dynamic_cast<ArrayObject*>(gc.objects[ref.as.obj_ref]);
            if (!arr) throw std::runtime_error("Reference is not an array.");
            if (idx_val.as.i < 0 || idx_val.as.i >= (int32_t)arr->data.size()) throw std::runtime_error("Array index out of bounds.");
            vm_stack.push_back(arr->data[idx_val.as.i]);
            DISPATCH();
        }
        lbl_OP_ADD: {
            if (vm_stack.size() < 2) throw std::runtime_error("Stack underflow during ADD.");
            Value b = vm_stack.back(); vm_stack.pop_back();
            Value a = vm_stack.back(); vm_stack.pop_back();
            if (a.type == ValueType::INT && b.type == ValueType::INT) {
                vm_stack.push_back(Value(a.as.i + b.as.i));
            } else if (a.type == ValueType::FLOAT && b.type == ValueType::FLOAT) {
                vm_stack.push_back(Value(a.as.f + b.as.f));
            } else if (a.type == ValueType::STRING_CONST && b.type == ValueType::STRING_CONST) {
                constants.push_back(constants[a.as.str_idx] + constants[b.as.str_idx]);
                gc.collect(vm_stack, globals, constants.size());
                vm_stack.push_back(Value::make_string(constants.size() - 1));
            } else {
                throw std::runtime_error("Type mismatch: Cannot add incompatible types.");
            }
            DISPATCH();
        }
        lbl_OP_SUB: {
            if (vm_stack.size() < 2) throw std::runtime_error("Stack underflow during SUB.");
            Value b = vm_stack.back(); vm_stack.pop_back();
            Value a = vm_stack.back(); vm_stack.pop_back();
            if (a.type == ValueType::INT && b.type == ValueType::INT) vm_stack.push_back(Value(a.as.i - b.as.i));
            else if (a.type == ValueType::FLOAT && b.type == ValueType::FLOAT) vm_stack.push_back(Value(a.as.f - b.as.f));
            else throw std::runtime_error("Type mismatch: Cannot subtract incompatible types.");
            DISPATCH();
        }
        lbl_OP_MUL: {
            if (vm_stack.size() < 2) throw std::runtime_error("Stack underflow during MUL.");
            Value b = vm_stack.back(); vm_stack.pop_back();
            Value a = vm_stack.back(); vm_stack.pop_back();
            if (a.type == ValueType::INT && b.type == ValueType::INT) vm_stack.push_back(Value(a.as.i * b.as.i));
            else if (a.type == ValueType::FLOAT && b.type == ValueType::FLOAT) vm_stack.push_back(Value(a.as.f * b.as.f));
            else throw std::runtime_error("Type mismatch: Cannot multiply incompatible types.");
            DISPATCH();
        }
        lbl_OP_DIV: {
            if (vm_stack.size() < 2) throw std::runtime_error("Stack underflow during DIV.");
            Value b = vm_stack.back(); vm_stack.pop_back();
            Value a = vm_stack.back(); vm_stack.pop_back();
            if (a.type == ValueType::INT && b.type == ValueType::INT) {
                if (b.as.i == 0) throw std::runtime_error("Division by zero.");
                vm_stack.push_back(Value(a.as.i / b.as.i));
            } else if (a.type == ValueType::FLOAT && b.type == ValueType::FLOAT) {
                if (b.as.f == 0.0f) throw std::runtime_error("Division by zero.");
                vm_stack.push_back(Value(a.as.f / b.as.f));
            } else throw std::runtime_error("Type mismatch: Cannot divide incompatible types.");
            DISPATCH();
        }
        lbl_OP_LESS: {
            if (vm_stack.size() < 2) throw std::runtime_error("Stack underflow during LESS.");
            Value b = vm_stack.back(); vm_stack.pop_back();
            Value a = vm_stack.back(); vm_stack.pop_back();
            if (a.type == ValueType::INT && b.type == ValueType::INT) vm_stack.push_back(Value(a.as.i < b.as.i));
            else if (a.type == ValueType::FLOAT && b.type == ValueType::FLOAT) vm_stack.push_back(Value(a.as.f < b.as.f));
            else if (a.type == ValueType::CHAR && b.type == ValueType::CHAR) vm_stack.push_back(Value(a.as.c < b.as.c));
            else throw std::runtime_error("Type mismatch: Cannot compare incompatible types with LESS.");
            DISPATCH();
        }
        lbl_OP_GREATER: {
            if (vm_stack.size() < 2) throw std::runtime_error("Stack underflow during GREATER.");
            Value b = vm_stack.back(); vm_stack.pop_back();
            Value a = vm_stack.back(); vm_stack.pop_back();
            if (a.type == ValueType::INT && b.type == ValueType::INT) vm_stack.push_back(Value(a.as.i > b.as.i));
            else if (a.type == ValueType::FLOAT && b.type == ValueType::FLOAT) vm_stack.push_back(Value(a.as.f > b.as.f));
            else if (a.type == ValueType::CHAR && b.type == ValueType::CHAR) vm_stack.push_back(Value(a.as.c > b.as.c));
            else throw std::runtime_error("Type mismatch: Cannot compare incompatible types with GREATER.");
            DISPATCH();
        }
        lbl_OP_EQUAL: {
            if (vm_stack.size() < 2) throw std::runtime_error("Stack underflow during EQUAL.");
            Value b = vm_stack.back(); vm_stack.pop_back();
            Value a = vm_stack.back(); vm_stack.pop_back();
            if (a.type == ValueType::INT && b.type == ValueType::INT) vm_stack.push_back(Value(a.as.i == b.as.i));
            else if (a.type == ValueType::FLOAT && b.type == ValueType::FLOAT) vm_stack.push_back(Value(a.as.f == b.as.f));
            else if (a.type == ValueType::BOOL && b.type == ValueType::BOOL) vm_stack.push_back(Value(a.as.b == b.as.b));
            else if (a.type == ValueType::CHAR && b.type == ValueType::CHAR) vm_stack.push_back(Value(a.as.c == b.as.c));
            else if (a.type == ValueType::STRING_CONST && b.type == ValueType::STRING_CONST) vm_stack.push_back(Value(constants[a.as.str_idx] == constants[b.as.str_idx]));
            else throw std::runtime_error("Type mismatch: Cannot compare incompatible types with EQUAL.");
            DISPATCH();
        }
        lbl_OP_NOT_EQUAL: {
            if (vm_stack.size() < 2) throw std::runtime_error("Stack underflow during NOT_EQUAL.");
            Value b = vm_stack.back(); vm_stack.pop_back();
            Value a = vm_stack.back(); vm_stack.pop_back();
            if (a.type == ValueType::INT && b.type == ValueType::INT) vm_stack.push_back(Value(a.as.i != b.as.i));
            else if (a.type == ValueType::FLOAT && b.type == ValueType::FLOAT) vm_stack.push_back(Value(a.as.f != b.as.f));
            else if (a.type == ValueType::BOOL && b.type == ValueType::BOOL) vm_stack.push_back(Value(a.as.b != b.as.b));
            else if (a.type == ValueType::CHAR && b.type == ValueType::CHAR) vm_stack.push_back(Value(a.as.c != b.as.c));
            else if (a.type == ValueType::STRING_CONST && b.type == ValueType::STRING_CONST) vm_stack.push_back(Value(constants[a.as.str_idx] != constants[b.as.str_idx]));
            else throw std::runtime_error("Type mismatch: Cannot compare incompatible types with NOT_EQUAL.");
            DISPATCH();
        }
        lbl_OP_LESS_EQUAL: {
            if (vm_stack.size() < 2) throw std::runtime_error("Stack underflow during LESS_EQUAL.");
            Value b = vm_stack.back(); vm_stack.pop_back();
            Value a = vm_stack.back(); vm_stack.pop_back();
            if (a.type == ValueType::INT && b.type == ValueType::INT) vm_stack.push_back(Value(a.as.i <= b.as.i));
            else if (a.type == ValueType::FLOAT && b.type == ValueType::FLOAT) vm_stack.push_back(Value(a.as.f <= b.as.f));
            else if (a.type == ValueType::CHAR && b.type == ValueType::CHAR) vm_stack.push_back(Value(a.as.c <= b.as.c));
            else throw std::runtime_error("Type mismatch: Cannot compare incompatible types with LESS_EQUAL.");
            DISPATCH();
        }
        lbl_OP_GREATER_EQUAL: {
            if (vm_stack.size() < 2) throw std::runtime_error("Stack underflow during GREATER_EQUAL.");
            Value b = vm_stack.back(); vm_stack.pop_back();
            Value a = vm_stack.back(); vm_stack.pop_back();
            if (a.type == ValueType::INT && b.type == ValueType::INT) vm_stack.push_back(Value(a.as.i >= b.as.i));
            else if (a.type == ValueType::FLOAT && b.type == ValueType::FLOAT) vm_stack.push_back(Value(a.as.f >= b.as.f));
            else if (a.type == ValueType::CHAR && b.type == ValueType::CHAR) vm_stack.push_back(Value(a.as.c >= b.as.c));
            else throw std::runtime_error("Type mismatch: Cannot compare incompatible types with GREATER_EQUAL.");
            DISPATCH();
        }
        lbl_OP_NEW_INSTANCE: {
            int32_t class_name_idx; std::memcpy(&class_name_idx, ip, 4); ip += 4;
            int32_t field_count; std::memcpy(&field_count, ip, 4); ip += 4;
            InstanceObject* obj = new InstanceObject(class_name_idx, field_count);
            vm_stack.push_back(Value::make_obj(gc.register_object(obj)));
            DISPATCH();
        }
        lbl_OP_GET_FIELD: {
            int32_t field_idx; std::memcpy(&field_idx, ip, 4); ip += 4;
            if (vm_stack.empty()) throw std::runtime_error("Stack underflow during GET_FIELD.");
            Value ref = vm_stack.back(); vm_stack.pop_back();
            if (ref.type != ValueType::OBJ_REF) throw std::runtime_error("Reference is not an object.");
            InstanceObject* obj = dynamic_cast<InstanceObject*>(gc.objects[ref.as.obj_ref]);
            if (!obj) throw std::runtime_error("Reference is not an instance.");
            if (field_idx < 0 || field_idx >= (int32_t)obj->fields.size()) throw std::runtime_error("Field index out of bounds.");
            vm_stack.push_back(obj->fields[field_idx]);
            DISPATCH();
        }
        lbl_OP_SET_FIELD: {
            int32_t field_idx; std::memcpy(&field_idx, ip, 4); ip += 4;
            if (vm_stack.size() < 2) throw std::runtime_error("Stack underflow during SET_FIELD.");
            Value val = vm_stack.back(); vm_stack.pop_back();
            Value ref = vm_stack.back(); vm_stack.pop_back();
            if (ref.type != ValueType::OBJ_REF) throw std::runtime_error("Reference is not an object.");
            InstanceObject* obj = dynamic_cast<InstanceObject*>(gc.objects[ref.as.obj_ref]);
            if (!obj) throw std::runtime_error("Reference is not an instance.");
            if (field_idx < 0 || field_idx >= (int32_t)obj->fields.size()) throw std::runtime_error("Field index out of bounds.");
            obj->fields[field_idx] = val;
            DISPATCH();
        }
        lbl_OP_NEW_LIST: {
            ListObject* list = new ListObject();
            vm_stack.push_back(Value::make_obj(gc.register_object(list)));
            DISPATCH();
        }
        lbl_OP_LIST_ADD: {
            if (vm_stack.size() < 2) throw std::runtime_error("Stack underflow during LIST_ADD.");
            Value val = vm_stack.back(); vm_stack.pop_back();
            Value ref = vm_stack.back(); vm_stack.pop_back();
            if (ref.type != ValueType::OBJ_REF) throw std::runtime_error("Reference is not an object.");
            ListObject* list = dynamic_cast<ListObject*>(gc.objects[ref.as.obj_ref]);
            if (!list) throw std::runtime_error("Reference is not a list.");
            list->items.push_back(val);
            DISPATCH();
        }
        lbl_OP_LIST_GET: {
            if (vm_stack.size() < 2) throw std::runtime_error("Stack underflow during LIST_GET.");
            Value idx_val = vm_stack.back(); vm_stack.pop_back();
            Value ref = vm_stack.back(); vm_stack.pop_back();
            if (ref.type != ValueType::OBJ_REF) throw std::runtime_error("Reference is not an object.");
            if (idx_val.type != ValueType::INT) throw std::runtime_error("List index must be an integer.");
            ListObject* list = dynamic_cast<ListObject*>(gc.objects[ref.as.obj_ref]);
            if (!list) throw std::runtime_error("Reference is not a list.");
            if (idx_val.as.i < 0 || idx_val.as.i >= (int32_t)list->items.size()) throw std::runtime_error("List index out of bounds.");
            vm_stack.push_back(list->items[idx_val.as.i]);
            DISPATCH();
        }
        lbl_OP_LIST_SET: {
            if (vm_stack.size() < 3) throw std::runtime_error("Stack underflow during LIST_SET.");
            Value val = vm_stack.back(); vm_stack.pop_back();
            Value idx_val = vm_stack.back(); vm_stack.pop_back();
            Value ref = vm_stack.back(); vm_stack.pop_back();
            if (ref.type != ValueType::OBJ_REF) throw std::runtime_error("Reference is not an object.");
            if (idx_val.type != ValueType::INT) throw std::runtime_error("List index must be an integer.");
            ListObject* list = dynamic_cast<ListObject*>(gc.objects[ref.as.obj_ref]);
            if (!list) throw std::runtime_error("Reference is not a list.");
            if (idx_val.as.i < 0 || idx_val.as.i >= (int32_t)list->items.size()) throw std::runtime_error("List index out of bounds.");
            list->items[idx_val.as.i] = val;
            DISPATCH();
        }
        lbl_OP_LIST_SIZE: {
            if (vm_stack.empty()) throw std::runtime_error("Stack underflow during LIST_SIZE.");
            Value ref = vm_stack.back(); vm_stack.pop_back();
            if (ref.type != ValueType::OBJ_REF) throw std::runtime_error("Reference is not an object.");
            ListObject* list = dynamic_cast<ListObject*>(gc.objects[ref.as.obj_ref]);
            if (!list) throw std::runtime_error("Reference is not a list.");
            vm_stack.push_back(Value((int32_t)list->items.size()));
            DISPATCH();
        }
        lbl_OP_CALL: {
            int32_t target_offset; std::memcpy(&target_offset, ip, 4); ip += 4;
            uint8_t arg_count = *ip++;
            if (vm_stack.size() < arg_count) throw std::runtime_error("Stack underflow during CALL.");
            fp_stack.push_back(fp);
            fp = vm_stack.size() - arg_count;
            call_stack.push_back(ip);
            ip = vm.bytecode->data() + target_offset;
            DISPATCH();
        }
        lbl_OP_RETURN: {
            if (call_stack.empty()) throw std::runtime_error("RETURN with empty call stack.");
            Value result = vm_stack.back(); vm_stack.pop_back();
            vm_stack.resize(fp);
            vm_stack.push_back(result);
            ip = call_stack.back(); call_stack.pop_back();
            fp = fp_stack.back(); fp_stack.pop_back();
            DISPATCH();
        }
        lbl_OP_CALL_NATIVE: {
            uint16_t native_id;
            std::memcpy(&native_id, ip, 2); ip += 2;
            auto& natives = Natives::registry();
            if (native_id >= natives.size()) throw std::runtime_error("Unknown native function ID.");
            NativeEntry& entry = natives[native_id];
            if (vm_stack.size() < (size_t)entry.arity) throw std::runtime_error("Stack underflow during CALL_NATIVE.");
            // Pop args in reverse so the args vector is in call order.
            native_args.resize(entry.arity);
            for (int i = entry.arity - 1; i >= 0; --i) {
                native_args[i] = vm_stack.back(); vm_stack.pop_back();
            }
            Value result = entry.fn(native_args, constants, gc);
            vm_stack.push_back(result);
            DISPATCH();
        }
        lbl_OP_POP:
            vm_stack.pop_back();
            DISPATCH();
        lbl_OP_PRINT: {
            if (vm_stack.empty()) throw std::runtime_error("Stack underflow during PRINT.");
            Value val = vm_stack.back(); vm_stack.pop_back();
            if (val.type == ValueType::STRING_CONST) {
                size_t idx = val.as.str_idx;
                if (idx < constants.size()) std::cout << constants[idx] << std::endl;
                else std::cout << "<Invalid String Index>" << std::endl;
            } else {
                std::cout << val << std::endl;
            }
            DISPATCH();
        }
        lbl_UNKNOWN:
            throw std::runtime_error("Unknown opcode encountered.");

#else
        // --- Fallback switch dispatch (MSVC) ---
        while (ip < end) {
            uint8_t instruction = *ip++;
            switch (instruction) {
                case OP_HALT: return 0;
                case OP_JUMP: {
                    int32_t offset; std::memcpy(&offset, ip, 4); ip += 4; ip += offset; break;
                }
                case OP_JUMP_IF_FALSE: {
                    int32_t offset; std::memcpy(&offset, ip, 4); ip += 4;
                    if (vm_stack.empty()) throw std::runtime_error("Stack underflow during JUMP_IF_FALSE.");
                    Value condition = vm_stack.back(); vm_stack.pop_back();
                    bool is_false = false;
                    if (condition.type == ValueType::BOOL) is_false = !condition.as.b;
                    else if (condition.type == ValueType::INT) is_false = (condition.as.i == 0);
                    else throw std::runtime_error("JUMP_IF_FALSE requires a boolean or integer condition.");
                    if (is_false) ip += offset;
                    break;
                }
                case OP_PUSH: {
                    int32_t value; std::memcpy(&value, ip, 4); ip += 4;
                    vm_stack.push_back(Value(value)); break;
                }
                case OP_PUSH_FLOAT: {
                    float value; std::memcpy(&value, ip, 4); ip += 4;
                    vm_stack.push_back(Value(value)); break;
                }
                case OP_PUSH_BOOL: { vm_stack.push_back(Value(*ip++ != 0)); break; }
                case OP_PUSH_CHAR: {
                    int32_t value; std::memcpy(&value, ip, 4); ip += 4;
                    vm_stack.push_back(Value((char)value)); break;
                }
                case OP_LOAD_CONST: {
                    int32_t index; std::memcpy(&index, ip, 4); ip += 4;
                    vm_stack.push_back(Value::make_string(index)); break;
                }
                case OP_STORE_GLOBAL: {
                    int32_t index; std::memcpy(&index, ip, 4); ip += 4;
                    if (vm_stack.empty()) throw std::runtime_error("Stack underflow during STORE.");
                    Value val = vm_stack.back(); vm_stack.pop_back();
                    if (index >= (int32_t)globals.size()) globals.resize(index + 1);
                    globals[index] = val; break;
                }
                case OP_LOAD_GLOBAL: {
                    int32_t index; std::memcpy(&index, ip, 4); ip += 4;
                    if (index < 0 || index >= (int32_t)globals.size()) throw std::runtime_error("Global variable index out of bounds.");
                    vm_stack.push_back(globals[index]); break;
                }
                case OP_STORE_LOCAL: {
                    int32_t index; std::memcpy(&index, ip, 4); ip += 4;
                    if (vm_stack.empty()) throw std::runtime_error("Stack underflow during STORE_LOCAL.");
                    Value val = vm_stack.back(); vm_stack.pop_back();
                    if (fp + index >= vm_stack.size()) vm_stack.resize(fp + index + 1);
                    vm_stack[fp + index] = val; break;
                }
                case OP_LOAD_LOCAL: {
                    int32_t index; std::memcpy(&index, ip, 4); ip += 4;
                    vm_stack.push_back(vm_stack[fp + index]); break;
                }
                case OP_NEW_ARRAY: {
                    if (vm_stack.empty()) throw std::runtime_error("Stack underflow during NEW_ARRAY.");
                    Value size_val = vm_stack.back(); vm_stack.pop_back();
                    if (size_val.type != ValueType::INT) throw std::runtime_error("Array size must be an integer.");
                    ArrayObject* arr = new ArrayObject(size_val.as.i);
                    vm_stack.push_back(Value::make_obj(gc.register_object(arr))); break;
                }
                case OP_STORE_ARRAY: {
                    if (vm_stack.size() < 3) throw std::runtime_error("Stack underflow during STORE_ARRAY.");
                    Value val = vm_stack.back(); vm_stack.pop_back();
                    Value idx_val = vm_stack.back(); vm_stack.pop_back();
                    Value ref = vm_stack.back(); vm_stack.pop_back();
                    if (ref.type != ValueType::OBJ_REF) throw std::runtime_error("Reference is not an object.");
                    if (idx_val.type != ValueType::INT) throw std::runtime_error("Array index must be an integer.");
                    ArrayObject* arr = dynamic_cast<ArrayObject*>(gc.objects[ref.as.obj_ref]);
                    if (!arr) throw std::runtime_error("Reference is not an array.");
                    if (idx_val.as.i < 0 || idx_val.as.i >= (int32_t)arr->data.size()) throw std::runtime_error("Array index out of bounds.");
                    arr->data[idx_val.as.i] = val; break;
                }
                case OP_LOAD_ARRAY: {
                    if (vm_stack.size() < 2) throw std::runtime_error("Stack underflow during LOAD_ARRAY.");
                    Value idx_val = vm_stack.back(); vm_stack.pop_back();
                    Value ref = vm_stack.back(); vm_stack.pop_back();
                    if (ref.type != ValueType::OBJ_REF) throw std::runtime_error("Reference is not an object.");
                    if (idx_val.type != ValueType::INT) throw std::runtime_error("Array index must be an integer.");
                    ArrayObject* arr = dynamic_cast<ArrayObject*>(gc.objects[ref.as.obj_ref]);
                    if (!arr) throw std::runtime_error("Reference is not an array.");
                    if (idx_val.as.i < 0 || idx_val.as.i >= (int32_t)arr->data.size()) throw std::runtime_error("Array index out of bounds.");
                    vm_stack.push_back(arr->data[idx_val.as.i]); break;
                }
                case OP_ADD: {
                    if (vm_stack.size() < 2) throw std::runtime_error("Stack underflow during ADD.");
                    Value b = vm_stack.back(); vm_stack.pop_back();
                    Value a = vm_stack.back(); vm_stack.pop_back();
                    if (a.type == ValueType::INT && b.type == ValueType::INT) vm_stack.push_back(Value(a.as.i + b.as.i));
                    else if (a.type == ValueType::FLOAT && b.type == ValueType::FLOAT) vm_stack.push_back(Value(a.as.f + b.as.f));
                    else if (a.type == ValueType::STRING_CONST && b.type == ValueType::STRING_CONST) {
                        constants.push_back(constants[a.as.str_idx] + constants[b.as.str_idx]);
                        gc.collect(vm_stack, globals, constants.size());
                        vm_stack.push_back(Value::make_string(constants.size() - 1));
                    } else throw std::runtime_error("Type mismatch: Cannot add incompatible types.");
                    break;
                }
                case OP_SUB: {
                    if (vm_stack.size() < 2) throw std::runtime_error("Stack underflow during SUB.");
                    Value b = vm_stack.back(); vm_stack.pop_back();
                    Value a = vm_stack.back(); vm_stack.pop_back();
                    if (a.type == ValueType::INT && b.type == ValueType::INT) vm_stack.push_back(Value(a.as.i - b.as.i));
                    else if (a.type == ValueType::FLOAT && b.type == ValueType::FLOAT) vm_stack.push_back(Value(a.as.f - b.as.f));
                    else throw std::runtime_error("Type mismatch: Cannot subtract incompatible types.");
                    break;
                }
                case OP_MUL: {
                    if (vm_stack.size() < 2) throw std::runtime_error("Stack underflow during MUL.");
                    Value b = vm_stack.back(); vm_stack.pop_back();
                    Value a = vm_stack.back(); vm_stack.pop_back();
                    if (a.type == ValueType::INT && b.type == ValueType::INT) vm_stack.push_back(Value(a.as.i * b.as.i));
                    else if (a.type == ValueType::FLOAT && b.type == ValueType::FLOAT) vm_stack.push_back(Value(a.as.f * b.as.f));
                    else throw std::runtime_error("Type mismatch: Cannot multiply incompatible types.");
                    break;
                }
                case OP_DIV: {
                    if (vm_stack.size() < 2) throw std::runtime_error("Stack underflow during DIV.");
                    Value b = vm_stack.back(); vm_stack.pop_back();
                    Value a = vm_stack.back(); vm_stack.pop_back();
                    if (a.type == ValueType::INT && b.type == ValueType::INT) {
                        if (b.as.i == 0) throw std::runtime_error("Division by zero.");
                        vm_stack.push_back(Value(a.as.i / b.as.i));
                    } else if (a.type == ValueType::FLOAT && b.type == ValueType::FLOAT) {
                        if (b.as.f == 0.0f) throw std::runtime_error("Division by zero.");
                        vm_stack.push_back(Value(a.as.f / b.as.f));
                    } else throw std::runtime_error("Type mismatch: Cannot divide incompatible types.");
                    break;
                }
                case OP_LESS: {
                    if (vm_stack.size() < 2) throw std::runtime_error("Stack underflow during LESS.");
                    Value b = vm_stack.back(); vm_stack.pop_back();
                    Value a = vm_stack.back(); vm_stack.pop_back();
                    if (a.type == ValueType::INT && b.type == ValueType::INT) vm_stack.push_back(Value(a.as.i < b.as.i));
                    else if (a.type == ValueType::FLOAT && b.type == ValueType::FLOAT) vm_stack.push_back(Value(a.as.f < b.as.f));
                    else if (a.type == ValueType::CHAR && b.type == ValueType::CHAR) vm_stack.push_back(Value(a.as.c < b.as.c));
                    else throw std::runtime_error("Type mismatch: Cannot compare incompatible types with LESS.");
                    break;
                }
                case OP_GREATER: {
                    if (vm_stack.size() < 2) throw std::runtime_error("Stack underflow during GREATER.");
                    Value b = vm_stack.back(); vm_stack.pop_back();
                    Value a = vm_stack.back(); vm_stack.pop_back();
                    if (a.type == ValueType::INT && b.type == ValueType::INT) vm_stack.push_back(Value(a.as.i > b.as.i));
                    else if (a.type == ValueType::FLOAT && b.type == ValueType::FLOAT) vm_stack.push_back(Value(a.as.f > b.as.f));
                    else if (a.type == ValueType::CHAR && b.type == ValueType::CHAR) vm_stack.push_back(Value(a.as.c > b.as.c));
                    else throw std::runtime_error("Type mismatch: Cannot compare incompatible types with GREATER.");
                    break;
                }
                case OP_EQUAL: {
                    if (vm_stack.size() < 2) throw std::runtime_error("Stack underflow during EQUAL.");
                    Value b = vm_stack.back(); vm_stack.pop_back();
                    Value a = vm_stack.back(); vm_stack.pop_back();
                    if (a.type == ValueType::INT && b.type == ValueType::INT) vm_stack.push_back(Value(a.as.i == b.as.i));
                    else if (a.type == ValueType::FLOAT && b.type == ValueType::FLOAT) vm_stack.push_back(Value(a.as.f == b.as.f));
                    else if (a.type == ValueType::BOOL && b.type == ValueType::BOOL) vm_stack.push_back(Value(a.as.b == b.as.b));
                    else if (a.type == ValueType::CHAR && b.type == ValueType::CHAR) vm_stack.push_back(Value(a.as.c == b.as.c));
                    else if (a.type == ValueType::STRING_CONST && b.type == ValueType::STRING_CONST) vm_stack.push_back(Value(constants[a.as.str_idx] == constants[b.as.str_idx]));
                    else throw std::runtime_error("Type mismatch: Cannot compare incompatible types with EQUAL.");
                    break;
                }
                case OP_NOT_EQUAL: {
                    if (vm_stack.size() < 2) throw std::runtime_error("Stack underflow during NOT_EQUAL.");
                    Value b = vm_stack.back(); vm_stack.pop_back();
                    Value a = vm_stack.back(); vm_stack.pop_back();
                    if (a.type == ValueType::INT && b.type == ValueType::INT) vm_stack.push_back(Value(a.as.i != b.as.i));
                    else if (a.type == ValueType::FLOAT && b.type == ValueType::FLOAT) vm_stack.push_back(Value(a.as.f != b.as.f));
                    else if (a.type == ValueType::BOOL && b.type == ValueType::BOOL) vm_stack.push_back(Value(a.as.b != b.as.b));
                    else if (a.type == ValueType::CHAR && b.type == ValueType::CHAR) vm_stack.push_back(Value(a.as.c != b.as.c));
                    else if (a.type == ValueType::STRING_CONST && b.type == ValueType::STRING_CONST) vm_stack.push_back(Value(constants[a.as.str_idx] != constants[b.as.str_idx]));
                    else throw std::runtime_error("Type mismatch: Cannot compare incompatible types with NOT_EQUAL.");
                    break;
                }
                case OP_LESS_EQUAL: {
                    if (vm_stack.size() < 2) throw std::runtime_error("Stack underflow during LESS_EQUAL.");
                    Value b = vm_stack.back(); vm_stack.pop_back();
                    Value a = vm_stack.back(); vm_stack.pop_back();
                    if (a.type == ValueType::INT && b.type == ValueType::INT) vm_stack.push_back(Value(a.as.i <= b.as.i));
                    else if (a.type == ValueType::FLOAT && b.type == ValueType::FLOAT) vm_stack.push_back(Value(a.as.f <= b.as.f));
                    else if (a.type == ValueType::CHAR && b.type == ValueType::CHAR) vm_stack.push_back(Value(a.as.c <= b.as.c));
                    else throw std::runtime_error("Type mismatch: Cannot compare incompatible types with LESS_EQUAL.");
                    break;
                }
                case OP_GREATER_EQUAL: {
                    if (vm_stack.size() < 2) throw std::runtime_error("Stack underflow during GREATER_EQUAL.");
                    Value b = vm_stack.back(); vm_stack.pop_back();
                    Value a = vm_stack.back(); vm_stack.pop_back();
                    if (a.type == ValueType::INT && b.type == ValueType::INT) vm_stack.push_back(Value(a.as.i >= b.as.i));
                    else if (a.type == ValueType::FLOAT && b.type == ValueType::FLOAT) vm_stack.push_back(Value(a.as.f >= b.as.f));
                    else if (a.type == ValueType::CHAR && b.type == ValueType::CHAR) vm_stack.push_back(Value(a.as.c >= b.as.c));
                    else throw std::runtime_error("Type mismatch: Cannot compare incompatible types with GREATER_EQUAL.");
                    break;
                }
                case OP_NEW_INSTANCE: {
                    int32_t class_name_idx; std::memcpy(&class_name_idx, ip, 4); ip += 4;
                    int32_t field_count; std::memcpy(&field_count, ip, 4); ip += 4;
                    InstanceObject* obj = new InstanceObject(class_name_idx, field_count);
                    vm_stack.push_back(Value::make_obj(gc.register_object(obj))); break;
                }
                case OP_GET_FIELD: {
                    int32_t field_idx; std::memcpy(&field_idx, ip, 4); ip += 4;
                    if (vm_stack.empty()) throw std::runtime_error("Stack underflow during GET_FIELD.");
                    Value ref = vm_stack.back(); vm_stack.pop_back();
                    if (ref.type != ValueType::OBJ_REF) throw std::runtime_error("Reference is not an object.");
                    InstanceObject* obj = dynamic_cast<InstanceObject*>(gc.objects[ref.as.obj_ref]);
                    if (!obj) throw std::runtime_error("Reference is not an instance.");
                    if (field_idx < 0 || field_idx >= (int32_t)obj->fields.size()) throw std::runtime_error("Field index out of bounds.");
                    vm_stack.push_back(obj->fields[field_idx]); break;
                }
                case OP_SET_FIELD: {
                    int32_t field_idx; std::memcpy(&field_idx, ip, 4); ip += 4;
                    if (vm_stack.size() < 2) throw std::runtime_error("Stack underflow during SET_FIELD.");
                    Value val = vm_stack.back(); vm_stack.pop_back();
                    Value ref = vm_stack.back(); vm_stack.pop_back();
                    if (ref.type != ValueType::OBJ_REF) throw std::runtime_error("Reference is not an object.");
                    InstanceObject* obj = dynamic_cast<InstanceObject*>(gc.objects[ref.as.obj_ref]);
                    if (!obj) throw std::runtime_error("Reference is not an instance.");
                    if (field_idx < 0 || field_idx >= (int32_t)obj->fields.size()) throw std::runtime_error("Field index out of bounds.");
                    obj->fields[field_idx] = val; break;
                }
                case OP_NEW_LIST: {
                    ListObject* list = new ListObject();
                    vm_stack.push_back(Value::make_obj(gc.register_object(list))); break;
                }
                case OP_LIST_ADD: {
                    if (vm_stack.size() < 2) throw std::runtime_error("Stack underflow during LIST_ADD.");
                    Value val = vm_stack.back(); vm_stack.pop_back();
                    Value ref = vm_stack.back(); vm_stack.pop_back();
                    if (ref.type != ValueType::OBJ_REF) throw std::runtime_error("Reference is not an object.");
                    ListObject* list = dynamic_cast<ListObject*>(gc.objects[ref.as.obj_ref]);
                    if (!list) throw std::runtime_error("Reference is not a list.");
                    list->items.push_back(val); break;
                }
                case OP_LIST_GET: {
                    if (vm_stack.size() < 2) throw std::runtime_error("Stack underflow during LIST_GET.");
                    Value idx_val = vm_stack.back(); vm_stack.pop_back();
                    Value ref = vm_stack.back(); vm_stack.pop_back();
                    if (ref.type != ValueType::OBJ_REF) throw std::runtime_error("Reference is not an object.");
                    if (idx_val.type != ValueType::INT) throw std::runtime_error("List index must be an integer.");
                    ListObject* list = dynamic_cast<ListObject*>(gc.objects[ref.as.obj_ref]);
                    if (!list) throw std::runtime_error("Reference is not a list.");
                    if (idx_val.as.i < 0 || idx_val.as.i >= (int32_t)list->items.size()) throw std::runtime_error("List index out of bounds.");
                    vm_stack.push_back(list->items[idx_val.as.i]); break;
                }
                case OP_LIST_SET: {
                    if (vm_stack.size() < 3) throw std::runtime_error("Stack underflow during LIST_SET.");
                    Value val = vm_stack.back(); vm_stack.pop_back();
                    Value idx_val = vm_stack.back(); vm_stack.pop_back();
                    Value ref = vm_stack.back(); vm_stack.pop_back();
                    if (ref.type != ValueType::OBJ_REF) throw std::runtime_error("Reference is not an object.");
                    if (idx_val.type != ValueType::INT) throw std::runtime_error("List index must be an integer.");
                    ListObject* list = dynamic_cast<ListObject*>(gc.objects[ref.as.obj_ref]);
                    if (!list) throw std::runtime_error("Reference is not a list.");
                    if (idx_val.as.i < 0 || idx_val.as.i >= (int32_t)list->items.size()) throw std::runtime_error("List index out of bounds.");
                    list->items[idx_val.as.i] = val; break;
                }
                case OP_LIST_SIZE: {
                    if (vm_stack.empty()) throw std::runtime_error("Stack underflow during LIST_SIZE.");
                    Value ref = vm_stack.back(); vm_stack.pop_back();
                    if (ref.type != ValueType::OBJ_REF) throw std::runtime_error("Reference is not an object.");
                    ListObject* list = dynamic_cast<ListObject*>(gc.objects[ref.as.obj_ref]);
                    if (!list) throw std::runtime_error("Reference is not a list.");
                    vm_stack.push_back(Value((int32_t)list->items.size())); break;
                }
                case OP_CALL: {
                    int32_t target_offset; std::memcpy(&target_offset, ip, 4); ip += 4;
                    uint8_t arg_count = *ip++;
                    if (vm_stack.size() < arg_count) throw std::runtime_error("Stack underflow during CALL.");
                    fp_stack.push_back(fp);
                    fp = vm_stack.size() - arg_count;
                    call_stack.push_back(ip);
                    ip = vm.bytecode->data() + target_offset; break;
                }
                case OP_RETURN: {
                    if (call_stack.empty()) throw std::runtime_error("RETURN with empty call stack.");
                    Value result = vm_stack.back(); vm_stack.pop_back();
                    vm_stack.resize(fp);
                    vm_stack.push_back(result);
                    ip = call_stack.back(); call_stack.pop_back();
                    fp = fp_stack.back(); fp_stack.pop_back(); break;
                }
                case OP_CALL_NATIVE: {
                    uint16_t native_id;
                    std::memcpy(&native_id, ip, 2); ip += 2;
                    auto& natives = Natives::registry();
                    if (native_id >= natives.size()) throw std::runtime_error("Unknown native function ID.");
                    NativeEntry& entry = natives[native_id];
                    if (vm_stack.size() < (size_t)entry.arity) throw std::runtime_error("Stack underflow during CALL_NATIVE.");
                    native_args.resize(entry.arity);
                    for (int i = entry.arity - 1; i >= 0; --i) {
                        native_args[i] = vm_stack.back(); vm_stack.pop_back();
                    }
                    Value result = entry.fn(native_args, constants, gc);
                    vm_stack.push_back(result); break;
                }
                case OP_POP: vm_stack.pop_back(); break;
                case OP_PRINT: {
                    if (vm_stack.empty()) throw std::runtime_error("Stack underflow during PRINT.");
                    Value val = vm_stack.back(); vm_stack.pop_back();
                    if (val.type == ValueType::STRING_CONST) {
                        size_t idx = val.as.str_idx;
                        if (idx < constants.size()) std::cout << constants[idx] << std::endl;
                        else std::cout << "<Invalid String Index>" << std::endl;
                    } else {
                        std::cout << val << std::endl;
                    }
                    break;
                }
                default:
                    throw std::runtime_error("Unknown opcode encountered.");
            }
        }
#endif
    } catch (const std::runtime_error& e) {
        std::cerr << "AVM Runtime Error: " << e.what() << std::endl;
        return 1;
    }
    return 0;
}
