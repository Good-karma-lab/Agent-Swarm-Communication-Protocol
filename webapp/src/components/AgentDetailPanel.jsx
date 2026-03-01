export default function AgentDetailPanel({ open, agent, onClose }) {
  if (!agent) return null

  function fmtDate(ts) {
    if (!ts) return '—'
    const s = Math.floor((Date.now() - new Date(ts)) / 1000)
    if (s < 60)   return `${s}s ago`
    if (s < 3600) return `${Math.floor(s/60)}m ago`
    if (s < 86400) return `${Math.floor(s/3600)}h ago`
    return new Date(ts).toLocaleDateString()
  }

  const tier = agent.tier || 'newcomer'

  return (
    <>
      <div className={`panel-overlay ${open ? 'open' : ''}`} onClick={onClose} />
      <div className={`slide-panel ${open ? 'open' : ''}`}>
        <div className="panel-header">
          <span className="panel-title">Agent Detail</span>
          <button className="panel-close" onClick={onClose}>✕</button>
        </div>
        <div className="panel-body">
          <div style={{marginBottom:20}}>
            <div style={{fontSize:18, fontWeight:700, color:'var(--teal)'}}>{agent.wws_name || '—'}</div>
            <div className={`tier-badge ${tier}`} style={{marginTop:6}}>{tier}</div>
            <div style={{marginTop:8, fontFamily:'JetBrains Mono,monospace', fontSize:11, color:'var(--text-muted)', wordBreak:'break-all'}}>
              {agent.did}
            </div>
          </div>

          <div className="section-header" style={{marginBottom:10}}>Reputation</div>
          <div style={{marginBottom:16, fontSize:13, fontFamily:'JetBrains Mono,monospace', color:'var(--text)'}}>
            {agent.score ?? '—'} pts · last seen {fmtDate(agent.last_seen)}
          </div>

          {agent.names && agent.names.length > 0 && (
            <>
              <div className="section-header" style={{marginBottom:10}}>Registered Names</div>
              {agent.names.map(n => (
                <div key={n} style={{color:'var(--teal)', fontWeight:600, padding:'3px 0'}}>{n}</div>
              ))}
              <div style={{marginBottom:16}} />
            </>
          )}

          {agent.connection_type && (
            <>
              <div className="section-header" style={{marginBottom:10}}>Connection</div>
              <div style={{fontSize:12, color:'var(--text-muted)', marginBottom:16}}>
                {agent.connection_type}
              </div>
            </>
          )}

          {agent.task_history && agent.task_history.length > 0 && (
            <>
              <div className="section-header" style={{marginBottom:10}}>Task History</div>
              {agent.task_history.slice(0,10).map(t => (
                <div key={t.id} style={{
                  padding:'4px 0', borderBottom:'1px solid var(--border)',
                  fontSize:12, color:'var(--text-muted)', fontFamily:'JetBrains Mono,monospace'
                }}>
                  {t.id?.slice(0,16)}… <span style={{color:'var(--text-dim)'}}>{t.status}</span>
                </div>
              ))}
            </>
          )}
        </div>
      </div>
    </>
  )
}
