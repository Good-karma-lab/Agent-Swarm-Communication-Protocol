import { useState, useEffect } from 'react'
import { getHolon, getHolons } from '../api/client.js'

function scrub(s) {
  return String(s || '').replace(/did:swarm:[A-Za-z0-9]+/g, m => '[' + m.slice(-6) + ']')
}

const STATUS_DOT = {
  Forming:      '#4a7a9b',
  Deliberating: '#ffaa00',
  Voting:       '#ff3355',
  Executing:    '#7c3aff',
  Synthesizing: '#7c3aff',
  Done:         '#00e5b0',
}

export default function HolonDetailPanel({ open, taskId, onClose }) {
  const [holon, setHolon] = useState(null)
  const [allHolons, setAllHolons] = useState([])

  useEffect(() => {
    if (!open || !taskId) return
    Promise.all([
      getHolon(taskId).catch(() => null),
      getHolons().catch(() => []),
    ]).then(([h, hs]) => {
      setHolon(h)
      setAllHolons(Array.isArray(hs) ? hs : [])
    })
  }, [open, taskId])

  const children = allHolons.filter(h => h.parent_holon === taskId)

  return (
    <>
      <div className={`panel-overlay ${open ? 'open' : ''}`} onClick={onClose} />
      <div className={`slide-panel ${open ? 'open' : ''}`}>
        <div className="panel-header">
          <span className="panel-title">Holon Detail</span>
          <button className="panel-close" onClick={onClose}>✕</button>
        </div>
        <div className="panel-body">
          {!holon && (
            <div style={{color:'var(--text-muted)', fontSize:12}}>
              {taskId ? 'Loading…' : 'No holon selected.'}
            </div>
          )}
          {holon && (
            <>
              <div style={{marginBottom:16}}>
                <div style={{fontFamily:'JetBrains Mono,monospace', fontSize:11, color:'var(--text-muted)', marginBottom:4}}>Task ID</div>
                <div style={{fontFamily:'JetBrains Mono,monospace', fontSize:12, color:'var(--teal)', wordBreak:'break-all'}}>{holon.task_id}</div>
              </div>

              <div style={{display:'flex', gap:8, flexWrap:'wrap', marginBottom:16}}>
                <span style={{
                  display:'inline-flex', alignItems:'center', gap:4,
                  padding:'2px 8px', border:`1px solid ${STATUS_DOT[holon.status] || '#4a7a9b'}`,
                  borderRadius:3, fontSize:10, fontWeight:700,
                  color: STATUS_DOT[holon.status] || '#4a7a9b'
                }}>{holon.status}</span>
                <span style={{color:'var(--text-muted)', fontSize:12}}>Depth {holon.depth}</span>
              </div>

              <div className="section-header" style={{marginBottom:8}}>Composition</div>
              <div style={{marginBottom:16}}>
                <div className="id-field">
                  <span className="id-label">Chair</span>
                  <span className="mono" style={{color:'var(--teal)'}}>{scrub(holon.chair)}</span>
                </div>
                {holon.adversarial_critic && (
                  <div className="id-field">
                    <span className="id-label">Critic</span>
                    <span className="mono" style={{color:'var(--coral)'}}>{scrub(holon.adversarial_critic)}</span>
                  </div>
                )}
                <div className="id-field">
                  <span className="id-label">Members</span>
                  <span className="mono">{holon.members?.length || 0}</span>
                </div>
                {holon.parent_holon && (
                  <div className="id-field">
                    <span className="id-label">Parent</span>
                    <span className="mono">{holon.parent_holon?.slice(0,16)}…</span>
                  </div>
                )}
              </div>

              {holon.members?.length > 0 && (
                <>
                  <div className="section-header" style={{marginBottom:8}}>Members</div>
                  <div style={{display:'flex', flexWrap:'wrap', gap:6, marginBottom:16}}>
                    {holon.members.map((m, i) => (
                      <span key={i} style={{
                        padding:'2px 8px', background:'var(--surface-2)',
                        border:'1px solid var(--border-2)', borderRadius:3,
                        fontFamily:'JetBrains Mono,monospace', fontSize:11, color:'var(--text-muted)'
                      }}>{scrub(m)}</span>
                    ))}
                  </div>
                </>
              )}

              {children.length > 0 && (
                <>
                  <div className="section-header" style={{marginBottom:8}}>Child Holons ({children.length})</div>
                  {children.map(child => (
                    <div key={child.task_id} style={{
                      padding:'6px 10px', background:'var(--surface-2)',
                      border:'1px solid var(--border)', borderRadius:5,
                      marginBottom:4, display:'flex', gap:10,
                      alignItems:'center', fontSize:11,
                      fontFamily:'JetBrains Mono,monospace',
                    }}>
                      <span style={{
                        color: STATUS_DOT[child.status] || '#4a7a9b',
                        fontWeight:700, fontSize:10
                      }}>{child.status}</span>
                      <span style={{color:'var(--teal)'}}>{(child.task_id || '').slice(0,14)}…</span>
                      <span style={{color:'var(--text-muted)'}}>d{child.depth}</span>
                    </div>
                  ))}
                </>
              )}
            </>
          )}
        </div>
      </div>
    </>
  )
}
