# Solana Arbitrage Engine - Build & Deployment Guide

## ⚡ Quick Start (No CLI Required)

### System Requirements
- **Windows 10/11** (x64)
- **Rust 1.70+** ([Download](https://rustup.rs/))
- **Node.js 16+** ([Download](https://nodejs.org/))
- **~2GB disk space** for dependencies

---

## 🚀 One-Click Build (Using Build Script)

### Step 1: Download Pre-Built Installer (Coming Soon)
The `.exe` installer will bundle everything needed. No CLI, no commands.

### Step 2: Or Build Yourself (First Time Setup)

#### Download Build Script
1. Save this file as **`BUILD.bat`** in your project root:

```batch
@echo off
REM Solana Arbitrage Engine - Automated Build Script
echo.
echo [*] Solana Arbitrage Engine Build System
echo [*] Building complete Windows installer...
echo.

REM Check for Rust
where rustc >nul 2>nul
if %ERRORLEVEL% NEQ 0 (
    echo [!] Rust not found. Installing Rust...
    powershell -Command "iwr https://win.rustup.rs -o rustup-init.exe; .\rustup-init.exe -y"
)

REM Check for Node.js
where node >nul 2>nul
if %ERRORLEVEL% NEQ 0 (
    echo [!] Node.js not found. Installing Node.js...
    powershell -Command "iwr https://nodejs.org/dist/v18.17.0/node-v18.17.0-x64.msi -o node-installer.msi; Start-Process node-installer.msi -Wait"
)

echo [+] Installing Rust frontend dependencies...
cd src-tauri\frontend
call npm install
cd ..\..

echo [+] Building Rust backend...
cargo build --release

echo [+] Building Tauri application...
cargo tauri build

echo.
echo [✓] Build complete! Installer is in src-tauri\target\release\bundle\nsis\
echo [✓] Installer filename: solana-arb-bot_x.x.x_x64-setup.exe
pause
```

2. **Double-click BUILD.bat** → Waits for everything to compile → Done!

The `.exe` installer will be in:
```
src-tauri/target/release/bundle/nsis/solana-arb-bot_x.x.x_x64-setup.exe
```

---

## 📦 Final Installer Behavior

When user runs `.exe`:
1. ✅ Windows UAC prompt (standard install)
2. ✅ Selects install directory
3. ✅ Extracts all files
4. ✅ Creates Start Menu shortcut
5. ✅ Double-click app icon to launch
6. ✅ **NO terminals, NO CLI, NO commands**

---

## 🔧 Manual Build Steps (If Needed)

If `BUILD.bat` doesn't work, here's the step-by-step:

### 1. Install Rust (One-time)
Download from https://rustup.rs/ and run the installer. Accept all defaults.

### 2. Install Node.js (One-time)
Download from https://nodejs.org/ (LTS version) and run installer.

### 3. Build Frontend
```
cd src-tauri/frontend
npm install
```

### 4. Build Rust Backend
```
cargo build --release
```

### 5. Package as Windows Installer
```
cargo tauri build
```

**Output:** `src-tauri/target/release/bundle/nsis/solana-arb-bot_x.x.x_x64-setup.exe`

---

## 🎯 Architecture Overview

```
solana-arb-bot/
├── src-tauri/
│   ├── frontend/                    # React UI (Vite + TypeScript)
│   │   ├── src/
│   │   │   ├── App.tsx             # Main app component
│   │   │   ├── components/         # UI components
│   │   │   │   ├── PairMatrix.tsx
│   │   │   │   ├── StreamStatus.tsx
│   │   │   │   ├── ConfigPanel.tsx
│   │   │   │   └── TradeJournal.tsx
│   │   │   └── styles/             # Component styles
│   │   └── package.json
│   └── tauri.conf.json             # Tauri config
│
├── src/backend/                    # Rust backend (Tokio + gRPC)
│   ├── main.rs                     # Sidecar process
│   ├── engine/                     # Arbitrage logic
│   ├── parsers/                    # AMM pool parsers
│   ├── vault/                      # AES-256 encryption
│   ├── streaming/                  # Geyser gRPC + fallover
│   └── ipc/                        # Inter-process communication
│
├── Cargo.toml                      # Rust dependencies
└── BUILD.bat                       # One-click build script
```

---

## 🔐 Configuration Paths

All configuration stored **locally** (no cloud upload):

```
C:\Users\YourUsername\AppData\Local\solana-arb-bot\
├── config.json                     # RPC URLs, Jito region
├── vault.enc                       # Encrypted wallet data (AES-256-GCM)
└── logs/                           # Application logs
```

---

## 🚀 Running the App

### After Installation:
1. Click **Start Menu** → **Solana Arbitrage Engine**
2. Or double-click desktop shortcut
3. App launches automatically (Rust backend included)

### First Launch:
- App initializes local vault
- Connects to Alchemy Node RPC
- Attempts Geyser gRPC connection
- Falls back to JSON-RPC if needed
- Ready to scan for opportunities

---

## 📊 Feature Checklist

- ✅ **Real-time pair matrix** (Raydium, Orca, Meteora)
- ✅ **Geyser gRPC streaming** with automatic fallover
- ✅ **30% gap rule** enforced
- ✅ **0.8% profit floor** filtering
- ✅ **$100k TVL minimum** filter
- ✅ **Jito bundle structure** visualization
- ✅ **AES-256 vault encryption**
- ✅ **CSV export** for trade journal
- ✅ **Dark mode UI** (professional design)
- ✅ **Zero CLI required**

---

## ⚠️ Phase 2: Live Execution (TODO)

To add actual flash loan execution:

1. Register a financial services entity (per your jurisdiction)
2. Uncomment code in `src/backend/engine/main.rs`:
   ```rust
   #[cfg(feature = "mainnet_trading")]
   ```
3. Implement wallet signing (Phase 2)
4. Implement Jito bundle submission (Phase 2)
5. Test on testnet first
6. Deploy to mainnet with compliance

---

## 🐛 Troubleshooting

### Build fails with "rustc not found"
→ Install Rust from https://rustup.rs/

### Node modules error
→ Run: `cd src-tauri/frontend && npm install`

### Tauri build fails
→ Check Windows Build Tools installed: `cargo tauri build --help`

### App crashes on launch
→ Check logs: `%LOCALAPPDATA%\solana-arb-bot\logs/`

---

## 📞 Support

- **Rust docs:** https://doc.rust-lang.org/
- **Tauri docs:** https://tauri.app/
- **Solana RPC:** https://docs.helius.xyz/

---

**Status:** Phase 1 (Simulation) ✓ Complete
**Next:** Phase 2 (Live Execution) - Requires financial services registration
