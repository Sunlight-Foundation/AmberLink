// amber-vm/src/threads.cpp
#include "threads.hpp"

std::mutex Gil::m;
thread_local bool Gil::held = false;
std::mutex g_out_mutex;

void Gil::acquire() { m.lock(); held = true; }
void Gil::release() { m.unlock(); held = false; }

namespace {
    std::mutex g_table_mutex;
    std::vector<std::shared_ptr<ThreadSlot>> g_table;
}

std::pair<int32_t, std::shared_ptr<ThreadSlot>> threads_alloc() {
    auto slot = std::make_shared<ThreadSlot>();
    std::lock_guard<std::mutex> lk(g_table_mutex);
    g_table.push_back(slot);
    return { static_cast<int32_t>(g_table.size()), slot };
}

std::shared_ptr<ThreadSlot> threads_get(int32_t handle) {
    std::lock_guard<std::mutex> lk(g_table_mutex);
    if (handle <= 0 || (size_t)handle > g_table.size()) return nullptr;
    return g_table[(size_t)handle - 1];
}

void threads_join_all() {
    // Snapshot joinables without holding the table lock across joins:
    // joining runs user code duration, and spawns can't happen anymore
    // (the main invocation has ended), so the table is append-frozen.
    std::vector<std::shared_ptr<ThreadSlot>> snapshot;
    {
        std::lock_guard<std::mutex> lk(g_table_mutex);
        snapshot = g_table;
    }
    for (auto& slot : snapshot) {
        bool do_join = false;
        {
            std::lock_guard<std::mutex> lk(slot->m);
            if (!slot->joined && !slot->join_started && slot->th.joinable()) {
                slot->join_started = true;
                do_join = true;
            }
        }
        if (do_join) {
            slot->th.join();
            std::lock_guard<std::mutex> lk(slot->m);
            slot->joined = true;
        }
    }
}
