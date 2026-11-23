param()
# Prepare the native build for a Windows release. Dot-source it before invoking
# the build so the linker and the temporary directory are visible to npm/tauri.
$ErrorActionPreference = 'Stop'
$taskdockRoot = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot 'Use-LocalToolchain.ps1')

# A copied .tools\vs layout is not registered with Visual Studio, so
# Launch-VsDevShell can report success while link.exe never reaches PATH. Fall
# back to the staged compiler that ships the binaries with the project.
if (-not (Get-Command link.exe -ErrorAction SilentlyContinue)) {
    $taskdockStaged = Join-Path $taskdockRoot '.tools/Use-StagedCompiler.ps1'
    if (Test-Path -LiteralPath $taskdockStaged) {
        . $taskdockStaged
    }
}
if (-not (Get-Command link.exe -ErrorAction SilentlyContinue)) {
    throw 'link.exe not found. Install the Visual Studio C++ build tools and Windows SDK, or restore .tools\Use-StagedCompiler.ps1 with its native-compiler payload.'
}

# link.exe writes lnk*.tmp into TMP; keep that inside the project so a restricted
# or unwritable %TEMP% cannot fail the link with LNK1104.
$taskdockTmp = Join-Path $taskdockRoot '.build-tmp'
New-Item -ItemType Directory -Path $taskdockTmp -Force | Out-Null
$env:TMP = $taskdockTmp
$env:TEMP = $taskdockTmp
