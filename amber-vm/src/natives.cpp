// amber-vm/src/natives.cpp
#include "natives.hpp"
#include <iostream>
#include <cmath>
#include <cstdlib>
#include <sstream>

namespace Natives {

static Value make_string_const(std::vector<std::string>& constants, const std::string& s) {
    // Reuse an existing constant if present, otherwise append.
    for (size_t i = 0; i < constants.size(); ++i) {
        if (constants[i] == s) return Value::make_string(static_cast<int32_t>(i));
    }
    constants.push_back(s);
    return Value::make_string(static_cast<int32_t>(constants.size() - 1));
}

// Pop args from the stack in reverse (args vector is in call order).
// Each native validates arg count and types, throwing on mismatch.

// --- len(collection) : int ---
// Works on String (char count) and List/Array (item count).
static Value native_len(std::vector<Value>& args, std::vector<std::string>& constants, Heap& heap) {
    (void)constants;
    if (args.size() != 1) throw std::runtime_error("len expects 1 argument.");
    const Value& v = args[0];
    switch (v.type) {
        case ValueType::STRING_CONST: return Value(static_cast<int32_t>(constants[v.as.str_idx].size()));
        case ValueType::OBJ_REF: {
            AmberObject* obj = heap.objects[v.as.obj_ref];
            if (obj->type == ObjType::LIST) {
                ListObject* list = static_cast<ListObject*>(obj);
                return Value(static_cast<int32_t>(list->items.size()));
            } else if (obj->type == ObjType::ARRAY) {
                ArrayObject* arr = static_cast<ArrayObject*>(obj);
                return Value(static_cast<int32_t>(arr->data.size()));
            }
            throw std::runtime_error("len expects a String, List, or Array.");
        }
        default:
            throw std::runtime_error("len expects a String, List, or Array.");
    }
}

// --- input() : String ---
// Reads one line from stdin (trimmed of trailing newline).
static Value native_input(std::vector<Value>& args, std::vector<std::string>& constants, Heap& heap) {
    (void)heap;
    if (!args.empty()) throw std::runtime_error("input expects 0 arguments.");
    std::string line;
    std::getline(std::cin, line);
    return make_string_const(constants, line);
}

// --- toString(value) : String ---
static Value native_toString(std::vector<Value>& args, std::vector<std::string>& constants, Heap& heap) {
    (void)heap;
    if (args.size() != 1) throw std::runtime_error("toString expects 1 argument.");
    std::ostringstream oss;
    const Value& v = args[0];
    switch (v.type) {
        case ValueType::INT: oss << v.as.i; break;
        case ValueType::FLOAT: oss << v.as.f; break;
        case ValueType::BOOL: oss << (v.as.b ? "true" : "false"); break;
        case ValueType::CHAR: oss << v.as.c; break;
        case ValueType::STRING_CONST: oss << constants[v.as.str_idx]; break;
        default: throw std::runtime_error("toString: unsupported value type.");
    }
    return make_string_const(constants, oss.str());
}

// --- toInt(value) : int ---
// Converts a String, Int, Float, or Char to an int.
static Value native_toInt(std::vector<Value>& args, std::vector<std::string>& constants, Heap& heap) {
    (void)heap;
    if (args.size() != 1) throw std::runtime_error("toInt expects 1 argument.");
    const Value& v = args[0];
    switch (v.type) {
        case ValueType::INT: return v;
        case ValueType::FLOAT: return Value(static_cast<int32_t>(v.as.f));
        case ValueType::CHAR: return Value(static_cast<int32_t>(v.as.c));
        case ValueType::STRING_CONST: return Value(std::stoi(constants[v.as.str_idx]));
        default: throw std::runtime_error("toInt: cannot convert value to int.");
    }
}

// --- toFloat(value) : float ---
static Value native_toFloat(std::vector<Value>& args, std::vector<std::string>& constants, Heap& heap) {
    (void)heap;
    if (args.size() != 1) throw std::runtime_error("toFloat expects 1 argument.");
    const Value& v = args[0];
    switch (v.type) {
        case ValueType::FLOAT: return v;
        case ValueType::INT: return Value(static_cast<float>(v.as.i));
        case ValueType::STRING_CONST: return Value(std::stof(constants[v.as.str_idx]));
        default: throw std::runtime_error("toFloat: cannot convert value to float.");
    }
}

// --- abs(value) : same type ---
static Value native_abs(std::vector<Value>& args, std::vector<std::string>& constants, Heap& heap) {
    (void)constants; (void)heap;
    if (args.size() != 1) throw std::runtime_error("abs expects 1 argument.");
    const Value& v = args[0];
    if (v.type == ValueType::INT) return Value(std::abs(v.as.i));
    if (v.type == ValueType::FLOAT) return Value(std::fabs(v.as.f));
    throw std::runtime_error("abs expects an int or float.");
}

std::vector<NativeEntry>& registry() {
    static std::vector<NativeEntry> reg = {
        {native_len, 1},
        {native_input, 0},
        {native_toString, 1},
        {native_toInt, 1},
        {native_toFloat, 1},
        {native_abs, 1},
    };
    return reg;
}

} // namespace Natives
