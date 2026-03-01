import { useState, useEffect, useCallback } from 'react'
import { getDirectory } from '../api/client.js'

const TIERS = ['all','veteran','established','trusted','member','newcomer']

export default function CenterDirectory({ onSelectAgent }) {
  const [query,   setQuery]   = useState('')
  const [tier,    setTier]    = useState('all')
  const [agents,  setAgents]  = useState([])
  const [loading, setLoading] = useState(false)

  const load = useCallback(async () => {
    setLoading(true)
    try {
      const params = { limit: 100 }
      if (query) params.q = query
      if (tier !== 'all') params.tier = tier
      const data = await getDirectory(params)
      setAgents(Array.isArray(data) ? data : [])
    } catch {
      setAgents([])
    } finally {
      setLoading(false)
    }
  }, [query, tier])

  useEffect(() => {
    const t = setTimeout(load, 250)
    return () => clearTimeout(t)
  }, [load])

  function lastSeenLabel(ts) {
    if (!ts) return '—'
    const s = Math.floor((Date.now() - new Date(ts)) / 1000)
    if (s < 60)  return `${s}s ago`
    if (s < 3600) return `${Math.floor(s/60)}m ago`
    return `${Math.floor(s/3600)}h ago`
  }

  return (
    <div className="directory-container">
      <div className="directory-toolbar">
        <input
          className="search-input"
          placeholder="Search wws:name or DID…"
          value={query}
          onChange={e => setQuery(e.target.value)}
        />
        <div className="filter-chips">
          {TIERS.map(t => (
            <button
              key={t}
              className={`filter-chip ${tier === t ? 'active' : ''}`}
              onClick={() => setTier(t)}
            >
              {t.charAt(0).toUpperCase() + t.slice(1)}
            </button>
          ))}
        </div>
      </div>
      <div className="directory-list">
        {loading && <div style={{padding:'16px',color:'var(--text-muted)',textAlign:'center'}}>Loading…</div>}
        {!loading && agents.length === 0 && (
          <div style={{padding:'16px',color:'var(--text-muted)',textAlign:'center'}}>
            No agents found
          </div>
        )}
        {agents.map(a => (
          <div className="agent-row" key={a.did || a.wws_name} onClick={() => onSelectAgent?.(a)}>
            <div style={{flex:'0 0 auto'}}>
              <div className="agent-row-name">{a.wws_name || '—'}</div>
              <div className="agent-row-did">{a.did}</div>
            </div>
            <div className="agent-row-meta">
              <span className={`tier-badge ${a.tier || 'newcomer'}`} style={{fontSize:9}}>
                {a.tier || 'newcomer'}
              </span>
              <span className="agent-row-score">{a.score ?? '—'} pts</span>
              <span className="agent-row-seen">{lastSeenLabel(a.last_seen)}</span>
            </div>
          </div>
        ))}
      </div>
    </div>
  )
}
