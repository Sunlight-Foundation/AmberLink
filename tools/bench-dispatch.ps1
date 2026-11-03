# Compare the AVM's computed-goto and portable switch dispatch implementations.
# Usage: powershell -NoProfile -ExecutionPolicy Bypass -File tools/bench-dispatch.ps1 [-Samples 31]
param([ValidateRange(3, 31)][int]$Samples = 31)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent (Split-Path -Parent $PSCommandPath)
Set-Location -LiteralPath $root
$ambc = Join-Path $root "bin\ambc.exe"
if (-not (Test-Path -LiteralPath $ambc)) { throw "Compiler not found. Build Amber first." }

$tmp = Join-Path ([IO.Path]::GetTempPath()) ("amber-dispatch-" + [guid]::NewGuid())
$computedBuild = Join-Path $tmp "computed"
$switchBuild = Join-Path $tmp "switch"
New-Item -ItemType Directory -Path $tmp | Out-Null

function Invoke-CMake([string[]]$Arguments) {
    & cmake @Arguments
    if ($LASTEXITCODE -ne 0) { throw "CMake failed with exit code $LASTEXITCODE" }
}
function Get-Median([double[]]$Values) {
    $sorted = @($Values | Sort-Object)
    $middle = [int][math]::Floor($sorted.Count / 2)
    if (($sorted.Count % 2) -eq 1) { return [double]$sorted[$middle] }
    return ([double]$sorted[$middle - 1] + [double]$sorted[$middle]) / 2
}
function Invoke-TimedRun([string]$Executable, [string]$Bytecode) {
    $watch = [Diagnostics.Stopwatch]::StartNew()
    & $Executable $Bytecode > $null 2>&1
    $exitCode = $LASTEXITCODE
    $watch.Stop()
    if ($exitCode -ne 0) { throw "AVM benchmark failed: $Executable (exit $exitCode)" }
    return $watch.Elapsed.TotalMilliseconds
}

try {
    Invoke-CMake @("-S", "amber-vm", "-B", $computedBuild, "-G", "Ninja", "-DCMAKE_BUILD_TYPE=Release")
    Invoke-CMake @("--build", $computedBuild, "--config", "Release")
    Invoke-CMake @("-S", "amber-vm", "-B", $switchBuild, "-G", "Ninja", "-DCMAKE_BUILD_TYPE=Release", "-DAVM_FORCE_SWITCH=ON")
    Invoke-CMake @("--build", $switchBuild, "--config", "Release")
    $computed = Join-Path $computedBuild "avm.exe"
    $switch = Join-Path $switchBuild "avm.exe"
    $results = @()

    foreach ($source in Get-ChildItem -LiteralPath (Join-Path $root "bench\parity") -Filter "*.amb" | Sort-Object Name) {
        $copy = Join-Path $tmp $source.Name
        Copy-Item -LiteralPath $source.FullName -Destination $copy
        & $ambc $copy 2>&1 | Out-Null
        if ($LASTEXITCODE -ne 0) { throw "Bytecode compilation failed: $($source.Name)" }
        $bytecode = [IO.Path]::ChangeExtension($copy, ".amc")

        $computedOutput = (& $computed $bytecode 2>&1 | Out-String).Trim()
        $computedExit = $LASTEXITCODE
        $switchOutput = (& $switch $bytecode 2>&1 | Out-String).Trim()
        $switchExit = $LASTEXITCODE
        if ($computedExit -ne 0 -or $switchExit -ne 0 -or $computedOutput -cne $switchOutput) {
            throw "Dispatch output mismatch for $($source.Name) (computed=$computedExit, switch=$switchExit)"
        }

        # Warm both variants, then alternate execution order to reduce order bias.
        [void](Invoke-TimedRun $computed $bytecode)
        [void](Invoke-TimedRun $switch $bytecode)
        $computedTimes = @()
        $switchTimes = @()
        $pairedRatios = @()
        for ($i = 0; $i -lt $Samples; $i++) {
            if (($i % 2) -eq 0) {
                $computedMs = Invoke-TimedRun $computed $bytecode
                $switchMs = Invoke-TimedRun $switch $bytecode
            } else {
                $switchMs = Invoke-TimedRun $switch $bytecode
                $computedMs = Invoke-TimedRun $computed $bytecode
            }
            $computedTimes += $computedMs
            $switchTimes += $switchMs
            $pairedRatios += ($switchMs / $computedMs)
        }

        $results += [pscustomobject]@{
            Benchmark = $source.BaseName
            ComputedMedianMs = [math]::Round((Get-Median $computedTimes), 3)
            ComputedMinMs = [math]::Round((($computedTimes | Measure-Object -Minimum).Minimum), 3)
            ComputedMaxMs = [math]::Round((($computedTimes | Measure-Object -Maximum).Maximum), 3)
            SwitchMedianMs = [math]::Round((Get-Median $switchTimes), 3)
            SwitchMinMs = [math]::Round((($switchTimes | Measure-Object -Minimum).Minimum), 3)
            SwitchMaxMs = [math]::Round((($switchTimes | Measure-Object -Maximum).Maximum), 3)
            PairedSwitchOverComputed = [math]::Round((Get-Median $pairedRatios), 4)
            SwitchSlowerPercent = [math]::Round(((Get-Median $pairedRatios) - 1) * 100, 2)
            Output = $computedOutput
        }
    }

    Write-Output "Windows process-inclusive timings; one warm-up per variant; alternating order; $Samples paired samples."
    $results | Format-Table Benchmark, ComputedMedianMs, ComputedMinMs, ComputedMaxMs, SwitchMedianMs, SwitchMinMs, SwitchMaxMs, SwitchSlowerPercent -AutoSize
    $results | ConvertTo-Json -Depth 3
} finally {
    Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue
}
