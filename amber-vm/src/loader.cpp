#include "avm.hpp"
#include "resources.hpp"
#include <fstream>
#include <iostream>
#include <cstring>
#include <vector>
#include <cstdint>
#include <sstream>
#include <iterator>

namespace {

// A tiny cursor-based reader over an in-memory byte buffer.
struct Reader {
    const uint8_t* data;
    size_t size;
    size_t pos = 0;

    Reader(const std::vector<uint8_t>& buf) : data(buf.data()), size(buf.size()) {}

    Reader(const uint8_t* data, size_t size) : data(data), size(size) {}

    bool read(void* dst, size_t n) {
        if (pos + n > size) return false;
        std::memcpy(dst, data + pos, n);
        pos += n;
        return true;
    }
    // Bounds-checked: never allocates more than the remaining input, so a
    // corrupt length prefix fails fast instead of attempting a giant allocation.
    bool readString(std::string& out, uint32_t len) {
        if (static_cast<size_t>(len) > size - pos) return false;
        out.resize(len);
        return read(&out[0], len);
    }
    uint32_t u32() {
        uint32_t v = 0;
        read(&v, 4);
        return v;
    }
    uint16_t u16() {
        uint16_t v = 0;
        read(&v, 2);
        return v;
    }
};

// Parses an AMBR (.amc) payload from an in-memory buffer.
bool parse_amc(Reader& r, std::vector<uint8_t>& bytecode, std::vector<std::string>& constants) {
    char magic[4];
    if (!r.read(magic, 4) || std::strncmp(magic, "AMBR", 4) != 0) {
        std::cerr << "Error: Invalid AMBR header." << std::endl;
        return false;
    }
    r.u16(); // version
    r.u32(); // entry point placeholder

    uint32_t poolCount = r.u32();
    for (uint32_t i = 0; i < poolCount; ++i) {
        std::string s;
        if (!r.readString(s, r.u32())) {
            std::cerr << "Error: Truncated constant pool." << std::endl;
            return false;
        }
        constants.push_back(s);
    }

    uint32_t codeLength = r.u32();
    if (codeLength > 0) {
        if (static_cast<size_t>(codeLength) > r.size - r.pos) {
            std::cerr << "Error: Truncated bytecode." << std::endl;
            return false;
        }
        bytecode.resize(codeLength);
        if (!r.read(bytecode.data(), codeLength)) {
            std::cerr << "Error: Unexpected end of input while reading bytecode." << std::endl;
            return false;
        }
    }
    return true;
}

} // namespace

namespace Loader {

    bool load(const char* filename, std::vector<uint8_t>& bytecode, std::vector<std::string>& constants) {
        std::ifstream file(filename, std::ios::binary);
        if (!file) {
            std::cerr << "Error: Could not open file " << filename << std::endl;
            return false;
        }
        std::vector<uint8_t> all((std::istreambuf_iterator<char>(file)), std::istreambuf_iterator<char>());
        Reader r(all);
        return parse_amc(r, bytecode, constants);
    }

    bool loadArchive(const char* filename, std::vector<uint8_t>& bytecode, std::vector<std::string>& constants) {
        std::ifstream file(filename, std::ios::binary);
        if (!file) {
            std::cerr << "Error: Could not open archive " << filename << std::endl;
            return false;
        }
        std::vector<uint8_t> all((std::istreambuf_iterator<char>(file)), std::istreambuf_iterator<char>());
        Reader r(all);

        char magic[4];
        if (!r.read(magic, 4) || std::strncmp(magic, "AMRA", 4) != 0) {
            std::cerr << "Error: Invalid archive. Expected 'AMRA' header." << std::endl;
            return false;
        }
        r.u16(); // archive version

        Resources::clear();
        uint32_t entryCount = r.u32();
        bool foundMain = false;

        for (uint32_t i = 0; i < entryCount; ++i) {
            std::string name;
            if (!r.readString(name, r.u32())) {
                std::cerr << "Error: Truncated archive entry." << std::endl;
                return false;
            }
            uint32_t dataLen = r.u32();

            if (name == "main") {
                // The main entry is an embedded .amc program. Parse it bounded
                // to exactly its entry bytes so a lying length can't over-read
                // into neighboring entries.
                if (static_cast<size_t>(dataLen) > r.size - r.pos) {
                    std::cerr << "Error: Truncated 'main' entry in archive." << std::endl;
                    return false;
                }
                Reader sub(r.data + r.pos, dataLen);
                if (!parse_amc(sub, bytecode, constants)) {
                    std::cerr << "Error: Failed to parse 'main' entry in archive." << std::endl;
                    return false;
                }
                foundMain = true;
                r.pos += dataLen;
            } else {
                std::string content;
                if (!r.readString(content, dataLen)) {
                    std::cerr << "Error: Truncated archive entry." << std::endl;
                    return false;
                }
                Resources::loaded[name] = content;
            }
        }

        if (!foundMain) {
            std::cerr << "Error: Archive has no 'main' entry." << std::endl;
            return false;
        }
        return true;
    }

} // namespace Loader
