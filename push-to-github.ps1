#!/usr/bin/env powershell
# Push to GitHub - Easy Setup Script
# Usage: .\push-to-github.ps1 -username YOUR_GITHUB_USERNAME -repo solana-arb-bot

param(
    [Parameter(Mandatory=$true)]
    [string]$username,
    
    [Parameter(Mandatory=$false)]
    [string]$repo = "solana-arb-bot"
)

Write-Host "╔════════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║ Solana Arbitrage Engine - GitHub Push Setup               ║" -ForegroundColor Cyan
Write-Host "╚════════════════════════════════════════════════════════════╝" -ForegroundColor Cyan

# Check if Git is installed
$gitExists = Get-Command git -ErrorAction SilentlyContinue
if (-not $gitExists) {
    Write-Host "`n❌ Git not found. Please install from: https://git-scm.com/download/win`n" -ForegroundColor Red
    exit 1
}

$repoUrl = "https://github.com/$username/$repo.git"

Write-Host "`n📋 Configuration:" -ForegroundColor Green
Write-Host "   Username:   $username"
Write-Host "   Repository: $repo"
Write-Host "   URL:        $repoUrl"
Write-Host ""

# Initialize git repo
Write-Host "📦 Initializing Git repository..." -ForegroundColor Yellow
git init
if ($LASTEXITCODE -ne 0) { exit 1 }

# Add all files
Write-Host "📝 Adding files..." -ForegroundColor Yellow
git add .
if ($LASTEXITCODE -ne 0) { exit 1 }

# Initial commit
Write-Host "💾 Creating initial commit..." -ForegroundColor Yellow
git commit -m "Initial commit: Solana arbitrage engine Phase 1 - simulation engine with Tauri UI"
if ($LASTEXITCODE -ne 0) { exit 1 }

# Add remote
Write-Host "🔗 Adding GitHub remote..." -ForegroundColor Yellow
git remote add origin $repoUrl
if ($LASTEXITCODE -ne 0) { exit 1 }

# Rename branch to main
Write-Host "🔀 Renaming branch to main..." -ForegroundColor Yellow
git branch -M main
if ($LASTEXITCODE -ne 0) { exit 1 }

# Push to GitHub
Write-Host "🚀 Pushing to GitHub (first time - requires authentication)..." -ForegroundColor Yellow
Write-Host "   A browser window will open to authenticate." -ForegroundColor Gray
Write-Host ""

git push -u origin main
if ($LASTEXITCODE -ne 0) {
    Write-Host "`n❌ Push failed. Check your credentials and try again." -ForegroundColor Red
    exit 1
}

# Success
Write-Host "`n✅ SUCCESS! Your repository is now on GitHub!" -ForegroundColor Green
Write-Host ""
Write-Host "📍 Next Steps:" -ForegroundColor Cyan
Write-Host "   1. Go to: https://github.com/$username/$repo/actions" -ForegroundColor Gray
Write-Host "   2. Watch the build run (takes 45 minutes first time)" -ForegroundColor Gray
Write-Host "   3. Click the workflow → Artifacts when done" -ForegroundColor Gray
Write-Host "   4. Download solana-arb-bot-windows and extract the .exe" -ForegroundColor Gray
Write-Host ""
Write-Host "🔄 Future Builds:" -ForegroundColor Cyan
Write-Host "   Just push changes with: git push" -ForegroundColor Gray
Write-Host "   GitHub Actions builds automatically!" -ForegroundColor Gray
Write-Host ""
