param(
    [Parameter(Mandatory = $true, Position = 0)]
    [string]$Version,

    [Parameter(Mandatory = $false, Position = 1)]
    [int]$BuildNumber = 0
)

$ErrorActionPreference = "Stop"

if ($Version.StartsWith("v")) {
    $Version = $Version.Substring(1)
}

if ($Version -notmatch '^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$') {
    throw "Invalid SemVer version '$Version'. Expected X.Y.Z or X.Y.Z-prerelease."
}

$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$CargoTomlPath = Join-Path $RepoRoot "Cargo.toml"
$PubspecPath = Join-Path $RepoRoot "app\pubspec.yaml"

if (-not (Test-Path $CargoTomlPath)) { throw "Missing Cargo.toml at $CargoTomlPath" }
if (-not (Test-Path $PubspecPath)) { throw "Missing app/pubspec.yaml at $PubspecPath" }

$PubspecRaw = Get-Content -Raw -Path $PubspecPath
if ($PubspecRaw -notmatch '(?m)^version:\s*([^\r\n+]+)(?:\+(\d+))?\s*$') {
    throw "Could not locate 'version:' line in $PubspecPath"
}

$CurrentBuild = if ($Matches[2]) { [int]$Matches[2] } else { 1 }
if ($BuildNumber -le 0) {
    $BuildNumber = $CurrentBuild + 1
}

$CargoRaw = Get-Content -Raw -Path $CargoTomlPath
$CargoPattern = '(?s)(\[workspace\.package\]\r?\n(?:[^\[]*?\r?\n)?version\s*=\s*")([^"]+)(")'
if ($CargoRaw -notmatch $CargoPattern) {
    throw "Could not locate [workspace.package] version in $CargoTomlPath"
}
$CargoUpdated = [regex]::Replace($CargoRaw, $CargoPattern, "`${1}$Version`${3}", 1)
$CargoUpdated = $CargoUpdated -replace "`r`n", "`n"
[System.IO.File]::WriteAllText($CargoTomlPath, $CargoUpdated, [System.Text.UTF8Encoding]::new($false))

$NewPubspecVersion = "$Version+$BuildNumber"
$PubspecUpdated = [regex]::Replace($PubspecRaw, '(?m)^version:\s*[^\r\n]+', "version: $NewPubspecVersion", 1)
$PubspecUpdated = $PubspecUpdated -replace "`r`n", "`n"
[System.IO.File]::WriteAllText($PubspecPath, $PubspecUpdated, [System.Text.UTF8Encoding]::new($false))

Write-Output "Updated version across workspace:"
Write-Output "  Cargo.toml:       $Version"
Write-Output "  app/pubspec.yaml: $NewPubspecVersion"
Write-Output ""
Write-Output "Next steps:"
Write-Output "  git add Cargo.toml app/pubspec.yaml"
Write-Output "  git commit -m `"chore(release): bump version to v$Version`""
Write-Output "  git tag -a v$Version -m `"Release v$Version`""
Write-Output "  git push origin main --follow-tags"
