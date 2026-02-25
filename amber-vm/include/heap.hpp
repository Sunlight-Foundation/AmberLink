#ifndef HEAP_HPP
#define HEAP_HPP

#include <vector>
#include <cstdint>
#include <cstddef>
#include "value.hpp"

// Offset to distinguish Heap Objects from Constant Pool indices in negative handles
constexpr int32_t HEAP_HANDLE_OFFSET = 0x40000000;

enum class ObjType {
    STRING,
    ARRAY,
    INSTANCE
};

struct AmberObject {
    bool marked = false;
    ObjType type;
    virtual ~AmberObject() = default;
};

struct ArrayObject : AmberObject {
    std::vector<Value> data;
    ArrayObject(size_t size) {
        type = ObjType::ARRAY;
        data.resize(size, Value()); // Initialize with default Value (INT 0)
    }
};

struct InstanceObject : AmberObject {
    uint32_t class_id;
    std::vector<Value> fields;
    InstanceObject(uint32_t cls_id, size_t field_count) : class_id(cls_id) {
        type = ObjType::INSTANCE;
        fields.resize(field_count, Value());
    }
};

class Heap {
public:
    std::vector<AmberObject*> objects; // Public for direct access by VM
    std::vector<size_t> free_slots;    // Indices of freed objects (holes)
    ~Heap();
    int32_t register_object(AmberObject* obj);
    void mark(AmberObject* obj, size_t constant_pool_size);
    void collect(const std::vector<Value>& stack, const std::vector<Value>& globals, size_t constant_pool_size);
    void sweep();
};

#endif