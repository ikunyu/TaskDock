param([switch]$NextBuild, [switch]$ci)
$ErrorActionPreference = 'Stop'
$taskdockRoot = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $taskdockRoot
. (Join-Path $PSScriptRoot 'Use-LocalToolchain.ps1')
$taskdockConfigPath = Join-Path $taskdockRoot 'release-config.json'
$taskdockConfig = Get-Content -LiteralPath $taskdockConfigPath -Raw | ConvertFrom-Json
if ($NextBuild) {
    $taskdockConfig.build = [long]$taskdockConfig.build + 1
    [System.IO.File]::WriteAllText($taskdockConfigPath, ($taskdockConfig | ConvertTo-Json) + "`n", [System.Text.UTF8Encoding]::new($false))
}
$taskdockKey = Join-Path $taskdockRoot '.private/taskdock-updater.key'
if (-not (Test-Path -LiteralPath $taskdockKey)) { throw 'Missing .private/taskdock-updater.key. Restore the original signing key; do not generate a new one for an existing release channel.' }
$taskdockOldKey = $env:TAURI_SIGNING_PRIVATE_KEY
$taskdockOldPassword = $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD
try {
    $env:TAURI_SIGNING_PRIVATE_KEY = $taskdockKey
    $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = ''
    & npm run package:windows:raw -- --ci
    if ($LASTEXITCODE -ne 0) { throw 'Windows build failed' }
    $taskdockInstallerName = "TaskDock_$($taskdockConfig.appVersion)_x64-setup.exe"
    $taskdockInstaller = Join-Path $taskdockRoot "src-tauri/target/release/bundle/nsis/$taskdockInstallerName"
    # This wire version and the legacy asset alias must remain compatible with
    # the already-installed 1.0.0 updater. The actual app version is independent.
    $taskdockVersion = "1.0.0+$($taskdockConfig.build)"
    # Signed version binds the fixed public version AND its internal build ID.
    $env:TAURI_SIGNING_PRIVATE_KEY = $null
    & npx tauri signer sign --private-key-path $taskdockKey --app-version $taskdockVersion $taskdockInstaller
    if ($LASTEXITCODE -ne 0) { throw 'Version-bound updater signing failed' }
    $taskdockRelease = Join-Path $taskdockRoot 'artifacts/release'
    New-Item -ItemType Directory -Path $taskdockRelease -Force | Out-Null
    Copy-Item -LiteralPath $taskdockInstaller -Destination (Join-Path $taskdockRelease 'TaskDock_1.0.0_x64-setup.exe') -Force
    Copy-Item -LiteralPath "$taskdockInstaller.sig" -Destination (Join-Path $taskdockRelease 'TaskDock_1.0.0_x64-setup.exe.sig') -Force
    if ($taskdockInstallerName -ne 'TaskDock_1.0.0_x64-setup.exe') {
        Copy-Item -LiteralPath $taskdockInstaller -Destination (Join-Path $taskdockRelease $taskdockInstallerName) -Force
        Copy-Item -LiteralPath "$taskdockInstaller.sig" -Destination (Join-Path $taskdockRelease "$taskdockInstallerName.sig") -Force
    }
    Copy-Item -LiteralPath (Join-Path $taskdockRoot 'src-tauri/target/release/taskdock.exe') -Destination (Join-Path $taskdockRelease 'TaskDock.exe') -Force
    & python (Join-Path $PSScriptRoot 'prepare-update.py')
    if ($LASTEXITCODE -ne 0) { throw 'Update manifest preparation failed' }
    & node (Join-Path $PSScriptRoot 'verify-update.mjs')
    if ($LASTEXITCODE -ne 0) { throw 'Update signature verification failed' }
} finally {
    $env:TAURI_SIGNING_PRIVATE_KEY = $taskdockOldKey
    $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = $taskdockOldPassword
}
