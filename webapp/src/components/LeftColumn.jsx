import { useState } from 'react'

const TIER_LABELS = {
  newcomer: 'Newcomer', member: 'Member', trusted: 'Trusted',
  established: 'Established', veteran: 'Veteran', suspended: 'Suspended'
}

function ttlLabel(expiresAt) {
  if (!expiresAt) return '—'
  const secs = Math.max(0, Math.floor((new Date(expiresAt) - Date.now()) / 1000))
  if (secs < 60)    return `${secs}s`
  if (secs < 3600)  return `${Math.floor(secs/60)}m`
  if (secs < 86400) return `${Math.floor(secs/3600)}h`
  return `${Math.floor(secs/86400)}d`
}

function truncate(s, len=10) {
  return s ? `[${s.slice(0,len)}…]` : '—'
}

function nextTier(tier) {
  const seq = ['newcomer','member','trusted','established','veteran']
  const idx = seq.indexOf(tier)
  return seq[Math.min(idx + 1, seq.length - 1)]
}

export default function LeftColumn({
  identity, reputation, names, keys, network,
  onOpenNameRegistry, onOpenKeyMgmt, onOpenReputation,
  onOpenTask, onOpenMessages
}) {
  const tier  = identity?.tier   || 'newcomer'
  const score = reputation?.score ?? 0
  const nextAt = reputation?.next_tier_at ?? 1000
  const pct   = Math.min(100, Math.round((score / nextAt) * 100))

  return (
    <aside className="col-left">
      {/* Identity */}
      <div className="section">
        <div className="section-header">My Agent</div>
        <div className="identity-name">{identity?.wws_name || '—'}</div>
        <div className={`tier-badge ${tier}`}>{TIER_LABELS[tier]}</div>
        <div className="rep-score"
             style={{cursor:'pointer'}}
             onClick={onOpenReputation}
             title="View reputation detail">
          {score} pts
        </div>
        <div className="rep-next">→ {TIER_LABELS[nextTier(tier)]} at {nextAt}</div>
        <div className="rep-bar-track">
          <div className="rep-bar-fill" style={{width:`${pct}%`}} />
        </div>
        <div className="id-field">
          <span className="id-label">DID</span>
          <span className="id-value mono" title={identity?.did}>
            {truncate(identity?.did?.replace('did:swarm:',''), 8)}
          </span>
        </div>
        <div className="id-field">
          <span className="id-label">PeerID</span>
          <span className="id-value mono" title={identity?.peer_id}>
            {truncate(identity?.peer_id, 8)}
          </span>
        </div>
      </div>

      {/* Names */}
      <div className="section">
        <div className="section-header">Names</div>
        {(!names || names.length === 0) && (
          <div className="dim" style={{fontSize:11}}>No registered names</div>
        )}
        {(names || []).map(n => {
          const ttl = ttlLabel(n.expires_at)
          const warn = n.expires_at && (new Date(n.expires_at) - Date.now()) < 7200_000
          return (
            <div className="name-row" key={n.name}>
              <span className="name-label">{n.name}</span>
              <span className={`name-ttl ${warn ? 'warning' : ''}`}>{ttl}</span>
              <button className="name-renew" title="Renew" onClick={() => {}}>↻</button>
            </div>
          )
        })}
        <button className="add-name-btn" onClick={onOpenNameRegistry}>+ Register name</button>
      </div>

      {/* Key Health */}
      <div className="section">
        <div className="section-header" style={{cursor:'pointer'}} onClick={onOpenKeyMgmt}>
          Key Health ›
        </div>
        <div className="status-row" onClick={onOpenKeyMgmt}>
          <div className={`status-dot ${identity?.key_healthy ? 'ok' : 'error'}`} />
          <span className="status-label">keypair</span>
          <span className="status-value">{identity?.key_healthy ? 'ok' : 'missing'}</span>
        </div>
        <div className="status-row" onClick={onOpenKeyMgmt}>
          <div className={`status-dot ${(keys?.guardian_count ?? 0) > 0 ? 'ok' : 'off'}`} />
          <span className="status-label">guardians</span>
          <span className="status-value">{keys?.guardian_count ?? 0}/{keys?.threshold ?? 0}</span>
        </div>
        <div className="status-row" onClick={onOpenKeyMgmt}>
          <div className={`status-dot ${keys?.last_rotation ? 'ok' : 'off'}`} />
          <span className="status-label">rotation</span>
          <span className="status-value">{keys?.last_rotation ? 'done' : 'never'}</span>
        </div>
      </div>

      {/* Network */}
      <div className="section">
        <div className="section-header">Network</div>
        <div className="status-row">
          <div className={`status-dot ${network?.bootstrap_connected ? 'ok' : 'error'}`} />
          <span className="status-label">bootstrap</span>
          <span className="status-value">{network?.bootstrap_connected ? 'ok' : 'offline'}</span>
        </div>
        <div className="status-row">
          <div className="status-dot ok" />
          <span className="status-label">NAT</span>
          <span className="status-value">{network?.nat_type || '—'}</span>
        </div>
        <div className="status-row">
          <div className={`status-dot ${(network?.peer_count ?? 0) > 0 ? 'ok' : 'off'}`} />
          <span className="status-label">peers</span>
          <span className="status-value">{network?.peer_count ?? 0} direct</span>
        </div>
      </div>

      {/* Quick Links */}
      <div className="section">
        <div className="section-header">Quick Links</div>
        {identity?.assigned_task_id && (
          <div className="quick-link" onClick={() => onOpenTask(identity.assigned_task_id)}>
            <span className="quick-link-icon">⚡</span>
            <span className="mono" style={{fontSize:11}}>{identity.assigned_task_id.slice(0,16)}…</span>
          </div>
        )}
        <div className="quick-link" onClick={onOpenMessages}>
          <span className="quick-link-icon">📨</span>
          <span>Messages</span>
        </div>
      </div>
    </aside>
  )
}
