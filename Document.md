# 📄 Amberlink: Technical Specification (v0.7)

Amberlink is a high-performance, multi-paradigm programming language designed to bridge the gap between the safety of Java and the raw power of C++. It utilizes a unique Dual-Backend approach, allowing code to run either on a dedicated Virtual Machine (AVM) or as a native binary.

## 1. System Architecture

Amberlink is split into two primary components to ensure memory safety and execution speed:

*   **The Brain (Amber-Core):** Built in Rust. It handles the frontend tasks: Lexing, Parsing, Semantic Analysis, and Bytecode Generation. Using Rust ensures the compiler is immune to memory-related crashes.
*   **The Body (Amber-VM / AVM):** Built in C++. A high-performance, stack-based virtual machine featuring a custom Mark-and-Sweep Garbage Collector and a lean object header system.

## 2. The Compilation Pipeline

Amberlink uses a Two-Pass Compilation strategy to solve the "Forward Declaration" problem found in older languages like C++.

1.  **Pass 1 (Discovery):** The compiler scans the `.amb` source file to identify all function signatures, class definitions, and global variables. It populates the Symbol Table.
2.  **Pass 2 (Validation & Emission):** The compiler verifies the logic and types. If successful, the Emitter generates a highly optimized binary file with the `.amc` (Amber Compiled) extension.

Between the passes, a local **constant-folding** rewrite (`amber-core/src/optimizer.rs`, on by
default, `--no-opt` to disable) evaluates literal-only expressions: int/float/char
arithmetic and comparisons, bool and string equality, and string `+` (which also
shrinks the constant pool). Anything else — including division by zero — is left
unfolded so runtime errors behave identically with the pass on or off.

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

## 4. Backend IR

The compiler lowers AST to bytes in one step today. The backend IR (`amber-core/src/codegen/ir.rs`)
models that same program as structured data — one `IrInstr` per operation with decoded
operands — and is the interface future backends (LLVM, AOT) consume instead of raw bytes.
The bytecode backend is its first consumer.

- **Coverage:** every opcode the emitter produces (`bytecode.rs`), with constant-pool-aware
  pretty-printing (`LoadConst` shows the pooled string, `NewInstance` the class name).
- **Jumps resolved:** targets print as absolute byte offsets (`jump @42`); on the wire they stay
  relative-to-operand-end as the VM executes them (`ip += 4; ip += offset` in `avm.cpp`).
- **Round-trip guarantee:** `encode(decode(bytes)) == bytes`. `ambc --emit-ir` dumps the IR
  listing and asserts the round-trip, so decoder drift fails the build loudly.
- **Design rule for new opcodes:** any new opcode must add its operand layout to `ir.rs`
  (`decode` + `encode` + `format_instr`), or `--emit-ir` rejects the program.

## 5. Concurrency Design (implemented as specified, with notes below)

**Decision: OS threads + a global interpreter lock (GIL).** One thread executes
bytecode at a time; threads run concurrently through blocking operations. This gives
real concurrent I/O — the useful kind for this language's stdlib (HTTP, files,
`sleep`, `input`) — while the existing single-threaded mark-and-sweep collector
stays correct unchanged. True parallel execution is a post-1.0 project (fine-grained
locking or GC rework), not this design. Rejected alternatives: no-GIL threads
(massive GC/memory-model project), async/await (needs compiler coroutines plus a
non-blocking rewrite of every native; un-Java-like), actors (alien paradigm for a
Java alternative).

### Proposed syntax (v1)

```java
var h1 = spawn fetch(url1)   // evaluates args now, runs fetch() on a new thread
var h2 = spawn fetch(url2)   // spawn is an expression; result is a thread handle
print join(h1)               // blocks until h1 finishes, yields its return value
print join(h2)
```

- The spawned function must be statically known (same rule as calls); communication
  is via shared globals in v1. Data races are the programmer's responsibility,
  as in Java without synchronization.
- Later sugar (v2): a `Thread` class with `start()`/`join()` and mutex natives.
  No channels or actors in v1.

### GIL mechanics

- One process-wide `std::mutex` guards all VM state. `execute()` holds it while
  interpreting; each thread runs its own invocation with its own value/call/fp
  stacks (already function-locals in `avm.cpp`, so this falls out naturally).
- Blocking natives release the GIL around the blocking syscall only and reacquire
  before touching VM state again:

  | Native | Releases GIL? | Reason |
  |--------|---------------|--------|
  | `sleep`, `input`, `httpGet`, `httpPost` | yes | network/console/timer waits |
  | FFI `callInt`, `callStr` | yes | C code may block; v1 FFI cannot call back |
  | `readFile`, `writeFile` | no | fast local I/O; not worth the handoff |
  | everything else | no | non-blocking |

### Required refactor (mechanical, no semantics change)

`execute()` currently owns the shared state (`Heap gc`, `globals`, `constants`,
`bytecode`) as locals alongside the per-thread stacks. Split it into a shared
`VM` context `{bytecode, constants, globals, heap, gil}` plus a per-thread
interpreter invocation. Spawning a thread = new `std::thread` running the same
interpreter loop over the shared context with fresh stacks.

### GC and shared-state rules

- The collector runs only with the GIL held (it already runs inside `execute()` and
  inside natives, both of which hold it except across released blocking calls, where
  they touch no VM state). No collector changes in v1.
- `Resources::loaded` and the FFI handle table are written only under the GIL
  (`loadArchive` at startup; `loadLib`/`freeLib` are short non-releasing natives).
- Heaps, globals, and the constant pool are shared across threads; per-thread
  stacks are private.

### Memory model (v1)

Sequentially consistent as observed: only one thread mutates VM state at a time.
`join(h)` is a happens-before edge (everything h did is visible after it returns).
Concurrent mutation of shared globals without `join` ordering is a data race with
undefined visible order — no atomics in v1.

### Bytecode sketch (additive, no format break)

`spawn` compiles like `Call`: `OP_SPAWN` + i32 absolute function address + u8 argc,
with the address patched by the existing `finalize()` pass. The VM allocates a
handle, starts the thread at the address with the popped args, and pushes the
handle. `join` is a blocking native on a handle table. Old programs never contain
the new opcode; old VMs reject new programs with the existing unknown-opcode error.

### Error semantics (proposed)

- A thread that hits a runtime error records it on its handle and terminates.
- `join` on a failed thread re-raises the error in the joining thread.
- Joining an invalid/finished handle is a runtime error. `exit()` terminates the
  whole process immediately, as today.

### Open questions (to settle before implementation)

1. `spawn` exact grammar: bare `spawn f(x)` vs `spawn(f, x)` form.
2. Cap on live threads; behavior when exceeded (error vs block).
3. Unhandled error with no joiner: process abort vs silent record.
4. Whether `print` from multiple threads needs an output lock (yes, almost surely).

### Implementation notes (decisions taken while building)

1. Grammar is bare `spawn f(x)` (new `spawn` keyword; `join` is native ID 44).
2. No thread cap in v1; the OS is the limit.
3. Errors are recorded on the handle; `join` re-raises, re-join returns the
   cached outcome, and unjoined threads are joined at program end (never silently
   dropped — plus every thread object must be joined before its slot dies).
4. `print`/`printStr` share one output mutex.
5. One real bug class found in testing: a finished-but-unjoined thread is still
   joinable, so the fast path must reap it too — or process exit aborts.

## 6. FFI Array/Buffer Marshaling Design (implemented as specified, with notes below)

Goal: let numeric kernels live in C (the NumPy arrangement - slow glue language,
fast kernels) by passing Amberlink arrays to C functions. Follows the v1 FFI
(`loadLib`/`freeLib`/`callInt`/`callStr`); same fail-soft-where-sensible contract.

### Proposed API

```java
var a = new int[4]
a[0] = 3
a[1] = 1
a[2] = 2
a[3] = 0
int lib = loadLib("msvcrt.dll")
if lib == 0 {
    lib = loadLib("libc.so.6")
}
print callInts(lib, "sum4", a)     // int f(int* data, int len), read-only
print callIntsMut(lib, "sort4", a) // same shape, writes the array back
print a[0]                         // sorted value visible
```

- `callInts(handle, symbol, arr)` calls `int f(int* data, int len)` with `len`
  derived from the array's actual length (never caller-supplied, so the classic
  length-mismatch bug class cannot occur). Read-only: no copy-back.
- `callIntsMut(handle, symbol, arr)` is the same call, then writes the buffer
  back into the array as Integers. Array length is immutable from C.
- Only `Array` (from `new int[n]`) is marshalled in v1, not `List`.
  `callStr` already covers read-only text bytes; binary data goes via int arrays.

### Marshaling rules

- Every element must be `INT`, else a runtime error (no silent coercion).
- Flatten into a thread-local `std::vector<int32_t>`; the GIL is released around
  the C call exactly like `callInt`/`callStr`, and the local buffer makes that
  safe (no aliasing into the shared constant pool or heap, even if another
  thread appends mid-call). Copy-back happens after reacquire.
- Type mapping assumes 32-bit `int` (true on all target platforms, Windows
  LLP64 included); assert it with `static_assert(sizeof(int) == 4)`.
- Bad handle / missing symbol / non-array argument: runtime errors, mirroring
  the existing FFI natives. A faulting C function is the C code's own bug -
  the buffer handed over is always valid.

### Non-goals for v1

Callbacks from C into Amberlink, structs, floats/doubles, 64-bit ints, returned
pointers, `List` marshaling, resizing arrays from C.

### Test strategy (required of the implementation PR)

System libc has no portable `int f(int*, int)` export, so unlike the v1 FFI
(which tested against `atoi`/`strlen`) this needs a ~10-line C fixture compiled
at test time where a C toolchain exists, skipped with a note where none does -
the same pattern as the Python echo server for networking tests.

### Open questions

1. Naming: `callInts`/`callIntsMut` vs a single `callBuf` that always copies back
   (simpler surface, pays an O(n) copy for read-only kernels).
2. Whether `char`/byte arrays deserve their own entry point or wait for a
   proper bytes type.

### Implementation notes (decisions taken while building)

1. Kept the two-function split: read-only kernels skip the copy-back entirely.
2. `char` arrays wait for a bytes type (int arrays only in v1).
3. Length is derived from the array, so there is no length argument to get wrong;
   empty arrays pass a possibly-null pointer with length 0, which well-behaved
   C functions handle like any zero-length call.