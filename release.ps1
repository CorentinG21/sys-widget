# SysmonWidget release script — semantic versioning
#
# Usage:
#   .\release.ps1 patch "fix: description"   # x.y.Z+1
#   .\release.ps1 minor "feat: description"  # x.Y+1.0
#   .\release.ps1 major "feat: description"  # X+1.0.0
#   .\release.ps1 2.5.0 "description"        # exact version (escape hatch)
#
# What it does:
#   1. Reads current version from tauri.conf.json
#   2. Calculates next version (or uses the one you gave)
#   3. Bumps tauri.conf.json + Cargo.toml
#   4. git commit + push + tag => triggers GitHub Actions

param(
    [Parameter(Mandatory=$true)]  [string]$Bump,
    [Parameter(Mandatory=$false)] [string]$Message
)

$ErrorActionPreference = "Stop"

# ── Read current version ──────────────────────────────────────────────────────
$confPath  = "src-tauri\tauri.conf.json"
$conf      = Get-Content $confPath -Raw
if ($conf -notmatch '"version":\s*"(\d+)\.(\d+)\.(\d+)"') {
    Write-Error "Could not read version from $confPath"
    exit 1
}
$curMajor = [int]$Matches[1]
$curMinor = [int]$Matches[2]
$curPatch = [int]$Matches[3]
$current  = "$curMajor.$curMinor.$curPatch"

# ── Calculate next version ────────────────────────────────────────────────────
switch ($Bump.ToLower()) {
    "major" { $next = "$($curMajor + 1).0.0" }
    "minor" { $next = "$curMajor.$($curMinor + 1).0" }
    "patch" { $next = "$curMajor.$curMinor.$($curPatch + 1)" }
    default {
        # Exact version passed (e.g. "2.5.0")
        if ($Bump -notmatch '^\d+\.\d+\.\d+$') {
            Write-Error "First argument must be: patch | minor | major | X.Y.Z"
            exit 1
        }
        $next = $Bump
    }
}

if (-not $Message) { $Message = "chore: release v$next" }
$tag = "v$next"

Write-Host "  $current  -->  $next  ($tag)" -ForegroundColor Cyan
Write-Host ""

# UTF-8 without BOM — PowerShell 5.1 -Encoding utf8 adds a BOM which breaks JSON parsers
$utf8NoBom = [System.Text.UTF8Encoding]::new($false)

# ── Bump tauri.conf.json ──────────────────────────────────────────────────────
$conf = $conf -replace '"version":\s*"\d+\.\d+\.\d+"', """version"": ""$next"""
[System.IO.File]::WriteAllText((Resolve-Path $confPath).Path, $conf, $utf8NoBom)
Write-Host "  tauri.conf.json updated"

# ── Bump Cargo.toml ([package] section only) ──────────────────────────────────
$cargoPath = "src-tauri\Cargo.toml"
$cargo     = Get-Content $cargoPath -Raw
$cargo = [regex]::Replace($cargo, '(?m)^version = "\d+\.\d+\.\d+"', "version = ""$next""")
[System.IO.File]::WriteAllText((Resolve-Path $cargoPath).Path, $cargo, $utf8NoBom)
Write-Host "  Cargo.toml updated"

# ── Git: commit + push + tag ──────────────────────────────────────────────────
# Stage ALL tracked modified files (not just the version files)
git add -u
git commit -m $Message
git push origin main
Write-Host "  Pushed to main"

git tag $tag
git push origin $tag
Write-Host "  Tagged $tag"

Write-Host ""
Write-Host "Done! GitHub Actions is building the release." -ForegroundColor Green
Write-Host "  https://github.com/CorentinG21/sys-widget/actions"
