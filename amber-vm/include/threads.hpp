// amber-vm/include/threads.hpp
#ifndef THREADS_HPP
#define THREADS_HPP

#include <mutex>
#include <thread>
#include <vector>
#include <memory>
#include <string>
#include "value.hpp"

// Global interpreter lock. Exactly one thread executes VM code (bytecode or a
// native that touches VM state) at a time. Blocking natives temporarily drop
// it around the blocking syscall only — see GilRelease.
struct Gil {
    static std::mutex m;
    static thread_local bool held;
    static void acquire();
    static void release();
};

// Holds the GIL for a scope; safe if the lock was released and never
// reacquired on an exceptional path (unlocks only when still held).
struct GilHold {
    GilHold() { Gil::acquire(); }
    ~GilHold() { if (Gil::held) Gil::release(); }
    GilHold(const GilHold&) = delete;
    GilHold& operator=(const GilHold&) = delete;
};

// Drops the GIL for a scope (blocking syscalls); always reacquires,
// including on exception unwind. Between ctor and dtor, touch NO VM state.
struct GilRelease {
    GilRelease() { Gil::release(); }
    ~GilRelease() { Gil::acquire(); }
    GilRelease(const GilRelease&) = delete;
    GilRelease& operator=(const GilRelease&) = delete;
};

// Serializes console output across threads (OP_PRINT, printStr).
extern std::mutex g_out_mutex;

// One spawned thread. Reference-counted: the table only stores the shared
// pointer, so joining never races slot creation (vector reallocation moves
// only the pointer, never the slot).
struct ThreadSlot {
    std::thread th;
    std::mutex m;
    bool finished = false;
    bool joined = false;
    bool join_started = false;
    Value result;
    std::string error;
    int code = 0;
};

// Allocates a slot and returns (handle, slot). Handles are table index+1 and
// never reused, so a handle always names the same thread.
std::pair<int32_t, std::shared_ptr<ThreadSlot>> threads_alloc();
// Returns the slot for a handle, or nullptr for a bad handle.
std::shared_ptr<ThreadSlot> threads_get(int32_t handle);
// Joins every still-joinable thread. Runs after the main invocation ends,
// with the GIL already released (joining while holding it would deadlock).
void threads_join_all();

#endif // THREADS_HPP
