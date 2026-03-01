import { useState } from 'react'

const TIER_LABELS = {
  newcomer: 'Newcomer', member: 'Member', trusted: 'Trusted',
  established: 'Established', veteran: 'Veteran', suspended: 'Suspended'
}

export default function Header({ identity, network, view, onViewChange, onAudit, onSettings }) {
  const name  = identity?.wws_name  || '—'
  const tier  = identity?.tier       || 'newcomer'
  const agents = network?.swarm_size_estimate ?? '—'
  const peers  = network?.peer_count ?? '—'

  return (
    <header className="app-header">
      <div className="header-row1">
        <span className="brand">WWS</span>
        <button className="header-identity" onClick={onSettings}>
          {name} <span className={`tier-badge ${tier}`}>{TIER_LABELS[tier]}</span>
        </button>
        <div className="header-stats">
          <span>◎ {agents} agents</span>
          <span>⬡ {peers} peers</span>
        </div>
        <div className="header-spacer" />
        <button className="btn" onClick={onAudit}>Audit</button>
        <button className="btn" style={{marginLeft:6}} onClick={onSettings}>⚙</button>
      </div>
      <div className="header-row2">
        <div className="view-tabs">
          {['graph','directory','activity'].map(v => (
            <button
              key={v}
              className={`view-tab ${view === v ? 'active' : ''}`}
              onClick={() => onViewChange(v)}
            >
              {v.charAt(0).toUpperCase() + v.slice(1)}
            </button>
          ))}
        </div>
      </div>
    </header>
  )
}
