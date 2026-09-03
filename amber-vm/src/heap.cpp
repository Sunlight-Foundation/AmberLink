#include "heap.hpp"
#include <iostream>
#include <algorithm>

Heap::~Heap() {
    for (AmberObject* obj : objects) {
        if (obj) delete obj;
    }
    objects.clear();
}

int32_t Heap::register_object(AmberObject* obj) {
    ++allocated_since_gc;
    if (!free_slots.empty()) {
        size_t idx = free_slots.back();
        free_slots.pop_back();
        objects[idx] = obj;
        // std::cout << "[GC] Reusing slot " << idx << std::endl;
        return static_cast<int32_t>(idx);
    } else {
        objects.push_back(obj);
        // std::cout << "[GC] Allocating new slot " << (objects.size() - 1) << std::endl;
        return static_cast<int32_t>(objects.size() - 1);
    }
}

void Heap::mark(AmberObject* obj, size_t constant_pool_size) {
    if (obj == nullptr || obj->marked) return;
    
    obj->marked = true;
    
    if (obj->type == ObjType::ARRAY) {
        ArrayObject* arr = static_cast<ArrayObject*>(obj);
        for (const Value& val : arr->data) {
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
            if (val.type == ValueType::OBJ_REF) {
                size_t heap_idx = val.as.obj_ref;
                if (heap_idx < objects.size()) mark(objects[heap_idx], constant_pool_size);
            }
        }
    }
}

void Heap::collect(const std::vector<Value>& stack, const std::vector<Value>& globals, size_t constant_pool_size) {
    // Cheap gate: skip the full mark-and-sweep until enough garbage may exist.
    if (allocated_since_gc < gc_threshold) return;
    allocated_since_gc = 0;

    // 1. Unmark all objects (Reset)
    for (AmberObject* obj : objects) {
        if (obj) obj->marked = false;
    }

    // 2. Mark Roots (Stack)
    for (const Value& val : stack) {
        if (val.type == ValueType::OBJ_REF) {
            size_t heap_idx = val.as.obj_ref;
            if (heap_idx < objects.size()) {
                mark(objects[heap_idx], constant_pool_size);
            }
        }
    }

    // 3. Mark Roots (Globals)
    for (const Value& val : globals) {
        if (val.type == ValueType::OBJ_REF) {
            size_t heap_idx = val.as.obj_ref;
            if (heap_idx < objects.size()) {
                mark(objects[heap_idx], constant_pool_size);
            }
        }
    }

    // 4. Sweep
    sweep();
}

void Heap::sweep() {
    live_objects = 0;
    for (size_t i = 0; i < objects.size(); ++i) {
        AmberObject* obj = objects[i];
        if (obj) {
            if (!obj->marked) {
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