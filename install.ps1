# Loom installer — Windows (PowerShell)
# Usage: irm https://raw.githubusercontent.com/YOUR_USERNAME/loom/main/install.ps1 | iex
#Requires -Version 5.1
$ErrorActionPreference = "Stop"

$REPO        = "anasx07/loom"
$BINARY      = "loom"
$ASSET       = "loom-windows-x86_64.exe"
$INSTALL_DIR = "$env:LOCALAPPDATA\Programs\loom"

function Write-Info    { Write-Host "[loom] $args" -ForegroundColor Cyan }
function Write-Success { Write-Host "[loom] $args" -ForegroundColor Green }
function Write-Warn    { Write-Host "[loom] $args" -ForegroundColor Yellow }
function Write-Fail    { Write-Host "[loom] $args" -ForegroundColor Red; exit 1 }

# ── Resolve latest release ────────────────────
Write-Info "Fetching latest release..."
try {
  $release = Invoke-RestMethod "https://api.github.com/repos/$REPO/releases/latest"
  $LATEST  = $release.tag_name
} catch {
  Write-Fail "Could not fetch latest release: $_"
}

if (-not $LATEST) { Write-Fail "Could not determine latest release." }

$URL  = "https://github.com/$REPO/releases/download/$LATEST/$ASSET"
$DEST = "$INSTALL_DIR\$BINARY.exe"

Write-Info "Installing loom $LATEST..."

# ── Download ──────────────────────────────────
New-Item -ItemType Directory -Force -Path $INSTALL_DIR | Out-Null

$TMP = [System.IO.Path]::GetTempFileName() + ".exe"
try {
  $client = New-Object System.Net.WebClient
  $client.DownloadFile($URL, $TMP)
} catch {
  Write-Fail "Download failed from $URL`n$_"
}

Move-Item -Force $TMP $DEST
Write-Success "loom installed to $DEST"

# ── PATH ──────────────────────────────────────
$userPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if (-not $userPath) { $userPath = "" }

if (($userPath -split ";") -notcontains $INSTALL_DIR) {
  $newPath = ($userPath.TrimEnd(";") + ";$INSTALL_DIR").TrimStart(";")
  [Environment]::SetEnvironmentVariable("PATH", $newPath, "User")
  Write-Success "Added $INSTALL_DIR to your user PATH."
  Write-Warn    "Restart your terminal for 'loom' to work."
} else {
  Write-Success "Already on PATH."
}

Write-Success "Done! Type 'loom' to get started."
