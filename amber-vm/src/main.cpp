#include <iostream>
#include <vector>
#include "avm.hpp"

int main(int argc, char* argv[]) {
    if (argc < 2) {
        std::cerr << "Usage: avm <file.amc>" << std::endl;
        return 1;
    }

    std::vector<uint8_t> bytecode;
    std::vector<std::string> constants;

    // Amberlink Archives (.ama) bundle the compiled program plus resources.
    std::string arg = argv[1];
    bool isArchive = arg.size() >= 4 && arg.compare(arg.size() - 4, 4, ".ama") == 0;

    bool ok = isArchive
        ? Loader::loadArchive(argv[1], bytecode, constants)
        : Loader::load(argv[1], bytecode, constants);
    if (!ok) {
        return 1;
    }

    return execute(bytecode, constants);
}