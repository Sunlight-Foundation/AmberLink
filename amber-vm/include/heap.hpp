#ifndef HEAP_HPP
#define HEAP_HPP

#include <vector>
#include <unordered_map>
#include <cstdint>
#include <cstddef>
#include <utility>
#ifdef AVM_GC_PROFILE
#include <chrono>
#endif
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
    uint32_t mark_epoch = 0;
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
#ifdef AVM_GC_PROFILE
    struct Profile {
        uint64_t allocations = 0;
        uint64_t object_creation_ns = 0;
        uint64_t object_creation_samples = 0;
        uint64_t register_ns = 0;
        uint64_t register_samples = 0;
        uint64_t collection_checks = 0;
        uint64_t threshold_check_ns = 0;
        uint64_t threshold_check_samples = 0;
        uint64_t collections = 0;
        uint64_t collection_ns = 0;
        uint64_t reset_ns = 0;
        uint64_t mark_ns = 0;
        uint64_t sweep_ns = 0;
        uint64_t reset_objects = 0;
        uint64_t sweep_slots_scanned = 0;
        uint64_t root_values = 0;
        uint64_t marked_objects = 0;
        uint64_t object_slots_scanned = 0;
        uint64_t reclaimed_objects = 0;
    } profile;
#endif
    std::vector<AmberObject*> objects; // Stable handle table; slots are never compacted.
    std::vector<size_t> free_slots;    // Indices of freed objects (holes)
    uint32_t mark_epoch = 0; // Generation used instead of clearing every object's mark bit.
    // Allocation-triggered GC policy: safe points check collect() after enough
    // objects were registered since the last collection. The threshold adapts
    // to 2x live objects (min 1024), amortizing mark-and-sweep cost.

    size_t allocated_since_gc = 0;
    size_t gc_threshold = 1024;
    size_t live_objects = 0;
    ~Heap();
    int32_t register_object(AmberObject* obj);
    template <typename T, typename... Args>
    int32_t create_object(Args&&... args) {
#ifdef AVM_GC_PROFILE
        const bool sample = (profile.allocations & 1023u) == 0;
        const auto start = sample ? std::chrono::steady_clock::now() : std::chrono::steady_clock::time_point{};
#endif
        T* obj = new T(std::forward<Args>(args)...);
        const int32_t handle = register_object(obj);
#ifdef AVM_GC_PROFILE
        if (sample) {
            const auto elapsed = std::chrono::duration_cast<std::chrono::nanoseconds>(
                std::chrono::steady_clock::now() - start).count();
            profile.object_creation_ns += static_cast<uint64_t>(elapsed);
            ++profile.object_creation_samples;
        }
#endif
        return handle;
    }
    void mark(AmberObject* obj, size_t constant_pool_size);
    void collect(const std::vector<std::vector<Value>*>& stacks, const std::vector<Value>& globals,
                 const std::vector<Value>& pending_thread_roots, const std::vector<Value>& thread_roots,
                 size_t constant_pool_size);
    void sweep();
};

#endif