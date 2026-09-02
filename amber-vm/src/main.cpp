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
    if (!Loader::load(argv[1], bytecode, constants)) {
        return 1;
    }

    return execute(bytecode, constants);
}