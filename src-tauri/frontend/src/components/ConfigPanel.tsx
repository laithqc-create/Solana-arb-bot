// src-tauri/frontend/src/components/ConfigPanel.tsx
import React, { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/tauri'
import '../styles/ConfigPanel.css'

interface Props {
  onConfigUpdate: () => void
}

const ConfigPanel: React.FC<Props> = ({ onConfigUpdate }) => {
  const [geyserUrl, setGeyserUrl] = useState('')
  const [backupUrl, setBackupUrl] = useState('')
  const [jitoRegion, setJitoRegion] = useState('us-west')
  const [saving, setSaving] = useState(false)
  const [message, setMessage] = useState('')
  const [showVault, setShowVault] = useState(false)
  const [vaultPassword, setVaultPassword] = useState('')

  // Load config on mount
  useEffect(() => {
    loadConfig()
  }, [])

  const loadConfig = async () => {
    try {
      const result = await invoke('get_vault_config') as string
      const data = JSON.parse(result)
      if (data.success) {
        setGeyserUrl(data.config.geyser_rpc_url)
        setBackupUrl(data.config.backup_rpc_url)
        setJitoRegion(data.config.jito_region)
      }
    } catch (error) {
      console.error('Failed to load config:', error)
      setMessage('⚠️ Failed to load configuration')
    }
  }

  const handleSaveConfig = async () => {
    setSaving(true)
    setMessage('')
    try {
      const result = await invoke('update_config', {
        geyserUrl,
        backupUrl,
      }) as string
      const data = JSON.parse(result)
      if (data.success) {
        setMessage('✅ Configuration saved successfully')
        onConfigUpdate()
        setTimeout(() => setMessage(''), 3000)
      } else {
        setMessage(`❌ ${data.error}`)
      }
    } catch (error) {
      console.error('Failed to save config:', error)
      setMessage('❌ Failed to save configuration')
    }
    setSaving(false)
  }

  return (
    <div className="config-panel">
      <div className="config-container">
        {/* RPC Configuration */}
        <div className="config-section">
          <h2>🔗 RPC Configuration</h2>
          
          <div className="config-group">
            <label htmlFor="geyser-url">
              <span>Primary: Yellowstone Geyser gRPC</span>
              <input
                id="geyser-url"
                type="text"
                value={geyserUrl}
                onChange={(e) => setGeyserUrl(e.target.value)}
                placeholder="wss://mainnet.helius-rpc.com/ws"
              />
            </label>
            <p className="help-text">Ultra-low latency gRPC stream for real-time pool updates</p>
          </div>

          <div className="config-group">
            <label htmlFor="backup-url">
              <span>Fallback: JSON-RPC Endpoint</span>
              <input
                id="backup-url"
                type="text"
                value={backupUrl}
                onChange={(e) => setBackupUrl(e.target.value)}
                placeholder="https://api.mainnet-beta.solana.com"
              />
            </label>
            <p className="help-text">Used if Geyser connection lags &gt;2 slots (800ms)</p>
          </div>
        </div>

        {/* Jito Configuration */}
        <div className="config-section">
          <h2>⚡ Jito Bundle Configuration</h2>
          
          <div className="config-group">
            <label htmlFor="jito-region">
              <span>Jito Region</span>
              <select 
                id="jito-region"
                value={jitoRegion}
                onChange={(e) => setJitoRegion(e.target.value)}
              >
                <option value="us-west">🌍 US-West (Default)</option>
                <option value="us-east">🌍 US-East</option>
                <option value="eu">🌍 Europe</option>
                <option value="asia">🌍 Asia</option>
              </select>
            </label>
            <p className="help-text">Geographic region for Jito block engine submission</p>
          </div>

          <div className="info-box">
            <p>
              <strong>Bundle Tipping:</strong> Dynamic tip calculated as 85-90% of gross arbitrage profit.
              Target positioning: [User Swap] → [Your Arbitrage + Jito Tip]
            </p>
          </div>
        </div>

        {/* Vault & Encryption */}
        <div className="config-section">
          <h2>🔐 Vault & Encryption</h2>
          
          <div className="config-group">
            <button 
              className="toggle-vault-btn"
              onClick={() => setShowVault(!showVault)}
            >
              {showVault ? '🔓 Hide Vault' : '🔒 Manage Vault'}
            </button>
          </div>

          {showVault && (
            <div className="vault-section">
              <p className="help-text">
                All sensitive data is encrypted with AES-256-GCM using your password.
              </p>
              
              <div className="config-group">
                <label htmlFor="vault-pass">
                  <span>Vault Password</span>
                  <input
                    id="vault-pass"
                    type="password"
                    value={vaultPassword}
                    onChange={(e) => setVaultPassword(e.target.value)}
                    placeholder="Enter encryption password"
                  />
                </label>
                <p className="help-text">Used to encrypt/decrypt wallet and API keys locally</p>
              </div>

              <div className="vault-info">
                <p>✅ Storage Location: <code>src/infra/vault/</code></p>
                <p>✅ Encryption: AES-256-GCM</p>
                <p>✅ Key Derivation: Argon2</p>
                <p>⚠️ Never store plaintext private keys</p>
              </div>
            </div>
          )}
        </div>

        {/* Strategy Parameters (Read-Only) */}
        <div className="config-section">
          <h2>📊 Strategy Parameters (Fixed)</h2>
          
          <div className="params-grid">
            <div className="param">
              <span className="param-label">TVL Minimum Filter</span>
              <span className="param-value">$100,000</span>
            </div>
            <div className="param">
              <span className="param-label">30% Gap Rule</span>
              <span className="param-value">Max 30% of spread</span>
            </div>
            <div className="param">
              <span className="param-label">Profit Floor</span>
              <span className="param-value">≥ 0.8% net</span>
            </div>
            <div className="param">
              <span className="param-label">Jito Tip Strategy</span>
              <span className="param-value">85-90% of gross</span>
            </div>
          </div>
        </div>

        {/* Action Buttons */}
        <div className="config-actions">
          <button 
            className="save-btn"
            onClick={handleSaveConfig}
            disabled={saving}
          >
            {saving ? '⟳ Saving...' : '💾 Save Configuration'}
          </button>
          <button 
            className="reset-btn"
            onClick={loadConfig}
          >
            ↻ Reload Defaults
          </button>
        </div>

        {/* Status Message */}
        {message && (
          <div className={`status-message ${message.includes('✅') ? 'success' : 'error'}`}>
            {message}
          </div>
        )}
      </div>

      {/* Information Panels */}
      <div className="info-panels">
        <div className="info-card">
          <h3>🟢 Geyser Advantages</h3>
          <ul>
            <li>Sub-slot latency (≈400ms)</li>
            <li>Real-time account updates</li>
            <li>Reduced slippage detection</li>
          </ul>
        </div>

        <div className="info-card">
          <h3>⚡ Jito Bundle Benefits</h3>
          <ul>
            <li>Private submission (no mempool)</li>
            <li>MEV-protected execution</li>
            <li>Atomic multi-instruction swaps</li>
          </ul>
        </div>

        <div className="info-card">
          <h3>🔐 Vault Security</h3>
          <ul>
            <li>Local-only encryption</li>
            <li>Password-derived keys</li>
            <li>No cloud upload</li>
          </ul>
        </div>
      </div>
    </div>
  )
}

export default ConfigPanel
