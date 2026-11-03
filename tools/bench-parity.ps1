# Run the common AVM/LLVM Native benchmark subset and compare end-to-end process time.
# Usage: powershell -NoProfile -ExecutionPolicy Bypass -File tools/bench-parity.ps1 [-Samples 7]
param([ValidateRange(3, 31)][int]$Samples = 7)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent (Split-Path -Parent $PSCommandPath)
Set-Location -LiteralPath $root
$ambc = Join-Path $root "bin\ambc.exe"
$avm = Join-Path $root "bin\avm.exe"
if (-not (Test-Path -LiteralPath $ambc) -or -not (Test-Path -LiteralPath $avm)) {
    throw "Toolchain not built. Run 'make init' first."
}
$cache = Join-Path $root "amber-vm\build\CMakeCache.txt"
if (Test-Path -LiteralPath $cache) {
    $cacheText = Get-Content -LiteralPath $cache -Raw
    $generator = [regex]::Match($cacheText, '(?m)^CMAKE_GENERATOR:INTERNAL=(.+)$').Groups[1].Value.Trim()
    $buildType = [regex]::Match($cacheText, '(?m)^CMAKE_BUILD_TYPE:STRING=(.*)$').Groups[1].Value.Trim()
    if ($generator -match '^(Ninja|Unix Makefiles|MinGW Makefiles|NMake Makefiles)$' -and $buildType -ne "Release") {
        throw "AVM must use a Release build (CMAKE_BUILD_TYPE=Release). Run 'make init' after updating the build configuration."
    }
}

$tmp = Join-Path ([IO.Path]::GetTempPath()) ("amber-parity-" + [guid]::NewGuid())
New-Item -ItemType Directory -Path $tmp | Out-Null
$results = @()

function Invoke-TimedRun([string]$Target, [string]$Executable, [string]$Bytecode, [int]$Count) {
    $times = @()
    for ($i = 0; $i -lt ($Count + 1); $i++) {
        $watch = [Diagnostics.Stopwatch]::StartNew()
        if ($Target -eq "AVM") {
            & $Executable $Bytecode > $null
        } else {
            & $Executable > $null
        }
        $exitCode = $LASTEXITCODE
        $watch.Stop()
        if ($exitCode -ne 0) { throw "$Target benchmark process failed with exit code $exitCode" }
        if ($i -gt 0) { $times += $watch.Elapsed.TotalMilliseconds }
    }
    $sorted = @($times | Sort-Object)
    $middle = [int][math]::Floor($sorted.Count / 2)
    if (($sorted.Count % 2) -eq 1) {
        $median = $sorted[$middle]
    } else {
        $median = ($sorted[$middle - 1] + $sorted[$middle]) / 2
    }
    return [pscustomobject]@{
        Target = $Target
        MedianMs = [math]::Round($median, 3)
        MinMs = [math]::Round($sorted[0], 3)
        MaxMs = [math]::Round($sorted[-1], 3)
        SamplesMs = (($times | ForEach-Object { [math]::Round($_, 3) }) -join ",")
    }
}

try {
    foreach ($source in Get-ChildItem -LiteralPath (Join-Path $root "bench\parity") -Filter "*.amb" | Sort-Object Name) {
        $copy = Join-Path $tmp $source.Name
        $native = Join-Path $tmp ($source.BaseName + ".exe")
        Copy-Item -LiteralPath $source.FullName -Destination $copy
        & $ambc $copy --native $native
        if ($LASTEXITCODE -ne 0) { throw "Native compilation failed for $($source.Name)" }
        $bytecode = [IO.Path]::ChangeExtension($copy, ".amc")
        $avmOutput = (& $avm $bytecode 2>&1 | Out-String).Trim()
        $avmExit = $LASTEXITCODE
        $nativeOutput = (& $native 2>&1 | Out-String).Trim()
        $nativeExit = $LASTEXITCODE
        if ($avmExit -ne 0 -or $nativeExit -ne 0 -or $avmOutput -cne $nativeOutput) {
            throw "AVM/Native output mismatch for $($source.Name) (AVM=$avmExit, Native=$nativeExit)"
        }
        foreach ($target in @(
            @{ Name = "AVM"; Executable = $avm },
            @{ Name = "Native"; Executable = $native }
        )) {
            $measurement = Invoke-TimedRun $target.Name $target.Executable $bytecode $Samples
            $results += [pscustomobject]@{
                Benchmark = $source.BaseName
                Target = $measurement.Target
                MedianMs = $measurement.MedianMs
                MinMs = $measurement.MinMs
                MaxMs = $measurement.MaxMs
                SamplesMs = $measurement.SamplesMs
            }
        }
    }
    Write-Output "Measurements include process startup and output redirection; one warm-up run is discarded."
    Write-Output "Samples per target: $Samples"
    $results | Format-Table Benchmark, Target, MedianMs, MinMs, MaxMs -AutoSize
    $results | ConvertTo-Json -Depth 3
} finally {
    Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue
}
