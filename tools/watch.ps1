# tools/watch.ps1 - rebuild + rerun on every *.amb change (Windows / PowerShell).
# Usage: make watch file=main.amb   (Ctrl+C to stop)
param([string]$file)
$root = Split-Path -Parent (Split-Path -Parent $PSCommandPath)
Set-Location -LiteralPath $root
if (-not $file) { Write-Output "Usage: make watch file=main.amb"; exit 1 }

function Run-File {
    if ($file -like "*.amb") {
        & ".\bin\ambc.exe" $file 2>&1 | Out-Null
        if ($LASTEXITCODE -ne 0) { Write-Output "[watch] compile failed"; return }
        & ".\bin\avm.exe" ($file -replace "\.amb$", ".amc")
    } else {
        & ".\bin\avm.exe" $file
    }
}

$last = @{}
function Snapshot {
    Get-ChildItem -Recurse -Filter *.amb -File -ErrorAction SilentlyContinue | ForEach-Object {
        $last[$_.FullName] = $_.LastWriteTimeUtc.Ticks
    }
}

Write-Output "[watch] watching *.amb under $root (Ctrl+C to stop)"
Snapshot
Run-File
while ($true) {
    Start-Sleep -Seconds 2
    $changed = $false
    Get-ChildItem -Recurse -Filter *.amb -File -ErrorAction SilentlyContinue | ForEach-Object {
        if ($last[$_.FullName] -ne $_.LastWriteTimeUtc.Ticks) {
            $last[$_.FullName] = $_.LastWriteTimeUtc.Ticks
            $changed = $true
        }
    }
    if ($changed) { Write-Output "[watch] change detected"; Run-File }
}
