// amber-vm/include/natives.hpp
#ifndef NATIVES_HPP
#define NATIVES_HPP

#include <functional>
#include <vector>
#include <string>
#include <cstdint>
#include "value.hpp"
#include "heap.hpp"

// A native function pops its arguments from the VM stack, returns a Value.
// args: the arguments, in order, already popped from the stack.
// constants: the string constant pool (can append to create new strings).
// heap: the object heap (for allocating objects).
using NativeFn = std::function<Value(std::vector<Value>& args,
                                     std::vector<std::string>& constants,
                                     Heap& heap)>;

// A registered native: the function plus its fixed arity (arg count).
// The VM uses arity to know how many values to pop before calling.
struct NativeEntry {
    NativeFn fn;
    int arity;
};

namespace Natives {
    // Builds the native function registry. Called once during VM startup.
    // Returns a vector indexed by the 2-byte native ID emitted by OP_CALL_NATIVE.
    std::vector<NativeEntry>& registry();
}

#endif // NATIVES_HPP
