<#
.SYNOPSIS
    Run Vitest from an ASCII-only drive mapping.

.DESCRIPTION
    When the project path contains non-ASCII characters (for example Chinese)
    or spaces, Vitest 1.6 on Windows fails to register any test and reports
    "No test suite found in file" for every test file.

    This script maps the project directory to an ASCII drive letter with
    `subst`, runs Vitest from that drive, then removes the mapping.
    Together with `resolve.preserveSymlinks: true` in vitest.config.ts this
    keeps module ids stable so `setupFiles` loads correctly.

    NOTE: This file is intentionally ASCII-only, because Windows PowerShell 5.1
    reads BOM-less .ps1 files as ANSI and would mangle non-ASCII characters.

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts/run-tests-ascii.ps1
    powershell -ExecutionPolicy Bypass -File scripts/run-tests-ascii.ps1 -Target tests/l6-ipc
#>
param(
    # Test paths passed to vitest (defaults to every runnable suite)
    [string[]]$Target = @('src/', 'tests/l3-fullstack', 'tests/l5-chaos', 'tests/l6-ipc'),
    # Keep the drive mapping after the run (useful for manual debugging)
    [switch]$KeepMapping,
    # Drive letter to use
    [string]$Drive = 'X:'
)

$ErrorActionPreference = 'Stop'

$projectRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path

function Invoke-Vitest {
    param([string[]]$Targets)
    $vitestArgs = @('vitest', 'run') + $Targets
    Write-Host "[run-tests-ascii] npx $($vitestArgs -join ' ')"
    & npx @vitestArgs
    return $LASTEXITCODE
}

# Already ASCII: run directly, no mapping needed
if ($projectRoot -match '^[\x20-\x7E]+$') {
    Write-Host "[run-tests-ascii] Project path is ASCII already: $projectRoot"
    Push-Location $projectRoot
    try {
        $code = Invoke-Vitest -Targets $Target
    } finally {
        Pop-Location
    }
    exit $code
}

if (Test-Path "$Drive\") {
    Write-Host "[run-tests-ascii] $Drive is in use, removing the existing mapping first"
    & subst $Drive /D 2>$null | Out-Null
}

Write-Host "[run-tests-ascii] Mapping $Drive -> $projectRoot"
& subst $Drive $projectRoot
if ($LASTEXITCODE -ne 0) {
    throw "subst failed. Move the project to an ASCII-only path, or pick another drive with -Drive Y:"
}

$exitCode = 1
try {
    Push-Location "$Drive\"
    $exitCode = Invoke-Vitest -Targets $Target
} finally {
    Pop-Location
    if ($KeepMapping) {
        Write-Host "[run-tests-ascii] Kept the $Drive mapping"
    } else {
        & subst $Drive /D | Out-Null
        Write-Host "[run-tests-ascii] Removed the $Drive mapping"
    }
}

exit $exitCode
