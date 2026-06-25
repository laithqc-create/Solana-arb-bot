// src-tauri/frontend/src/components/PairMatrix.tsx
import React, { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/tauri'
import '../styles/PairMatrix.css'

interface PoolData {
  spot_price: number
  tvl_usd: number
  amm_type: string
}

interface Opportunity {
  pair: string
  raw_spread_bps: number
  net_profit_bps: number
  profitable: boolean
  entry_pool: PoolData
  exit_pool: PoolData
}

const PairMatrix: React.FC = () => {
  const [opportunities, setOpportunities] = useState<Opportunity[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    const fetchOpportunities = async () => {
      try {
        setLoading(true)
        const result = await invoke<Opportunity[]>('get_opportunities')
        setOpportunities(result || [])
        setError(null)
      } catch (err) {
        setError(String(err))
        console.error('Error fetching opportunities:', err)
      } finally {
        setLoading(false)
      }
    }

    // Fetch immediately
    fetchOpportunities()

    // Then fetch every 3 seconds
    const interval = setInterval(fetchOpportunities, 3000)
    return () => clearInterval(interval)
  }, [])

  // Group by token pair
  const groupedByPair = opportunities.reduce((acc, opp) => {
    const pair = opp.pair
    if (!acc[pair]) acc[pair] = []
    acc[pair].push(opp)
    return acc
  }, {} as Record<string, Opportunity[]>)

  const getAmmColor = (type: string) => {
    switch (type.toLowerCase()) {
      case 'raydium': return '#38C7AC'
      case 'orca': return '#4A6FA5'
      case 'meteora': return '#A3621B'
      default: return '#666'
    }
  }

  const formatPrice = (price: number) => {
    return price > 1 ? price.toFixed(2) : price.toFixed(8)
  }

  const formatUsd = (usd: number) => {
    if (usd > 1_000_000) return `$${(usd / 1_000_000).toFixed(2)}M`
    if (usd > 1_000) return `$${(usd / 1_000).toFixed(1)}K`
    return `$${usd.toFixed(0)}`
  }

  if (loading) {
    return <div className="pair-matrix loading">⏳ Loading pair matrix...</div>
  }

  if (error) {
    return <div className="pair-matrix error">❌ Error: {error}</div>
  }

  return (
    <div className="pair-matrix">
      {Object.entries(groupedByPair).length === 0 ? (
        <div className="empty-state">
          <p>🔍 Scanning pools...</p>
          <p className="subtitle">Waiting for opportunities with TVL &gt; $100K</p>
        </div>
      ) : (
        Object.entries(groupedByPair).map(([pair, opps]) => (
          <div key={pair} className="pair-group">
            <h3 className="pair-title">{pair}</h3>
            
            {opps.map((opp, idx) => (
              <div key={idx} className={`opportunity-row ${opp.profitable ? 'profitable' : 'unprofitable'}`}>
                <div className="row-grid">
                  {/* Entry Pool */}
                  <div className="pool-cell">
                    <div className="pool-header">
                      <span className="amm-badge" style={{ backgroundColor: getAmmColor(opp.entry_pool.amm_type) }}>
                        {opp.entry_pool.amm_type}
                      </span>
                      <span className="price-label">BUY</span>
                    </div>
                    <div className="pool-details">
                      <div className="detail-row">
                        <span>Price:</span>
                        <strong>${formatPrice(opp.entry_pool.spot_price)}</strong>
                      </div>
                      <div className="detail-row">
                        <span>TVL:</span>
                        <strong>{formatUsd(opp.entry_pool.tvl_usd)}</strong>
                      </div>
                    </div>
                  </div>

                  {/* Spread Arrow */}
                  <div className="spread-cell">
                    <div className="spread-arrow">→</div>
                    <div className="spread-metric">
                      <div className="raw-spread">
                        <span>Raw Spread</span>
                        <strong>{(opp.raw_spread_bps / 100).toFixed(2)}%</strong>
                      </div>
                      <div className="gap-visual">
                        <div 
                          className="gap-bar"
                          style={{ 
                            width: `${Math.min(opp.raw_spread_bps / 100, 100)}%`,
                            backgroundColor: opp.raw_spread_bps > 50 ? '#00FF00' : '#FFA500'
                          }}
                        ></div>
                      </div>
                    </div>
                  </div>

                  {/* Exit Pool */}
                  <div className="pool-cell">
                    <div className="pool-header">
                      <span className="amm-badge" style={{ backgroundColor: getAmmColor(opp.exit_pool.amm_type) }}>
                        {opp.exit_pool.amm_type}
                      </span>
                      <span className="price-label">SELL</span>
                    </div>
                    <div className="pool-details">
                      <div className="detail-row">
                        <span>Price:</span>
                        <strong>${formatPrice(opp.exit_pool.spot_price)}</strong>
                      </div>
                      <div className="detail-row">
                        <span>TVL:</span>
                        <strong>{formatUsd(opp.exit_pool.tvl_usd)}</strong>
                      </div>
                    </div>
                  </div>

                  {/* Profit Indicator */}
                  <div className="profit-cell">
                    <div className={`profit-badge ${opp.profitable ? 'profitable' : ''}`}>
                      <div className="profit-pct">{(opp.net_profit_bps / 100).toFixed(2)}%</div>
                      <div className="profit-label">
                        {opp.profitable ? '✓ VIABLE' : '✗ Below 0.8%'}
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            ))}
          </div>
        ))
      )}
    </div>
  )
}

export default PairMatrix
