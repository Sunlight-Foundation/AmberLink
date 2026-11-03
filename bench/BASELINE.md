# Benchmark baseline (v0.9, Release builds)

Baseline captured on 2026-11-02. These results are a starting point for tracking changes, not performance targets.

## Environment

| Component | Configuration |
|---|---|
| OS | Windows 10 IoT Enterprise LTSC 2021, 21H2 (build 19044.7807) |
| CPU | AMD Ryzen 7 5800X, 8 cores / 16 threads |
| Rust compiler | rustc 1.96.0, Cargo release profile |
| AVM compiler | MinGW-w64 GCC 14.2.0, CMake/Ninja Release, `-O3 -DNDEBUG` |
| Native compiler | Clang 19.1.7, LLVM IR compiled with `-O2` |
| CMake | 3.31.5 |
| Build configuration | `CMAKE_BUILD_TYPE=Release` |

The Makefile now explicitly configures CMake's single-configuration generators for Release builds. It also runs the Windows PowerShell test/benchmark/watch scripts on Windows even when Make is launched from Git Bash, and forces the toolchain targets through their incremental build steps when requested.

## AVM microbenchmark results

These use the existing `bench/*.amb` programs and their in-program `clock()` timer. Compilation and process startup are excluded. Lower is better.

| Benchmark | Workload | Result | Time (s) |
|---|---|---:|---:|
| fib.amb | Recursive fib(25) | 75025 | 0.007850 |
| gc.amb | 20k live Points, then 20k string concatenations (timed section is the concatenation phase) | 20000 | 0.061357 |
| list.amb | 200k linked-list appends + 200k indexed reads | 200000 | 0.015639 |
| loop.amb | 10M integer loop iterations | 10000000 | 0.313006 |
| map.amb | 100k hash-map puts + 100k gets | 100000 | 0.027920 |
| methods.amb | 2M method calls | 2000000 | 0.101793 |
| strcat.amb | 20k string concatenations | 20000 | 0.059732 |

Run with:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools/bench.ps1
```

## Shared AVM / Native results

The shared subset lives in `bench/parity/`. Each program is compiled once to bytecode and LLVM Native. The harness checks that both targets produce identical output before timing them.

The measurements below use 15 samples per target after one warm-up run. They include process startup and shutdown, with standard output redirected to `$null`; compilation is excluded. Times are median wall-clock milliseconds. This is an end-to-end invocation measurement, not a pure in-process execution measurement.

| Workload | AVM median (ms) | Native median (ms) | AVM min-max (ms) | Native min-max (ms) |
|---|---:|---:|---:|---:|
| fib - recursive fib(25) | 18.272 | 11.483 | 17.778-18.800 | 10.836-12.533 |
| loop - 10M integer iterations | 325.132 | 101.045 | 319.337-336.395 | 99.313-120.363 |
| strcat - 20k concatenations | 98.833 | 105.589 | 82.552-107.475 | 89.040-111.019 |

Run with:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools/bench-parity.ps1 -Samples 15
```

The process-startup cost is material for the short Fibonacci workload. The string-concatenation ranges also show enough variance that the small median difference should not be treated as a meaningful regression or win. The loop workload is currently the clearest Native advantage in this small shared subset. These results are not a claim of overall target superiority.

## Coverage and limitations

- The AVM-only suite covers recursion, loops, GC pressure, linked lists, hash maps, method calls, and string concatenation.
- The current LLVM Native subset covers recursion, arithmetic/loop control flow, and string concatenation. The AOT support matrix in [LLVM-AOT.md](../docs/LLVM-AOT.md) remains authoritative; collections, arrays, object dispatch, native calls, threads, exceptions, and multi-module AOT are not yet supported.
- The shared harness currently targets Windows and reports process-inclusive timing. It does not yet isolate startup from steady-state execution or collect memory, allocation, GC pause, or compiler-time metrics.
- Results are from one machine and one OS. Cross-platform baselines and additional workloads are still needed.
- Do not set arbitrary performance budgets from this initial sample. Expand the common workload set and add runtime-level instrumentation before using these figures as hard regression gates.

## Historical note

The previous baseline was v0.7 and used a different workload/measurement state. Do not compare its numbers directly with this v0.9 baseline as if they were measured under identical conditions.
