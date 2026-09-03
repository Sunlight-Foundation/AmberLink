// amber-vm/include/resources.hpp
#ifndef RESOURCES_HPP
#define RESOURCES_HPP

#include <map>
#include <string>

// Bundled resources loaded from a .ama archive. Populated by Loader::loadArchive
// before execute() runs, and read by the readResource / hasResource / resourceNames
// natives. Keeping these in a single global store means the NativeFn signature
// (args, constants, heap) does not need to change.
namespace Resources {
    // name -> content for every non-code entry in the archive.
    extern std::map<std::string, std::string> loaded;

    void clear();
    // All resource names concatenated with newlines (used by resourceNames()).
    std::string names();
    bool has(const std::string& name);
    const std::string* get(const std::string& name);
}

#endif // RESOURCES_HPP
