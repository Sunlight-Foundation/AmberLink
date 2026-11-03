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
    src/threads.cpp
)

option(AVM_PROFILE "Count dynamically executed AVM opcodes (profiling builds only)" OFF)
if(AVM_PROFILE)
    target_compile_definitions(avm PRIVATE AVM_PROFILE)
endif()

option(AVM_GC_PROFILE "Collect AVM heap allocation and garbage-collection metrics" OFF)
if(AVM_GC_PROFILE)
    target_compile_definitions(avm PRIVATE AVM_GC_PROFILE)
endif()

option(AVM_FORCE_SWITCH "Force portable switch dispatch (testing)" OFF)
if(AVM_FORCE_SWITCH)
    target_compile_definitions(avm PRIVATE AVM_FORCE_SWITCH)
endif()

# Sockets for the HTTP natives
if(WIN32)
    target_link_libraries(avm ws2_32)
endif()