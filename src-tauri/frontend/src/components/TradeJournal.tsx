// src-tauri/frontend/src/components/TradeJournal.tsx
import React, { useState } from 'react'
import '../styles/TradeJournal.css'

interface Opportunity {
  pair: string
  raw_spread_bps: number
  net_profit_bps: number
  profitable: boolean
  entry_pool: {
    spot_price: number
    tvl_usd: number
    amm_type: string
  }
  exit_pool: {
    spot_price: number
    tvl_usd: number
    amm_type: string
  }
}

interface Props {
  opportunities: Opportunity[]
}

const TradeJournal: React.FC<Props> = ({ opportunities }) => {
  const [filterProfitableOnly, setFilterProfitableOnly] = useState(true)
  const [sortBy, setSortBy] = useState<'profit' | 'spread' | 'pair'>('profit')

  const filtered = filterProfitableOnly 
    ? opportunities.filter(o => o.profitable)
    : opportunities

  const sorted = [...filtered].sort((a, b) => {
    switch (sortBy) {
      case 'profit':
        return b.net_profit_bps - a.net_profit_bps
      case 'spread':
        return b.raw_spread_bps - a.raw_spread_bps
      case 'pair':
        return a.pair.localeCompare(b.pair)
      default:
        return 0
    }
  })

  const handleExportCSV = () => {
    const headers = ['Pair', 'AMM Entry', 'AMM Exit', 'Entry Price', 'Exit Price', 'Raw Spread %', 'Net Profit %', 'Viable']
    const rows = sorted.map(opp => [
      opp.pair,
      opp.entry_pool.amm_type,
      opp.exit_pool.amm_type,
      opp.entry_pool.spot_price.toFixed(8),
      opp.exit_pool.spot_price.toFixed(8),
      (opp.raw_spread_bps / 100).toFixed(2),
      (opp.net_profit_bps / 100).toFixed(2),
      opp.profitable ? 'YES' : 'NO'
    ])

    const csv = [headers, ...rows].map(row => row.join(',')).join('\n')
    
    const blob = new Blob([csv], { type: 'text/csv' })
    const url = window.URL.createObjectURL(blob)
    const a = document.createElement('a')
    a.href = url
    a.download = `arbitrage-journal-${new Date().toISOString().split('T')[0]}.csv`
    a.click()
    window.URL.revokeObjectURL(url)
  }

  const stats = {
    total: opportunities.length,
    profitable: opportunities.filter(o => o.profitable).length,
    avgProfit: opportunities.length > 0
      ? opportunities.reduce((sum, o) => sum + o.net_profit_bps, 0) / opportunities.length
      : 0,
    maxProfit: opportunities.length > 0
      ? Math.max(...opportunities.map(o => o.net_profit_bps))
      : 0
  }

  return (
    <div className="trade-journal">
      {/* Statistics Panel */}
      <div className="journal-stats">
        <div className="stat-box">
          <div className="stat-value">{stats.total}</div>
          <div className="stat-label">Total Opportunities</div>
        </div>
        <div className="stat-box profitable">
          <div className="stat-value">{stats.profitable}</div>
          <div className="stat-label">Profitable</div>
        </div>
        <div className="stat-box">
          <div className="stat-value">{(stats.avgProfit / 100).toFixed(2)}%</div>
          <div className="stat-label">Avg Net Profit</div>
        </div>
        <div className="stat-box highlight">
          <div className="stat-value">{(stats.maxProfit / 100).toFixed(2)}%</div>
          <div className="stat-label">Best Opportunity</div>
        </div>
      </div>

      {/* Controls */}
      <div className="journal-controls">
        <div className="controls-left">
          <label className="checkbox-label">
            <input
              type="checkbox"
              checked={filterProfitableOnly}
              onChange={(e) => setFilterProfitableOnly(e.target.checked)}
            />
            Show Profitable Only
          </label>
          
          <select 
            value={sortBy}
            onChange={(e) => setSortBy(e.target.value as any)}
            className="sort-select"
          >
            <option value="profit">Sort by: Profit ↓</option>
            <option value="spread">Sort by: Spread ↓</option>
            <option value="pair">Sort by: Pair A-Z</option>
          </select>
        </div>

        <button className="export-btn" onClick={handleExportCSV}>
          📥 Export CSV
        </button>
      </div>

      {/* Journal Table */}
      <div className="journal-table-container">
        {sorted.length === 0 ? (
          <div className="empty-journal">
            <p>📊 No opportunities to display</p>
            <p className="subtitle">
              {filterProfitableOnly 
                ? 'No opportunities meet the 0.8% profit floor'
                : 'Scan for new opportunities'}
            </p>
          </div>
        ) : (
          <table className="journal-table">
            <thead>
              <tr>
                <th>Token Pair</th>
                <th>Entry Pool</th>
                <th>Entry Price</th>
                <th>Exit Pool</th>
                <th>Exit Price</th>
                <th>Raw Spread</th>
                <th>Net Profit</th>
                <th>Status</th>
              </tr>
            </thead>
            <tbody>
              {sorted.map((opp, idx) => (
                <tr key={idx} className={opp.profitable ? 'profitable-row' : 'unprofitable-row'}>
                  <td className="pair-cell">
                    <strong>{opp.pair}</strong>
                  </td>
                  <td className="pool-cell">
                    <span className="amm-badge">{opp.entry_pool.amm_type}</span>
                  </td>
                  <td className="price-cell">
                    ${opp.entry_pool.spot_price > 1 
                      ? opp.entry_pool.spot_price.toFixed(2)
                      : opp.entry_pool.spot_price.toFixed(8)
                    }
                  </td>
                  <td className="pool-cell">
                    <span className="amm-badge">{opp.exit_pool.amm_type}</span>
                  </td>
                  <td className="price-cell">
                    ${opp.exit_pool.spot_price > 1 
                      ? opp.exit_pool.spot_price.toFixed(2)
                      : opp.exit_pool.spot_price.toFixed(8)
                    }
                  </td>
                  <td className="spread-cell">
                    <strong>{(opp.raw_spread_bps / 100).toFixed(2)}%</strong>
                  </td>
                  <td className="profit-cell">
                    <span className={opp.profitable ? 'profit-text' : 'loss-text'}>
                      {(opp.net_profit_bps / 100).toFixed(2)}%
                    </span>
                  </td>
                  <td className="status-cell">
                    {opp.profitable ? (
                      <span className="badge-profitable">✓ VIABLE</span>
                    ) : (
                      <span className="badge-below-floor">✗ &lt;0.8%</span>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>

      {/* Help Text */}
      <div className="journal-help">
        <p>
          <strong>What these columns mean:</strong><br/>
          Raw Spread = Price difference between pools (before fees).
          Net Profit = Your actual profit after pool fees, compute costs, and Jito tips.
          Status shows if the opportunity meets our ≥0.8% profit floor.
        </p>
      </div>
    </div>
  )
}

export default TradeJournal
