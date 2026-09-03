cmake_minimum_required(VERSION 3.10)
project(AmberVM)

set(CMAKE_CXX_STANDARD 17)

include_directories(include)

add_executable(avm
    src/avm.cpp
    src/heap.cpp
    src/loader.cpp
    src/main.cpp
    src/memory.cpp
    src/natives.cpp
    src/resources.cpp
    src/roots.cpp
)

# Sockets for the HTTP natives
if(WIN32)
    target_link_libraries(avm ws2_32)
endif()