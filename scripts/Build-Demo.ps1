[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [ValidateSet('zig64', 'mingw64', 'mingw32', 'showcase')]
    [string] $Profile,

    [Parameter(Mandatory)]
    [string] $HarbourRoot,

    [string] $CompilerBin,

    [switch] $SkipRust,
    [switch] $Run
)

$ErrorActionPreference = 'Stop'
$ProjectRoot = Split-Path -Parent $PSScriptRoot
$HarbourRoot = [System.IO.Path]::GetFullPath($HarbourRoot)
$Hbmk2 = Join-Path $HarbourRoot 'bin\hbmk2.exe'

if (-not (Test-Path -LiteralPath $Hbmk2 -PathType Leaf)) {
    throw "hbmk2.exe was not found under '$HarbourRoot\bin'."
}

if ($CompilerBin) {
    $CompilerBin = [System.IO.Path]::GetFullPath($CompilerBin)
    if (-not (Test-Path -LiteralPath $CompilerBin -PathType Container)) {
        throw "Compiler directory '$CompilerBin' does not exist."
    }
    $env:PATH = "$CompilerBin;$env:PATH"
}

$Profiles = @{
    zig64 = @{
        CargoTarget = $null
        Compiler = 'zig'
        Cpu = 'x86_64'
        Project = 'demo_zig64.hbp'
        OutputName = 'demo_zig64'
        Program = 'demo_zig64.exe'
    }
    mingw64 = @{
        CargoTarget = $null
        Compiler = 'mingw64'
        Cpu = 'x86_64'
        Project = 'demo_mingw64.hbp'
        OutputName = 'demo_mingw64'
        Program = 'demo_mingw64.exe'
    }
    mingw32 = @{
        CargoTarget = 'i686-pc-windows-msvc'
        Compiler = 'mingw'
        Cpu = 'x86'
        Project = 'demo_mingw32.hbp'
        OutputName = 'demo_mingw32'
        Program = 'demo_mingw32.exe'
    }
    showcase = @{
        CargoTarget = $null
        Compiler = 'zig'
        Cpu = 'x86_64'
        Project = 'showcase.hbp'
        OutputName = 'showcase'
        Program = 'showcase.exe'
    }
}

$Config = $Profiles[$Profile]
$DemoDirectory = Join-Path $ProjectRoot "demos\$Profile"

if (-not $SkipRust) {
    Push-Location $ProjectRoot
    try {
        if ($null -eq $Config.CargoTarget) {
            & cargo build --release
            if ($LASTEXITCODE -ne 0) { throw 'The Rust x64 DLL build failed.' }
            $RustDll = Join-Path $ProjectRoot 'target\release\harbour_ratatui.dll'
        }
        else {
            $InstalledTargets = & rustup target list --installed
            if ($InstalledTargets -notcontains $Config.CargoTarget) {
                throw "Missing Rust target '$($Config.CargoTarget)'. Install it with: rustup target add $($Config.CargoTarget)"
            }
            & cargo build --release --target $Config.CargoTarget
            if ($LASTEXITCODE -ne 0) { throw 'The Rust x86 DLL build failed.' }
            $RustDll = Join-Path $ProjectRoot "target\$($Config.CargoTarget)\release\harbour_ratatui.dll"
        }
    }
    finally {
        Pop-Location
    }
}
else {
    $RustDll = Join-Path $DemoDirectory 'harbour_ratatui.dll'
}

if (-not (Test-Path -LiteralPath $RustDll -PathType Leaf)) {
    throw "Ratatui DLL was not found at '$RustDll'."
}

if (-not $SkipRust) {
    Copy-Item -LiteralPath $RustDll -Destination (Join-Path $DemoDirectory 'harbour_ratatui.dll') -Force
}

if ($Profile -eq 'zig64' -or $Profile -eq 'showcase') {
    $env:ZIG_LOCAL_CACHE_DIR = Join-Path $ProjectRoot '.cache\zig-local'
    $env:ZIG_GLOBAL_CACHE_DIR = Join-Path $ProjectRoot '.cache\zig-global'
}

Push-Location $DemoDirectory
try {
    & $Hbmk2 $Config.Project "-comp=$($Config.Compiler)" "-cpu=$($Config.Cpu)" "-o$($Config.OutputName)"
    if ($LASTEXITCODE -ne 0) { throw "Harbour $Profile demo build failed." }

    if ($Run) {
        & (Join-Path $DemoDirectory $Config.Program)
        if ($LASTEXITCODE -ne 0) { throw "The $Profile demo returned exit code $LASTEXITCODE." }
    }
}
finally {
    Pop-Location
}

Write-Host "Built $Profile demo: $(Join-Path $DemoDirectory $Config.Program)"
