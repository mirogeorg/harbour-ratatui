[CmdletBinding()]
param(
    [Parameter(Mandatory)] [string] $HarbourRoot,
    [string] $CompilerBin,
    [switch] $SkipRust,
    [switch] $Run
)

& "$PSScriptRoot\..\..\scripts\Build-Demo.ps1" `
    -Profile zig64 -HarbourRoot $HarbourRoot -CompilerBin $CompilerBin `
    -SkipRust:$SkipRust -Run:$Run
