// amber-vm/src/resources.cpp
#include "resources.hpp"

namespace Resources {
    std::map<std::string, std::string> loaded;

    void clear() {
        loaded.clear();
    }

    std::string names() {
        std::string out;
        bool first = true;
        for (const auto& kv : loaded) {
            if (!first) out += '\n';
            out += kv.first;
            first = false;
        }
        return out;
    }

    bool has(const std::string& name) {
        return loaded.find(name) != loaded.end();
    }

    const std::string* get(const std::string& name) {
        auto it = loaded.find(name);
        return it == loaded.end() ? nullptr : &it->second;
    }
}
