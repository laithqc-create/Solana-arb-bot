# ⚡ Solana Arbitrage Engine - Quick Start (Windows)

## 30-Second Setup

### Option A: One-Click Build (Recommended First Time)
1. **Find:** `BUILD.bat` in your project folder
2. **Double-click it** ← That's it!
3. Waits for build to complete (~10-15 minutes)
4. Opens folder with your `.exe` installer automatically
5. **Double-click the .exe** to install

### Option B: Use Pre-Built Installer
1. Download `.exe` from releases
2. Run installer
3. Click "Next" → "Install" → "Finish"
4. Open Start Menu → "Solana Arbitrage Engine"

---

## First Launch

App will:
- ✅ Initialize local vault (asks for password first time)
- ✅ Connect to Alchemy Node RPC
- ✅ Try Geyser gRPC (🟢 green if successful)
- ✅ Start scanning pools automatically
- ✅ Show opportunities in real-time

---

## Dashboard Walkthrough

### **Pair Matrix Tab** (Default)
Shows all token pairs with detected arbitrage gaps
- **Left side:** Pool buying from (cheapest)
- **Arrow:** Price gap visualization
- **Right side:** Pool selling to (most expensive)
- **Green box:** Profitable opportunity (≥0.8%)
- **Red box:** Below profit floor

### **Trade Journal Tab**
All opportunities logged here
- Filter by profitable only
- Sort by profit, spread, or pair name
- **Export CSV:** Download for Excel analysis

### **Configuration Tab**
- Change RPC endpoints
- Select Jito region
- Manage vault encryption

---

## Status Indicators

| Icon | Meaning |
|------|---------|
| 🟢 | Geyser gRPC connected (best latency) |
| 🟡 | Geyser lagging, monitoring fallback |
| 🟠 | Using JSON-RPC fallback (slower but stable) |
| 🔴 | Disconnected, attempting reconnect |

---

## What It Does (Phase 1)

✅ **Real-time monitoring:**
- Scans Raydium, Orca, Meteora pools
- Detects price gaps
- Calculates net profits

✅ **Risk management:**
- Filters out small pools ($100k TVL minimum)
- Enforces 30% gap rule
- Requires 0.8% minimum profit
- Subtracts all fees (pool, compute, tips)

✅ **Analysis tools:**
- Real-time pair matrix
- Trade journal with history
- CSV export for spreadsheets

---

## What It Does NOT Do (Phase 1)

❌ **No actual trading:**
- Shows opportunities, doesn't execute
- No real token swaps
- No flash loans
- No fund transfers

❌ **Why?**
Live MEV extraction needs financial services license (that's on you)

---

## Troubleshooting

### App won't launch
- Check: Windows Defender/antivirus not blocking
- Try: Reinstall from .exe

### "Geyser connection failed"
- This is normal, app auto-falls back to JSON-RPC
- RPC still works, just slower (🟠 icon)

### Want to monitor longer
- Leave app running, update every 2-5 seconds
- CSV export opportunities to analyze

---

## Next Steps (If You Want to Trade)

To use this as a **live trading bot**, you need to:

1. **Register as trading entity** (jurisdiction-specific)
   - Sole proprietor registration, LLC formation, etc.
   - Get financial services license if required

2. **Implement Phase 2** (we can help)
   - Add flash loan logic
   - Add wallet signing
   - Add Jito submission

3. **Test on testnet first**
   - Use devnet SOL (free)
   - Verify profit calculations

4. **Go live on mainnet**
   - Real opportunities
   - Real capital
   - Real responsibility

---

## Questions?

- **"How profitable is this?"** → Depends on spreads. Phase 1 shows you real data.
- **"Can I modify code?"** → Yes! Everything is in `src/` folder.
- **"How much does it cost?"** → Free to build and run. No subscriptions.
- **"Will it run 24/7?"** → Yes, leave your PC on.

---

**Status:** Ready to use! Phase 1 complete ✅

See `BUILD_GUIDE.md` for detailed technical setup.
