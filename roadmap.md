# Amberlink Development Roadmap

The development of Amberlink is planned in distinct phases, each building upon the last to create a robust and feature-rich language.

## Phase 1: Core Language Features (The "Usable" Milestone)
This phase focuses on implementing the fundamental building blocks required to write simple, yet complete, programs.

- [x] **Control Flow:** Implement `if/else` statements, `while` loops, and `for` loops. This requires adding jump opcodes (`JUMP`, `JUMP_IF_FALSE`) to the VM and the corresponding logic in the compiler's Emitter.
- [x] **Full Function Support:** Parse function bodies (`{...}`), parameters, and `return` statements. Implement a proper call stack in the VM with `CALL` and `RETURN` opcodes.
- [x] **Scoping:** Differentiate between global and local (stack-allocated) variables to enable proper encapsulation and recursion.

## Phase 2: Data Structures & Memory (The "Robust" Milestone)
This phase moves beyond simple numbers and introduces the ability to manage more complex data.

- [x] **String & Constant Pool:** Implement heap-allocated strings and a "constant pool" in the bytecode to efficiently store and reuse literals.
- [x] **GC Root Scanning:** Fully implement the Garbage Collector by teaching it to scan the VM's stack and global variables for "roots" to determine which objects are still in use.
- [x] **Arrays:** Introduce a built-in array/list type as the first user-creatable, heap-allocated collection.
- [x] **Basic Types:** Expand fundamental types to include `float`/`double`, `boolean`/`bool`, `char`.
- [x] **Collections:** Introduce built-in collection types (List, Map, Set) in the standard library.

## Phase 3: Object-Oriented Programming (The "Modern" Milestone)
This phase brings Amberlink closer to its goal of being a modern, Java-like language.

- [x] **Classes & Instances:** Implement `class` definitions, fields, and object instantiation (`new MyClass()`).
- [x] **Methods & `this`:** Allow methods to be defined within classes and implement the `this` keyword to refer to the current instance.
- [ ] **Constructors:** Implement explicit constructors for classes to allow controlled object initialization. (Target: v0.6.0)
- [ ] **Access Modifiers:** Add `public`, `private`, `protected` access modifiers for fields and methods to enforce encapsulation. (Target: v0.6.0)
- [ ] **Method Overloading/Overriding:** Support method overloading (multiple methods with the same name but different parameters) and overriding (reimplementing a superclass method). (Target: v0.6.0)
- [ ] **Static Members:** Support `static` fields and methods for class-level functionality. (Target: v0.6.0)
- [ ] **Interfaces:** Implement interfaces for abstract type definitions and polymorphism. (Target: v0.6.0)
- [ ] **Inheritance:** Implement single inheritance for classes, allowing for code reuse and polymorphism. (Target: v0.6.0)

## Phase 4: Ecosystem & Tooling (The "Mature" Milestone)
This phase focuses on building the tools and libraries that make a language productive and enjoyable to use.

- [ ] **Standard Library:** Expand foundational `stdlib` with modules for string manipulation, math operations, comprehensive file I/O, advanced data structures (e.g., HashMap, LinkedList), system interaction (environment variables, process control), and basic networking capabilities. (Target: v0.7.0)
- [ ] **Amberlink Archive Format:** Develop a custom container format (e.g., `.ama`) to package compiled bytecode and resources, mirroring the functionality of Java JARs. (Target: v0.7.0)
- [ ] **Module System:** Implement `import` statements and file linking to organize code across multiple files and support the package manager. (Target: v0.7.0)
- [ ] **Developer Tooling:**
    - [ ] **CLI Tool (`Amberlink.py`):** Enhance `scripts/Amberlink.py` with a unified `run` command (compile and execute), improved error handling, and cleaner module imports. Improve project initialization options. (Target: v0.7.0)
    - [ ] **Workspace Management:** Ensure optimal configuration for `Cargo.toml` and `CMakeLists.txt`; explore tighter integration between Rust and C++ builds. (Target: v0.7.0)
    - [ ] **Compiler Improvements:** Improve error reporting (descriptive messages, line numbers), implement compiler optimizations, and enhance type checking in `semant.rs`. Consider a more sophisticated Intermediate Representation (IR). (Target: v0.7.0)
    - [ ] **VM Improvements:** Implement debugging support (line number info, simple debugger interface). Explore advanced GC algorithms. (Target: v0.7.0)
    - [ ] **Language Server (LSP):** Provide IDE support for features like autocompletion and error highlighting. (Target: v0.7.0)
    - [ ] **Debugger:** Create a tool to step through Amberlink code, inspect variables, and analyze the stack. (Target: v0.7.0)
    - [ ] **Package Manager:** Develop a system for sharing and managing third-party Amberlink libraries. (Target: v0.7.0)

## Phase 5: Performance & Interoperability (The "Power" Milestone)
This phase unlocks the raw performance and flexibility promised by the "Dual-Backend" architecture.

- [ ] **Concurrency:** Introduce built-in concurrency primitives (e.g., lightweight threads, async/await, actors) to support modern application development and distinguish from traditional threading models. (Target: v0.8.0)
- [ ] **Native Compilation:** Implement the second backend to compile Amberlink source code directly to native machine code (via LLVM or C transpilation). (Target: v0.8.0)
- [ ] **Foreign Function Interface (FFI):** Enable Amberlink code to call C functions, allowing access to existing system libraries. (Target: v0.8.0)
- [ ] **JIT Compilation:** Implement a Just-In-Time compiler within the AVM to compile hot bytecode paths to machine code at runtime. (Target: v0.8.0)

## Phase 6: Advanced Language Features (The "Expressive" Milestone)
This phase introduces sophisticated features for complex application development.

- [ ] **Null Safety:** Implement null safety features (e.g., optional types, non-nullable by default) to prevent common runtime errors and improve code robustness. (Target: v0.9.0)
- [ ] **Generics:** Implement type parameters (e.g., `List<T>`) to allow for type-safe, reusable data structures. (Target: v0.9.0)
- [ ] **Exception Handling:** Introduce `try`, `catch`, and `throw` keywords for robust error management. (Target: v0.9.0)
- [ ] **Pattern Matching:** Add support for advanced control flow structures like `match` or `switch` expressions. (Target: v0.9.0)

## Phase 7: JVM Integration (The "Universal" Milestone)
This phase expands Amberlink's reach by integrating with the vast Java ecosystem.

- [ ] **Java Interop:** Enable seamless interoperability, allowing Amberlink projects to import Java classes and libraries, and vice versa. (Target: v1.0.0)
- [ ] **Hybrid Project Support:** Enable the compiler to handle mixed source directories of Amberlink and Java files, allowing direct usage of Java code within Amberlink projects. (Target: v1.0.0)

## Phase 8: Documentation & Community (The "Growth" Milestone)
This phase focuses on fostering growth, adoption, and a thriving ecosystem around Amberlink.

- [ ] **Comprehensive Language Guide:** Expand documentation with detailed explanations of language features, syntax, and semantics, including more complex examples. (Target: v1.0.0)
- [ ] **API Documentation:** Generate API documentation for the standard library and built-in features. (Target: v1.0.0)
- [ ] **Updated `README.md`:** Ensure `README.md` provides clear instructions for getting started, building, and running Amberlink programs. (Target: v1.0.0)
- [ ] **Contributing Guide:** Provide clear guidelines for community contributions (code, documentation, examples). (Target: v1.0.0)
- [ ] **Cookbook/Tutorials:** Create short, focused tutorials demonstrating how to achieve common tasks. (Target: v1.0.0)
- [ ] **Community Channels:** Establish and promote community channels (e.g., forum, chat, mailing list) for discussion and support. (Target: v1.0.0)