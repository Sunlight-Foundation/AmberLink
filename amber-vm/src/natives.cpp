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
#include <unordered_map>
#include <cstring>
#include "resources.hpp"

#ifdef _WIN32
#include <winsock2.h>
#include <ws2tcpip.h>
#else
#include <sys/types.h>
#include <sys/socket.h>
#include <netdb.h>
#include <unistd.h>
#include <arpa/inet.h>
#endif

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

// --- Collections: HashMap ---

// Structural equality of two Values (by value, not by identity).
static bool value_equal(const Value& a, const Value& b) {
    if (a.type != b.type) return false;
    switch (a.type) {
        case ValueType::INT: return a.as.i == b.as.i;
        case ValueType::FLOAT: return a.as.f == b.as.f;
        case ValueType::BOOL: return a.as.b == b.as.b;
        case ValueType::CHAR: return a.as.c == b.as.c;
        case ValueType::STRING_CONST: return a.as.str_idx == b.as.str_idx;
        case ValueType::OBJ_REF: return a.as.obj_ref == b.as.obj_ref;
    }
    return false;
}

// Hash of a Value, consistent with value_equal: equal values hash equal.
// (STRING_CONST keys compare by pool index, so the index — not the content —
// is hashed. Float -0.0 is normalized to +0.0 to match == semantics.)
static uint64_t value_hash(const Value& v) {
    uint64_t h = static_cast<uint64_t>(static_cast<int>(v.type));
    uint64_t p = 0;
    switch (v.type) {
        case ValueType::INT: p = static_cast<uint64_t>(static_cast<uint32_t>(v.as.i)); break;
        case ValueType::FLOAT: {
            float f = v.as.f;
            if (f == 0.0f) f = 0.0f; // normalize -0.0
            uint32_t b = 0;
            std::memcpy(&b, &f, sizeof(b));
            p = b;
            break;
        }
        case ValueType::BOOL: p = v.as.b ? 1u : 0u; break;
        case ValueType::CHAR: p = static_cast<uint64_t>(static_cast<unsigned char>(v.as.c)); break;
        case ValueType::STRING_CONST: p = static_cast<uint64_t>(static_cast<uint32_t>(v.as.str_idx)); break;
        case ValueType::OBJ_REF: p = static_cast<uint64_t>(static_cast<uint32_t>(v.as.obj_ref)); break;
    }
    // splitmix64 finalizer over the combined input.
    uint64_t z = h + 0x9E3779B97F4A7C15ULL + p;
    z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9ULL;
    z = (z ^ (z >> 27)) * 0x94D049BB133111EBULL;
    return z ^ (z >> 31);
}

// Finds key in hm->entries via the hash index. Returns true + position if present.
static bool map_find(HashMapObject* hm, const Value& key, size_t& pos) {
    auto it = hm->index.find(value_hash(key));
    if (it == hm->index.end()) return false;
    for (size_t p : it->second) {
        if (p < hm->entries.size() && value_equal(hm->entries[p].key, key)) {
            pos = p;
            return true;
        }
    }
    return false;
}

// Rebuilds the whole index. Used after removal (which shifts positions);
// simple and obviously correct, and removal is already O(n) via vector erase.
static void map_rebuild_index(HashMapObject* hm) {
    hm->index.clear();
    for (size_t i = 0; i < hm->entries.size(); ++i) {
        hm->index[value_hash(hm->entries[i].key)].push_back(i);
    }
}

// --- mapNew() : HashMap ---
static Value native_mapNew(std::vector<Value>& args, std::vector<std::string>& constants, Heap& heap) {
    (void)args; (void)constants;
    HashMapObject* hm = new HashMapObject();
    return Value::make_obj(heap.register_object(hm));
}

// --- mapPut(map, key, value) : void ---
static Value native_mapPut(std::vector<Value>& args, std::vector<std::string>& constants, Heap& heap) {
    (void)constants;
    if (args.size() != 3) throw std::runtime_error("mapPut expects 3 arguments.");
    if (args[0].type != ValueType::OBJ_REF) throw std::runtime_error("mapPut expects a HashMap.");
    HashMapObject* hm = static_cast<HashMapObject*>(heap.objects[args[0].as.obj_ref]);
    if (!hm || hm->type != ObjType::HASH_MAP) throw std::runtime_error("mapPut expects a HashMap.");
    size_t pos = 0;
    if (map_find(hm, args[1], pos)) { hm->entries[pos].value = args[2]; return Value(); }
    hm->entries.push_back(HashEntry(args[1], args[2]));
    hm->index[value_hash(args[1])].push_back(hm->entries.size() - 1);
    return Value();
}

// --- mapGet(map, key) : value ---
// Returns the value for key, or a default (int 0) if the key is absent.
static Value native_mapGet(std::vector<Value>& args, std::vector<std::string>& constants, Heap& heap) {
    (void)constants;
    if (args.size() != 2) throw std::runtime_error("mapGet expects 2 arguments.");
    if (args[0].type != ValueType::OBJ_REF) throw std::runtime_error("mapGet expects a HashMap.");
    HashMapObject* hm = static_cast<HashMapObject*>(heap.objects[args[0].as.obj_ref]);
    if (!hm || hm->type != ObjType::HASH_MAP) throw std::runtime_error("mapGet expects a HashMap.");
    size_t pos = 0;
    if (map_find(hm, args[1], pos)) return hm->entries[pos].value;
    return Value(); // absent key -> int 0
}

// --- mapContainsKey(map, key) : bool ---
static Value native_mapContainsKey(std::vector<Value>& args, std::vector<std::string>& constants, Heap& heap) {
    (void)constants;
    if (args.size() != 2) throw std::runtime_error("mapContainsKey expects 2 arguments.");
    if (args[0].type != ValueType::OBJ_REF) throw std::runtime_error("mapContainsKey expects a HashMap.");
    HashMapObject* hm = static_cast<HashMapObject*>(heap.objects[args[0].as.obj_ref]);
    if (!hm || hm->type != ObjType::HASH_MAP) throw std::runtime_error("mapContainsKey expects a HashMap.");
    size_t pos = 0;
    return Value(map_find(hm, args[1], pos));
}

// --- mapRemove(map, key) : void ---
static Value native_mapRemove(std::vector<Value>& args, std::vector<std::string>& constants, Heap& heap) {
    (void)constants;
    if (args.size() != 2) throw std::runtime_error("mapRemove expects 2 arguments.");
    if (args[0].type != ValueType::OBJ_REF) throw std::runtime_error("mapRemove expects a HashMap.");
    HashMapObject* hm = static_cast<HashMapObject*>(heap.objects[args[0].as.obj_ref]);
    if (!hm || hm->type != ObjType::HASH_MAP) throw std::runtime_error("mapRemove expects a HashMap.");
    size_t pos = 0;
    if (map_find(hm, args[1], pos)) {
        hm->entries.erase(hm->entries.begin() + pos);
        map_rebuild_index(hm);
    }
    return Value();
}

// --- mapSize(map) : int ---
static Value native_mapSize(std::vector<Value>& args, std::vector<std::string>& constants, Heap& heap) {
    (void)constants;
    if (args.size() != 1) throw std::runtime_error("mapSize expects 1 argument.");
    if (args[0].type != ValueType::OBJ_REF) throw std::runtime_error("mapSize expects a HashMap.");
    HashMapObject* hm = static_cast<HashMapObject*>(heap.objects[args[0].as.obj_ref]);
    if (!hm || hm->type != ObjType::HASH_MAP) throw std::runtime_error("mapSize expects a HashMap.");
    return Value(static_cast<int32_t>(hm->entries.size()));
}

// --- Collections: LinkedList ---

// --- llNew() : LinkedList ---
static Value native_llNew(std::vector<Value>& args, std::vector<std::string>& constants, Heap& heap) {
    (void)args; (void)constants;
    LinkedListObject* ll = new LinkedListObject();
    return Value::make_obj(heap.register_object(ll));
}

// --- llAddFirst(ll, value) : void ---
static Value native_llAddFirst(std::vector<Value>& args, std::vector<std::string>& constants, Heap& heap) {
    (void)constants;
    if (args.size() != 2) throw std::runtime_error("llAddFirst expects 2 arguments.");
    if (args[0].type != ValueType::OBJ_REF) throw std::runtime_error("llAddFirst expects a LinkedList.");
    LinkedListObject* ll = static_cast<LinkedListObject*>(heap.objects[args[0].as.obj_ref]);
    if (!ll || ll->type != ObjType::LINKED_LIST) throw std::runtime_error("llAddFirst expects a LinkedList.");
    ll->items.insert(ll->items.begin(), args[1]);
    return Value();
}

// --- llAddLast(ll, value) : void ---
static Value native_llAddLast(std::vector<Value>& args, std::vector<std::string>& constants, Heap& heap) {
    (void)constants;
    if (args.size() != 2) throw std::runtime_error("llAddLast expects 2 arguments.");
    if (args[0].type != ValueType::OBJ_REF) throw std::runtime_error("llAddLast expects a LinkedList.");
    LinkedListObject* ll = static_cast<LinkedListObject*>(heap.objects[args[0].as.obj_ref]);
    if (!ll || ll->type != ObjType::LINKED_LIST) throw std::runtime_error("llAddLast expects a LinkedList.");
    ll->items.push_back(args[1]);
    return Value();
}

// --- llRemoveFirst(ll) : value ---
static Value native_llRemoveFirst(std::vector<Value>& args, std::vector<std::string>& constants, Heap& heap) {
    (void)constants;
    if (args.size() != 1) throw std::runtime_error("llRemoveFirst expects 1 argument.");
    if (args[0].type != ValueType::OBJ_REF) throw std::runtime_error("llRemoveFirst expects a LinkedList.");
    LinkedListObject* ll = static_cast<LinkedListObject*>(heap.objects[args[0].as.obj_ref]);
    if (!ll || ll->type != ObjType::LINKED_LIST) throw std::runtime_error("llRemoveFirst expects a LinkedList.");
    if (ll->items.empty()) throw std::runtime_error("llRemoveFirst: empty list.");
    Value front = ll->items.front();
    ll->items.erase(ll->items.begin());
    return front;
}

// --- llRemoveLast(ll) : value ---
static Value native_llRemoveLast(std::vector<Value>& args, std::vector<std::string>& constants, Heap& heap) {
    (void)constants;
    if (args.size() != 1) throw std::runtime_error("llRemoveLast expects 1 argument.");
    if (args[0].type != ValueType::OBJ_REF) throw std::runtime_error("llRemoveLast expects a LinkedList.");
    LinkedListObject* ll = static_cast<LinkedListObject*>(heap.objects[args[0].as.obj_ref]);
    if (!ll || ll->type != ObjType::LINKED_LIST) throw std::runtime_error("llRemoveLast expects a LinkedList.");
    if (ll->items.empty()) throw std::runtime_error("llRemoveLast: empty list.");
    Value back = ll->items.back();
    ll->items.pop_back();
    return back;
}

// --- llGet(ll, index) : value ---
static Value native_llGet(std::vector<Value>& args, std::vector<std::string>& constants, Heap& heap) {
    (void)constants;
    if (args.size() != 2) throw std::runtime_error("llGet expects 2 arguments.");
    if (args[0].type != ValueType::OBJ_REF) throw std::runtime_error("llGet expects a LinkedList.");
    if (args[1].type != ValueType::INT) throw std::runtime_error("llGet expects an int index.");
    LinkedListObject* ll = static_cast<LinkedListObject*>(heap.objects[args[0].as.obj_ref]);
    if (!ll || ll->type != ObjType::LINKED_LIST) throw std::runtime_error("llGet expects a LinkedList.");
    int32_t idx = args[1].as.i;
    if (idx < 0 || idx >= static_cast<int32_t>(ll->items.size())) throw std::runtime_error("llGet: index out of range.");
    return ll->items[idx];
}

// --- llSize(ll) : int ---
static Value native_llSize(std::vector<Value>& args, std::vector<std::string>& constants, Heap& heap) {
    (void)constants;
    if (args.size() != 1) throw std::runtime_error("llSize expects 1 argument.");
    if (args[0].type != ValueType::OBJ_REF) throw std::runtime_error("llSize expects a LinkedList.");
    LinkedListObject* ll = static_cast<LinkedListObject*>(heap.objects[args[0].as.obj_ref]);
    if (!ll || ll->type != ObjType::LINKED_LIST) throw std::runtime_error("llSize expects a LinkedList.");
    return Value(static_cast<int32_t>(ll->items.size()));
}

// --- llEmpty(ll) : bool ---
static Value native_llEmpty(std::vector<Value>& args, std::vector<std::string>& constants, Heap& heap) {
    (void)constants;
    if (args.size() != 1) throw std::runtime_error("llEmpty expects 1 argument.");
    if (args[0].type != ValueType::OBJ_REF) throw std::runtime_error("llEmpty expects a LinkedList.");
    LinkedListObject* ll = static_cast<LinkedListObject*>(heap.objects[args[0].as.obj_ref]);
    if (!ll || ll->type != ObjType::LINKED_LIST) throw std::runtime_error("llEmpty expects a LinkedList.");
    return Value(ll->items.empty());
}

// --- readResource(name) : String ---
// Returns the contents of a bundled .ama resource, or "" if absent.
static Value native_readResource(std::vector<Value>& args, std::vector<std::string>& constants, Heap& heap) {
    (void)heap;
    if (args.size() != 1) throw std::runtime_error("readResource expects 1 argument.");
    if (args[0].type != ValueType::STRING_CONST) throw std::runtime_error("readResource expects a String name.");
    const std::string* content = Resources::get(constants[args[0].as.str_idx]);
    return make_string_const(constants, content ? *content : "");
}

// --- hasResource(name) : bool ---
// Whether a bundled resource exists in the archive.
static Value native_hasResource(std::vector<Value>& args, std::vector<std::string>& constants, Heap& heap) {
    (void)heap;
    if (args.size() != 1) throw std::runtime_error("hasResource expects 1 argument.");
    if (args[0].type != ValueType::STRING_CONST) throw std::runtime_error("hasResource expects a String name.");
    return Value(Resources::has(constants[args[0].as.str_idx]));
}

// --- resourceNames() : String ---
// All bundled resource names, newline-separated.
static Value native_resourceNames(std::vector<Value>& args, std::vector<std::string>& constants, Heap& heap) {
    (void)args; (void)heap;
    return make_string_const(constants, Resources::names());
}

// --- Minimal HTTP/1.0 client (http:// only, no TLS) ---
// Fail-soft like readFile: any failure yields "".
#ifdef _WIN32
using sock_t = SOCKET;
constexpr sock_t BAD_SOCK = INVALID_SOCKET;
inline void close_sock(sock_t s) { closesocket(s); }
#else
using sock_t = int;
constexpr sock_t BAD_SOCK = -1;
inline void close_sock(sock_t s) { ::close(s); }
#endif

static bool http_fetch(const std::string& url, const std::string& method, const std::string& body, std::string& out_body) {
    const std::string scheme = "http://";
    if (url.compare(0, scheme.size(), scheme) != 0) return false;
    std::string rest = url.substr(scheme.size());

    std::string hostport, path = "/";
    size_t slash = rest.find('/');
    if (slash == std::string::npos) {
        hostport = rest;
    } else {
        hostport = rest.substr(0, slash);
        path = rest.substr(slash);
        if (path.empty()) path = "/";
    }
    std::string host = hostport, port = "80";
    size_t colon = hostport.rfind(':');
    if (colon != std::string::npos) {
        host = hostport.substr(0, colon);
        port = hostport.substr(colon + 1);
    }
    if (host.empty() || port.empty()) return false;

#ifdef _WIN32
    static bool wsa_init = false;
    if (!wsa_init) {
        WSADATA wsa;
        if (WSAStartup(MAKEWORD(2, 2), &wsa) != 0) return false;
        wsa_init = true;
    }
#endif

    struct addrinfo hints{};
    hints.ai_family = AF_UNSPEC;
    hints.ai_socktype = SOCK_STREAM;
    struct addrinfo* list = nullptr;
    if (getaddrinfo(host.c_str(), port.c_str(), &hints, &list) != 0) return false;

    sock_t sock = BAD_SOCK;
    for (struct addrinfo* p = list; p; p = p->ai_next) {
        sock_t s = socket(p->ai_family, p->ai_socktype, p->ai_protocol);
        if (s == BAD_SOCK) continue;
        if (connect(s, p->ai_addr, static_cast<int>(p->ai_addrlen)) != 0) {
            close_sock(s);
            continue;
        }
        sock = s;
        break;
    }
    freeaddrinfo(list);
    if (sock == BAD_SOCK) return false;

    std::ostringstream req;
    req << method << " " << path << " HTTP/1.0\r\n"
        << "Host: " << host << "\r\n"
        << "Connection: close\r\n";
    if (method == "POST") req << "Content-Length: " << body.size() << "\r\n";
    req << "\r\n";
    if (method == "POST") req << body;
    std::string reqStr = req.str();

    size_t sent = 0;
    while (sent < reqStr.size()) {
        int n = send(sock, reqStr.data() + sent, static_cast<int>(reqStr.size() - sent), 0);
        if (n <= 0) { close_sock(sock); return false; }
        sent += static_cast<size_t>(n);
    }

    std::string raw;
    char buf[4096];
    for (;;) {
        int n = recv(sock, buf, sizeof(buf), 0);
        if (n <= 0) break;
        raw.append(buf, static_cast<size_t>(n));
    }
    close_sock(sock);

    size_t hdrEnd = raw.find("\r\n\r\n");
    if (hdrEnd == std::string::npos) return false;
    out_body = raw.substr(hdrEnd + 4);
    return true;
}

// --- httpGet(url) : String ---
// Returns the response body, or "" on any failure / non-http URL.
static Value native_httpGet(std::vector<Value>& args, std::vector<std::string>& constants, Heap& heap) {
    (void)heap;
    if (args.size() != 1) throw std::runtime_error("httpGet expects 1 argument.");
    if (args[0].type != ValueType::STRING_CONST) throw std::runtime_error("httpGet expects a String URL.");
    std::string out;
    if (!http_fetch(constants[args[0].as.str_idx], "GET", "", out)) return make_string_const(constants, "");
    return make_string_const(constants, out);
}

// --- httpPost(url, body) : String ---
// POSTs body and returns the response body, or "" on any failure.
static Value native_httpPost(std::vector<Value>& args, std::vector<std::string>& constants, Heap& heap) {
    (void)heap;
    if (args.size() != 2) throw std::runtime_error("httpPost expects 2 arguments.");
    if (args[0].type != ValueType::STRING_CONST) throw std::runtime_error("httpPost expects a String URL.");
    if (args[1].type != ValueType::STRING_CONST) throw std::runtime_error("httpPost expects a String body.");
    std::string out;
    if (!http_fetch(constants[args[0].as.str_idx], "POST", constants[args[1].as.str_idx], out)) return make_string_const(constants, "");
    return make_string_const(constants, out);
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
        {native_mapNew, 0},
        {native_mapPut, 3},
        {native_mapGet, 2},
        {native_mapContainsKey, 2},
        {native_mapRemove, 2},
        {native_mapSize, 1},
        {native_llNew, 0},
        {native_llAddFirst, 2},
        {native_llAddLast, 2},
        {native_llRemoveFirst, 1},
        {native_llRemoveLast, 1},
        {native_llGet, 2},
        {native_llSize, 1},
        {native_llEmpty, 1},
        {native_readResource, 1},
        {native_hasResource, 1},
        {native_resourceNames, 0},
        {native_httpGet, 1},
        {native_httpPost, 2},
    };
    return reg;
}

} // namespace Natives
