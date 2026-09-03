#ifndef VALUE_HPP
#define VALUE_HPP

#include <cstdint>
#include <iostream>

enum class ValueType {
    INT,
    FLOAT,
    BOOL,
    CHAR,
    OBJ_REF,      // Index into Heap::objects
    STRING_CONST  // Index into Constant Pool
};

struct Value {
    ValueType type;
    union {
        int32_t i;
        float f;
        bool b;
        char c;
        int32_t obj_ref; // Index
        int32_t str_idx; // Index
    } as;

    // Constructors
    Value() : type(ValueType::INT) { as.i = 0; }
    explicit Value(int32_t v) : type(ValueType::INT) { as.i = v; }
    explicit Value(float v) : type(ValueType::FLOAT) { as.f = v; }
    explicit Value(bool v) : type(ValueType::BOOL) { as.b = v; }
    explicit Value(char v) : type(ValueType::CHAR) { as.c = v; }

    // Static helpers
    static Value make_obj(int32_t idx) {
        Value v;
        v.type = ValueType::OBJ_REF;
        v.as.obj_ref = idx;
        return v;
    }

    static Value make_string(int32_t idx) {
        Value v;
        v.type = ValueType::STRING_CONST;
        v.as.str_idx = idx;
        return v;
    }
};

inline std::ostream& operator<<(std::ostream& os, const Value& v) {
    switch (v.type) {
        case ValueType::INT: os << v.as.i; break;
        case ValueType::FLOAT: os << v.as.f; break;
        case ValueType::BOOL: os << (v.as.b ? "true" : "false"); break;
        case ValueType::CHAR: os << "'" << v.as.c << "'"; break;
        case ValueType::OBJ_REF: os << "ObjRef(" << v.as.obj_ref << ")"; break;
        case ValueType::STRING_CONST: os << "StrConst(" << v.as.str_idx << ")"; break;
    }
    return os;
}

#endif // VALUE_HPP