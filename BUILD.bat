@echo off
REM ============================================================================
REM Solana Arbitrage Engine - Windows One-Click Build Script
REM ============================================================================
REM This script:
REM 1. Checks for Rust & Node.js installations
REM 2. Installs them if missing (automatically)
REM 3. Builds frontend React app
REM 4. Builds Rust backend
REM 5. Packages everything into Windows .exe installer
REM ============================================================================

setlocal enabledelayedexpansion
cls

echo.
echo ╔════════════════════════════════════════════════════════════════╗
echo ║  ⚡  SOLANA ARBITRAGE ENGINE - BUILD SYSTEM                    ║
echo ║                                                                ║
echo ║  Compiling production-ready Windows installer...              ║
echo ╚════════════════════════════════════════════════════════════════╝
echo.

REM ============================================================================
REM Check and Install Rust
REM ============================================================================
echo [1/5] Checking Rust installation...
where rustc >nul 2>nul
if %ERRORLEVEL% NEQ 0 (
    echo.
    echo     ⚠️  Rust not found. Installing Rust toolchain...
    echo     (This will download ~1GB and take 2-3 minutes)
    echo.
    powershell -NoProfile -Command "iwr https://win.rustup.rs -o rustup-init.exe; .\rustup-init.exe -y --default-toolchain stable --default-host x86_64-pc-windows-msvc"
    if %ERRORLEVEL% NEQ 0 (
        echo.
        echo ❌ Rust installation failed. Please install manually:
        echo    https://rustup.rs/
        pause
        exit /b 1
    )
    echo ✓ Rust installed successfully
) else (
    echo ✓ Rust found
)

REM ============================================================================
REM Check and Install Node.js
REM ============================================================================
echo [2/5] Checking Node.js installation...
where node >nul 2>nul
if %ERRORLEVEL% NEQ 0 (
    echo.
    echo     ⚠️  Node.js not found. Installing Node.js...
    echo     (This will download ~150MB and take 1-2 minutes)
    echo.
    powershell -NoProfile -Command "iwr https://nodejs.org/dist/v18.17.0/node-v18.17.0-x64.msi -o node-installer.msi; msiexec /i node-installer.msi /quiet"
    if %ERRORLEVEL% NEQ 0 (
        echo.
        echo ❌ Node.js installation failed. Please install manually:
        echo    https://nodejs.org/ (LTS version)
        pause
        exit /b 1
    )
    echo ✓ Node.js installed successfully
    REM Refresh PATH
    call :refresh_path
) else (
    echo ✓ Node.js found
)

REM ============================================================================
REM Build Frontend (React + Vite)
REM ============================================================================
echo [3/5] Building React frontend...
cd src-tauri\frontend 2>nul || (
    echo ❌ Frontend directory not found. Ensure you're in project root.
    pause
    exit /b 1
)

call npm install >nul 2>&1
if %ERRORLEVEL% NEQ 0 (
    echo ❌ npm install failed
    pause
    exit /b 1
)

call npm run build >nul 2>&1
if %ERRORLEVEL% NEQ 0 (
    echo ❌ Frontend build failed
    pause
    exit /b 1
)

echo ✓ Frontend built successfully

cd ..\.. || exit /b 1

REM ============================================================================
REM Build Rust Backend
REM ============================================================================
echo [4/5] Building Rust backend (this may take 5-10 minutes)...
call cargo build --release 2>nul
if %ERRORLEVEL% NEQ 0 (
    echo ❌ Rust build failed. Details:
    call cargo build --release
    pause
    exit /b 1
)
echo ✓ Rust backend compiled successfully

REM ============================================================================
REM Package Tauri Application
REM ============================================================================
echo [5/5] Packaging Windows installer...
call cargo tauri build 2>nul
if %ERRORLEVEL% NEQ 0 (
    echo ❌ Tauri build failed. Details:
    call cargo tauri build
    pause
    exit /b 1
)
echo ✓ Installer packaged successfully

REM ============================================================================
REM Success!
REM ============================================================================
cls
echo.
echo ╔════════════════════════════════════════════════════════════════╗
echo ║  ✅  BUILD SUCCESSFUL!                                        ║
echo ╠════════════════════════════════════════════════════════════════╣
echo ║                                                                ║
echo ║  📦 Your Windows installer is ready:                          ║
echo ║                                                                ║
echo ║     solana-arb-bot_x.x.x_x64-setup.exe                        ║
echo ║                                                                ║
echo ║  📍 Location:                                                  ║
echo ║     src-tauri\target\release\bundle\nsis\                     ║
echo ║                                                                ║
echo ║  🚀 Next Steps:                                               ║
echo ║     1. Find the .exe file (see location above)                ║
echo ║     2. Double-click to run installer                          ║
echo ║     3. Follow installation prompts                            ║
echo ║     4. Launch from Start Menu                                 ║
echo ║                                                                ║
echo ║  💡 No CLI required. Everything runs automatically!           ║
echo ║                                                                ║
echo ╚════════════════════════════════════════════════════════════════╝
echo.

REM Open explorer to installer directory
start "" "src-tauri\target\release\bundle\nsis"

pause
exit /b 0

REM ============================================================================
REM Helper Function: Refresh PATH
REM ============================================================================
:refresh_path
for /f "tokens=2*" %%a in ('reg query HKCU\Environment /v PATH') do set "USERPATH=%%b"
for /f "tokens=2*" %%a in ('reg query HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\Environment /v Path') do set "SYSPATH=%%b"
set "PATH=%USERPATH%;%SYSPATH%"
exit /b 0
