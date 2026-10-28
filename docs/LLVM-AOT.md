# LLVM AOT Backend: Support and Test Scope

The LLVM AOT backend is an optional compilation path, not yet a feature-complete replacement for AVM. It must reject unsupported bytecode explicitly; a successful AOT compile does not imply that every Amberlink program can be compiled natively.

## Currently exercised by the AOT differential suite

`tools/test-native.sh` compiles each program through `ambc --native`, executes the generated native binary, runs the same compiler-produced `.amc` through AVM, and compares standard output:

- `basic_types_test`
- `factorial`
- `float_test`
- `fold_test`
- `hello`
- `hi`
- `match_test`
- `null_test`
- `test_static`

This is differential smoke/regression coverage for the supported subset. It does not establish complete semantic equivalence, memory-safety equivalence, or matching behavior for every possible input. The current harness compares stdout for these deterministic, zero-exit examples; it is not yet a randomized or property-based differential test.

## Supported lowering categories

The backend currently lowers basic tagged values (integers, floats, booleans, chars, strings, and null), arithmetic and comparisons, string concatenation/comparison/printing, globals, function definitions/calls, conditional and loop control flow, match lowering, and null checks.

## Known unsupported categories

The backend reports a compile error rather than silently substituting behavior for unsupported bytecode. Known exclusions include:

- Arrays and array reads/writes.
- List creation and list operations.
- Object construction, fields, and class/method dispatch.
- Native calls, including standard-library, filesystem, networking, collection, and FFI natives.
- OS-thread spawn/join.
- Exception-handling instructions (`try`, `end_try`, and `throw`).
- Cross-module function linking and multi-module AOT compilation.

Keep these exclusions explicit in release notes. Add a category to the supported matrix only after its semantics are implemented and covered by differential tests. The AVM remains the compatibility baseline for features outside this subset.
