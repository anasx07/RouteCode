$repo = "anasx07/routecode"
$latestRelease = Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/releases/latest"
$tag = $latestRelease.tag_name

$assetName = "routecode-windows-x86_64.exe"
$url = "https://github.com/$repo/releases/download/$tag/$assetName"

$installDir = "$HOME\.routecode\bin"
if (!(Test-Path $installDir)) {
    New-Item -ItemType Directory -Path $installDir | Out-Null
}

$destPath = "$installDir\routecode.exe"

Write-Host "Downloading RouteCode $tag..." -ForegroundColor Cyan
Invoke-WebRequest -Uri $url -OutFile $destPath

# Add to PATH for current session if not already there
if ($env:PATH -notlike "*$installDir*") {
    $env:PATH += ";$installDir"
}

# Add to User PATH permanently if not already there
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$installDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$userPath;$installDir", "User")
    Write-Host "Added $installDir to User PATH." -ForegroundColor Yellow
}

Write-Host "RouteCode installed successfully!" -ForegroundColor Green
Write-Host "You may need to restart your terminal for PATH changes to take effect." -ForegroundColor Gray
& routecode --version
