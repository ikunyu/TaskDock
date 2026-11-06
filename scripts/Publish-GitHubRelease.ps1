param([switch]$Draft)
$ErrorActionPreference = 'Stop'
$taskdockRoot = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $taskdockRoot
# -Encoding utf8 is explicit: Windows PowerShell 5.1 otherwise decodes as ANSI,
# which corrupts the UTF-8 release notes inside latest.json.
$taskdockConfig = Get-Content -LiteralPath 'release-config.json' -Raw -Encoding utf8 | ConvertFrom-Json
if ($taskdockConfig.repository -ne 'ikunyu/TaskDock') { throw 'Unexpected repository' }
$taskdockGh = Join-Path $taskdockRoot '.tools/github-cli/bin/gh.exe'
if (-not (Test-Path -LiteralPath $taskdockGh)) { $taskdockGh = (Get-Command gh -ErrorAction Stop).Source }
$env:GH_CONFIG_DIR = Join-Path $taskdockRoot '.private/gh'
if (-not $env:HTTPS_PROXY) {
    $taskdockProxy = [System.Net.WebRequest]::DefaultWebProxy.GetProxy([uri]'https://github.com')
    if ($taskdockProxy.Host -ne 'github.com') { $env:HTTPS_PROXY = $taskdockProxy.GetComponents([uriComponents]::SchemeAndServer,[uriFormat]::SafeUnescaped) }
}
& $taskdockGh auth status --hostname github.com
if ($LASTEXITCODE -ne 0) { throw 'Sign in with gh auth login first. Never embed GitHub tokens in the app.' }
$taskdockManifest = Get-Content -LiteralPath 'artifacts/release/latest.json' -Raw -Encoding utf8 | ConvertFrom-Json
if ($taskdockManifest.version -ne "1.0.0+$($taskdockConfig.build)") { throw 'Build and release manifest differ' }
if ($taskdockManifest.appVersion -ne $taskdockConfig.appVersion) { throw 'Application and release manifest differ' }
& node (Join-Path $PSScriptRoot 'verify-update.mjs')
if ($LASTEXITCODE -ne 0) { throw 'Update signature verification failed' }
$taskdockTag = "build-$($taskdockConfig.build)"
# Only a fixed list of public updater artifacts is uploaded. No source archive,
# private signing key, credentials, database, backup or data folder is included.
$taskdockFiles = @('TaskDock_1.0.0_x64-setup.exe','TaskDock_1.0.0_x64-setup.exe.sig','latest.json','UPDATE-SHA256SUMS.txt') | ForEach-Object { Join-Path 'artifacts/release' $_ }
if ($taskdockConfig.appVersion -ne '1.0.0') {
    $taskdockFiles += @("TaskDock_$($taskdockConfig.appVersion)_x64-setup.exe", "TaskDock_$($taskdockConfig.appVersion)_x64-setup.exe.sig") | ForEach-Object { Join-Path 'artifacts/release' $_ }
}
foreach ($taskdockFile in $taskdockFiles) { if (-not (Test-Path -LiteralPath $taskdockFile)) { throw "Missing release file: $taskdockFile" } }
& $taskdockGh release create $taskdockTag @taskdockFiles --repo 'ikunyu/TaskDock' --target main --title "TaskDock $($taskdockConfig.appVersion) ($($taskdockConfig.build))" --notes-file release-notes.md --draft
if ($LASTEXITCODE -ne 0) { throw 'Release upload failed; existing releases were not overwritten' }
if (-not $Draft) {
    & $taskdockGh release edit $taskdockTag --repo 'ikunyu/TaskDock' --draft=false --latest
    if ($LASTEXITCODE -ne 0) { throw 'Draft is uploaded but was not published' }
}
