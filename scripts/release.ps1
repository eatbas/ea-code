# Maestro Release Script (PowerShell)
# Usage: .\scripts\release.ps1 [version|patch|minor|major]

param(
    [Parameter(Position = 0)]
    [string]$Arg = "patch"
)

$ErrorActionPreference = "Stop"

$Root = git rev-parse --show-toplevel 2>$null
if (-not $Root) {
    Write-Host "Error: Not in a git repository" -ForegroundColor Red
    exit 1
}

$TauriPath = Join-Path $Root "frontend/desktop/src-tauri/tauri.conf.json"
$CargoPath = Join-Path $Root "frontend/desktop/src-tauri/Cargo.toml"
$PackagePath = Join-Path $Root "frontend/desktop/package.json"

$TauriConf = Get-Content $TauriPath -Raw | ConvertFrom-Json
$CurrentVersion = $TauriConf.version

$PackageConf = Get-Content $PackagePath -Raw | ConvertFrom-Json
$PackageCurrent = $PackageConf.version

$CargoCurrent = $null
foreach ($line in Get-Content $CargoPath) {
    if ($line -match '^version\s*=\s*"([^"]+)"') {
        $CargoCurrent = $Matches[1]
        break
    }
}

# Fail-fast if the three version files disagree. Silent drift is how a previous
# release shipped with Cargo.toml pinned to an older version — the regex
# replacement below only fires when the "from" version matches, so mismatched
# sources silently produced no change.
if (-not $CurrentVersion -or -not $CargoCurrent -or -not $PackageCurrent) {
    Write-Host "Error: Could not read a version from one of the sources:" -ForegroundColor Red
    Write-Host "  tauri.conf.json : '$CurrentVersion'" -ForegroundColor Red
    Write-Host "  Cargo.toml      : '$CargoCurrent'" -ForegroundColor Red
    Write-Host "  package.json    : '$PackageCurrent'" -ForegroundColor Red
    exit 1
}

if ($CurrentVersion -ne $CargoCurrent -or $CurrentVersion -ne $PackageCurrent) {
    Write-Host "Error: Version sources are out of sync — aborting to avoid a partial bump." -ForegroundColor Red
    Write-Host "  tauri.conf.json : $CurrentVersion" -ForegroundColor Red
    Write-Host "  Cargo.toml      : $CargoCurrent" -ForegroundColor Red
    Write-Host "  package.json    : $PackageCurrent" -ForegroundColor Red
    Write-Host "Sync them manually to the same value, commit, then re-run this script." -ForegroundColor Red
    exit 1
}

$Parts = $CurrentVersion.Split(".")
$Major = [int]$Parts[0]
$Minor = [int]$Parts[1]
$Patch = [int]$Parts[2]

if ($Arg -match "^\d+\.\d+\.\d+$") {
    $Version = $Arg
} elseif ($Arg -eq "patch") {
    $Version = "$Major.$Minor.$($Patch + 1)"
} elseif ($Arg -eq "minor") {
    $Version = "$Major.$($Minor + 1).0"
} elseif ($Arg -eq "major") {
    $Version = "$($Major + 1).0.0"
} else {
    Write-Host "Usage: .\scripts\release.ps1 [version|patch|minor|major]" -ForegroundColor Cyan
    exit 1
}

$Tag = "v$Version"

Write-Host "Current version: $CurrentVersion" -ForegroundColor Cyan
Write-Host "New version:     $Version  (tag: $Tag)" -ForegroundColor Green
Write-Host ""

$Status = git status --porcelain
if ($Status) {
    Write-Host "Error: You have uncommitted changes. Commit or stash them first." -ForegroundColor Red
    git status --short
    exit 1
}

$ExistingTag = git tag -l $Tag
if ($ExistingTag) {
    Write-Host "Error: Tag $Tag already exists" -ForegroundColor Red
    exit 1
}

$Confirm = Read-Host "Proceed? [Y/n]"
if ($Confirm -and $Confirm -ne "y" -and $Confirm -ne "Y") {
    Write-Host "Aborted."
    exit 0
}

$TauriContent = Get-Content $TauriPath -Raw
$TauriContent = $TauriContent -replace [regex]::Escape("`"version`": `"$CurrentVersion`""), "`"version`": `"$Version`""
Set-Content $TauriPath $TauriContent -NoNewline
Write-Host "[1/6] Bumped frontend/desktop/src-tauri/tauri.conf.json" -ForegroundColor Green

$CargoContent = Get-Content $CargoPath -Raw
$CargoContent = $CargoContent -replace "(?m)^(version\s*=\s*)`"$([regex]::Escape($CurrentVersion))`"", "`$1`"$Version`""
Set-Content $CargoPath $CargoContent -NoNewline
Write-Host "[2/6] Bumped frontend/desktop/src-tauri/Cargo.toml" -ForegroundColor Green

$PackageContent = Get-Content $PackagePath -Raw
$PackageContent = $PackageContent -replace [regex]::Escape("`"version`": `"$CurrentVersion`""), "`"version`": `"$Version`""
Set-Content $PackagePath $PackageContent -NoNewline
Write-Host "[3/6] Bumped frontend/desktop/package.json" -ForegroundColor Green

# Verify all three files actually landed on $Version before committing.
$TauriAfter = (Get-Content $TauriPath -Raw | ConvertFrom-Json).version
$PackageAfter = (Get-Content $PackagePath -Raw | ConvertFrom-Json).version
$CargoAfter = $null
foreach ($line in Get-Content $CargoPath) {
    if ($line -match '^version\s*=\s*"([^"]+)"') {
        $CargoAfter = $Matches[1]
        break
    }
}

if ($TauriAfter -ne $Version -or $CargoAfter -ne $Version -or $PackageAfter -ne $Version) {
    Write-Host "Error: Post-bump verification failed — files did not land on ${Version}:" -ForegroundColor Red
    Write-Host "  tauri.conf.json : $TauriAfter" -ForegroundColor Red
    Write-Host "  Cargo.toml      : $CargoAfter" -ForegroundColor Red
    Write-Host "  package.json    : $PackageAfter" -ForegroundColor Red
    Write-Host "No commit or tag created. Restore and investigate." -ForegroundColor Red
    exit 1
}

git add $TauriPath $CargoPath $PackagePath
git commit -m "chore: bump version to $Version"
Write-Host "[4/6] Committed version bump" -ForegroundColor Green

git tag $Tag
Write-Host "[5/6] Created tag $Tag" -ForegroundColor Green

git push origin main --tags
Write-Host "[6/6] Pushed commit and tag to origin/main" -ForegroundColor Green

Write-Host ""
Write-Host "Release $Tag triggered. Monitor at:" -ForegroundColor Cyan
Write-Host "  https://github.com/eatbas/maestro/actions" -ForegroundColor Yellow
