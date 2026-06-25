// src-tauri/frontend/src/components/StreamStatus.tsx
import React, { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/tauri'
import '../styles/StreamStatus.css'

interface StreamData {
  active: boolean
  opportunities_found: number
  status: string
  last_update: string
}

const StreamStatus: React.FC = () => {
  const [streamData, setStreamData] = useState<StreamData | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    const fetchStatus = async () => {
      try {
        setLoading(true)
        const result = await invoke<StreamData>('get_stream_status')
        setStreamData(result)
        setError(null)
      } catch (err) {
        setError(String(err))
        console.error('Error fetching stream status:', err)
      } finally {
        setLoading(false)
      }
    }

    // Fetch immediately
    fetchStatus()

    // Then fetch every 2 seconds
    const interval = setInterval(fetchStatus, 2000)
    return () => clearInterval(interval)
  }, [])

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

  if (loading) {
    return <div className="stream-status loading">⏳ Loading...</div>
  }

  if (error) {
    return <div className="stream-status error">❌ Error: {error}</div>
  }

  if (!streamData) {
    return <div className="stream-status">No data</div>
  }

  const info = getStatusInfo(streamData.status)

  return (
    <div className="stream-status">
      <div className="status-indicator" style={{ color: info.color }}>
        {info.icon}
      </div>
      <div className="status-info">
        <div className="status-label">{info.label}</div>
        <div className="status-description">{info.description}</div>
        <div className="status-metrics">
          <span className="metric">Active: {streamData.active ? '✅' : '❌'}</span>
          <span className="metric">Opportunities: {streamData.opportunities_found}</span>
          <span className="metric">Updated: {new Date(streamData.last_update).toLocaleTimeString()}</span>
        </div>
      </div>
    </div>
  )
}

export default StreamStatus
