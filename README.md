# Amberlink

The alternative to Java Programming

Amberlink is a high-performance, multi-paradigm programming language designed to bridge the gap between the safety of Java and the raw power of C++. It utilizes a unique Dual-Backend approach, allowing code to run either on a dedicated Virtual Machine (AVM) or as a native binary.

## Features

*   **Familiar Syntax:** Uses C-style static typing and function definitions to feel intuitive for developers coming from Java, C++, or C#.
*   **Script-like Simplicity:** No mandatory classes or `public static void main` boilerplate. Code executes from top to bottom.
*   **Memory Management:** Features a lean Mark-and-Sweep Garbage Collector.
*   **Dual-Backend:** Runs on the Amber Virtual Machine (AVM) for safety or compiles to native binaries for speed (planned).

###️ Build and Run

Amberlink currently uses a Python script as a unified interface for building the toolchain and compiling user code.

### Prerequisites
Ensure you have the following installed:
*   Rust (Cargo)
*   C++ Compiler (CMake)
*   Python 3

### Quick Start

1.  **Initialize the Toolchain:**
    Compiles `amber-core` (Rust) and `amber-vm` (C++) and places binaries in `bin/`.
    ```bash
    python scripts/Amberlink.py init
    ```

2.  **Build Code:**
    Compiles an `.amb` file to `.amc` bytecode.
    ```bash
    python scripts/Amberlink.py build main.amb
    ```

3.  **Run Bytecode:**
    Execute the compiled file using the VM.
    ```bash
    ./bin/avm output.amc
    ```

## Roadmap

Check out roadmap.md for the detailed development plan.