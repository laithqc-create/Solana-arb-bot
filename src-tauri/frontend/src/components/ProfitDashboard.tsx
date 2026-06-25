// src-tauri/frontend/src/components/ProfitDashboard.tsx
import React, { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/tauri'
import '../styles/ProfitDashboard.css'

interface OptimizationMetrics {
  total_profit: number
  win_count: number
  loss_count: number
  win_rate: number
  avg_profit_per_trade: number
  largest_win: number
  largest_loss: number
  current_streak: number
}

const ProfitDashboard: React.FC = () => {
  const [metrics, setMetrics] = useState<OptimizationMetrics | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    const fetchMetrics = async () => {
      try {
        setLoading(true)
        const result = await invoke<OptimizationMetrics>('get_optimization_metrics')
        setMetrics(result)
        setError(null)
      } catch (err) {
        setError(String(err))
        console.error('Error fetching metrics:', err)
      } finally {
        setLoading(false)
      }
    }

    // Fetch immediately
    fetchMetrics()

    // Then fetch every 10 seconds
    const interval = setInterval(fetchMetrics, 10000)
    return () => clearInterval(interval)
  }, [])

  const formatProfit = (lamports: number) => {
    return (lamports / 1_000_000).toFixed(4)
  }

  const getStreakColor = (streak: number) => {
    if (streak === 0) return '#999'
    if (streak > 0) return '#00AA00'
    return '#FF6666'
  }

  if (loading) {
    return <div className="profit-dashboard loading">⏳ Loading metrics...</div>
  }

  if (error) {
    return <div className="profit-dashboard error">❌ Error: {error}</div>
  }

  if (!metrics) {
    return <div className="profit-dashboard">No data available</div>
  }

  return (
    <div className="profit-dashboard">
      <h2>📊 Profit Dashboard</h2>

      {/* Main Profit Display */}
      <div className="profit-display">
        <div className="total-profit">
          <div className="label">Total Profit</div>
          <div className="value profit-amount">
            {formatProfit(metrics.total_profit)} SOL
          </div>
          <div className="subtext">Across all trades</div>
        </div>
      </div>

      {/* Win/Loss Stats */}
      <div className="stats-grid">
        <div className="stat-card wins">
          <div className="stat-label">Wins</div>
          <div className="stat-value">{metrics.win_count}</div>
          <div className="stat-subtext">Profitable trades</div>
        </div>

        <div className="stat-card losses">
          <div className="stat-label">Losses</div>
          <div className="stat-value">{metrics.loss_count}</div>
          <div className="stat-subtext">Unprofitable trades</div>
        </div>

        <div className="stat-card winrate">
          <div className="stat-label">Win Rate</div>
          <div className="stat-value">{(metrics.win_rate * 100).toFixed(1)}%</div>
          <div className="stat-subtext">Success percentage</div>
        </div>

        <div className="stat-card average">
          <div className="stat-label">Avg Profit</div>
          <div className="stat-value">
            {formatProfit(metrics.avg_profit_per_trade)} SOL
          </div>
          <div className="stat-subtext">Per trade</div>
        </div>
      </div>

      {/* Best & Worst */}
      <div className="extremes-grid">
        <div className="extreme-card best">
          <div className="extreme-label">Best Trade</div>
          <div className="extreme-value">
            +{formatProfit(metrics.largest_win)} SOL
          </div>
          <div className="extreme-subtext">Largest win</div>
        </div>

        <div className="extreme-card worst">
          <div className="extreme-label">Worst Trade</div>
          <div className="extreme-value">
            -{formatProfit(Math.abs(metrics.largest_loss))} SOL
          </div>
          <div className="extreme-subtext">Largest loss</div>
        </div>

        <div className="extreme-card streak">
          <div className="extreme-label">Current Streak</div>
          <div className="extreme-value" style={{ color: getStreakColor(metrics.current_streak) }}>
            {metrics.current_streak > 0 ? '+' : ''}{metrics.current_streak}
          </div>
          <div className="extreme-subtext">
            {metrics.current_streak > 0 ? 'Winning' : metrics.current_streak < 0 ? 'Losing' : 'No'} streak
          </div>
        </div>
      </div>

      {/* Performance Metrics */}
      <div className="performance-section">
        <h3>Performance Analysis</h3>
        
        <div className="metric-row">
          <span className="metric-label">Total Trades</span>
          <span className="metric-value">{metrics.win_count + metrics.loss_count}</span>
        </div>

        <div className="metric-row">
          <span className="metric-label">Profit Factor</span>
          <span className="metric-value">
            {metrics.loss_count === 0 
              ? '∞' 
              : (metrics.total_profit / Math.abs(metrics.largest_loss || 1)).toFixed(2)}
          </span>
        </div>

        <div className="metric-row">
          <span className="metric-label">Expected Value</span>
          <span className="metric-value">
            {formatProfit(metrics.avg_profit_per_trade * (metrics.win_count + metrics.loss_count))} SOL
          </span>
        </div>
      </div>

      {/* Health Indicator */}
      <div className="health-indicator">
        <div className="health-bar">
          <div 
            className="health-fill" 
            style={{
              width: `${Math.min(metrics.win_rate * 100, 100)}%`,
              backgroundColor: metrics.win_rate > 0.7 ? '#00FF00' : metrics.win_rate > 0.5 ? '#FFA500' : '#FF6666'
            }}
          />
        </div>
        <div className="health-label">System Health</div>
      </div>

      {/* Tips */}
      <div className="tips-section">
        <h4>💡 Tips for Better Performance</h4>
        <ul>
          <li>Monitor your win rate regularly to ensure strategy viability</li>
          <li>Adjust slippage settings based on market conditions</li>
          <li>Use gap thresholds to filter low-profit opportunities</li>
          <li>Track largest losses to understand risk exposure</li>
        </ul>
      </div>

      <style jsx>{`
        .profit-dashboard {
          padding: 20px;
          background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
          border-radius: 12px;
          color: white;
          min-height: 600px;
        }

        .profit-dashboard.loading,
        .profit-dashboard.error {
          display: flex;
          align-items: center;
          justify-content: center;
          font-size: 18px;
        }

        h2 {
          margin-top: 0;
          margin-bottom: 20px;
          font-size: 28px;
        }

        .profit-display {
          background: rgba(255, 255, 255, 0.1);
          padding: 30px;
          border-radius: 12px;
          text-align: center;
          margin-bottom: 20px;
          backdrop-filter: blur(10px);
        }

        .total-profit {
          display: flex;
          flex-direction: column;
          gap: 10px;
        }

        .label {
          font-size: 14px;
          opacity: 0.9;
        }

        .profit-amount {
          font-size: 48px;
          font-weight: bold;
          color: #00FF00;
          text-shadow: 0 0 10px rgba(0, 255, 0, 0.3);
        }

        .subtext {
          font-size: 12px;
          opacity: 0.7;
        }

        .stats-grid {
          display: grid;
          grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
          gap: 12px;
          margin-bottom: 20px;
        }

        .stat-card {
          background: rgba(255, 255, 255, 0.1);
          padding: 20px;
          border-radius: 8px;
          text-align: center;
          backdrop-filter: blur(10px);
          border-left: 4px solid;
        }

        .stat-card.wins {
          border-left-color: #00FF00;
        }

        .stat-card.losses {
          border-left-color: #FF6666;
        }

        .stat-card.winrate {
          border-left-color: #FFB700;
        }

        .stat-card.average {
          border-left-color: #00BBFF;
        }

        .stat-label {
          font-size: 12px;
          opacity: 0.8;
          margin-bottom: 8px;
        }

        .stat-value {
          font-size: 28px;
          font-weight: bold;
          margin-bottom: 6px;
        }

        .stat-subtext {
          font-size: 11px;
          opacity: 0.6;
        }

        .extremes-grid {
          display: grid;
          grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
          gap: 12px;
          margin-bottom: 20px;
        }

        .extreme-card {
          background: rgba(255, 255, 255, 0.1);
          padding: 20px;
          border-radius: 8px;
          text-align: center;
          backdrop-filter: blur(10px);
          border-top: 3px solid;
        }

        .extreme-card.best {
          border-top-color: #00FF00;
        }

        .extreme-card.worst {
          border-top-color: #FF6666;
        }

        .extreme-card.streak {
          border-top-color: #FFA500;
        }

        .extreme-label {
          font-size: 12px;
          opacity: 0.7;
          margin-bottom: 10px;
        }

        .extreme-value {
          font-size: 24px;
          font-weight: bold;
          margin-bottom: 6px;
        }

        .extreme-subtext {
          font-size: 11px;
          opacity: 0.6;
        }

        .performance-section {
          background: rgba(255, 255, 255, 0.1);
          padding: 20px;
          border-radius: 8px;
          margin-bottom: 20px;
          backdrop-filter: blur(10px);
        }

        .performance-section h3 {
          margin-top: 0;
          margin-bottom: 15px;
          font-size: 16px;
        }

        .metric-row {
          display: flex;
          justify-content: space-between;
          padding: 10px 0;
          border-bottom: 1px solid rgba(255, 255, 255, 0.1);
          font-size: 14px;
        }

        .metric-label {
          opacity: 0.8;
        }

        .metric-value {
          font-weight: bold;
        }

        .health-indicator {
          background: rgba(255, 255, 255, 0.1);
          padding: 20px;
          border-radius: 8px;
          margin-bottom: 20px;
          backdrop-filter: blur(10px);
        }

        .health-bar {
          width: 100%;
          height: 30px;
          background: rgba(0, 0, 0, 0.3);
          border-radius: 15px;
          overflow: hidden;
          margin-bottom: 10px;
        }

        .health-fill {
          height: 100%;
          transition: width 0.3s ease;
        }

        .health-label {
          text-align: center;
          font-size: 12px;
          opacity: 0.8;
        }

        .tips-section {
          background: rgba(255, 255, 255, 0.1);
          padding: 20px;
          border-radius: 8px;
          backdrop-filter: blur(10px);
        }

        .tips-section h4 {
          margin-top: 0;
          margin-bottom: 12px;
          font-size: 14px;
        }

        .tips-section ul {
          margin: 0;
          padding-left: 20px;
          font-size: 12px;
          opacity: 0.8;
        }

        .tips-section li {
          margin-bottom: 8px;
        }
      `}</style>
    </div>
  )
}

export default ProfitDashboard
