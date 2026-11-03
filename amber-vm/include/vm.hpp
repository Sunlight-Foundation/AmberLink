// amber-vm/include/vm.hpp
#ifndef VM_HPP
#define VM_HPP

#include <vector>
#include <string>
#include <cstdint>
#include "value.hpp"
#include "heap.hpp"

// State shared by every thread running the same program. The interpreter
// stacks (value/call/fp) stay per-thread inside run_loop(); everything here
// is either read-only after load (bytecode, constants) or guarded by the GIL
// once threads exist (globals, heap, and GC root registry).
struct VMContext {
    const std::vector<uint8_t>* bytecode = nullptr;
    std::vector<std::string>* constants = nullptr;
    std::vector<Value> globals;
    // All active interpreter stacks are GC roots; modified only while holding the GIL.
    std::vector<std::vector<Value>*> active_stacks;
    // Spawn arguments remain roots until the worker stack is registered.
    std::vector<Value> pending_thread_roots;
    // Completed worker results remain roots while their handles can be re-joined.
    std::vector<Value> thread_roots;
    Heap heap;
};

#endif // VM_HPP
