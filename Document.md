# 📄 Amberlink: Technical Specification (v0.6)

Amberlink is a high-performance, multi-paradigm programming language designed to bridge the gap between the safety of Java and the raw power of C++. It utilizes a unique Dual-Backend approach, allowing code to run either on a dedicated Virtual Machine (AVM) or as a native binary.

## 1. System Architecture

Amberlink is split into two primary components to ensure memory safety and execution speed:

*   **The Brain (Amber-Core):** Built in Rust. It handles the frontend tasks: Lexing, Parsing, Semantic Analysis, and Bytecode Generation. Using Rust ensures the compiler is immune to memory-related crashes.
*   **The Body (Amber-VM / AVM):** Built in C++. A high-performance, stack-based virtual machine featuring a custom Mark-and-Sweep Garbage Collector and a lean object header system.

## 2. The Compilation Pipeline

Amberlink uses a Two-Pass Compilation strategy to solve the "Forward Declaration" problem found in older languages like C++.

1.  **Pass 1 (Discovery):** The compiler scans the `.amb` source file to identify all function signatures, class definitions, and global variables. It populates the Symbol Table.
2.  **Pass 2 (Validation & Emission):** The compiler verifies the logic and types. If successful, the Emitter generates a highly optimized binary file with the `.amc` (Amber Compiled) extension.

## 3. Language Design Philosophy

Amberlink is designed to be cleaner and more stable than Java, offering a familiar environment for existing developers while removing boilerplate.

*   **Familiar Syntax:** Uses C-style static typing and function definitions (`int add(int a, int b)`) to feel intuitive for developers coming from Java, C++, or C#.
*   **Script-like Simplicity:** No mandatory classes or `public static void main` boilerplate. Code executes from top to bottom, making it easy to write simple scripts and test ideas quickly.
*   **Newline-Based:** No semicolons required. The parser uses significant newlines to delimit statements, leading to cleaner code.
*   **Multi-Paradigm:** Supports both Object-Oriented Programming (OOP) via `class` and Functional Programming via standalone functions.

### Language Features Overview

#### Variables & Types
Amberlink is statically typed. Variables must be declared with a type (`int`, `String`, `void`).
```java
int count = 10
String message = "Hello World"
```

**Arrays**
Arrays are heap-allocated and garbage collected.
```java
int[] numbers = new int[5]
numbers[0] = 100
print numbers[0]
```

**Control Flow**
Standard `if/else` and `while` loops are supported, but parentheses are optional.
```java
if count < 20 {
    print "Small"
}

while count > 0 {
    count = count - 1
}
```

**Native Functions (Standard Library)**
Built-in functions (string manipulation, math, file I/O, time, and process control) are implemented directly in the AVM and callable from any source file without an import. The compiler emits an `OP_CALL_NATIVE` (`0x32`) instruction followed by a 2-byte native ID; the VM looks the function up in its registry (`Natives::registry()` in `amber-vm/src/natives.cpp`) and invokes it. The native IDs registered in the Rust compiler (`init_native_registry` in `semant.rs`) must exactly match the registry order in `natives.cpp`. See the [Language Guide](LanguageGuide.md#8-standard-library) for the full list.

4. The AVM Bytecode Format (.amc)
The `.amc` binary format is designed to be compact and fast to load. It consists of a simple header followed by the raw bytecode instructions.

| Offset | Size (bytes) | Description                               |
|:-------|:-------------|:------------------------------------------|
| 0      | 4            | **Magic Number:** `AMBR` (0x41, 0x4D, 0x42, 0x52) |
| 4      | 2            | **Version:** A `u16` for the bytecode version.    |
| 6      | 4            | **Entry Point:** A `u32` offset to the `main` function (future use). |
| 10     | 4            | **Pool Count:** A `u32` count of strings in the constant pool. |
| 14     | Variable     | **Constant Pool:** Sequence of [Len(u32) + Bytes] for each string. |
| ...    | 4            | **Code Length:** A `u32` indicating the size of the code section. |
| ...    | N            | **Code Section:** The raw bytecode instructions.  |

5. Memory Management (The GC)
Unlike the heavy, unpredictable JVM Garbage Collector, the Amber-VM uses a lean and efficient Mark-and-Sweep collector.
*   **Object Header:** A minimal header per object stores metadata required by the GC, such as the "marked" flag.
*   **GC Strategy:** A classic Mark-and-Sweep algorithm. The "Mark" phase traverses all reachable objects from a set of roots (e.g., the stack, global variables), and the "Sweep" phase frees all unmarked objects.
*   **Future Work:** The design allows for future enhancements like a generational collector and manual GC hinting for performance-critical sections of code.


6. Project Structure
Amberlink/
├── amber-core/    # Rust: Lexer, Parser, Emitter
├── amber-vm/      # C++: Interpreter, GC, Loader
├── amber-native/  # 
├── bin/           # Final tool binaries (ambc, avm)
├── stdlib/        # Standard Amberlink libraries
└── scripts/       # Python build automation


7. Build and Run
Amberlink uses `make` as a unified interface for building the toolchain and compiling user code.

1. **Prerequisites:** Ensure Rust (Cargo), C++ (CMake), and `make` are installed.

2. **Initialize the Toolchain:**
   This compiles `amber-core` (Rust) and `amber-vm` (C++) and places binaries in `bin/`.
   ```bash
   make init
   ```

3. **Build Code:**
   Compiles an `.amb` file to `.amc` bytecode using the built compiler.
   ```bash
   make build file=main.amb
   ```

4. **Run Bytecode:**
   Execute the compiled file using the VM.
   ```bash
   make run file=main.amb
   ```
