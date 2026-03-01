import { useState, useEffect } from 'react'
import { getRepEvents } from '../api/client.js'

const TIER_THRESHOLDS = {
  newcomer: 0, member: 100, trusted: 500, established: 1000, veteran: 2500
}

export default function ReputationPanel({ open, reputation, onClose }) {
  const [events, setEvents] = useState([])
  const [loading, setLoading] = useState(false)

  useEffect(() => {
    if (!open) return
    setLoading(true)
    getRepEvents(50, 0)
      .then(data => setEvents(Array.isArray(data) ? data : []))
      .catch(() => setEvents([]))
      .finally(() => setLoading(false))
  }, [open])

  if (!reputation) return null

  const { score = 0, positive_total = 0, negative_total = 0, tier = 'newcomer', next_tier_at = 100, decay = 0 } = reputation

  return (
    <>
      <div className={`panel-overlay ${open ? 'open' : ''}`} onClick={onClose} />
      <div className={`slide-panel ${open ? 'open' : ''}`}>
        <div className="panel-header">
          <span className="panel-title">Reputation</span>
          <button className="panel-close" onClick={onClose}>✕</button>
        </div>
        <div className="panel-body">
          <div style={{marginBottom:20}}>
            <div style={{fontSize:28, fontWeight:700, color:'var(--teal)', fontFamily:'JetBrains Mono,monospace'}}>
              {score} pts
            </div>
            <div className={`tier-badge ${tier}`} style={{marginTop:6}}>{tier}</div>
          </div>

          <div className="section-header" style={{marginBottom:10}}>Score Breakdown</div>
          <div style={{display:'grid', gridTemplateColumns:'1fr 1fr', gap:8, marginBottom:20}}>
            <div style={{background:'var(--surface-2)', border:'1px solid var(--border)', borderRadius:6, padding:10}}>
              <div style={{fontSize:10, color:'var(--text-muted)', marginBottom:4}}>POSITIVE</div>
              <div style={{fontSize:16, fontWeight:700, color:'var(--teal)', fontFamily:'JetBrains Mono,monospace'}}>+{positive_total}</div>
            </div>
            <div style={{background:'var(--surface-2)', border:'1px solid var(--border)', borderRadius:6, padding:10}}>
              <div style={{fontSize:10, color:'var(--text-muted)', marginBottom:4}}>NEGATIVE</div>
              <div style={{fontSize:16, fontWeight:700, color:'var(--coral)', fontFamily:'JetBrains Mono,monospace'}}>−{Math.abs(negative_total)}</div>
            </div>
            <div style={{background:'var(--surface-2)', border:'1px solid var(--border)', borderRadius:6, padding:10}}>
              <div style={{fontSize:10, color:'var(--text-muted)', marginBottom:4}}>DECAY</div>
              <div style={{fontSize:16, fontWeight:700, color:'var(--amber)', fontFamily:'JetBrains Mono,monospace'}}>−{decay}</div>
            </div>
            <div style={{background:'var(--surface-2)', border:'1px solid var(--border)', borderRadius:6, padding:10}}>
              <div style={{fontSize:10, color:'var(--text-muted)', marginBottom:4}}>NEXT TIER</div>
              <div style={{fontSize:14, fontWeight:700, color:'var(--text)', fontFamily:'JetBrains Mono,monospace'}}>{next_tier_at}</div>
            </div>
          </div>

          <div className="section-header" style={{marginBottom:10}}>Recent Events</div>
          {loading && <div style={{color:'var(--text-muted)', fontSize:12}}>Loading…</div>}
          {events.map((ev, i) => (
            <div key={i} style={{
              display:'flex', alignItems:'center', gap:8,
              padding:'6px 0', borderBottom:'1px solid var(--border)',
              fontSize:12
            }}>
              <span style={{
                color: ev.points > 0 ? 'var(--teal)' : 'var(--coral)',
                fontFamily:'JetBrains Mono,monospace',
                fontWeight:700,
                minWidth:40
              }}>
                {ev.points > 0 ? '+' : ''}{ev.points}
              </span>
              <span style={{flex:1, color:'var(--text-muted)'}}>{ev.reason || ev.event_type}</span>
              <span style={{fontFamily:'JetBrains Mono,monospace', fontSize:10, color:'var(--text-dim)'}}>
                {ev.timestamp ? new Date(ev.timestamp).toLocaleDateString() : '—'}
              </span>
            </div>
          ))}
          {!loading && events.length === 0 && (
            <div style={{color:'var(--text-muted)', fontSize:12}}>No events yet.</div>
          )}
        </div>
      </div>
    </>
  )
}
