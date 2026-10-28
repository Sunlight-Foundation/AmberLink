# tools/bench.ps1 — run all benchmarks (Windows / PowerShell).
# Each bench self-times via clock() and prints result + seconds.
# Usage: make bench   (or: powershell -File tools/bench.ps1)
$root = Split-Path -Parent (Split-Path -Parent $PSCommandPath)
Set-Location -LiteralPath $root
if (-not (Test-Path -LiteralPath "bin\ambc.exe") -or -not (Test-Path -LiteralPath "bin\avm.exe")) {
    Write-Output "ERROR: toolchain not built. Run 'make init' first."
    exit 1
}
$fail = 0
foreach ($b in Get-ChildItem -LiteralPath "bench" -Filter "*.amb" -Name) {
    Write-Output "== ${b}"
    & ".\bin\ambc.exe" "bench\${b}" 2>&1 | Out-Null
    if ($LASTEXITCODE -ne 0) { Write-Output "${b} : COMPILE FAIL"; $fail++; continue }
    & ".\bin\avm.exe" ("bench\" + ([IO.Path]::ChangeExtension($b, ".amc")))
    if ($LASTEXITCODE -ne 0) { Write-Output "${b} : RUN FAIL"; $fail++ }
}
if ($fail -gt 0) { exit 1 }
