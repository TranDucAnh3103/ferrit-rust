#!/usr/bin/env pwsh
<#
.SYNOPSIS
    GitUpload v2.5.0-enterprise Master Verification Pipeline (Sprints 1 + 2 + 3 + 4).
.DESCRIPTION
    Executes frontend TypeScript/Vite build, rustfmt, workspace clippy (-D warnings),
    unit & ephemeral integration tests, crate boundary checks, and security hygiene scans.
#>

$ErrorActionPreference = "Stop"
$RepoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $RepoRoot

function Write-Step($Title) {
    Write-Host "`n============================================================" -ForegroundColor Cyan
    Write-Host "  $Title" -ForegroundColor Cyan
    Write-Host "============================================================" -ForegroundColor Cyan
}

# 1. Frontend TypeScript + Vite production bundle
Write-Step "1/5 Frontend UI Build (tsc && vite build)"
Push-Location (Join-Path $RepoRoot "ui")
try {
    npm run build
    if ($LASTEXITCODE -ne 0) { throw "UI build failed with exit code $LASTEXITCODE" }
}
finally {
    Pop-Location
}

# 2. Rust formatting gate
Write-Step "2/5 Code Formatting Check (cargo fmt --all -- --check)"
cargo fmt --all -- --check
if ($LASTEXITCODE -ne 0) { throw "cargo fmt failed. Run 'cargo fmt --all' to format." }

# 3. Strict Workspace Clippy gate
Write-Step "3/5 Clippy Static Analysis (cargo clippy --workspace --all-targets -- -D warnings)"
cargo clippy --workspace --all-targets -- -D warnings
if ($LASTEXITCODE -ne 0) { throw "cargo clippy failed with warnings or errors." }

# 4. Full Unit + Integration Test Suite
Write-Step "4/5 Workspace Tests (cargo test --workspace)"
cargo test --workspace
if ($LASTEXITCODE -ne 0) { throw "cargo test failed." }

# 5. Architectural Boundary & Security Guardrail Audit
Write-Step "5/5 Crate Boundary & Safety Guardrail Audit"
& (Join-Path $PSScriptRoot "audit_crates.ps1")
if ($LASTEXITCODE -ne 0) { throw "audit_crates.ps1 failed." }

Write-Host "`n[PASS] All Sprint 1 + Sprint 2 verification gates succeeded!" -ForegroundColor Green
exit 0
