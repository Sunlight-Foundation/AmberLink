# tools/test.ps1 - Amberlink regression harness (Windows / PowerShell).
# Compiles every example + stdlib file, runs every example, reports failures.
# Usage: make test   (or: powershell -File tools/test.ps1)
# Exit code: 0 = all green, 1 = any failure.

$ErrorActionPreference = "Continue"
$root = Split-Path -Parent (Split-Path -Parent $PSCommandPath)
Set-Location -LiteralPath $root

$ambc = "bin\ambc.exe"
$avm = "bin\avm.exe"
if (-not (Test-Path -LiteralPath $ambc) -or -not (Test-Path -LiteralPath $avm)) {
    Write-Output "ERROR: toolchain not built. Run 'make init' first."
    exit 1
}

$examples = @('factorial','hello','hi','basic_types_test','float_test','list_test','test_init','test_methods','test_oop','test_overload','test_static','test_visibility','gc_test','native_test','file_io_test','collections_test','import_test','archive_test','net_test','ffi_test','threads_test','fold_test','ffi_buf_test')
$stdlib = @('stdlib\core.amb','stdlib\io.amb','stdlib\collections.amb','stdlib\net.amb','stdlib\ffi.amb')
$fail = 0

# net_test needs the local echo server; start it when python exists.
$server = $null
$python = Get-Command python -ErrorAction SilentlyContinue
if ($python) {
    $server = Start-Process -FilePath "python" -ArgumentList "examples\resources\echo_server.py" -PassThru
    Start-Sleep -Seconds 2
} else {
    Write-Output "NOTE: python not found - net_test run step will be skipped (compile only)."
}

# ffi_buf_test needs the C fixture; build it when a C toolchain exists.
$fficmp = $false
$cc = Get-Command gcc -ErrorAction SilentlyContinue
if (-not $cc) { $cc = Get-Command cc -ErrorAction SilentlyContinue }
if ($cc) {
    & $cc.Source -shared -o bin\fficmp.dll examples\resources\fficmp.c 2>&1 | Out-Null
    if ($LASTEXITCODE -eq 0) { $fficmp = $true }
}
if (-not $fficmp) {
    Write-Output "NOTE: no C toolchain - ffi_buf_test run step will be skipped (compile only)."
}

foreach ($e in $examples) {
    # --emit-ir also asserts the backend-IR decode/re-encode round-trip.
    & ".\${ambc}" "examples\${e}.amb" --emit-ir 2>&1 | Out-Null
    if ($LASTEXITCODE -ne 0) { Write-Output "${e} : COMPILE FAIL"; $fail++; continue }
    if ($e -eq 'net_test' -and -not $server) { Write-Output "${e} : compile OK (run skipped, no python)"; continue }
    if ($e -eq 'ffi_buf_test' -and -not $fficmp) { Write-Output "${e} : compile OK (run skipped, no C toolchain)"; continue }
    & ".\${avm}" "examples\${e}.amc" 2>&1 | Out-Null
    if ($LASTEXITCODE -ne 0) { Write-Output "${e} : RUN FAIL"; $fail++ } else { Write-Output "${e} : OK" }
}

foreach ($s in $stdlib) {
    & ".\${ambc}" $s --emit-ir 2>&1 | Out-Null
    if ($LASTEXITCODE -ne 0) { Write-Output "${s} : COMPILE FAIL"; $fail++ } else { Write-Output "${s} : OK" }
}

if ($server) { Stop-Process -Id $server.Id -ErrorAction SilentlyContinue }

Write-Output "FAILURES=${fail}"
if ($fail -gt 0) { exit 1 }
