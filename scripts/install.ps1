# Wrath CLI installer for Windows
# Usage: irm https://raw.githubusercontent.com/AymericChaverot/wrath/main/scripts/install.ps1 | iex

$ErrorActionPreference = "Stop"

$Repo = "AymericChaverot/wrath"
$BinaryName = "wrath.exe"
$InstallDir = "$env:USERPROFILE\.wrath\bin"
$Target = "x86_64-pc-windows-msvc"

function Write-Info($msg) {
    Write-Host "[*] " -ForegroundColor Green -NoNewline
    Write-Host $msg
}

function Write-Warn($msg) {
    Write-Host "[!] " -ForegroundColor Yellow -NoNewline
    Write-Host $msg
}

function Write-Err($msg) {
    Write-Host "[x] " -ForegroundColor Red -NoNewline
    Write-Host $msg
    exit 1
}

function Get-LatestVersion {
    Write-Info "Fetching latest release..."
    try {
        $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" -Headers @{ "User-Agent" = "wrath-installer" }
        $version = $release.tag_name
        Write-Info "Latest version: $version"
        return $version
    }
    catch {
        Write-Err "Failed to fetch latest version: $_"
    }
}

function Install-Wrath {
    param([string]$Version)

    $DownloadUrl = "https://github.com/$Repo/releases/download/$Version/wrath-$Target.zip"
    $TempDir = Join-Path $env:TEMP "wrath-install"
    $ZipPath = Join-Path $TempDir "wrath.zip"

    # Clean up any previous temp files
    if (Test-Path $TempDir) {
        Remove-Item -Recurse -Force $TempDir
    }
    New-Item -ItemType Directory -Path $TempDir -Force | Out-Null

    Write-Info "Downloading from $DownloadUrl"
    try {
        Invoke-WebRequest -Uri $DownloadUrl -OutFile $ZipPath -UseBasicParsing
    }
    catch {
        Write-Err "Download failed. Check if a release exists for your platform: $Target"
    }

    Write-Info "Extracting..."
    Expand-Archive -Path $ZipPath -DestinationPath $TempDir -Force

    # Create install directory
    if (-not (Test-Path $InstallDir)) {
        New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    }

    Write-Info "Installing to $InstallDir\$BinaryName"
    Copy-Item -Path (Join-Path $TempDir $BinaryName) -Destination (Join-Path $InstallDir $BinaryName) -Force

    # Clean up
    Remove-Item -Recurse -Force $TempDir

    # Add to PATH if not already there
    $UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($UserPath -notlike "*$InstallDir*") {
        Write-Info "Adding $InstallDir to user PATH"
        [Environment]::SetEnvironmentVariable("Path", "$UserPath;$InstallDir", "User")
        $env:Path = "$env:Path;$InstallDir"
        Write-Warn "PATH updated. Restart your terminal for changes to take effect."
    }

    Write-Info "Installed wrath $Version to $InstallDir\$BinaryName"
}

function Test-Installation {
    $wrathPath = Join-Path $InstallDir $BinaryName
    if (Test-Path $wrathPath) {
        $version = & $wrathPath --version 2>&1
        Write-Info "Verification: $version"
        Write-Host ""
        Write-Host "Installation complete. " -ForegroundColor Green -NoNewline
        Write-Host "Run 'wrath --help' to get started."
    }
    else {
        Write-Warn "Binary not found at expected location."
    }
}

# Main
Write-Host ""
Write-Host "  ========================================" -ForegroundColor DarkGray
Write-Host "   W R A T H  -  Installer (Windows)" -ForegroundColor White
Write-Host "  ========================================" -ForegroundColor DarkGray
Write-Host ""

$LatestVersion = Get-LatestVersion
Install-Wrath -Version $LatestVersion
Test-Installation
