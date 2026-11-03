# Dot-source this file to enable project-local tools in the current shell only.
$taskDockRoot = Split-Path -Parent $PSScriptRoot
$taskDockTools = Join-Path $taskDockRoot '.tools'
if (Test-Path -LiteralPath (Join-Path $taskDockTools 'cargo\bin\cargo.exe')) {
  $env:CARGO_HOME = Join-Path $taskDockTools 'cargo'
  $env:RUSTUP_HOME = Join-Path $taskDockTools 'rustup'
  $env:PATH = "$(Join-Path $taskDockTools 'cargo\bin');$env:PATH"
}
$taskDockVsDev = Join-Path $taskDockTools 'vs\Common7\Tools\Launch-VsDevShell.ps1'
if (Test-Path -LiteralPath $taskDockVsDev) {
  & $taskDockVsDev -Arch amd64 -HostArch amd64 -SkipAutomaticLocation | Out-Null
}
