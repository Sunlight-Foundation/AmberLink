#include "heap.hpp"
#include <iostream>
#include <algorithm>
#ifdef AVM_GC_PROFILE
#include <chrono>
#endif

Heap::~Heap() {
#ifdef AVM_GC_PROFILE
    std::cerr << "[AVM GC] allocations=" << profile.allocations
              << " object_creation_sample_ns=" << profile.object_creation_ns
              << " object_creation_samples=" << profile.object_creation_samples
              << " register_sample_ns=" << profile.register_ns
              << " register_samples=" << profile.register_samples
              << " collection_checks=" << profile.collection_checks
              << " threshold_check_sample_ns=" << profile.threshold_check_ns
              << " threshold_check_samples=" << profile.threshold_check_samples
              << " collections=" << profile.collections
              << " collection_ns=" << profile.collection_ns
              << " reset_ns=" << profile.reset_ns
              << " mark_ns=" << profile.mark_ns
              << " sweep_ns=" << profile.sweep_ns
              << " reset_objects=" << profile.reset_objects
              << " root_values=" << profile.root_values
              << " marked_objects=" << profile.marked_objects
              << " object_slots_scanned=" << profile.object_slots_scanned
              << " reclaimed_objects=" << profile.reclaimed_objects
              << " last_gc_live_objects=" << live_objects
              << " heap_objects_at_exit=" << std::count_if(objects.begin(), objects.end(), [](const AmberObject* obj) { return obj != nullptr; })
              << " heap_slots=" << objects.size() << std::endl;
#endif
    for (AmberObject* obj : objects) {
        if (obj) delete obj;
    }
    objects.clear();
}

int32_t Heap::register_object(AmberObject* obj) {
#ifdef AVM_GC_PROFILE
    const bool sample = (profile.allocations & 1023u) == 0;
    const auto profile_start = sample ? std::chrono::steady_clock::now() : std::chrono::steady_clock::time_point{};
    ++profile.allocations;
#endif
    ++allocated_since_gc;
    if (!free_slots.empty()) {
        size_t idx = free_slots.back();
        free_slots.pop_back();
        objects[idx] = obj;
        // std::cout << "[GC] Reusing slot " << idx << std::endl;
#ifdef AVM_GC_PROFILE
        if (sample) {
            const auto elapsed = std::chrono::duration_cast<std::chrono::nanoseconds>(
                std::chrono::steady_clock::now() - profile_start).count();
            profile.register_ns += static_cast<uint64_t>(elapsed);
            ++profile.register_samples;
        }
#endif
        return static_cast<int32_t>(idx);
    } else {
        objects.push_back(obj);
        // std::cout << "[GC] Allocating new slot " << (objects.size() - 1) << std::endl;
#ifdef AVM_GC_PROFILE
        if (sample) {
            const auto elapsed = std::chrono::duration_cast<std::chrono::nanoseconds>(
                std::chrono::steady_clock::now() - profile_start).count();
            profile.register_ns += static_cast<uint64_t>(elapsed);
            ++profile.register_samples;
        }
#endif
        return static_cast<int32_t>(objects.size() - 1);
    }
}

void Heap::mark(AmberObject* obj, size_t constant_pool_size) {
    if (obj == nullptr || obj->marked) return;
    
    obj->marked = true;
#ifdef AVM_GC_PROFILE
    ++profile.marked_objects;
#endif
    
    if (obj->type == ObjType::ARRAY) {
        ArrayObject* arr = static_cast<ArrayObject*>(obj);
        for (const Value& val : arr->data) {
#ifdef AVM_GC_PROFILE
            ++profile.object_slots_scanned;
#endif
            if (val.type == ValueType::OBJ_REF) {
                size_t heap_idx = val.as.obj_ref;
                if (heap_idx < objects.size()) {
                    mark(objects[heap_idx], constant_pool_size);
                }
            }
        }
    } else if (obj->type == ObjType::INSTANCE) {
        InstanceObject* inst = static_cast<InstanceObject*>(obj);
        for (const Value& val : inst->fields) {
#ifdef AVM_GC_PROFILE
            ++profile.object_slots_scanned;
#endif
            if (val.type == ValueType::OBJ_REF) {
                size_t heap_idx = val.as.obj_ref;
                if (heap_idx < objects.size()) {
                    mark(objects[heap_idx], constant_pool_size);
                }
            }
        }
    } else if (obj->type == ObjType::LIST) {
        ListObject* list = static_cast<ListObject*>(obj);
        for (const Value& val : list->items) {
#ifdef AVM_GC_PROFILE
            ++profile.object_slots_scanned;
#endif
            if (val.type == ValueType::OBJ_REF) {
                size_t heap_idx = val.as.obj_ref;
                if (heap_idx < objects.size()) {
                    mark(objects[heap_idx], constant_pool_size);
                }
            }
        }
    } else if (obj->type == ObjType::HASH_MAP) {
        HashMapObject* hm = static_cast<HashMapObject*>(obj);
        for (const HashEntry& entry : hm->entries) {
#ifdef AVM_GC_PROFILE
            profile.object_slots_scanned += 2;
#endif
            if (entry.key.type == ValueType::OBJ_REF) {
                size_t heap_idx = entry.key.as.obj_ref;
                if (heap_idx < objects.size()) mark(objects[heap_idx], constant_pool_size);
            }
            if (entry.value.type == ValueType::OBJ_REF) {
                size_t heap_idx = entry.value.as.obj_ref;
                if (heap_idx < objects.size()) mark(objects[heap_idx], constant_pool_size);
            }
        }
    } else if (obj->type == ObjType::LINKED_LIST) {
        LinkedListObject* ll = static_cast<LinkedListObject*>(obj);
        for (const Value& val : ll->items) {
#ifdef AVM_GC_PROFILE
            ++profile.object_slots_scanned;
#endif
            if (val.type == ValueType::OBJ_REF) {
                size_t heap_idx = val.as.obj_ref;
                if (heap_idx < objects.size()) mark(objects[heap_idx], constant_pool_size);
            }
        }
    }
}

void Heap::collect(const std::vector<std::vector<Value>*>& stacks, const std::vector<Value>& globals,
                   const std::vector<Value>& pending_thread_roots, const std::vector<Value>& thread_roots,
                   size_t constant_pool_size) {
    // Cheap gate: skip the full mark-and-sweep until enough garbage may exist.
#ifdef AVM_GC_PROFILE
    ++profile.collection_checks;
    const bool sample_check = (profile.collection_checks & 1023u) == 1u;
    const auto check_start = sample_check ? std::chrono::steady_clock::now() : std::chrono::steady_clock::time_point{};
#endif
    if (allocated_since_gc < gc_threshold) {
#ifdef AVM_GC_PROFILE
        if (sample_check) {
            const auto elapsed = std::chrono::duration_cast<std::chrono::nanoseconds>(
                std::chrono::steady_clock::now() - check_start).count();
            profile.threshold_check_ns += static_cast<uint64_t>(elapsed);
            ++profile.threshold_check_samples;
        }
#endif
        return;
    }
#ifdef AVM_GC_PROFILE
    if (sample_check) {
        profile.threshold_check_ns += static_cast<uint64_t>(std::chrono::duration_cast<std::chrono::nanoseconds>(std::chrono::steady_clock::now() - check_start).count());
        ++profile.threshold_check_samples;
    }
#endif
    allocated_since_gc = 0;
#ifdef AVM_GC_PROFILE
    ++profile.collections;
    const auto collection_start = std::chrono::steady_clock::now();
    auto phase_start = collection_start;
#endif

    // 1. Unmark all objects (Reset)
    for (AmberObject* obj : objects) {
        if (obj) {
#ifdef AVM_GC_PROFILE
            ++profile.reset_objects;
#endif
            obj->marked = false;
        }
    }
#ifdef AVM_GC_PROFILE
    phase_start = std::chrono::steady_clock::now();
    profile.reset_ns += static_cast<uint64_t>(std::chrono::duration_cast<std::chrono::nanoseconds>(
        phase_start - collection_start).count());
#endif

    // 2. Mark Roots (all active interpreter stacks).
    for (const std::vector<Value>* stack : stacks) {
        if (!stack) continue;
        for (const Value& val : *stack) {
#ifdef AVM_GC_PROFILE
            ++profile.root_values;
#endif
            if (val.type == ValueType::OBJ_REF) {
                size_t heap_idx = val.as.obj_ref;
                if (heap_idx < objects.size()) mark(objects[heap_idx], constant_pool_size);
            }
        }
    }

    // 3. Mark Roots (Globals)
    for (const Value& val : globals) {
#ifdef AVM_GC_PROFILE
        ++profile.root_values;
#endif
        if (val.type == ValueType::OBJ_REF) {
            size_t heap_idx = val.as.obj_ref;
            if (heap_idx < objects.size()) {
                mark(objects[heap_idx], constant_pool_size);
            }
        }
    }

    // 4. Mark Roots (arguments waiting for a worker to start).
    for (const Value& val : pending_thread_roots) {
#ifdef AVM_GC_PROFILE
        ++profile.root_values;
#endif
        if (val.type == ValueType::OBJ_REF) {
            size_t heap_idx = val.as.obj_ref;
            if (heap_idx < objects.size()) mark(objects[heap_idx], constant_pool_size);
        }
    }

    // 5. Mark Roots (completed thread results).
    for (const Value& val : thread_roots) {
#ifdef AVM_GC_PROFILE
        ++profile.root_values;
#endif
        if (val.type == ValueType::OBJ_REF) {
            size_t heap_idx = val.as.obj_ref;
            if (heap_idx < objects.size()) mark(objects[heap_idx], constant_pool_size);
        }
    }

    // 6. Sweep
#ifdef AVM_GC_PROFILE
    const auto sweep_start = std::chrono::steady_clock::now();
    profile.mark_ns += static_cast<uint64_t>(std::chrono::duration_cast<std::chrono::nanoseconds>(
        sweep_start - phase_start).count());
#endif
    sweep();
#ifdef AVM_GC_PROFILE
    const auto collection_end = std::chrono::steady_clock::now();
    profile.sweep_ns += static_cast<uint64_t>(std::chrono::duration_cast<std::chrono::nanoseconds>(
        collection_end - sweep_start).count());
    profile.collection_ns += static_cast<uint64_t>(std::chrono::duration_cast<std::chrono::nanoseconds>(
        collection_end - collection_start).count());
#endif
}

void Heap::sweep() {
    live_objects = 0;
    for (size_t i = 0; i < objects.size(); ++i) {
        AmberObject* obj = objects[i];
        if (obj) {
            if (!obj->marked) {
#ifdef AVM_GC_PROFILE
                ++profile.reclaimed_objects;
#endif
                delete obj;
                objects[i] = nullptr;
                free_slots.push_back(i);
            } else {
                ++live_objects;
            }
        }
    }
    // Adapt: next collection only after the heap could have doubled.
    size_t next = live_objects * 2;
    gc_threshold = next < 1024 ? 1024 : next;
}