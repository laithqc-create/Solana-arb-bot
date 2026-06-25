// src-tauri/frontend/src/App.tsx
import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import PairMatrix from './components/PairMatrix'
import StreamStatus from './components/StreamStatus'
import ConfigPanel from './components/ConfigPanel'
import TradeJournal from './components/TradeJournal'
import ExecutionPanel from './components/ExecutionPanel'
import ProfitDashboard from './components/ProfitDashboard'
import './App.css'

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

function App() {
  const [opportunities, setOpportunities] = useState<Opportunity[]>([])
  const [streamStatus, setStreamStatus] = useState('disconnected')
  const [loading, setLoading] = useState(false)
  const [activeTab, setActiveTab] = useState('matrix')
  const [refreshInterval, setRefreshInterval] = useState(2000)

  // Fetch opportunities from backend
  const fetchOpportunities = async () => {
    setLoading(true)
    try {
      const result = await invoke('get_opportunities') as string
      const data = JSON.parse(result)
      if (data.success) {
        setOpportunities(data.opportunities)
      }
    } catch (error) {
      console.error('Failed to fetch opportunities:', error)
    }
    setLoading(false)
  }

  // Fetch stream status
  const fetchStreamStatus = async () => {
    try {
      const result = await invoke('get_stream_status') as string
      const data = JSON.parse(result)
      setStreamStatus(data.status)
    } catch (error) {
      console.error('Failed to fetch stream status:', error)
    }
  }

  // Auto-refresh opportunities
  useEffect(() => {
    fetchOpportunities()
    fetchStreamStatus()

    const opportunityInterval = setInterval(fetchOpportunities, refreshInterval)
    const statusInterval = setInterval(fetchStreamStatus, 5000)

    return () => {
      clearInterval(opportunityInterval)
      clearInterval(statusInterval)
    }
  }, [refreshInterval])

  return (
    <div className="app-container">
      {/* Header */}
      <header className="app-header">
        <div className="header-left">
          <h1>⚡ Solana Arbitrage Engine</h1>
          <p className="subtitle">Real-time cross-DEX opportunity detection</p>
        </div>
        <div className="header-right">
          <StreamStatus />
          <button 
            className="refresh-btn"
            onClick={fetchOpportunities}
            disabled={loading}
          >
            {loading ? '⟳ Scanning...' : '↻ Scan Now'}
          </button>
        </div>
      </header>

      {/* Navigation Tabs */}
      <nav className="tab-nav">
        <button 
          className={`tab-btn ${activeTab === 'dashboard' ? 'active' : ''}`}
          onClick={() => setActiveTab('dashboard')}
        >
          📊 Dashboard
        </button>
        <button 
          className={`tab-btn ${activeTab === 'matrix' ? 'active' : ''}`}
          onClick={() => setActiveTab('matrix')}
        >
          🔍 Pair Matrix
        </button>
        <button 
          className={`tab-btn ${activeTab === 'journal' ? 'active' : ''}`}
          onClick={() => setActiveTab('journal')}
        >
          📝 Trade Journal
        </button>
        <button 
          className={`tab-btn ${activeTab === 'execute' ? 'active' : ''}`}
          onClick={() => setActiveTab('execute')}
        >
          🚀 Execute
        </button>
        <button 
          className={`tab-btn ${activeTab === 'config' ? 'active' : ''}`}
          onClick={() => setActiveTab('config')}
        >
          ⚙️ Config
        </button>
      </nav>

      {/* Main Content */}
      <main className="app-main">
        {activeTab === 'dashboard' && (
          <section className="content-section">
            <ProfitDashboard />
          </section>
        )}

        {activeTab === 'matrix' && (
          <section className="content-section">
            <div className="controls">
              <label>
                Refresh Interval:
                <select 
                  value={refreshInterval} 
                  onChange={(e) => setRefreshInterval(Number(e.target.value))}
                >
                  <option value={1000}>1 second</option>
                  <option value={2000}>2 seconds</option>
                  <option value={5000}>5 seconds</option>
                  <option value={10000}>10 seconds</option>
                </select>
              </label>
              <div className="stats">
                <span>Opportunities Found: <strong>{opportunities.length}</strong></span>
                <span>Profitable: <strong>{opportunities.filter(o => o.profitable).length}</strong></span>
              </div>
            </div>
            <PairMatrix />
          </section>
        )}

        {activeTab === 'journal' && (
          <section className="content-section">
            <TradeJournal />
          </section>
        )}

        {activeTab === 'execute' && (
          <section className="content-section">
            <ExecutionPanel />
          </section>
        )}

        {activeTab === 'config' && (
          <section className="content-section">
            <ConfigPanel onConfigUpdate={fetchStreamStatus} />
          </section>
        )}
      </main>

      {/* Footer */}
      <footer className="app-footer">
        <span>🟢 Solana Arbitrage Engine v1.0.5 | Mainnet Ready | Jito MEV Protection</span>
      </footer>
    </div>
  )
}

export default App
