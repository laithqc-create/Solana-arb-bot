// src-tauri/frontend/src/components/ExecutionPanel.tsx
import React, { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/tauri'
import '../styles/ExecutionPanel.css'

interface ExecutionResult {
  state: string
  signature: string
  profit: number
  execution_time_ms: number
  error?: string
}

interface ExecutionStatus {
  last_execution: ExecutionResult | null
  status: 'idle' | 'executing' | 'success' | 'error'
  message: string
}

const ExecutionPanel: React.FC = () => {
  const [executionStatus, setExecutionStatus] = useState<ExecutionStatus>({
    last_execution: null,
    status: 'idle',
    message: 'Ready to execute'
  })
  const [executing, setExecuting] = useState(false)
  const [selectedGap, setSelectedGap] = useState<number>(5)
  const [slippage, setSlippage] = useState<number>(30)
  const [loading, setLoading] = useState(false)

  // Fetch execution status periodically
  useEffect(() => {
    const fetchStatus = async () => {
      try {
        const result = await invoke<ExecutionStatus>('get_execution_status')
        setExecutionStatus(result)
      } catch (err) {
        console.error('Error fetching execution status:', err)
      }
    }

    fetchStatus()
    const interval = setInterval(fetchStatus, 5000)
    return () => clearInterval(interval)
  }, [])

  const handleExecuteArbitrage = async () => {
    if (executing) return

    setExecuting(true)
    setLoading(true)

    try {
      setExecutionStatus({
        ...executionStatus,
        status: 'executing',
        message: 'Executing arbitrage...'
      })

      const result = await invoke<ExecutionResult>('execute_arbitrage_optimized', {
        profitLamports: (selectedGap * 1_000_000).toString(), // Convert to lamports
        slippageBps: (slippage * 100).toString(), // Convert to basis points
        bundleId: `bundle_${Date.now()}`
      })

      setExecutionStatus({
        last_execution: result,
        status: result.state === 'success' ? 'success' : 'error',
        message: result.state === 'success' 
          ? `✅ Trade executed! Profit: ${(result.profit / 1_000_000).toFixed(4)} SOL`
          : `❌ Trade failed: ${result.error || 'Unknown error'}`
      })
    } catch (err) {
      const errorMsg = String(err)
      setExecutionStatus({
        last_execution: null,
        status: 'error',
        message: `❌ Execution error: ${errorMsg}`
      })
    } finally {
      setExecuting(false)
      setLoading(false)
    }
  }

  const handleRecoverFromFailure = async () => {
    try {
      setExecutionStatus({
        ...executionStatus,
        status: 'executing',
        message: 'Recovering from failure...'
      })

      await invoke('recover_from_failure')

      setExecutionStatus({
        last_execution: null,
        status: 'success',
        message: '✅ Recovery initiated successfully'
      })
    } catch (err) {
      setExecutionStatus({
        last_execution: null,
        status: 'error',
        message: `❌ Recovery failed: ${String(err)}`
      })
    }
  }

  const getStatusColor = () => {
    switch (executionStatus.status) {
      case 'success': return '#00FF00'
      case 'error': return '#FF0000'
      case 'executing': return '#FFA500'
      default: return '#4CAF50'
    }
  }

  return (
    <div className="execution-panel">
      <div className="execution-container">
        {/* Status Display */}
        <div className="status-display" style={{ borderColor: getStatusColor() }}>
          <div className="status-header">
            <span className="status-icon" style={{ color: getStatusColor() }}>
              {executionStatus.status === 'executing' && '⏳'}
              {executionStatus.status === 'success' && '✅'}
              {executionStatus.status === 'error' && '❌'}
              {executionStatus.status === 'idle' && '🎯'}
            </span>
            <h2>Execution Status</h2>
          </div>

          <div className="status-message">{executionStatus.message}</div>

          {executionStatus.last_execution && (
            <div className="last-execution">
              <div className="exec-detail">
                <label>Signature:</label>
                <code>{executionStatus.last_execution.signature}</code>
              </div>
              <div className="exec-detail">
                <label>Profit:</label>
                <span className="profit">
                  {(executionStatus.last_execution.profit / 1_000_000).toFixed(4)} SOL
                </span>
              </div>
              <div className="exec-detail">
                <label>Execution Time:</label>
                <span>{executionStatus.last_execution.execution_time_ms}ms</span>
              </div>
            </div>
          )}
        </div>

        {/* Execution Controls */}
        <div className="execution-controls">
          <div className="control-group">
            <label>
              <span>Minimum Gap Threshold (%)</span>
              <input
                type="range"
                min="0.5"
                max="10"
                step="0.5"
                value={selectedGap}
                onChange={(e) => setSelectedGap(parseFloat(e.target.value))}
                disabled={executing}
              />
              <span className="value-display">{selectedGap.toFixed(1)}%</span>
            </label>
            <p className="help-text">
              Only execute arbitrage if profit gap exceeds this threshold
            </p>
          </div>

          <div className="control-group">
            <label>
              <span>Maximum Slippage (%) </span>
              <input
                type="range"
                min="1"
                max="100"
                step="1"
                value={slippage}
                onChange={(e) => setSlippage(parseFloat(e.target.value))}
                disabled={executing}
              />
              <span className="value-display">{slippage}%</span>
            </label>
            <p className="help-text">
              Protect against price impact and sandwich attacks
            </p>
          </div>
        </div>

        {/* Action Buttons */}
        <div className="action-buttons">
          <button
            className="execute-btn"
            onClick={handleExecuteArbitrage}
            disabled={executing || loading}
          >
            {executing ? (
              <>
                <span className="spinner">⏳</span> Executing...
              </>
            ) : (
              <>
                <span>🚀</span> Execute Arbitrage
              </>
            )}
          </button>

          {executionStatus.status === 'error' && (
            <button
              className="recovery-btn"
              onClick={handleRecoverFromFailure}
              disabled={executing}
            >
              <span>🔄</span> Recover from Failure
            </button>
          )}
        </div>

        {/* Risk Warning */}
        <div className="risk-warning">
          <p>
            <strong>⚠️ Risk Disclaimer:</strong> Cryptocurrency trading and arbitrage involves
            significant risk. You may lose money. Always verify parameters before executing trades.
            This tool is provided as-is without warranties.
          </p>
        </div>
      </div>

      <style jsx>{`
        .execution-panel {
          padding: 20px;
          background: #f5f5f5;
          border-radius: 8px;
        }

        .status-display {
          border-left: 4px solid;
          padding: 15px;
          background: white;
          border-radius: 6px;
          margin-bottom: 20px;
        }

        .status-header {
          display: flex;
          align-items: center;
          gap: 10px;
          margin-bottom: 10px;
        }

        .status-icon {
          font-size: 24px;
        }

        .status-message {
          font-size: 14px;
          color: #333;
          margin-bottom: 10px;
        }

        .last-execution {
          margin-top: 15px;
          padding-top: 15px;
          border-top: 1px solid #eee;
        }

        .exec-detail {
          display: flex;
          justify-content: space-between;
          padding: 8px 0;
          font-size: 12px;
        }

        .profit {
          color: #00AA00;
          font-weight: bold;
        }

        .execution-controls {
          background: white;
          padding: 20px;
          border-radius: 6px;
          margin-bottom: 20px;
        }

        .control-group {
          margin-bottom: 20px;
        }

        .control-group label {
          display: flex;
          align-items: center;
          gap: 10px;
          margin-bottom: 8px;
        }

        .control-group input[type="range"] {
          flex: 1;
          max-width: 200px;
        }

        .value-display {
          font-weight: bold;
          color: #1976d2;
          min-width: 50px;
        }

        .help-text {
          color: #666;
          font-size: 12px;
          margin: 0;
        }

        .action-buttons {
          display: flex;
          gap: 10px;
          margin-bottom: 20px;
        }

        .execute-btn,
        .recovery-btn {
          flex: 1;
          padding: 12px;
          border: none;
          border-radius: 6px;
          font-size: 14px;
          font-weight: bold;
          cursor: pointer;
          transition: all 0.3s;
        }

        .execute-btn {
          background: #4CAF50;
          color: white;
        }

        .execute-btn:hover:not(:disabled) {
          background: #45a049;
        }

        .execute-btn:disabled {
          background: #ccc;
          cursor: not-allowed;
        }

        .recovery-btn {
          background: #FF9800;
          color: white;
        }

        .recovery-btn:hover:not(:disabled) {
          background: #e68900;
        }

        .risk-warning {
          background: #fff3cd;
          border-left: 4px solid #ffc107;
          padding: 15px;
          border-radius: 6px;
          font-size: 12px;
          color: #333;
        }

        .risk-warning p {
          margin: 0;
        }
      `}</style>
    </div>
  )
}

export default ExecutionPanel
