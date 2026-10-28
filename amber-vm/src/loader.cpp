#include "avm.hpp"
#include "resources.hpp"
#include "bytecode.hpp"
#include <fstream>
#include <iostream>
#include <cstring>
#include <vector>
#include <cstdint>
#include <sstream>
#include <iterator>
#include <algorithm>

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
    struct Export { std::string name; uint32_t addr; };
    struct Import { std::string name; uint32_t operand; };
    struct Module { std::string name; std::vector<uint8_t> code; std::vector<std::string> pool; std::vector<Export> exports; std::vector<Import> imports; size_t base = 0; };
    std::ifstream file(filename, std::ios::binary);
    if (!file) { std::cerr << "Error: Could not open archive " << filename << std::endl; return false; }
    std::vector<uint8_t> all((std::istreambuf_iterator<char>(file)), std::istreambuf_iterator<char>());
    Reader r(all); char magic[4];
    if (!r.read(magic, 4) || std::strncmp(magic, "AMRA", 4) != 0) { std::cerr << "Error: Invalid archive. Expected 'AMRA' header." << std::endl; return false; }
    uint16_t version = r.u16(); uint32_t entries = r.u32(); Resources::clear();
    std::vector<Module> modules; bool foundMain = false;
    for (uint32_t i=0; i<entries; ++i) {
        std::string name; uint32_t nameLen=0;
        if (!r.read(&nameLen,4) || !r.readString(name,nameLen)) { std::cerr << "Error: Truncated archive entry name." << std::endl; return false; }
        uint32_t len=0; if(!r.read(&len,4) || len > r.size-r.pos) { std::cerr << "Error: Truncated archive entry '"<<name<<"'."<<std::endl; return false; }
        bool isModule = name=="main" || (version>=2 && name.size()>=4 && name.compare(name.size()-4,4,".amc")==0);
        if (!isModule) { std::string content; if(!r.readString(content,len)) return false; Resources::loaded[name]=content; continue; }
        Reader sub(r.data+r.pos,len); Module m; m.name=name;
        if(!parse_amc(sub,m.code,m.pool)) return false;
        if(name=="main") foundMain=true;
        if(sub.pos+4<=sub.size && std::memcmp(sub.data+sub.pos,"AMLK",4)==0) {
            sub.pos+=4;
            auto read32=[&](uint32_t& v){ if(sub.pos+4>sub.size)return false; std::memcpy(&v,sub.data+sub.pos,4);sub.pos+=4;return true;};
            uint32_t count=0;
            if(!read32(count)||count>100000){std::cerr<<"Error: Invalid export table."<<std::endl;return false;}
            for(uint32_t j=0;j<count;++j){uint32_t n=0,a=0;std::string s;if(!read32(n)||!sub.readString(s,n)||!read32(a)){std::cerr<<"Error: Truncated export table."<<std::endl;return false;}m.exports.push_back({s,a});}
            if(!read32(count)||count>100000){std::cerr<<"Error: Invalid import table."<<std::endl;return false;}
            for(uint32_t j=0;j<count;++j){uint32_t n=0,o=0;std::string s;if(!read32(n)||!sub.readString(s,n)||!read32(o)){std::cerr<<"Error: Truncated import table."<<std::endl;return false;}m.imports.push_back({s,o});}
        } else if(version>=2) { std::cerr<<"Error: Module '"<<name<<"' has no link metadata."<<std::endl;return false; }
        modules.push_back(std::move(m)); r.pos+=len;
    }
    if(!foundMain || modules.empty()){std::cerr<<"Error: Archive has no 'main' entry."<<std::endl;return false;}
    if(version>=2){
        auto mainIt=std::find_if(modules.begin(),modules.end(),[](const Module& m){return m.name=="main";});
        if(mainIt==modules.end())return false;
        Module mainModule=std::move(*mainIt); modules.erase(mainIt);
        for(auto& m:modules){
            if(m.code.empty() || m.code.back()!=OP_HALT){std::cerr<<"Error: Module '"<<m.name<<"' has no terminal HALT for initialization chaining."<<std::endl;return false;}
            m.code.back()=OP_JUMP; m.code.insert(m.code.end(),4,0);
        }
        modules.push_back(std::move(mainModule));
    }
    std::map<std::string,uint32_t> symbols; size_t total=0;
    for(auto& m:modules){m.base=total; if(total+m.code.size()>static_cast<size_t>(INT32_MAX)){std::cerr<<"Error: Archive bytecode is too large."<<std::endl;return false;} total+=m.code.size(); for(auto& e:m.exports){if(e.addr>=m.code.size()){std::cerr<<"Error: Invalid exported address for '"<<e.name<<"'."<<std::endl;return false;}if(!symbols.emplace(e.name,static_cast<uint32_t>(m.base+e.addr)).second){std::cerr<<"Error: Duplicate exported function '"<<e.name<<"'."<<std::endl;return false;}}}
    auto get32=[](const std::vector<uint8_t>& c,size_t p){int32_t v=0;std::memcpy(&v,c.data()+p,4);return v;};
    auto put32=[](std::vector<uint8_t>& c,size_t p,int32_t v){std::memcpy(c.data()+p,&v,4);};
    bytecode.clear();constants.clear(); int32_t globalBase=0;
    for(auto& m:modules){
        int32_t cb=static_cast<int32_t>(constants.size()); constants.insert(constants.end(),m.pool.begin(),m.pool.end());
        int32_t moduleGlobalCount=0; size_t p=0;
        while(p<m.code.size()){
            uint8_t op=m.code[p++];
            auto operand=[&](int32_t add){if(p+4>m.code.size())return false;put32(m.code,p,get32(m.code,p)+add);p+=4;return true;};
            switch(op){
                case OP_STORE_GLOBAL: case OP_LOAD_GLOBAL: {
                    if(p+4>m.code.size())return false;
                    int32_t index=get32(m.code,p); if(index<0)return false;
                    moduleGlobalCount=std::max(moduleGlobalCount,index+1); put32(m.code,p,index+globalBase); p+=4; break;
                }
                case OP_JUMP: case OP_JUMP_IF_FALSE: case OP_PUSH: case OP_STORE_LOCAL: case OP_LOAD_LOCAL: case OP_PUSH_FLOAT: case OP_PUSH_CHAR: case OP_GET_FIELD: case OP_SET_FIELD:
                    if(p+4>m.code.size())return false;p+=4;break;
                case OP_LOAD_CONST: if(!operand(cb))return false;break;
                case OP_NEW_INSTANCE: if(p+8>m.code.size())return false;put32(m.code,p,get32(m.code,p)+cb);p+=8;break;
                case OP_CALL: case OP_SPAWN: case OP_TRY: if(!operand(static_cast<int32_t>(m.base)))return false;if(op!=OP_TRY){if(p>=m.code.size())return false;++p;}break;
                case OP_CALL_NATIVE: if(p+2>m.code.size())return false;p+=2;break;
                case OP_PUSH_BOOL: if(p>=m.code.size())return false;++p;break;
                case OP_HALT: case OP_NEW_ARRAY: case OP_STORE_ARRAY: case OP_LOAD_ARRAY: case OP_ADD: case OP_SUB: case OP_MUL: case OP_DIV: case OP_LESS: case OP_GREATER: case OP_EQUAL: case OP_NOT_EQUAL: case OP_LESS_EQUAL: case OP_GREATER_EQUAL: case OP_RETURN: case OP_NEW_LIST: case OP_LIST_ADD: case OP_LIST_GET: case OP_LIST_SET: case OP_LIST_SIZE: case OP_PUSH_NULL: case OP_CHECK_NULL: case OP_END_TRY: case OP_THROW: case OP_POP: case OP_PRINT: break;
                default: std::cerr<<"Error: Unknown opcode in module '"<<m.name<<"'."<<std::endl;return false;
            }
        }
        bytecode.insert(bytecode.end(),m.code.begin(),m.code.end());
        globalBase+=moduleGlobalCount;
    }
    for(const auto& m:modules) for(const auto& imp:m.imports){
        auto it=symbols.find(imp.name);if(it==symbols.end()){std::cerr<<"Error: Unresolved external function '"<<imp.name<<"' in module '"<<m.name<<"'."<<std::endl;return false;}
        size_t at=m.base+imp.operand;if(at+4>bytecode.size()){std::cerr<<"Error: Invalid relocation for '"<<imp.name<<"'."<<std::endl;return false;}put32(bytecode,at,static_cast<int32_t>(it->second));
    }
    return true;
}
} // namespace Loader
