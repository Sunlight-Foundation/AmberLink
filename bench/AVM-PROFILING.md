# AVM opcode profiling

The AVM can be built with opt-in dynamic opcode counters. Normal builds leave profiling disabled and contain no counter increments or profile output.

## Build a profiling VM

From the repository root on Windows (Ninja and the MinGW-w64 toolchain must be available):

```powershell
$profileBuild = Join-Path $env:TEMP "amber-vm-profile"
cmake -S amber-vm -B $profileBuild -G Ninja -DCMAKE_BUILD_TYPE=Release -DAVM_PROFILE=ON
cmake --build $profileBuild --config Release
```

Compile a benchmark to bytecode with the regular compiler, then run it with the profiling VM:

```powershell
bin/ambc.exe bench/loop.amb
$profileBuild = Join-Path $env:TEMP "amber-vm-profile"
& (Join-Path $profileBuild "avm.exe") bench/loop.amc 2>&1 | Select-String AVM_OPCODE_COUNT
```

Each output line is `AVM_OPCODE_COUNT <opcode-byte> <dynamic-execution-count>`. Opcode byte values are defined in `amber-vm/include/bytecode.hpp`. The counters include dispatches, not operand bytes, and are emitted to standard error when the program halts.

## Test the portable switch dispatcher

GCC and Clang use computed-goto dispatch by default. To compile and test the portable switch implementation on those compilers:

```powershell
$switchBuild = Join-Path $env:TEMP "amber-vm-switch"
cmake -S amber-vm -B $switchBuild -G Ninja -DCMAKE_BUILD_TYPE=Release -DAVM_FORCE_SWITCH=ON
cmake --build $switchBuild --config Release
```

Add `-DAVM_PROFILE=ON` to that configure command if opcode counts are also needed. `AVM_FORCE_SWITCH` is a testing option; normal builds continue to use the compiler's default dispatch implementation.

Profiling changes execution cost, so use it to understand instruction mix—not to collect performance timings. Use the ordinary Release VM and the scripts in `tools/bench.ps1` / `tools/bench-parity.ps1` for timing comparisons.
