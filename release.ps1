# SysmonWidget release script
# Usage: .\release.ps1 2.3.1 "description du changement"
#
# Ce script :
#   1. Bumpe la version dans tauri.conf.json + Cargo.toml
#   2. Commit + push sur main
#   3. Crée le tag vX.X.X + push → déclenche GitHub Actions

param(
    [Parameter(Mandatory=$true)]
    [string]$Version,

    [Parameter(Mandatory=$false)]
    [string]$Message = "chore: release v$Version"
)

$ErrorActionPreference = "Stop"

# Validate semver format
if ($Version -notmatch '^\d+\.\d+\.\d+$') {
    Write-Error "Version must be in format X.Y.Z (e.g. 2.3.1)"
    exit 1
}

$TagName = "v$Version"

Write-Host "🚀 Releasing $TagName..." -ForegroundColor Cyan

# 1. Bump tauri.conf.json
$confPath = "src-tauri\tauri.conf.json"
$conf = Get-Content $confPath -Raw
$conf = $conf -replace '"version": "\d+\.\d+\.\d+"', """version"": ""$Version"""
Set-Content $confPath $conf -NoNewline
Write-Host "  ✓ tauri.conf.json → $Version"

# 2. Bump Cargo.toml (only the [package] version, not dependency versions)
$cargoPath = "src-tauri\Cargo.toml"
$cargo = Get-Content $cargoPath -Raw
$cargo = $cargo -replace '^version = "\d+\.\d+\.\d+"', "version = ""$Version"""
Set-Content $cargoPath $cargo -NoNewline
Write-Host "  ✓ Cargo.toml → $Version"

# 3. Add + commit + push
git add src-tauri/tauri.conf.json src-tauri/Cargo.toml
git commit -m $Message
git push origin main
Write-Host "  ✓ Pushed to main"

# 4. Tag + push tag
git tag $TagName
git push origin $TagName
Write-Host "  ✓ Tagged $TagName and pushed"

Write-Host ""
Write-Host "✅ Done! GitHub Actions is building the release." -ForegroundColor Green
Write-Host "   https://github.com/CorentinG21/sys-widget/actions"
