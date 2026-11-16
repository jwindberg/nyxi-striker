# Build the new app (app/) and install it for this user on Windows. Rerun to upgrade.
# Needs Rust, Node.js/npm and the Visual Studio C++ build tools.
# Run from PowerShell: powershell -ExecutionPolicy Bypass -File install-app.ps1
$ErrorActionPreference = "Stop"
$Root = $PSScriptRoot

foreach ($tool in "cargo", "npm") {
    if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
        throw "$tool not found; install Rust (rustup) and Node.js first"
    }
}

Push-Location "$Root\app"
try {
    npm ci --no-audit --no-fund
    if ($LASTEXITCODE) { throw "npm ci failed" }
    npx tauri build --bundles nsis
    if ($LASTEXITCODE) { throw "build failed" }
} finally {
    Pop-Location
}

# Older versions installed as "NYXI"; uninstall that copy so only one app remains. Its uninstaller
# keeps the saved screens and pictures unless asked otherwise, which /S never does.
$Old = Get-ItemProperty "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\NYXI" -ErrorAction SilentlyContinue
if ($Old -and $Old.UninstallString) {
    Start-Process ($Old.UninstallString.Trim('"')) -ArgumentList "/S" -Wait
}

# The NSIS installer installs for the current user; /S runs it without prompts.
$Setup = Get-ChildItem "$Root\target\release\bundle\nsis\*-setup.exe" |
    Sort-Object LastWriteTime | Select-Object -Last 1
Start-Process $Setup.FullName -ArgumentList "/S" -Wait
Write-Host "Installed. Start NYXI Striker from the Start menu."
