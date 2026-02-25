#include "avm.hpp"
#include "bytecode.hpp"
#include "heap.hpp"
#include "value.hpp"
#include <iostream>
#include <vector>
#include <stack>
#include <stdexcept>
#include <cstdint>
#include <cstring> // For std::memcpy

void execute(const std::vector<uint8_t>& bytecode, std::vector<std::string>& constants) {
    if (bytecode.empty()) {
        std::cout << "AVM Warning: No bytecode to execute." << std::endl;
        return; // Nothing to execute
    }

    // Use vector for cache locality (contiguous memory) instead of std::stack/deque
    std::vector<Value> vm_stack;
    vm_stack.reserve(1024); // Pre-allocate reasonable stack space

    // Garbage Collector
    Heap gc;

    // Global variables storage
    std::vector<Value> globals;

    // Call Stack (Stores return addresses)
    std::vector<const uint8_t*> call_stack;
    
    // Frame Pointer Stack (Stores previous frame pointers)
    std::vector<size_t> fp_stack;
    size_t fp = 0; // Current Frame Pointer

    const uint8_t* ip = bytecode.data();
    const uint8_t* end = ip + bytecode.size();

    try {
        while (ip < end) {
            uint8_t instruction = *ip++;
            switch (instruction) {
                // --- Control Flow ---
                case OP_HALT: {
                    return; // End execution
                }
                case OP_JUMP: {
                    int32_t offset;
                    std::memcpy(&offset, ip, sizeof(int32_t));
                    ip += 4;      // Consume the 4-byte offset from the instruction stream
                    ip += offset; // Apply the relative jump
                    break;
                }
                case OP_JUMP_IF_FALSE: {
                    int32_t offset;
                    std::memcpy(&offset, ip, sizeof(int32_t));
                    ip += 4; // Advance past the offset bytes

                    if (vm_stack.empty()) throw std::runtime_error("Stack underflow during JUMP_IF_FALSE.");
                    Value condition = vm_stack.back(); vm_stack.pop_back();
                    
                    bool is_false = false;
                    if (condition.type == ValueType::BOOL) {
                        is_false = !condition.as.b;
                    } else if (condition.type == ValueType::INT) {
                        is_false = (condition.as.i == 0);
                    } else {
                        throw std::runtime_error("JUMP_IF_FALSE requires a boolean or integer condition.");
                    }

                    if (is_false) {
                        ip += offset;
                    }
                    break;
                }

                // --- Constants & Variables ---
                case OP_PUSH: {
                    int32_t value;
                    std::memcpy(&value, ip, sizeof(int32_t));
                    vm_stack.push_back(Value(value));
                    ip += sizeof(int32_t);
                    break;
                }
                case OP_PUSH_FLOAT: {
                    float value;
                    std::memcpy(&value, ip, sizeof(float));
                    vm_stack.push_back(Value(value));
                    ip += sizeof(float);
                    break;
                }
                case OP_PUSH_BOOL: {
                    uint8_t value = *ip++;
                    vm_stack.push_back(Value(value != 0));
                    break;
                }
                case OP_PUSH_CHAR: {
                    int32_t value; // UTF-32 char
                    std::memcpy(&value, ip, sizeof(int32_t));
                    vm_stack.push_back(Value((char)value)); // Cast to char for now
                    ip += sizeof(int32_t);
                    break;
                }
                case OP_LOAD_CONST: {
                    int32_t index;
                    std::memcpy(&index, ip, sizeof(int32_t));
                    ip += sizeof(int32_t);
                    vm_stack.push_back(Value::make_string(index));
                    break;
                }
                case OP_STORE_GLOBAL: {
                    int32_t index;
                    std::memcpy(&index, ip, sizeof(int32_t));
                    ip += sizeof(int32_t);

                    if (vm_stack.empty()) throw std::runtime_error("Stack underflow during STORE.");
                    Value val = vm_stack.back();
                    vm_stack.pop_back();

                    if (index >= globals.size()) globals.resize(index + 1);
                    globals[index] = val;
                    break;
                }
                case OP_LOAD_GLOBAL: {
                    int32_t index;
                    std::memcpy(&index, ip, sizeof(int32_t));
                    ip += sizeof(int32_t);

                    if (index < 0 || index >= globals.size()) throw std::runtime_error("Global variable index out of bounds.");
                    vm_stack.push_back(globals[index]);
                    break;
                }
                case OP_NEW_ARRAY: {
                    if (vm_stack.empty()) throw std::runtime_error("Stack underflow during NEW_ARRAY.");
                    Value size_val = vm_stack.back(); vm_stack.pop_back();
                    
                    if (size_val.type != ValueType::INT) throw std::runtime_error("Array size must be an integer.");
                    int32_t size = size_val.as.i;

                    ArrayObject* arr = new ArrayObject(size);
                    int32_t heap_idx = gc.register_object(arr);
                    
                    vm_stack.push_back(Value::make_obj(heap_idx));
                    break;
                }
                case OP_STORE_ARRAY: {
                    if (vm_stack.size() < 3) throw std::runtime_error("Stack underflow during STORE_ARRAY.");
                    Value val = vm_stack.back(); vm_stack.pop_back();
                    Value idx_val = vm_stack.back(); vm_stack.pop_back();
                    Value ref = vm_stack.back(); vm_stack.pop_back();

                    if (ref.type != ValueType::OBJ_REF) throw std::runtime_error("Reference is not an object.");
                    if (idx_val.type != ValueType::INT) throw std::runtime_error("Array index must be an integer.");

                    int32_t heap_idx = ref.as.obj_ref;
                    int32_t idx = idx_val.as.i;
                    
                    if (heap_idx < 0 || heap_idx >= gc.objects.size()) throw std::runtime_error("Invalid array reference.");
                    ArrayObject* arr = dynamic_cast<ArrayObject*>(gc.objects[heap_idx]);
                    if (!arr) throw std::runtime_error("Reference is not an array.");
                    if (idx < 0 || idx >= arr->data.size()) throw std::runtime_error("Array index out of bounds.");
                    
                    arr->data[idx] = val;
                    break;
                }
                case OP_LOAD_ARRAY: {
                    if (vm_stack.size() < 2) throw std::runtime_error("Stack underflow during LOAD_ARRAY.");
                    Value idx_val = vm_stack.back(); vm_stack.pop_back();
                    Value ref = vm_stack.back(); vm_stack.pop_back();

                    if (ref.type != ValueType::OBJ_REF) throw std::runtime_error("Reference is not an object.");
                    if (idx_val.type != ValueType::INT) throw std::runtime_error("Array index must be an integer.");

                    int32_t heap_idx = ref.as.obj_ref;
                    int32_t idx = idx_val.as.i;
                    
                    if (heap_idx < 0 || heap_idx >= gc.objects.size()) throw std::runtime_error("Invalid array reference.");
                    ArrayObject* arr = dynamic_cast<ArrayObject*>(gc.objects[heap_idx]);
                    if (!arr) throw std::runtime_error("Reference is not an array.");
                    if (idx < 0 || idx >= arr->data.size()) throw std::runtime_error("Array index out of bounds.");
                    
                    vm_stack.push_back(arr->data[idx]);
                    break;
                }
                case OP_STORE_LOCAL: {
                    int32_t index;
                    std::memcpy(&index, ip, sizeof(int32_t));
                    ip += sizeof(int32_t);

                    if (vm_stack.empty()) throw std::runtime_error("Stack underflow during STORE_LOCAL.");
                    Value val = vm_stack.back();
                    vm_stack.pop_back();
                    
                    if (fp + index >= vm_stack.size()) vm_stack.resize(fp + index + 1);
                    vm_stack[fp + index] = val;
                    break;
                }
                case OP_LOAD_LOCAL: {
                    int32_t index;
                    std::memcpy(&index, ip, sizeof(int32_t));
                    ip += sizeof(int32_t);
                    vm_stack.push_back(vm_stack[fp + index]);
                    break;
                }
                
                // --- Object-Oriented ---
                case OP_NEW_INSTANCE: {
                    int32_t class_name_idx;
                    std::memcpy(&class_name_idx, ip, sizeof(int32_t)); ip += 4;
                    int32_t field_count;
                    std::memcpy(&field_count, ip, sizeof(int32_t)); ip += 4;

                    InstanceObject* obj = new InstanceObject(class_name_idx, field_count);
                    int32_t heap_idx = gc.register_object(obj);
                    
                    vm_stack.push_back(Value::make_obj(heap_idx));
                    break;
                }
                case OP_GET_FIELD: {
                    int32_t field_idx;
                    std::memcpy(&field_idx, ip, sizeof(int32_t)); ip += 4;

                    if (vm_stack.empty()) throw std::runtime_error("Stack underflow during GET_FIELD.");
                    Value ref = vm_stack.back(); vm_stack.pop_back();

                    if (ref.type != ValueType::OBJ_REF) throw std::runtime_error("Reference is not an object.");
                    int32_t heap_idx = ref.as.obj_ref;

                    if (heap_idx < 0 || heap_idx >= gc.objects.size()) throw std::runtime_error("Invalid instance reference.");
                    
                    InstanceObject* obj = dynamic_cast<InstanceObject*>(gc.objects[heap_idx]);
                    if (!obj) throw std::runtime_error("Reference is not an instance.");
                    if (field_idx < 0 || field_idx >= obj->fields.size()) throw std::runtime_error("Field index out of bounds.");

                    vm_stack.push_back(obj->fields[field_idx]);
                    break;
                }
                case OP_SET_FIELD: {
                    int32_t field_idx;
                    std::memcpy(&field_idx, ip, sizeof(int32_t)); ip += 4;

                    if (vm_stack.size() < 2) throw std::runtime_error("Stack underflow during SET_FIELD.");
                    Value val = vm_stack.back(); vm_stack.pop_back();
                    Value ref = vm_stack.back(); vm_stack.pop_back();

                    if (ref.type != ValueType::OBJ_REF) throw std::runtime_error("Reference is not an object.");
                    int32_t heap_idx = ref.as.obj_ref;

                    if (heap_idx < 0 || heap_idx >= gc.objects.size()) throw std::runtime_error("Invalid instance reference.");
                    
                    InstanceObject* obj = dynamic_cast<InstanceObject*>(gc.objects[heap_idx]);
                    if (!obj) throw std::runtime_error("Reference is not an instance.");
                    if (field_idx < 0 || field_idx >= obj->fields.size()) throw std::runtime_error("Field index out of bounds.");

                    obj->fields[field_idx] = val;
                    break;
                }

                // --- Arithmetic & Logic ---
                case OP_ADD: {
                    if (vm_stack.size() < 2) throw std::runtime_error("Stack underflow during ADD.");
                    Value b = vm_stack.back(); vm_stack.pop_back();
                    Value a = vm_stack.back(); vm_stack.pop_back();

                    if (a.type == ValueType::INT && b.type == ValueType::INT) {
                        vm_stack.push_back(Value(a.as.i + b.as.i));
                    } else if (a.type == ValueType::FLOAT && b.type == ValueType::FLOAT) {
                        vm_stack.push_back(Value(a.as.f + b.as.f));
                    } else if (a.type == ValueType::STRING_CONST && b.type == ValueType::STRING_CONST) {
                        const std::string& str_a = constants[a.as.str_idx];
                        const std::string& str_b = constants[b.as.str_idx];
                        
                        std::string result_str = str_a + str_b;
                        constants.push_back(result_str);

                        // Trigger GC (Simulate allocation pressure)
                        gc.collect(vm_stack, globals, constants.size());
                        
                        vm_stack.push_back(Value::make_string(constants.size() - 1));
                    } else {
                        throw std::runtime_error("Type mismatch: Cannot add incompatible types.");
                    }
                    break;
                }
                case OP_SUB: {
                    if (vm_stack.size() < 2) throw std::runtime_error("Stack underflow during SUB.");
                    Value b = vm_stack.back(); vm_stack.pop_back();
                    Value a = vm_stack.back(); vm_stack.pop_back();

                    if (a.type == ValueType::INT && b.type == ValueType::INT) {
                        vm_stack.push_back(Value(a.as.i - b.as.i));
                    } else if (a.type == ValueType::FLOAT && b.type == ValueType::FLOAT) {
                        vm_stack.push_back(Value(a.as.f - b.as.f));
                    } else {
                        throw std::runtime_error("Type mismatch: Cannot subtract incompatible types.");
                    }
                    break;
                }
                case OP_MUL: {
                    if (vm_stack.size() < 2) throw std::runtime_error("Stack underflow during MUL.");
                    Value b = vm_stack.back(); vm_stack.pop_back();
                    Value a = vm_stack.back(); vm_stack.pop_back();

                    if (a.type == ValueType::INT && b.type == ValueType::INT) {
                        vm_stack.push_back(Value(a.as.i * b.as.i));
                    } else if (a.type == ValueType::FLOAT && b.type == ValueType::FLOAT) {
                        vm_stack.push_back(Value(a.as.f * b.as.f));
                    } else {
                        throw std::runtime_error("Type mismatch: Cannot multiply incompatible types.");
                    }
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
                    } else {
                        throw std::runtime_error("Type mismatch: Cannot divide incompatible types.");
                    }
                    break;
                }
                case OP_LESS: {
                    if (vm_stack.size() < 2) throw std::runtime_error("Stack underflow during LESS.");
                    Value b = vm_stack.back(); vm_stack.pop_back();
                    Value a = vm_stack.back(); vm_stack.pop_back();

                    if (a.type == ValueType::INT && b.type == ValueType::INT) {
                        vm_stack.push_back(Value(a.as.i < b.as.i));
                    } else if (a.type == ValueType::FLOAT && b.type == ValueType::FLOAT) {
                        vm_stack.push_back(Value(a.as.f < b.as.f));
                    } else if (a.type == ValueType::CHAR && b.type == ValueType::CHAR) {
                        vm_stack.push_back(Value(a.as.c < b.as.c));
                    } else {
                        throw std::runtime_error("Type mismatch: Cannot compare incompatible types with LESS.");
                    }
                    break;
                }
                case OP_GREATER: {
                    if (vm_stack.size() < 2) throw std::runtime_error("Stack underflow during GREATER.");
                    Value b = vm_stack.back(); vm_stack.pop_back();
                    Value a = vm_stack.back(); vm_stack.pop_back();

                    if (a.type == ValueType::INT && b.type == ValueType::INT) {
                        vm_stack.push_back(Value(a.as.i > b.as.i));
                    } else if (a.type == ValueType::FLOAT && b.type == ValueType::FLOAT) {
                        vm_stack.push_back(Value(a.as.f > b.as.f));
                    } else if (a.type == ValueType::CHAR && b.type == ValueType::CHAR) {
                        vm_stack.push_back(Value(a.as.c > b.as.c));
                    } else {
                        throw std::runtime_error("Type mismatch: Cannot compare incompatible types with GREATER.");
                    }
                    break;
                }

                // --- Functions & Calls ---
                case OP_CALL: {
                    int32_t target_offset;
                    std::memcpy(&target_offset, ip, sizeof(int32_t));
                    ip += 4;

                    uint8_t arg_count = *ip++;
                    
                    if (vm_stack.size() < arg_count) throw std::runtime_error("Stack underflow during CALL.");
                    
                    fp_stack.push_back(fp);
                    // FP points to the first argument
                    fp = vm_stack.size() - arg_count; 

                    call_stack.push_back(ip); // Save return address
                    ip = bytecode.data() + target_offset; // Jump to function
                    break;
                }
                case OP_RETURN: {
                    if (call_stack.empty()) return; // Or halt
                    
                    Value result = vm_stack.back(); vm_stack.pop_back();
                    
                    // Restore stack (remove args/locals)
                    vm_stack.resize(fp); 
                    vm_stack.push_back(result); // Push result back

                    ip = call_stack.back();
                    call_stack.pop_back();
                    fp = fp_stack.back();
                    fp_stack.pop_back();
                    break;
                }

                // --- Utilities ---
                case OP_POP: vm_stack.pop_back(); break;
                case OP_PRINT: {
                    if (vm_stack.empty()) throw std::runtime_error("Stack underflow during PRINT.");
                    Value val = vm_stack.back();
                    vm_stack.pop_back();
                    
                    if (val.type == ValueType::STRING_CONST) {
                        size_t idx = val.as.str_idx;
                        if (idx < constants.size()) std::cout << "Amber Out: " << constants[idx] << std::endl;
                        else std::cout << "Amber Out: <Invalid String Index>" << std::endl;
                    } else {
                        std::cout << "Amber Out: " << val << std::endl;
                    }
                    break;
                }

                default: {
                    throw std::runtime_error("Unknown opcode encountered.");
                }
            }
        }
    } catch (const std::runtime_error& e) {
        std::cerr << "AVM Runtime Error: " << e.what() << std::endl;
    }
}