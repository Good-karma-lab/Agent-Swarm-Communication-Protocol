import { useState, useEffect } from 'react'
import { getAuditLog } from '../api/client.js'

function scrub(s) {
  return String(s || '').replace(/did:swarm:[A-Za-z0-9]+/g, m => '[' + m.slice(-6) + ']')
}

export default function AuditPanel({ open, onClose }) {
  const [events, setEvents] = useState([])

  useEffect(() => {
    if (!open) return
    getAuditLog()
      .then(data => setEvents(Array.isArray(data) ? data : (data?.events || [])))
      .catch(() => setEvents([]))
  }, [open])

  return (
    <>
      <div className={`panel-overlay ${open ? 'open' : ''}`} onClick={onClose} />
      <div className={`slide-panel ${open ? 'open' : ''}`}>
        <div className="panel-header">
          <span className="panel-title">Audit Log</span>
          <button className="panel-close" onClick={onClose}>✕</button>
        </div>
        <div className="panel-body">
          <div className="section-header" style={{marginBottom:12}}>Operator Audit Log</div>
          <div style={{fontFamily:'JetBrains Mono,monospace', fontSize:11, lineHeight:1.8}}>
            {events.length === 0 && <div style={{color:'var(--text-dim)'}}>No audit events yet.</div>}
            {events.map((e, i) => (
              <div key={i} style={{marginBottom:2}}>
                <span style={{color:'var(--text-muted)'}}>[{e.timestamp}]</span>{' '}
                {scrub(e.message)}
              </div>
            ))}
          </div>
        </div>
      </div>
    </>
  )
}
