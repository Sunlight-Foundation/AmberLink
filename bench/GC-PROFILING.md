# AVM heap and garbage-collection profiling

The AVM supports opt-in heap/GC counters and phase timings. Normal builds leave `AVM_GC_PROFILE` disabled; they do not contain the profiling counters or emit GC profile output.

## Build a profiling VM

From the repository root on Windows (Ninja and MinGW-w64 available):

```powershell
$profileBuild = Join-Path $env:TEMP "amber-vm-gc-profile"
cmake -S amber-vm -B $profileBuild -G Ninja -DCMAKE_BUILD_TYPE=Release -DAVM_GC_PROFILE=ON
cmake --build $profileBuild --config Release
bin/ambc.exe bench/gc-churn.amb
& (Join-Path $profileBuild "avm.exe") bench/gc-churn.amc 2>&1 | Select-String "AVM GC"
```

The profile is emitted to standard error when the VM heap is destroyed. Useful fields:

- `allocations`: objects registered with the heap.
- `object_creation_sample_ns`: sampled end-to-end object construction plus heap registration time.
- `register_sample_ns`: sampled time spent registering the object in the heap slot table; included in `object_creation_sample_ns`.
- `collection_checks`: calls to the collection safe-point gate.
- `threshold_check_sample_ns`: sampled time for the cheap threshold check.
- `collections`: full mark-and-sweep cycles actually run.
- `reset_ns`, `mark_ns`, `sweep_ns`: cumulative time spent clearing mark bits, tracing roots/object references, and sweeping heap slots.
- `reset_objects`, `root_values`, `marked_objects`, `object_slots_scanned`, `reclaimed_objects`: work counters across all collections.
- `heap_objects_at_exit` and `heap_slots`: occupied heap entries and the heap table's high-water size at process exit.

Allocation/registration and threshold-check timings are sampled once per 1,024 operations to keep instrumentation overhead low. The object-creation sample includes the nested registration sample. GC phase timers run once per actual collection. Profiling still changes execution cost; use an ordinary Release VM for end-to-end performance comparisons.

## Workloads

- `bench/gc-allocation.amb`: 1,000 short-lived arrays, below the initial 1,024-object collection threshold. Isolates allocation and safe-point overhead without a full collection.
- `bench/gc-churn.amb`: 100,000 short-lived arrays. Exercises repeated slot reuse and collections where almost all objects are garbage.
- `bench/gc-sweep.amb`: builds a 20,000-node live graph, drops its only root, then allocates 50,000 arrays. Exercises sweeping a large heap table after a large object graph becomes unreachable.
- `bench/gc.amb`: retains 20,000 instances through a list and then performs 20,000 string concatenations. Exercises tracing a large live graph.

## Initial Windows / Ryzen 7 5800X profile

A Release profiling build produced the following representative cumulative timings (single runs; these are diagnostic phase totals, not performance budgets):

| Workload | Collections | Reset | Mark | Sweep | Total collection time | Reclaimed |
|---|---:|---:|---:|---:|---:|---:|
| `gc-churn` | 97 | 70.7 us | 5.4 us | 3.663 ms | 3.739 ms | 99,327 |
| `gc-sweep` | 45 | 632.1 us | 69.3 us | 2.923 ms | 3.625 ms | 69,629 |
| `gc` | 3 | 20.8 us | 68.8 us | 14.6 us | 104.2 us | 0 |

The results show different GC cost profiles:

- In churn workloads, sweeping dominates because each collection walks the heap slot table and reclaims many dead entries. Marking is nearly free when very few objects remain reachable.
- After dropping a large graph, sweeping still dominates, while reset cost becomes noticeable because the heap table has grown to 27,648 slots.
- With a large retained graph, marking dominates: the tracer follows tens of thousands of object fields/list entries, while sweep has little garbage to reclaim.

A 15-pair alternating normal-Release versus profiling-Release process-time check measured profile-build median overhead of +0.7% for both `gc-allocation` and `gc-churn`; `gc` was -2.6%. Ranges overlapped. Treat that as a sanity check only, not a general guarantee that profiling is free.

**Current conclusion:** the next optimization candidate is the sweep/reset pass over the heap slot table, particularly when many entries are holes or most objects are dead. Mark traversal is workload-dependent and is not yet a universal bottleneck. Do not change collection policy or heap representation until a targeted A/B experiment demonstrates a repeatable benefit without weakening GC correctness.
