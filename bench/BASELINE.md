# Benchmark baseline (v0.7, Release build)

Machine: Windows x64, mingw64 build. Lower is better (seconds).

| bench | workload | result | time (s) |
|-------|----------|--------|----------|
| fib.amb | fib(25) recursive | 75025 | 0.106 |
| list.amb | 200k llAddLast + 200k llGet | 200000 | 0.229 |
| loop.amb | 10M integer add loop | 10000000 | 4.167 |
| map.amb | 10k mapPut + 10k mapGet | 10000 | 0.880 |
| methods.amb | 2M method calls | 2000000 | 1.467 |
| strcat.amb | 20k string concats | 20000 | 0.073 |

Notes:
- Integer results are i32; benches print loop counters, not sums, to avoid overflow.
- map lookup is a linear scan (O(n)); N kept small. Hash-indexed map is future work.
- Re-run with `make bench` after each optimization and compare.
