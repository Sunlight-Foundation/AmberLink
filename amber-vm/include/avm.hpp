#ifndef AVM_HPP
#define AVM_HPP

#include <vector>
#include <cstdint>
#include <string>

// Forward-declare the loader function used in main.cpp
namespace Loader {
    bool load(const char* filename, std::vector<uint8_t>& bytecode, std::vector<std::string>& constants);

    // Loads an Amberlink Archive (.ama): extracts the "main" .amc entry into
    // bytecode/constants and stores every other entry in the Resources store.
    bool loadArchive(const char* filename, std::vector<uint8_t>& bytecode, std::vector<std::string>& constants);
}

// Runs the bytecode. Returns 0 on success, non-zero if a runtime error occurred.
// Runtime errors are caught internally and reported to stderr.
int execute(const std::vector<uint8_t>& bytecode, std::vector<std::string>& constants);

#endif // AVM_HPP