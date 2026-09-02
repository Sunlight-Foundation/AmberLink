// amber-vm/src/natives.cpp
#include "natives.hpp"
#include <iostream>
#include <cmath>
#include <cstdlib>
#include <cctype>
#include <chrono>
#include <thread>
#include <fstream>
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

// --- printStr(str) : void ---
// Prints a string without the DEBUG-printer decoration the VM's print opcode uses.
static Value native_printStr(std::vector<Value>& args, std::vector<std::string>& constants, Heap& heap) {
    (void)heap;
    if (args.size() != 1) throw std::runtime_error("printStr expects 1 argument.");
    const Value& v = args[0];
    if (v.type == ValueType::STRING_CONST) {
        std::cout << constants[v.as.str_idx];
    } else {
        throw std::runtime_error("printStr expects a String.");
    }
    return Value();
}

// --- readFile(path) : String ---
// Reads the entire contents of the file as a string, or "" if it cannot be read.
static Value native_readFile(std::vector<Value>& args, std::vector<std::string>& constants, Heap& heap) {
    (void)heap;
    if (args.size() != 1) throw std::runtime_error("readFile expects 1 argument.");
    if (args[0].type != ValueType::STRING_CONST) throw std::runtime_error("readFile expects a String path.");
    std::ifstream in(constants[args[0].as.str_idx], std::ios::binary);
    if (!in) return make_string_const(constants, "");
    std::ostringstream ss;
    ss << in.rdbuf();
    return make_string_const(constants, ss.str());
}

// --- writeFile(path, content) : bool ---
// Writes content to the file. Returns true on success.
static Value native_writeFile(std::vector<Value>& args, std::vector<std::string>& constants, Heap& heap) {
    (void)heap;
    if (args.size() != 2) throw std::runtime_error("writeFile expects 2 arguments.");
    if (args[0].type != ValueType::STRING_CONST) throw std::runtime_error("writeFile expects a String path.");
    if (args[1].type != ValueType::STRING_CONST) throw std::runtime_error("writeFile expects a String content.");
    std::ofstream out(constants[args[0].as.str_idx], std::ios::binary);
    if (!out) return Value(false);
    out << constants[args[1].as.str_idx];
    out.close();
    return Value(true);
}

// --- exit(status) : never ---
// Terminates the program with the given exit status.
static Value native_exit(std::vector<Value>& args, std::vector<std::string>& constants, Heap& heap) {
    (void)constants; (void)heap;
    if (args.size() != 1) throw std::runtime_error("exit expects 1 argument.");
    int32_t status = 0;
    if (args[0].type == ValueType::INT) status = args[0].as.i;
    else throw std::runtime_error("exit expects an int status.");
    std::exit(status);
    return Value(); // unreachable
}

// --- sleep(milliseconds) : void ---
static Value native_sleep(std::vector<Value>& args, std::vector<std::string>& constants, Heap& heap) {
    (void)heap;
    if (args.size() != 1) throw std::runtime_error("sleep expects 1 argument.");
    if (args[0].type != ValueType::INT) throw std::runtime_error("sleep expects an int (milliseconds).");
    std::this_thread::sleep_for(std::chrono::milliseconds(args[0].as.i));
    return Value();
}

// --- clock() : float ---
// Returns seconds elapsed since some fixed point (monotonic).
static Value native_clock(std::vector<Value>& args, std::vector<std::string>& constants, Heap& heap) {
    (void)args; (void)constants; (void)heap;
    static const auto start = std::chrono::steady_clock::now();
    auto now = std::chrono::steady_clock::now();
    double secs = std::chrono::duration<double>(now - start).count();
    return Value(static_cast<float>(secs));
}

// --- strLen(str) : int ---
static Value native_strLen(std::vector<Value>& args, std::vector<std::string>& constants, Heap& heap) {
    (void)heap;
    if (args.size() != 1) throw std::runtime_error("strLen expects 1 argument.");
    if (args[0].type != ValueType::STRING_CONST) throw std::runtime_error("strLen expects a String.");
    return Value(static_cast<int32_t>(constants[args[0].as.str_idx].size()));
}

// --- strCharAt(str, index) : char ---
static Value native_strCharAt(std::vector<Value>& args, std::vector<std::string>& constants, Heap& heap) {
    (void)heap;
    if (args.size() != 2) throw std::runtime_error("strCharAt expects 2 arguments.");
    if (args[0].type != ValueType::STRING_CONST) throw std::runtime_error("strCharAt expects a String.");
    if (args[1].type != ValueType::INT) throw std::runtime_error("strCharAt expects an int index.");
    const std::string& s = constants[args[0].as.str_idx];
    int32_t idx = args[1].as.i;
    if (idx < 0 || idx >= static_cast<int32_t>(s.size())) throw std::runtime_error("strCharAt: index out of range.");
    return Value(s[idx]);
}

// --- strSubstring(str, start, length) : String ---
static Value native_strSubstring(std::vector<Value>& args, std::vector<std::string>& constants, Heap& heap) {
    (void)heap;
    if (args.size() != 3) throw std::runtime_error("strSubstring expects 3 arguments.");
    if (args[0].type != ValueType::STRING_CONST) throw std::runtime_error("strSubstring expects a String.");
    if (args[1].type != ValueType::INT || args[2].type != ValueType::INT) throw std::runtime_error("strSubstring expects int arguments.");
    const std::string& s = constants[args[0].as.str_idx];
    int32_t start = args[1].as.i;
    int32_t len = args[2].as.i;
    if (start < 0 || start > static_cast<int32_t>(s.size())) throw std::runtime_error("strSubstring: start out of range.");
    if (len < 0) throw std::runtime_error("strSubstring: length must be >= 0.");
    int32_t end = start + len;
    if (end > static_cast<int32_t>(s.size())) end = static_cast<int32_t>(s.size());
    return make_string_const(constants, s.substr(start, end - start));
}

// --- strIndexOf(str, substr) : int ---
// Returns the index of the first occurrence of substr, or -1 if not found.
static Value native_strIndexOf(std::vector<Value>& args, std::vector<std::string>& constants, Heap& heap) {
    (void)heap;
    if (args.size() != 2) throw std::runtime_error("strIndexOf expects 2 arguments.");
    if (args[0].type != ValueType::STRING_CONST || args[1].type != ValueType::STRING_CONST)
        throw std::runtime_error("strIndexOf expects two Strings.");
    const std::string& hay = constants[args[0].as.str_idx];
    const std::string& needle = constants[args[1].as.str_idx];
    size_t pos = hay.find(needle);
    if (pos == std::string::npos) return Value(-1);
    return Value(static_cast<int32_t>(pos));
}

// --- strEquals(str, str) : bool ---
static Value native_strEquals(std::vector<Value>& args, std::vector<std::string>& constants, Heap& heap) {
    (void)heap;
    if (args.size() != 2) throw std::runtime_error("strEquals expects 2 arguments.");
    if (args[0].type != ValueType::STRING_CONST || args[1].type != ValueType::STRING_CONST)
        throw std::runtime_error("strEquals expects two Strings.");
    return Value(constants[args[0].as.str_idx] == constants[args[1].as.str_idx]);
}

// --- strToUpper(str) : String ---
static Value native_strToUpper(std::vector<Value>& args, std::vector<std::string>& constants, Heap& heap) {
    (void)heap;
    if (args.size() != 1) throw std::runtime_error("strToUpper expects 1 argument.");
    if (args[0].type != ValueType::STRING_CONST) throw std::runtime_error("strToUpper expects a String.");
    std::string out = constants[args[0].as.str_idx];
    for (char& c : out) c = static_cast<char>(std::toupper(static_cast<unsigned char>(c)));
    return make_string_const(constants, out);
}

// --- strToLower(str) : String ---
static Value native_strToLower(std::vector<Value>& args, std::vector<std::string>& constants, Heap& heap) {
    (void)heap;
    if (args.size() != 1) throw std::runtime_error("strToLower expects 1 argument.");
    if (args[0].type != ValueType::STRING_CONST) throw std::runtime_error("strToLower expects a String.");
    std::string out = constants[args[0].as.str_idx];
    for (char& c : out) c = static_cast<char>(std::tolower(static_cast<unsigned char>(c)));
    return make_string_const(constants, out);
}

// --- mathSqrt(value) : float ---
static Value native_mathSqrt(std::vector<Value>& args, std::vector<std::string>& constants, Heap& heap) {
    (void)constants; (void)heap;
    if (args.size() != 1) throw std::runtime_error("mathSqrt expects 1 argument.");
    const Value& v = args[0];
    if (v.type == ValueType::INT) return Value(static_cast<float>(std::sqrt(static_cast<double>(v.as.i))));
    if (v.type == ValueType::FLOAT) return Value(static_cast<float>(std::sqrt(static_cast<double>(v.as.f))));
    throw std::runtime_error("mathSqrt expects a number.");
}

// --- mathPow(base, exp) : float ---
static Value native_mathPow(std::vector<Value>& args, std::vector<std::string>& constants, Heap& heap) {
    (void)constants; (void)heap;
    if (args.size() != 2) throw std::runtime_error("mathPow expects 2 arguments.");
    double b, e;
    if (args[0].type == ValueType::INT) b = args[0].as.i;
    else if (args[0].type == ValueType::FLOAT) b = args[0].as.f;
    else throw std::runtime_error("mathPow expects numbers.");
    if (args[1].type == ValueType::INT) e = args[1].as.i;
    else if (args[1].type == ValueType::FLOAT) e = args[1].as.f;
    else throw std::runtime_error("mathPow expects numbers.");
    return Value(static_cast<float>(std::pow(b, e)));
}

std::vector<NativeEntry>& registry() {
    static std::vector<NativeEntry> reg = {
        {native_len, 1},
        {native_input, 0},
        {native_toString, 1},
        {native_toInt, 1},
        {native_toFloat, 1},
        {native_abs, 1},
        {native_printStr, 1},
        {native_readFile, 1},
        {native_writeFile, 2},
        {native_exit, 1},
        {native_sleep, 1},
        {native_clock, 0},
        {native_strLen, 1},
        {native_strCharAt, 2},
        {native_strSubstring, 3},
        {native_strIndexOf, 2},
        {native_strEquals, 2},
        {native_strToUpper, 1},
        {native_strToLower, 1},
        {native_mathSqrt, 1},
        {native_mathPow, 2},
    };
    return reg;
}

} // namespace Natives
