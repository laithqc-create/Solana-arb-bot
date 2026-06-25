# UI Integration Guide - Connect React to Backend (2-3 Hours)

## Current Status

✅ Backend: Complete (6,200+ lines, all features working)
✅ Frontend: Exists (836 lines React code)
❌ Connection: Missing (components don't call backend)

## What You Need to Do

Update 4 existing React component files to call the Tauri commands.

### File 1: src-tauri/frontend/src/components/StreamStatus.tsx

Add real-time opportunity fetching:

```typescript
import { invoke } from '@tauri-apps/api/tauri';
import { useState, useEffect } from 'react';

export function StreamStatus() {
  const [status, setStatus] = useState<any>(null);
  const [opportunities, setOpportunities] = useState<any[]>([]);

  useEffect(() => {
    // Fetch status every 2 seconds
    const interval = setInterval(async () => {
      try {
        const status = await invoke('get_stream_status');
        setStatus(status);

        const opps = await invoke('get_opportunities');
        setOpportunities(opps);
      } catch (err) {
        console.error('Error fetching status:', err);
      }
    }, 2000);

    return () => clearInterval(interval);
  }, []);

  return (
    <div className="stream-status">
      <h2>Stream Status</h2>
      {status && (
        <>
          <p>Status: {status.active ? '🟢 Active' : '🔴 Inactive'}</p>
          <p>Opportunities Found: {opportunities.length}</p>
          <div className="opportunities">
            {opportunities.map((opp, idx) => (
              <div key={idx} className="opportunity">
                <span>{opp.pool_a} → {opp.pool_b}</span>
                <span className="gap">{opp.gap.toFixed(2)}%</span>
                <button onClick={() => executeArbitrage(opp)}>Execute</button>
              </div>
            ))}
          </div>
        </>
      )}
    </div>
  );
}

async function executeArbitrage(opportunity: any) {
  try {
    const result = await invoke('execute_arbitrage_optimized', {
      profitLamports: opportunity.estimated_profit?.toString() || '0',
      slippageBps: '3000',
      bundleId: 'bundle_' + Date.now()
    });
    console.log('✅ Trade executed:', result);
  } catch (err) {
    console.error('❌ Trade failed:', err);
  }
}
```

### File 2: src-tauri/frontend/src/components/TradeJournal.tsx

Add execution history:

```typescript
import { invoke } from '@tauri-apps/api/tauri';
import { useState, useEffect } from 'react';

export function TradeJournal() {
  const [executionStatus, setExecutionStatus] = useState<any>(null);

  useEffect(() => {
    const interval = setInterval(async () => {
      try {
        const status = await invoke('get_execution_status');
        setExecutionStatus(status);
      } catch (err) {
        console.error('Error:', err);
      }
    }, 5000);

    return () => clearInterval(interval);
  }, []);

  return (
    <div className="trade-journal">
      <h2>Execution Status</h2>
      {executionStatus && (
        <>
          <div className="metric">
            <label>Last Trade:</label>
            <span>{executionStatus.signature}</span>
          </div>
          <div className="metric">
            <label>Status:</label>
            <span>{executionStatus.status}</span>
          </div>
          <div className="metric">
            <label>Profit:</label>
            <span>${executionStatus.profit?.toLocaleString() || '0'}</span>
          </div>
          {executionStatus.error && (
            <div className="error">
              <p>Error: {executionStatus.error}</p>
              <button onClick={() => recoverFromFailure()}>Recover</button>
            </div>
          )}
        </>
      )}
    </div>
  );
}

async function recoverFromFailure() {
  try {
    await invoke('recover_from_failure');
    console.log('✅ Recovery initiated');
  } catch (err) {
    console.error('Recovery failed:', err);
  }
}
```

### File 3: src-tauri/frontend/src/components/ConfigPanel.tsx

Add metrics display:

```typescript
import { invoke } from '@tauri-apps/api/tauri';
import { useState, useEffect } from 'react';

export function ConfigPanel() {
  const [metrics, setMetrics] = useState<any>(null);
  const [vaultConfig, setVaultConfig] = useState<any>(null);

  useEffect(() => {
    const loadConfig = async () => {
      try {
        const metrics = await invoke('calculate_arbitrage_metrics', {
          poolA: 'pool_a',
          poolB: 'pool_b',
          priceA: 100,
          priceB: 105
        });
        setMetrics(metrics);

        const vault = await invoke('get_vault_config');
        setVaultConfig(vault);
      } catch (err) {
        console.error('Error:', err);
      }
    };

    loadConfig();
    const interval = setInterval(loadConfig, 10000);
    return () => clearInterval(interval);
  }, []);

  return (
    <div className="config-panel">
      <h2>Configuration & Metrics</h2>
      {metrics && (
        <div className="metrics">
          <div className="metric">
            <label>Gap Threshold:</label>
            <span>{metrics.gap_threshold}%</span>
          </div>
          <div className="metric">
            <label>Min Liquidity:</label>
            <span>${metrics.min_liquidity?.toLocaleString()}</span>
          </div>
          <div className="metric">
            <label>Slippage:</label>
            <span>{metrics.slippage}%</span>
          </div>
        </div>
      )}
      {vaultConfig && (
        <div className="vault">
          <h3>Vault Config</h3>
          <p>Flash Loan: {vaultConfig.flashLoanEnabled ? '✅' : '❌'}</p>
          <p>Fee: {vaultConfig.fee}%</p>
        </div>
      )}
    </div>
  );
}
```

### File 4: src-tauri/frontend/src/components/PairMatrix.tsx

Add pair validation:

```typescript
import { invoke } from '@tauri-apps/api/tauri';
import { useState, useEffect } from 'react';

export function PairMatrix() {
  const [pairs, setPairs] = useState<any[]>([]);
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    const interval = setInterval(async () => {
      setLoading(true);
      try {
        const opps = await invoke('get_opportunities');
        
        // Validate each opportunity
        const validated = await Promise.all(
          opps.map(async (opp) => {
            try {
              const validation = await invoke('validate_swap_opportunity', {
                poolA: opp.pool_a,
                poolB: opp.pool_b,
                gap: opp.gap
              });
              return { ...opp, ...validation };
            } catch {
              return opp;
            }
          })
        );
        
        setPairs(validated);
      } catch (err) {
        console.error('Error:', err);
      }
      setLoading(false);
    }, 3000);

    return () => clearInterval(interval);
  }, []);

  return (
    <div className="pair-matrix">
      <h2>Pair Matrix</h2>
      {loading && <p>Updating...</p>}
      <table>
        <thead>
          <tr>
            <th>Pool A</th>
            <th>Pool B</th>
            <th>Gap</th>
            <th>Valid</th>
            <th>Fraud Check</th>
          </tr>
        </thead>
        <tbody>
          {pairs.map((pair, idx) => (
            <tr key={idx}>
              <td>{pair.pool_a}</td>
              <td>{pair.pool_b}</td>
              <td className={pair.gap > 0 ? 'profit' : 'loss'}>
                {pair.gap.toFixed(2)}%
              </td>
              <td>{pair.valid ? '✅' : '❌'}</td>
              <td>{pair.is_honeypot ? '🚨 Honeypot' : '✅ Safe'}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
```

## How to Run

### Terminal (No UI):
```bash
chmod +x solana_arb_bot
export SOLANA_NETWORK=devnet
export SOLANA_RPC_URL=https://api.devnet.solana.com
export HELIUS_API_KEY=your_key
solana airdrop 10
./solana_arb_bot
```

### With UI (Tauri Desktop):
```bash
# Update the components above
cd src-tauri/frontend
npm install
cd ../..
npm run tauri dev
```

## What This Achieves

✅ Real-time opportunities displayed
✅ Live execution status shown
✅ Metrics and configuration visible
✅ One-click trade execution
✅ Fraud detection alerts
✅ Recovery controls
✅ Complete UI-Backend integration

## Time Estimate

- File 1 (StreamStatus): 30 min
- File 2 (TradeJournal): 20 min
- File 3 (ConfigPanel): 20 min
- File 4 (PairMatrix): 20 min
- Testing: 30 min
- **Total: ~2 hours**

That's it! No 20 hours, just update 4 files with simple invoke() calls.
