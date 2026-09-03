# Amberlink Development Roadmap

Amberlink is a statically-typed, Java-alternative language built at Sunlight Foundation / Parafield Studios.
Development is split into phases, each building toward a robust, modern, high-performance language and runtime.

---

## Phase 1 — Core Language (v0.1–0.3) ✅
*The "Usable" milestone — write complete programs.*

- [x] Control flow: `if/else`, `while`, `for`
- [x] Functions: parameters, `return`, call stack (`CALL` / `RETURN` opcodes)
- [x] Scoping: global vs local variables, frame pointer

---

## Phase 2 — Data & Memory (v0.4) ✅
*The "Robust" milestone — manage complex data safely.*

- [x] String constant pool
- [x] Garbage collector: mark-and-sweep with root scanning
- [x] Arrays
- [x] Basic types: `float`, `bool`, `char`
- [x] Collections: built-in `List`

---

## Phase 3 — Object-Oriented Programming (v0.6) ✅
*The "Modern" milestone — full OOP support.*

- [x] Classes, fields, instantiation (`new MyClass()`)
- [x] Methods and `this`
- [x] Constructors (`init`)
- [x] Access modifiers (`public`, `private`, `protected`)
- [x] Static fields and methods
- [x] Inheritance (`extends`)
- [x] Interfaces (`implements`)
- [x] Method overloading

---

## Phase 4 — Ecosystem & Tooling (v0.7) ✅
*The "Mature" milestone — productive developer experience.*

- [x] **Standard Library** — native string manipulation, math, file I/O, time, process, `HashMap`/`LinkedList`, resource, and networking functions shipped (`stdlib/core.amb`, `stdlib/io.amb`, `stdlib/collections.amb`, `stdlib/net.amb`)
- [x] **Module system** — `import` statements, multi-file projects
- [x] **Amberlink Archive (`.ama`)** — packaged bytecode + resources, like Java JARs
- [x] **CLI (`make`)** — `new`/`test`/`watch` targets, arg validation with usage errors, `.ama`-aware `run`
- [x] **Compiler improvements** — descriptive error messages with `line:col` and error recovery in the parser/emitter (`error.rs`, `synchronize()`), static type checking in `semant.rs`, backend IR data model with `ambc --emit-ir` dump and decode/re-encode round-trip (`codegen/ir.rs`)

---

## Phase 5 — Performance (v0.8)
*The "Power" milestone — close the gap with the JVM, then surpass it.*

- [x] **Threaded dispatch (computed gotos)** — replace the `switch` opcode loop with a jump table to eliminate branch misprediction; one of the highest-impact pure interpreter optimizations
- [x] **GC collect threshold** — `collect()` was a full mark-and-sweep on every string concat; now gated on allocations-since-GC with an adaptive 2x-live threshold
- [ ] **NaN-boxing** — pack `Value` into a single 64-bit double using NaN bits for type tags; cuts memory bandwidth and struct size significantly
- [ ] **Register-based bytecode** — migrate AVM from stack-based to register-based instruction set to reduce unnecessary push/pop; significant rewrite but foundational for JIT
- [ ] **Inline caching** — cache resolved field/method indices at call sites after first lookup; eliminates linear scans on repeat access
- [ ] **Concurrency** — lightweight threads, `async`/`await`, or actor model
- [x] **FFI** — call C functions from Amberlink (`loadLib`/`callInt`/`callStr` for int and string marshaling); access system libraries

---

## Phase 6 — Advanced Language Features (v0.9)
*The "Expressive" milestone — sophisticated type system.*

- [ ] **Null safety** — non-nullable by default, optional types
- [ ] **Generics** — type parameters (`List<T>`, `Map<K, V>`)
- [ ] **Exception handling** — `try`, `catch`, `throw`
- [ ] **Pattern matching** — `match` expressions
- [ ] **Multi-module archives (upgraded .AMB files)** — separately-compiled modules with runtime linking/loading inside `.ama` (true multi-unit JARs); extends the single-unit archive

---

## Phase 7 — JVM Integration (v1.0)
*The "Universal" milestone — tap into the Java ecosystem.*

- [ ] **Java interop** — import Java classes and libraries into Amberlink projects
- [ ] **Hybrid project support** — mixed Amberlink + Java source directories

---

## Phase 8 — Documentation & Community (v1.0)
*The "Growth" milestone — adoption and ecosystem.*

- [ ] Comprehensive language guide
- [ ] Standard library API docs
- [ ] Contributing guide
- [ ] Tutorials / cookbook
- [ ] Community channels (forum, Discord, etc.)

---

## MISC — Later / Unscheduled
*Worth doing, but premature until the language, bytecode, and stdlib stabilize.*

### Tooling
- [ ] **Language Server (LSP)** — autocompletion and error highlighting for VS Code / IntelliJ
- [ ] **Debugger** — step through code, inspect variables, view the stack
- [ ] **Package manager** — share and manage third-party Amberlink libraries

### Backends
- [ ] **Native compilation via LLVM** — compile `.amb` directly to native machine code through LLVM IR; inherits all LLVM optimizations; beats JVM on startup and memory
- [ ] **AOT backend (`amber-native`)** — flesh out the existing C++ native codegen stub for platforms without LLVM
- [ ] **JIT compilation** — compile hot bytecode paths to native code at runtime inside the AVM
- [ ] **WASM target** — compile Amberlink to WebAssembly for browser execution
