[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [int] $ProcessId
)

$ErrorActionPreference = 'Stop'
$ProjectRoot = Split-Path -Parent $PSScriptRoot
$KeyTool = Join-Path $ProjectRoot 'target\console_key.exe'
$ProbeScript = Join-Path $ProjectRoot 'scripts\Test-VisibleConsole.ps1'
$Report = Join-Path $ProjectRoot 'target\console_probe.txt'

& rustc --edition 2024 "$ProjectRoot\tools\console_key.rs" -o $KeyTool
if ($LASTEXITCODE -ne 0) { throw 'Could not compile console_key.' }

& $ProbeScript -ProcessId $ProcessId
if (-not (Select-String -LiteralPath $Report -SimpleMatch 'New dashboard' -Quiet)) {
    throw 'The File menu is not visible before navigation.'
}

& $KeyTool $ProcessId right
Start-Sleep -Milliseconds 250
& $ProbeScript -ProcessId $ProcessId
if (-not (Select-String -LiteralPath $Report -SimpleMatch 'Performance' -Quiet)) {
    throw 'Right Arrow did not select the View menu.'
}

& $KeyTool $ProcessId left
Start-Sleep -Milliseconds 250
& $ProbeScript -ProcessId $ProcessId
if (-not (Select-String -LiteralPath $Report -SimpleMatch 'New dashboard' -Quiet)) {
    throw 'Left Arrow did not return to the File menu.'
}

& $KeyTool $ProcessId tab
Start-Sleep -Milliseconds 250
& $ProbeScript -ProcessId $ProcessId
if (Select-String -LiteralPath $Report -SimpleMatch 'New dashboard' -Quiet) {
    throw 'Tab did not close the File menu.'
}

& $KeyTool $ProcessId right
Start-Sleep -Milliseconds 250
& $ProbeScript -ProcessId $ProcessId
if (-not (Select-String -LiteralPath $Report -SimpleMatch 'Performance' -Quiet)) {
    throw 'Right Arrow did not reopen the menu and select View.'
}

& $KeyTool $ProcessId left
& $KeyTool $ProcessId tab
Start-Sleep -Milliseconds 250
& $ProbeScript -ProcessId $ProcessId
if (-not (Select-String -LiteralPath $Report -SimpleMatch '│▶ ◆ Workspace' -Quiet)) {
    throw 'The tree does not initially select Workspace.'
}
if (-not (Select-String -LiteralPath $Report -SimpleMatch '▶ Tree: hierarchy + ticks' -Quiet)) {
    throw 'The tree does not initially own the data-panel focus.'
}

& $KeyTool $ProcessId down
Start-Sleep -Milliseconds 250
& $ProbeScript -ProcessId $ProcessId
if (-not (Select-String -LiteralPath $Report -SimpleMatch '│▶   ☑ Harbour VM' -Quiet)) {
    throw 'Down Arrow did not move the tree selection to Harbour VM.'
}
if (Select-String -LiteralPath $Report -SimpleMatch '│▸ Ratatui FFI' -Quiet) {
    throw 'Tree navigation incorrectly moved the table selection.'
}

& $KeyTool $ProcessId down
& $KeyTool $ProcessId minus
Start-Sleep -Milliseconds 250
& $ProbeScript -ProcessId $ProcessId
if (-not (Select-String -LiteralPath $Report -SimpleMatch '│▶   [+] Toolchains' -Quiet)) {
    throw 'Minus did not collapse the selected Toolchains group.'
}
if (Select-String -LiteralPath $Report -SimpleMatch '☑ Zig64' -Quiet) {
    throw 'Collapsed Toolchains still renders its child rows.'
}

& $KeyTool $ProcessId down
Start-Sleep -Milliseconds 250
& $ProbeScript -ProcessId $ProcessId
if (-not (Select-String -LiteralPath $Report -SimpleMatch '│▶   [-] Widgets' -Quiet)) {
    throw 'Down Arrow did not skip collapsed Toolchains children.'
}

& $KeyTool $ProcessId up
& $KeyTool $ProcessId plus
Start-Sleep -Milliseconds 250
& $ProbeScript -ProcessId $ProcessId
if (-not (Select-String -LiteralPath $Report -SimpleMatch '│▶   [-] Toolchains' -Quiet)) {
    throw 'Plus did not expand the selected Toolchains group.'
}
if (-not (Select-String -LiteralPath $Report -SimpleMatch '☑ Zig64' -Quiet)) {
    throw 'Expanded Toolchains did not restore its child rows.'
}

& $KeyTool $ProcessId down
& $KeyTool $ProcessId down
& $KeyTool $ProcessId down
& $KeyTool $ProcessId down
& $KeyTool $ProcessId minus
Start-Sleep -Milliseconds 250
& $ProbeScript -ProcessId $ProcessId
if (-not (Select-String -LiteralPath $Report -SimpleMatch '│▶   [+] Widgets' -Quiet)) {
    throw 'Minus did not collapse the selected Widgets group.'
}
if (Select-String -LiteralPath $Report -SimpleMatch '☑ Charts' -Quiet) {
    throw 'Collapsed Widgets still renders its child rows.'
}

& $KeyTool $ProcessId plus
Start-Sleep -Milliseconds 250
& $ProbeScript -ProcessId $ProcessId
if (-not (Select-String -LiteralPath $Report -SimpleMatch '│▶   [-] Widgets' -Quiet)) {
    throw 'Plus did not expand the selected Widgets group.'
}
if (-not (Select-String -LiteralPath $Report -SimpleMatch '☑ Charts' -Quiet)) {
    throw 'Expanded Widgets did not restore its child rows.'
}

& $KeyTool $ProcessId f6
Start-Sleep -Milliseconds 250
& $ProbeScript -ProcessId $ProcessId
if (-not (Select-String -LiteralPath $Report -SimpleMatch '▶ Table + live data' -Quiet)) {
    throw 'F6 did not move focus from Tree to Table.'
}
if (-not (Select-String -LiteralPath $Report -SimpleMatch '│▸ Harbour VM' -Quiet)) {
    throw 'The focused table did not restore its own selection.'
}

& $KeyTool $ProcessId down
Start-Sleep -Milliseconds 250
& $ProbeScript -ProcessId $ProcessId
if (-not (Select-String -LiteralPath $Report -SimpleMatch '│▸ Ratatui FFI' -Quiet)) {
    throw 'Down Arrow did not move the focused table selection.'
}
& $KeyTool $ProcessId f6
& $KeyTool $ProcessId tab

Write-Host 'PASS: menu, F6 focus, and +/- collapsible tree navigation work in the live Harbour console.'
