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
    }
}

void Heap::collect(const std::vector<Value>& stack, const std::vector<Value>& globals, size_t constant_pool_size) {
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
    for (size_t i = 0; i < objects.size(); ++i) {
        AmberObject* obj = objects[i];
        if (obj) {
            if (!obj->marked) {
                delete obj;
                objects[i] = nullptr;
                free_slots.push_back(i);
            }
        }
    }
}