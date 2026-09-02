[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [int] $ProcessId
)

$ErrorActionPreference = 'Stop'
$ProjectRoot = Split-Path -Parent $PSScriptRoot
$Probe = Join-Path $ProjectRoot 'target\console_probe.exe'
$Report = Join-Path $ProjectRoot 'target\console_probe.txt'

& rustc --edition 2024 "$ProjectRoot\tools\console_probe.rs" -o $Probe
if ($LASTEXITCODE -ne 0) { throw 'Could not compile console_probe.' }

& $Probe $ProcessId $Report
if ($LASTEXITCODE -ne 0) {
    throw 'Visible console contains ANSI text or UTF-8/OEM mojibake.'
}

Write-Host (Get-Content -LiteralPath $Report -TotalCount 1)
