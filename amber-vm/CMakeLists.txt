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
    src/roots.cpp
)