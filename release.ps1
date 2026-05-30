# SysmonWidget release script
# Usage: .\release.ps1 2.3.3 "description"
#
# 1. Bumpe tauri.conf.json + Cargo.toml
# 2. Commit + push main
# 3. Tag vX.X.X + push => GitHub Actions builds the release

param(
    [Parameter(Mandatory=$true)]  [string]$Version,
    [Parameter(Mandatory=$false)] [string]$Message = "chore: release v$Version"
)

$ErrorActionPreference = "Stop"

if ($Version -notmatch '^\d+\.\d+\.\d+$') {
    Write-Error "Version must be X.Y.Z (e.g. 2.3.3)"
    exit 1
}

$TagName = "v$Version"
Write-Host "Releasing $TagName ..." -ForegroundColor Cyan

# 1. Bump tauri.conf.json
$confPath = "src-tauri\tauri.conf.json"
$conf = Get-Content $confPath -Raw
$conf = $conf -replace '"version": "\d+\.\d+\.\d+"', """version"": ""$Version"""
Set-Content $confPath $conf -NoNewline -Encoding utf8
Write-Host "  tauri.conf.json -> $Version"

# 2. Bump Cargo.toml ([package] version only)
$cargoPath = "src-tauri\Cargo.toml"
$cargo = Get-Content $cargoPath -Raw
$cargo = $cargo -replace '(?m)^version = "\d+\.\d+\.\d+"', "version = ""$Version"""
Set-Content $cargoPath $cargo -NoNewline -Encoding utf8
Write-Host "  Cargo.toml -> $Version"

# 3. Commit + push
git add src-tauri/tauri.conf.json src-tauri/Cargo.toml
git commit -m $Message
git push origin main
Write-Host "  Pushed to main"

# 4. Tag + push
git tag $TagName
git push origin $TagName
Write-Host "  Tagged $TagName"

Write-Host ""
Write-Host "Done! Check GitHub Actions for the build:" -ForegroundColor Green
Write-Host "  https://github.com/CorentinG21/sys-widget/actions"
