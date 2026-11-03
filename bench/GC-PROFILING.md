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

- `allocations` and `allocations_<type>`: total registered objects and counts by object family (string, array, instance, list, hash map, linked list).
- `object_construct_sample_ns`: sampled `new T(...)` time, including the object's constructor and any initial payload allocation.
- `object_creation_sample_ns`: sampled end-to-end construction plus heap registration time; it includes both construction and registration samples.
- `register_sample_ns`: sampled time spent registering an object in the heap slot table. `register_reuse_sample_ns` and `register_append_sample_ns` split those samples by whether a free handle was reused or the table was appended.
- `reused_slots` and `appended_slots`: counts of handle reuse and new high-water slots.
- `object_table_growths` / `object_table_growth_ns`: number and cumulative duration of `objects` vector capacity expansions.
- `free_slot_growths` / `free_slot_growth_ns`: number and cumulative duration of `free_slots` vector capacity expansions; `free_slots_high_water` is its maximum occupancy.
- `heap_slot_capacity`, `heap_slot_storage_bytes`, `free_slot_capacity`, and `free_slot_storage_bytes`: vector backing-store capacity at exit, not total process or object-payload memory.
- `collection_checks`: calls to the collection safe-point gate.
- `threshold_check_sample_ns`: sampled time for the cheap threshold check.
- `collections`: full mark-and-sweep cycles actually run.
- `reset_ns`: cumulative time to advance the marking generation. It no longer clears every object's mark bit; a full reset occurs only if the 32-bit generation wraps.
- `mark_ns`, `sweep_ns`: cumulative time spent tracing roots/object references and sweeping the stable heap-handle table.
- `reset_objects`: objects reset only on generation wrap (normally zero); `root_values`, `marked_objects`, `object_slots_scanned`, `sweep_slots_scanned`, and `reclaimed_objects` describe work across all collections.
- `heap_objects_at_exit` and `heap_slots`: occupied heap entries and the heap table's high-water size at process exit.

Allocation/registration and threshold-check timings are sampled once per 1,024 operations to keep instrumentation overhead low. The object-creation sample includes the nested registration sample. GC phase timers run once per actual collection. Profiling still changes execution cost; use an ordinary Release VM for end-to-end performance comparisons.

## Workloads

- `bench/gc-allocation.amb`: 1,000 short-lived arrays, below the initial 1,024-object collection threshold. Isolates allocation and safe-point overhead without a full collection.
- `bench/gc-mixed-allocation.amb`: retains 5,000 each of instances, arrays, hash maps, and linked lists (plus their containing list). Exercises the different object constructors and nested payload containers.
- `bench/gc-churn.amb`: 100,000 short-lived arrays. Exercises repeated slot reuse and collections where almost all objects are garbage.
- `bench/gc-sweep.amb`: builds a 20,000-node live graph, drops its only root, then allocates 50,000 arrays. Exercises sweeping a large heap table after a large object graph becomes unreachable.
- `bench/gc.amb`: retains 20,000 instances through a list and then performs 20,000 string concatenations. Exercises tracing a large live graph.

## Allocation and heap-management observations

A Release profiling build on the Windows / Ryzen 7 5800X host produced these single-run structural measurements. Construction and registration timings are sampled; vector-growth timings are accumulated for every capacity expansion.

| Workload | Allocations | Reused / appended handles | Object-table growths / time | Free-slot growths / time | Collections |
|---|---:|---:|---:|---:|---:|
| `gc-allocation` | 1,000 | 0 / 1,000 | 11 / 9.4 us | 0 / 0 us | 0 |
| `gc-churn` | 100,000 | 98,975 / 1,025 | 12 / 12.4 us | 11 / 4.6 us | 97 |
| `gc-sweep` | 70,002 | 42,354 / 27,648 | 16 / 81.2 us | 16 / 63.1 us | 45 |
| `gc` | 20,001 | 0 / 20,001 | 16 / 94.9 us | 0 / 0 us | 3 |
| `gc-mixed-allocation` | 20,001 | 0 / 20,001 | 16 / 83.6 us | 0 / 0 us | 3 |

Key observations:

- Churn reuses 98.98% of registrations after the initial heap growth. The stable handle table reaches 1,025 slots and does not grow with the full 100,000-allocation count.
- The large dropped-graph workload reaches a 27,648-slot table and a 27,645-entry free-slot high-water mark. Both backing vectors reach capacity 32,768 (256 KiB each on this 64-bit host).
- Across these workloads, the measured vector-capacity expansions are infrequent and their cumulative durations are small compared with the multi-millisecond collection cost in churn/sweep. The data does not justify replacing the current vectors or adding a more complex free-slot structure.
- Retained-object profiles still spend more time marking than sweeping: the mixed workload recorded 251.3 us marking versus 34.4 us sweeping, with 53,227 object values scanned. This confirms that graph shape and payload density affect the collector's dominant phase.
- The constructor sample measures `new T(...)` (including constructor-time payload initialization), but does not separately time later growth inside `List.add`, `mapPut`, or `llAddLast`. That is a distinct follow-up measurement if collection payload mutation becomes a suspected bottleneck.

A 15-pair alternating process-time comparison of normal Release and profiling Release builds measured the profiling-build delta at -0.27% for `gc-churn`, +0.84% for `gc-sweep`, and +0.54% for `gc-mixed-allocation`. All ranges overlapped. These measurements include process-launch overhead and are only an instrumentation-overhead sanity check, not a speed comparison.

**Current conclusion:** handle reuse is working effectively, and the two heap bookkeeping vectors are not currently a compelling optimization target. Keep the stable handle table and free-slot stack unchanged. If more allocation work is warranted, instrument dynamic payload-container growth (especially retained list/map workloads) before changing allocator or heap representation.

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
