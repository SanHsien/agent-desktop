[CmdletBinding()]
param(
    [switch]$Quick,
    [switch]$Full
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $repoRoot

$env:PYTHONUTF8 = "1"
$env:PYTHONIOENCODING = "utf-8"

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host " [SanHsien/agent-desktop] Fork Developer Quality Gate" -ForegroundColor Cyan
Write-Host "==========================================================" -ForegroundColor Cyan

# 1. 檢查 Git Remote 與預設倉庫設定
Write-Host "==> Checking Git Remotes and gh default repo..." -ForegroundColor Yellow
$remotes = (git remote -v | Out-String)
if ($remotes -notmatch "SanHsien/agent-desktop") {
    throw "origin must point to SanHsien/agent-desktop. Current remotes:`n$remotes"
}

if (Get-Command gh -ErrorAction SilentlyContinue) {
    try {
        $defaultRepo = (gh repo set-default --view 2>&1).Trim()
        if ($defaultRepo -ne "SanHsien/agent-desktop") {
            Write-Warning "gh repo set-default is not SanHsien/agent-desktop ($defaultRepo). Fixing..."
            gh repo set-default SanHsien/agent-desktop
        }
    } catch {
        Write-Warning "Failed to check gh default repo: $_"
    }
}
Write-Host "[PASS] Git remotes configured correctly." -ForegroundColor Green

# 2. 執行 Fork 衛生單元測試
Write-Host "==> Running Fork hygiene test suite..." -ForegroundColor Yellow
python -m unittest discover -s tests/fork
if ($LASTEXITCODE -ne 0) {
    throw "Fork hygiene unit tests failed with exit code $LASTEXITCODE"
}
Write-Host "[PASS] Fork hygiene unit tests passed." -ForegroundColor Green

Write-Host "==> Checking product divergence ledger..." -ForegroundColor Yellow
python tools/check_divergence.py
if ($LASTEXITCODE -ne 0) {
    throw "Product divergence check failed with exit code $LASTEXITCODE"
}
Write-Host "[PASS] Product divergence ledger matches." -ForegroundColor Green

# 3. 檢查本地 Native Binary 與狀態
Write-Host "==> Verifying local native binary executable..." -ForegroundColor Yellow
$binaryPath = Join-Path $repoRoot "npm\bin\agent-desktop-win32-x64.exe"
if (Test-Path -LiteralPath $binaryPath) {
    $statusOut = & $binaryPath status 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "agent-desktop status command failed:`n$statusOut"
    }
    Write-Host "[PASS] agent-desktop native binary responds normally." -ForegroundColor Green
} else {
    Write-Warning "Native binary not found at $binaryPath. Run 'node npm/scripts/postinstall.js' to install."
}

# 4. Full 模式額外檢查
if ($Full) {
    Write-Host "==> [Full] Checking documentation links and encoding..." -ForegroundColor Yellow
    # 檢查文檔換行與 BOM
    $docs = Get-ChildItem -Path "$repoRoot\docs" -Recurse -Filter "*.md"
    foreach ($d in $docs) {
        $bytes = [System.IO.File]::ReadAllBytes($d.FullName)
        if ($bytes.Length -ge 3 -and $bytes[0] -eq 0xEF -and $bytes[1] -eq 0xBB -and $bytes[2] -eq 0xBF) {
            Write-Warning "$($d.Name) has UTF-8 BOM, prefer clean UTF-8."
        }
    }
    Write-Host "[PASS] Full checks complete." -ForegroundColor Green
}

Write-Host "==========================================================" -ForegroundColor Cyan
Write-Host " [SUCCESS] ALL FORK QUALITY GATES PASSED (GREEN)" -ForegroundColor Green
Write-Host "==========================================================" -ForegroundColor Cyan
