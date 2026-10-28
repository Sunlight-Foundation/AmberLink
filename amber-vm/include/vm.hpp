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
// once threads exist (globals, heap, thread table in a later slice).
struct VMContext {
    const std::vector<uint8_t>* bytecode = nullptr;
    std::vector<std::string>* constants = nullptr;
    std::vector<Value> globals;
    Heap heap;
};

#endif // VM_HPP
