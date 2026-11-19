#!/usr/bin/env pwsh
<#
.SYNOPSIS
    GitUpload v2.3.0 Crate Boundary, Safety Guardrail & Static Quality Audit.
#>

$ErrorActionPreference = "Stop"
$RepoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $RepoRoot

Write-Host "[1/5] Checking crate root #![forbid(unsafe_code)] / #![deny(unsafe_code)]..." -ForegroundColor Cyan
$crateDirs = Get-ChildItem (Join-Path $RepoRoot "crates") -Directory
foreach ($crate in $crateDirs) {
    $libPath = Join-Path $crate.FullName "src/lib.rs"
    $mainPath = Join-Path $crate.FullName "src/main.rs"
    $entry = if (Test-Path $libPath) { $libPath } else { $mainPath }
    if (Test-Path $entry) {
        $head = Get-Content $entry -TotalCount 10 | Out-String
        if ($head -notmatch '#!\[(forbid|deny)\(unsafe_code\)\]') {
            throw "SECURITY AUDIT FAILED: $($crate.Name) missing unsafe_code ban."
        }
    }
}

Write-Host "[2/5] Checking forbidden local LLM / anyhow dependencies..." -ForegroundColor Cyan
$cargoTomls = Get-ChildItem (Join-Path $RepoRoot "crates") -Filter "Cargo.toml" -Recurse
foreach ($toml in $cargoTomls) {
    $content = Get-Content $toml.FullName -Raw
    if ($content -match '(?m)^(anyhow|ollama|llama-cpp|candle)') {
        throw "DEPENDENCY AUDIT FAILED: Forbidden dependency in $($toml.FullName)"
    }
}

Write-Host "[3/5] Scanning frontend UI for raw emoji policy compliance..." -ForegroundColor Cyan
$uiFiles = Get-ChildItem (Join-Path $RepoRoot "ui/src") -Include "*.tsx","*.ts" -Recurse
foreach ($f in $uiFiles) {
    $raw = Get-Content $f.FullName -Raw
    if ($raw -match '(\uD83C[\uDF00-\uDFFF]|\uD83D[\uDC00-\uDFFF]|\uD83E[\uDD00-\uDDFF])') {
        throw "UI AUDIT FAILED: Raw emoji glyph detected in $($f.FullName). Use Octicons/Lucide icons instead."
    }
}

Write-Host "[4/5] Running cargo fmt --all -- --check..." -ForegroundColor Cyan
cargo fmt --all -- --check
if ($LASTEXITCODE -ne 0) { throw "cargo fmt check failed" }

Write-Host "[5/5] Running cargo clippy --workspace --all-targets -- -D warnings..." -ForegroundColor Cyan
cargo clippy --workspace --all-targets -- -D warnings
if ($LASTEXITCODE -ne 0) { throw "cargo clippy check failed" }

Write-Host "`n[PASS] Crate boundary, safety, UI icon, fmt, and clippy audits succeeded!" -ForegroundColor Green
exit 0
