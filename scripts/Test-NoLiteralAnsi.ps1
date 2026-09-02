[CmdletBinding()]
param(
    [string] $Program = "$PSScriptRoot\..\demos\showcase\showcase.exe"
)

$ErrorActionPreference = 'Stop'
$PreviousAutoClose = $env:HB_RATATUI_AUTOCLOSE

try {
    $env:HB_RATATUI_AUTOCLOSE = '1'
    $Captured = & $Program 2>&1 | Out-String
    if ($LASTEXITCODE -ne 0) {
        throw "Showcase returned exit code $LASTEXITCODE."
    }
}
finally {
    $env:HB_RATATUI_AUTOCLOSE = $PreviousAutoClose
}

if ($Captured.Contains([char] 27)) {
    throw 'Literal ANSI escape sequences reached the console output path.'
}

Write-Host 'PASS: showcase output contains no literal ANSI escape sequences.'

