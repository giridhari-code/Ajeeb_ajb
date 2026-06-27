# Ajeeb — Windows Install Script (PowerShell)
# irm https://raw.githubusercontent.com/giridhari-code/Ajeeb_ajb/main/install.ps1 | iex
$ErrorActionPreference = "Stop"

$REPO = "giridhari-code/Ajeeb_ajb"
$BIN_DIR = "$env:USERPROFILE\.ajeeb\bin"
$PLATFORM = "windows-x86_64"

Write-Host ""
Write-Host "================================================" -ForegroundColor Cyan
Write-Host "  Ajeeb install ho raha hai... ($PLATFORM)" -ForegroundColor Cyan
Write-Host "================================================" -ForegroundColor Cyan
Write-Host ""

# ── Dependency checks ──────────────────────────────
$missing = @()
if (-not (Get-Command gcc -ErrorAction SilentlyContinue)) {
    $missing += "  gcc nahi mila! Install karo: choco install mingw"
}
if (-not (Get-Command llc -ErrorAction SilentlyContinue)) {
    $missing += "  llc (LLVM) nahi mila! Install karo: choco install llvm"
}

if ($missing.Count -gt 0) {
    Write-Host "⚠️  Zaroorat hai:" -ForegroundColor Yellow
    $missing | ForEach-Object { Write-Host $_ -ForegroundColor Yellow }
    Write-Host ""
    Write-Host "Ajeeb binaries download ho jayenge, lekin compile tabhi karega"
    Write-Host "jab upar ke tools available honge."
    Write-Host "──────────────────────────────────────────────────────────"
}

# ── Latest version check ───────────────────────────
Write-Host "  Checking latest version..."
try {
    $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$REPO/releases/latest"
    $VERSION = $release.tag_name
} catch {
    $VERSION = "v1.0.1"
}
Write-Host "  Version: $VERSION"
Write-Host ""

# ── Create bin directory ───────────────────────────
if (-not (Test-Path $BIN_DIR)) {
    New-Item -ItemType Directory -Path $BIN_DIR -Force | Out-Null
}

# ── Download function ──────────────────────────────
function Download-Binary {
    param([string]$Name)
    $url = "https://github.com/$REPO/releases/download/$VERSION/$Name-$PLATFORM.exe"
    $out = "$BIN_DIR\$Name.exe"
    Write-Host "  Downloading: $Name..."
    try {
        Invoke-WebRequest -Uri $url -OutFile $out -UseBasicParsing
        Write-Host "  ✓ $Name" -ForegroundColor Green
        return $true
    } catch {
        Write-Host "  ⚠️  $Name ($PLATFORM) release mein nahi hai" -ForegroundColor Yellow
        return $false
    }
}

# ── Download binaries ──────────────────────────────
$ajeebc_ok = Download-Binary "ajeebc"
$piri_ok = Download-Binary "piri"
$parth_ok  = Download-Binary "parth"

if (-not $ajeebc_ok) {
    Write-Host ""
    Write-Host "❌ Ajeebc binary release mein nahi hai ($PLATFORM)" -ForegroundColor Red
    Write-Host "   GitHub issue karo: https://github.com/$REPO/issues" -ForegroundColor Red
    exit 1
}

# ── Runtime ────────────────────────────────────────
Write-Host ""
Write-Host "  Downloading runtime library..."
$runtime_url = "https://raw.githubusercontent.com/$REPO/$VERSION/ajeebc/runtime/ajeeb_runtime.c"
try {
    Invoke-WebRequest -Uri $runtime_url -OutFile "$BIN_DIR\ajeeb_runtime.c" -UseBasicParsing
    Write-Host "  ✓ ajeeb_runtime.c" -ForegroundColor Green
} catch {
    Write-Host "  ⚠️  Runtime download fail" -ForegroundColor Yellow
}

# ── Standard library ──────────────────────────────
Write-Host ""
Write-Host "  Downloading ajeeb-std packages..."
$std_dir = "$env:USERPROFILE\.ajeeb\packages\ajeeb-std"
if (-not (Test-Path $std_dir)) {
    New-Item -ItemType Directory -Path $std_dir -Force | Out-Null
}
@("io", "math", "string", "array", "fs", "result", "collections", "option", "path", "process", "test", "time", "json") | ForEach-Object {
    $url = "https://raw.githubusercontent.com/$REPO/$VERSION/ajeeb-lang/std/$_.ajb"
    try {
        Invoke-WebRequest -Uri $url -OutFile "$std_dir\$_.ajb" -UseBasicParsing
        Write-Host "  ✓ ajeeb-std/$_.ajb" -ForegroundColor Green
    } catch {
        # skip
    }
}

# ── Template ──────────────────────────────────────
$tpl = @"
[package]
name = "my-project"
version = "0.1.0"
author = ""

[dependencies]

[compiler]
target = "native"
output = "build/"
runtime = "runtime/ajeeb_runtime.c"
"@
Set-Content -Path "$BIN_DIR\parth.das.template" -Value $tpl
Write-Host "  ✓ parth.das template" -ForegroundColor Green

# ── PATH setup ─────────────────────────────────────
$currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($currentPath -notlike "*$BIN_DIR*") {
    [Environment]::SetEnvironmentVariable("Path", "$BIN_DIR;$currentPath", "User")
    $env:Path = "$BIN_DIR;$env:Path"
    Write-Host "  ✓ Added to User PATH" -ForegroundColor Green
}

Write-Host ""
Write-Host "================================================" -ForegroundColor Cyan
Write-Host "  Install complete!" -ForegroundColor Green
Write-Host "================================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "Naya terminal kholo ya PATH refresh karo, phir:"
Write-Host ""
Write-Host "  ajeebc file.ajb              # compile"
Write-Host "  piri file.ajb              # MIR interpreter se chalao"
Write-Host "  parth init my-project        # naya project banao"
Write-Host "  parth build                  # compile karo"
Write-Host "  parth run                    # build + chalao"
Write-Host ""
Write-Host "Pehli baar? Ye karo:"
Write-Host "  parth init hello-ajeeb"
Write-Host "  cd hello-ajeeb"
Write-Host "  parth run"
