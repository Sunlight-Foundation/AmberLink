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

## Phase 2 optimization check: Native numeric fast paths

Compared the unmodified `beta_development` worktree with the LLVM numeric fast-path change on the same machine. Both runs used 30 samples per target after one warm-up, included process startup/shutdown, redirected standard output, and excluded compilation. The AVM was unchanged during these measurements and is included as a control; the separate AVM integer-division edge-case guard added afterward does not affect these workloads.

| Workload | AVM before (ms) | AVM after (ms) | Native before (ms) | Native after (ms) | Native median change |
|---|---:|---:|---:|---:|---:|
| fib - recursive fib(25) | 19.346 | 19.349 | 12.517 | 11.027 | -11.9% |
| loop - 10M integer iterations | 335.134 | 336.292 | 103.654 | 54.394 | -47.5% |
| strcat - 20k concatenations | 101.555 | 100.838 | 110.019 | 108.381 | -1.5% |

The updated Native ranges were 10.170–13.590 ms for fib, 53.370–58.291 ms for loop, and 91.531–143.495 ms for strcat. The original Native ranges were 11.617–14.932 ms, 100.792–113.084 ms, and 94.564–122.408 ms respectively. The loop improvement is clear and repeatable: the common integer arithmetic and comparisons now lower directly to LLVM instructions rather than making an out-of-line C runtime call for each operation. Fibonacci also improved, while the string workload continues through the runtime fallback and is effectively unchanged within its measurement variability. AVM timings remained stable as expected because this change does not modify the AVM.

These are workload-specific observations, not universal performance guarantees or new regression budgets.

## Phase 2 optimization check: AVM instance-field access

An opt-in opcode counter (`AVM_PROFILE=ON`) was added for profiling builds. It has no counter increments or output in normal builds. The counter output was verified under both dispatch implementations, and a forced switch-dispatch Release build matched computed-goto output across all 27 example regression programs. See [AVM-PROFILING.md](AVM-PROFILING.md) for the build and usage instructions.

Dynamic counts from the 10M loop show that global/stack traffic dominates that workload: 40,000,003 `LOAD_GLOBAL`, 20,000,003 `STORE_GLOBAL`, 20,000,003 `PUSH`, 20,000,000 `ADD`, 10,000,000 `JUMP`, 10,000,001 `JUMP_IF_FALSE`, and 10,000,001 `LESS` executions. The 2M method-call workload executes 2,000,000 each of `CALL`, `RETURN`, `GET_FIELD`, and `SET_FIELD`, plus 6,000,000 `LOAD_LOCAL` operations.

The measured method hot path was `GET_FIELD` / `SET_FIELD`: both dispatch implementations previously used RTTI `dynamic_cast` for every instance field read/write. They now validate the heap object's existing `ObjType::INSTANCE` tag and use `static_cast`, preserving the same type-mismatch error while avoiding RTTI on this hot path. Array and list casts were deliberately left unchanged because the measured workloads did not establish them as bottlenecks.

For a 31-pair alternating A/B run against an unmodified `beta_development` worktree, the 2M method benchmark measured 114.083 ms before and 104.848 ms after (median of paired ratios: -10.5%; individual ranges: 105.769–136.730 ms before, 95.870–125.998 ms after). The unchanged loop control also shifted from 331.102 ms to 314.031 ms (-5.2%) in that run, indicating material run-to-run variation; the ranges overlap, so do not attribute the full difference to this change. Treat it as an encouraging workload-specific signal rather than a universal or guaranteed speedup. The interpreter change does not alter the bytecode format or Amber semantics.

## Coverage and limitations

- The AVM-only suite covers recursion, loops, GC pressure, linked lists, hash maps, method calls, and string concatenation.
- The current LLVM Native subset covers recursion, arithmetic/loop control flow, and string concatenation. The AOT support matrix in [LLVM-AOT.md](../docs/LLVM-AOT.md) remains authoritative; collections, arrays, object dispatch, native calls, threads, exceptions, and multi-module AOT are not yet supported.
- The shared harness currently targets Windows and reports process-inclusive timing. It does not yet isolate startup from steady-state execution or collect memory, allocation, GC pause, or compiler-time metrics.
- Results are from one machine and one OS. Cross-platform baselines and additional workloads are still needed.
- Do not set arbitrary performance budgets from this initial sample. Expand the common workload set and add runtime-level instrumentation before using these figures as hard regression gates.

## Historical note

The previous baseline was v0.7 and used a different workload/measurement state. Do not compare its numbers directly with this v0.9 baseline as if they were measured under identical conditions.

## AVM binary stack-result reuse experiment

The AVM's profiled 10M loop executes 20M `ADD` operations and 10M `LESS` operations. Both previously removed two operands from the value stack and then pushed the result. The interpreter now writes the result into the lower operand slot and removes only the upper slot for the numeric `ADD` and `LESS` cases. String concatenation keeps its existing collection-safe path. Computed-goto and portable switch dispatch use the same behavior.

A 31-pair alternating A/B run compared the merged pre-change revision (`d89a404`) with this change on `bench/parity/loop.amb`:

| Measurement | Before | After |
|---|---:|---:|
| Median process time | 375.753 ms | 336.788 ms |
| Median paired change | n/a | -11.38% |
| Observed range | 352.727-547.078 ms | 305.246-534.360 ms |

The ranges overlap and the run contains substantial timing outliers. Treat the paired result as an encouraging, workload-specific signal rather than a guaranteed general speedup. A separate 31-pair `fib` run also improved by 6.46%; that workload uses `ADD` too, so it is corroborating evidence, not an unaffected control.

The global opcodes remain frequent in the loop (`LOAD_GLOBAL` 40,000,003 and `STORE_GLOBAL` 20,000,003 in the current profile), but this change deliberately leaves their implementation alone: frequency alone does not establish that changing global access would improve execution. Further global-slot work should be based on an isolated measurement.

## AVM global-slot versus local-slot experiment

To separate global-slot traffic from other work in the 10M integer loop, two AVM-focused programs perform the same loop and produce the same result (`10000000`):

- [`avm-global-loop.amb`](avm-global-loop.amb) keeps the loop variables in module globals.
- [`avm-local-loop.amb`](avm-local-loop.amb) passes the loop variables as function parameters, so the compiler uses local slots.

Opcode profiling confirms that the loop body is otherwise matched: both execute 20,000,000 `ADD`, 10,000,001 `LESS`, 10,000,000 `JUMP`, and 10,000,001 `JUMP_IF_FALSE` operations. The global version executes 40,000,002 `LOAD_GLOBAL` and 20,000,002 `STORE_GLOBAL`; the local version replaces those with the same counts of `LOAD_LOCAL` and `STORE_LOCAL`. The local version additionally has one `CALL` and one `RETURN`.

A 31-pair alternating process-time comparison on the merged runtime measured:

| Measurement | Global slots | Local slots |
|---|---:|---:|
| Median process time | 340.983 ms | 337.804 ms |
| Observed range | 335.410-365.747 ms | 333.634-366.508 ms |

The median paired global/local ratio was 1.0090 (global 0.90% slower). The ranges overlap substantially, and the difference is small relative to normal timing variation. This is not strong enough evidence to justify changing global storage or removing runtime bounds handling. The local variant also includes a function call/return, so this is a practical slot-access comparison rather than a perfectly isolated instruction microbenchmark.

**Conclusion:** global slot access may carry a small overhead relative to local slots, but the current evidence does not justify a speculative runtime or bytecode change. Keep the explicit global load bounds check and current module relocation behavior. Revisit only with a tighter isolated benchmark or stronger repeatable signal.

## AVM dispatch implementation comparison

The AVM supports GCC/Clang computed-goto dispatch and a portable switch dispatcher. The latter is also used by MSVC and can be forced on GCC/Clang with `AVM_FORCE_SWITCH=ON`. To measure the difference without profiling overhead, run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools/bench-dispatch.ps1 -Samples 31
```

The script builds both implementations from the same source in separate Release directories, compiles the shared AVM/Native parity workloads to bytecode, verifies identical output, warms both executables, and alternates execution order for each paired sample. Timings include process startup and output redirection.

On the Ryzen 7 5800X / Windows 10 IoT Enterprise LTSC 2021 system, a 31-pair run produced:

| Workload | Computed-goto median | Switch median | Paired switch/computed ratio | Observed ranges (computed / switch) |
|---|---:|---:|---:|---:|
| fib | 20.000 ms | 20.268 ms | 1.0152 (+1.52%) | 18.253-31.402 / 18.156-77.977 ms |
| loop | 319.387 ms | 330.226 ms | 1.0342 (+3.42%) | 306.024-354.819 / 314.851-415.185 ms |
| strcat | 90.682 ms | 94.683 ms | 1.0060 (+0.60%) | 83.553-129.720 / 84.045-106.979 ms |

All workload outputs matched. The observed ranges overlap for every workload, and the process-inclusive measurements contain substantial noise. Computed-goto has a small median advantage in this run, most visible in the loop workload, but the evidence does not establish a large or universal dispatch bottleneck. Keep computed-goto as the GCC/Clang default and retain switch dispatch as the portable fallback; do not make a dispatch rewrite based on this result alone.

## AVM heap collection and thread roots

An audit found that `Heap::register_object` counted allocations toward `gc_threshold`, but the interpreter only called `collect()` from string concatenation. Programs allocating arrays, instances, or collections without concatenating strings could therefore grow the heap without triggering collection. A second correctness issue was that collection scanned only the currently executing thread's stack, even though the VM shares one heap across spawned threads. A collection in one worker could reclaim an object still held only by another worker's stack.

The interpreter now checks the adaptive collection threshold after heap-object creation (`NEW_ARRAY`, `NEW_INSTANCE`, `NEW_LIST`, and allocating native calls), after pushing the new value onto the current stack. String concatenation remains a safe point. Collection scans every active interpreter stack, globals, spawn arguments waiting for a worker to start, and completed worker results. Stack registration and root handoff are serialized by the existing GIL. Pending spawn roots are removed once the worker's stack has been registered; completed results remain rooted while thread handles can still be joined repeatedly.

Two regression workloads cover both sides of the thread-root handoff:

- `examples/gc_thread_roots_test.amb`: a worker holds an instance on its active stack while the main thread allocates enough arrays to trigger collection. Before the fix, the final field access failed with `Reference is not an instance`; after the fix it returns `42`.
- `examples/gc_thread_pending_roots_test.amb`: the main thread allocates enough arrays immediately after spawning, while the worker is still waiting for the GIL. The worker subsequently accesses its argument successfully and returns `42`.

Both workloads run in the normal regression harness. They passed with computed-goto and forced-switch dispatch. The full regression suite and 15-sample AVM/Native parity suite also passed. This change addresses collection correctness and trigger coverage; it is not presented as a measured performance improvement.


## AVM heap and garbage-collection profiling

An opt-in `AVM_GC_PROFILE=ON` build now reports heap allocation counts, sampled object-construction/registration and collection-threshold costs, collection counts, mark/reset/sweep phase timings, root/edge traversal counts, reclaimed objects, and heap table occupancy. Allocation and threshold-check timings are sampled once per 1,024 operations to limit profiling overhead. See [GC-PROFILING.md](GC-PROFILING.md) for build instructions and workload definitions.

Four focused workloads separate allocation without collection (`gc-allocation`), short-lived allocation churn (`gc-churn`), sweeping after a large graph becomes unreachable (`gc-sweep`), and tracing a large retained graph (`gc`). On the Ryzen 7 5800X Windows host, the diagnostic profiles show that sweep dominates churn and large-dead-heap workloads, while mark traversal dominates the retained-graph workload. In the 100,000-array churn case, 97 collections reclaimed 99,327 objects; cumulative sweep time was 3.663 ms versus 5.4 us of mark time. In the retained 20,000-instance graph, mark time was 68.8 us of 104.2 us total collection time. These are single-run phase diagnostics, not performance budgets.

The initial profile identified reset/sweep as optimization candidates. The follow-up mark-generation change below removes the per-collection reset pass; the stable handle-table sweep remains unchanged because sparse-index experiments did not produce a repeatable phase-time improvement. The profiling option is disabled in normal builds.


## AVM mark-generation reset optimization

The AVM collector now uses a 32-bit mark generation instead of clearing every object's boolean mark before each collection. Advancing the generation replaces the reset pass; a full reset remains as a wraparound safeguard. `AmberObject` remains 16 bytes on the Windows / MinGW x64 build. Stable object handles, thread roots, GC thresholds, and the original contiguous sweep are unchanged.

In a 15-pair profiling-build A/B against pre-change `beta_development`, cumulative reset time fell from 82.6 us to 3.7 us in `gc-churn`, from 609.4 us to 1.8 us in `gc-sweep`, and from 32.0 us to 0.1 us in the retained-graph `gc` workload. Total collection time medians improved by 1.8%, 8.2%, and 11.8%, respectively, in those instrumented runs. A separate 31-pair normal-Release A/B showed no material end-to-end change; all paired changes were within broad overlapping timing ranges. These are diagnostic results, not general speedup claims.

A sparse active-slot/occupancy-index sweep was prototyped and discarded because reduced slot visits did not translate into a repeatable sweep-time improvement and added allocation bookkeeping. The final implementation keeps stable heap slots and the original contiguous sweep. See [GC-PROFILING.md](GC-PROFILING.md) for the methodology and phase data.
