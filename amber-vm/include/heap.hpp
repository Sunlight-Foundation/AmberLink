#ifndef HEAP_HPP
#define HEAP_HPP

#include <vector>
#include <unordered_map>
#include <cstdint>
#include <cstddef>
#include "value.hpp"

// Offset to distinguish Heap Objects from Constant Pool indices in negative handles
constexpr int32_t HEAP_HANDLE_OFFSET = 0x40000000;

enum class ObjType {
    STRING,
    ARRAY,
    INSTANCE,
    LIST,
    HASH_MAP,
    LINKED_LIST
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

struct ListObject : AmberObject {
    std::vector<Value> items;
    ListObject() {
        type = ObjType::LIST;
    }
};

// A key/value pair stored in a HashMapObject.
struct HashEntry {
    Value key;
    Value value;
    HashEntry(const Value& k, const Value& v) : key(k), value(v) {}
};

struct HashMapObject : AmberObject {
    std::vector<HashEntry> entries;
    // Hash index: key hash -> positions in entries. Turns O(n) scans into
    // O(1) lookups. Holds indices (not pointers), so the GC needs no changes.
    std::unordered_map<uint64_t, std::vector<size_t>> index;
    HashMapObject() {
        type = ObjType::HASH_MAP;
    }
};

struct LinkedListObject : AmberObject {
    std::vector<Value> items;
    LinkedListObject() {
        type = ObjType::LINKED_LIST;
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
    // Allocation-triggered GC policy: collect() is a no-op until enough new
    // objects were registered since the last collection. After each real
    // collection the threshold adapts to 2x live objects (min 1024), so GC
    // cost stays amortized even when call sites (e.g. string concat) ask often.
    size_t allocated_since_gc = 0;
    size_t gc_threshold = 1024;
    size_t live_objects = 0;
    ~Heap();
    int32_t register_object(AmberObject* obj);
    void mark(AmberObject* obj, size_t constant_pool_size);
    void collect(const std::vector<Value>& stack, const std::vector<Value>& globals, size_t constant_pool_size);
    void sweep();
};

#endif