// src-tauri/frontend/src/components/StreamStatus.tsx
import React from 'react'
import '../styles/StreamStatus.css'

interface Props {
  status: string
}

const StreamStatus: React.FC<Props> = ({ status }) => {
  const getStatusInfo = (status: string) => {
    switch (status) {
      case 'GeyserConnected':
        return {
          icon: '🟢',
          label: 'Geyser Connected',
          description: 'Low latency Yellowstone stream',
          color: '#00FF00'
        }
      case 'GeyserLagging':
        return {
          icon: '🟡',
          label: 'Geyser Lagging',
          description: 'High latency, monitoring fallback',
          color: '#FFA500'
        }
      case 'RPCFallback':
        return {
          icon: '🟠',
          label: 'RPC Fallback',
          description: 'Using JSON-RPC polling',
          color: '#FF6600'
        }
      case 'Disconnected':
      default:
        return {
          icon: '🔴',
          label: 'Disconnected',
          description: 'Attempting to reconnect...',
          color: '#FF0000'
        }
    }
  }

  const info = getStatusInfo(status)

  return (
    <div className="stream-status">
      <div className="status-indicator" style={{ color: info.color }}>
        {info.icon}
      </div>
      <div className="status-info">
        <div className="status-label">{info.label}</div>
        <div className="status-description">{info.description}</div>
      </div>
    </div>
  )
}

export default StreamStatus
