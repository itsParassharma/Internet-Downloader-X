# SPDX-License-Identifier: MIT
[CmdletBinding()]
param(
    [string] $Target,
    [string] $IsccPath = $env:IDX_ISCC,
    [switch] $SkipInstaller,
    [switch] $RunTests
)

$ErrorActionPreference = 'Stop'
$idxSourceRoot = $PSScriptRoot
$idxOriginalRustFlags = $env:RUSTFLAGS
$idxOriginalEncodedRustFlags = $env:CARGO_ENCODED_RUSTFLAGS
$idxOriginalLibraryPath = $env:LIBRARY_PATH
$idxOriginalGnuLinker = $env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER
$idxOriginalWindres = $env:IDX_WINDRES
$idxOriginalPath = $env:PATH

function Find-IdxLlvmMsvcrt {
    param([string] $ExplicitLinker)
    $idxCompilerCandidates = @()
    if ($ExplicitLinker) {
        $idxExplicitCommand = Get-Command $ExplicitLinker -ErrorAction Stop
        $idxExplicitBin = Split-Path -Parent $idxExplicitCommand.Source
        # Leave a caller-selected non-LLVM toolchain to Cargo unchanged.
        if (-not (Test-Path -LiteralPath (Join-Path $idxExplicitBin 'llvm-windres.exe') -PathType Leaf)) { return $null }
        $idxCompilerCandidates += $idxExplicitCommand.Source
    } else {
        foreach ($idxWindresCommand in @(Get-Command llvm-windres.exe -All -ErrorAction SilentlyContinue)) {
            $idxCompilerCandidates += Join-Path (Split-Path -Parent $idxWindresCommand.Source) 'x86_64-w64-mingw32-gcc.exe'
        }
    }
    foreach ($idxCompilerPath in $idxCompilerCandidates) {
        if (-not (Test-Path -LiteralPath $idxCompilerPath -PathType Leaf)) { continue }
        $idxMacros = '' | & $idxCompilerPath -dM -E -x c -include _mingw.h - 2>$null
        if ($LASTEXITCODE -ne 0) { continue }
        $idxMacroText = $idxMacros -join "`n"
        if ($idxMacroText -match '(?m)^#define _UCRT(?:\s|$)' -or $idxMacroText -notmatch '(?m)^#define __MSVCRT_VERSION__ ') { continue }
        $idxBinDirectory = Split-Path -Parent $idxCompilerPath
        $idxLibraries = Join-Path (Split-Path -Parent $idxBinDirectory) 'x86_64-w64-mingw32\lib'
        if (-not (Test-Path -LiteralPath (Join-Path $idxLibraries 'libshlwapi.a') -PathType Leaf)) { continue }
        return [pscustomobject]@{ Compiler = $idxCompilerPath; Bin = $idxBinDirectory; Libraries = $idxLibraries }
    }
    if ($idxCompilerCandidates.Count) {
        throw 'No compatible LLVM-MinGW MSVCRT distribution was found. Use the Windows x64 MSVCRT archive and keep its complete bin/include/lib layout. The UCRT distribution is incompatible with this validated GNU build configuration.'
    }
    return $null
}

Push-Location -LiteralPath $idxSourceRoot
try {
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        throw 'Install a Rust toolchain and the matching Windows linker, then place cargo on PATH.'
    }
    if (-not (Test-Path -LiteralPath (Join-Path $idxSourceRoot 'Cargo.lock'))) {
        throw 'Cargo.lock is missing. Restore it from the source release before building with --locked.'
    }
    $idxRustVersion = & rustc -vV
    if ($LASTEXITCODE -ne 0) { throw 'Cannot inspect the selected Rust compiler.' }
    $idxHostLine = $idxRustVersion | Where-Object { $_ -match '^host: ' } | Select-Object -First 1
    $idxHostTarget = $idxHostLine -replace '^host: ', ''
    $idxExplicitTarget = if ($Target) { $Target } elseif ($env:CARGO_BUILD_TARGET) { $env:CARGO_BUILD_TARGET } else { $null }
    $idxEffectiveTarget = if ($idxExplicitTarget) { $idxExplicitTarget } else { $idxHostTarget }
    $idxLlvmToolchain = if ($idxEffectiveTarget -eq 'x86_64-pc-windows-gnu') { Find-IdxLlvmMsvcrt $env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER } else { $null }
    if ($idxLlvmToolchain) {
        $env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER = $idxLlvmToolchain.Compiler
        $env:PATH = $idxLlvmToolchain.Bin + [IO.Path]::PathSeparator + $env:PATH
        $idxLibraryEntries = @($idxLlvmToolchain.Libraries) + @($env:LIBRARY_PATH -split [IO.Path]::PathSeparator | Where-Object { $_ -and $_ -ne $idxLlvmToolchain.Libraries })
        $env:LIBRARY_PATH = $idxLibraryEntries -join [IO.Path]::PathSeparator
        if (-not $env:IDX_WINDRES) { $env:IDX_WINDRES = Join-Path $idxLlvmToolchain.Bin 'llvm-windres.exe' }
        # LLVM-MinGW's import libraries can differ from Rust's GNU runtime. Keep Rust's
        # bundled GNU runtime libraries while using the available compiler tools.
        $idxExistingFlags = if ($env:CARGO_ENCODED_RUSTFLAGS) { $env:CARGO_ENCODED_RUSTFLAGS } else { $env:RUSTFLAGS }
        if ($idxExistingFlags -notmatch 'link-self-contained') {
            if ($env:CARGO_ENCODED_RUSTFLAGS) {
                $env:CARGO_ENCODED_RUSTFLAGS += [char]31 + '-Clink-self-contained=yes'
            } else {
                $env:RUSTFLAGS = (($env:RUSTFLAGS + ' -C link-self-contained=yes').Trim())
            }
        }
        Write-Host "Using LLVM-MinGW MSVCRT: $($idxLlvmToolchain.Compiler)"
    }
    $idxCargoArguments = @('build', '--release', '--locked', '--bins')
    if ($Target) { $idxCargoArguments += @('--target', $Target) }
    & cargo @idxCargoArguments
    if ($LASTEXITCODE -ne 0) { throw 'Rust release build failed.' }

    if ($RunTests) {
        $idxTestArguments = @('test', '--locked', '--bins')
        if ($Target) { $idxTestArguments += @('--target', $Target) }
        & cargo @idxTestArguments
        if ($LASTEXITCODE -ne 0) { throw 'Rust tests failed.' }
    }

    $idxTargetRoot = if ($env:CARGO_TARGET_DIR) { [IO.Path]::GetFullPath($env:CARGO_TARGET_DIR) } else { Join-Path $idxSourceRoot 'target' }
    $idxReleasePath = if ($idxExplicitTarget) { Join-Path $idxTargetRoot "$idxExplicitTarget\release" } else { Join-Path $idxTargetRoot 'release' }
    foreach ($idxBinary in @('idx.exe', 'idx-native-host.exe')) {
        if (-not (Test-Path -LiteralPath (Join-Path $idxReleasePath $idxBinary) -PathType Leaf)) { throw "Missing $idxBinary in $idxReleasePath" }
    }

    # Keep one packaging implementation for versioned archives and native-host setup.
    $idxPackageArguments = @((Join-Path $idxSourceRoot 'tools\package_release.py'), '--skip-build', '--release-dir', $idxReleasePath)
    if ($SkipInstaller) { $idxPackageArguments += '--skip-installer' }
    if ($IsccPath) { $idxPackageArguments += @('--iscc', $IsccPath) }
    & python @idxPackageArguments
    if ($LASTEXITCODE -ne 0) { throw 'Release packaging failed.' }

} finally {
    $env:RUSTFLAGS = $idxOriginalRustFlags
    $env:CARGO_ENCODED_RUSTFLAGS = $idxOriginalEncodedRustFlags
    $env:LIBRARY_PATH = $idxOriginalLibraryPath
    $env:CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER = $idxOriginalGnuLinker
    $env:IDX_WINDRES = $idxOriginalWindres
    $env:PATH = $idxOriginalPath
    Pop-Location
}
