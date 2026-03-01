import { useState } from 'react'
import { registerName, renewName, releaseName } from '../api/client.js'

function ttlLabel(expiresAt) {
  if (!expiresAt) return '—'
  const secs = Math.max(0, Math.floor((new Date(expiresAt) - Date.now()) / 1000))
  if (secs < 60)    return `${secs}s`
  if (secs < 3600)  return `${Math.floor(secs/60)}m`
  if (secs < 86400) return `${Math.floor(secs/3600)}h`
  return `${Math.floor(secs/86400)}d`
}

export default function NameRegistryPanel({ open, names, onClose, onRefresh }) {
  const [newName, setNewName]   = useState('')
  const [loading, setLoading]   = useState(false)
  const [error,   setError]     = useState('')
  const [success, setSuccess]   = useState('')

  async function handleRegister(e) {
    e.preventDefault()
    if (!newName.trim()) return
    setLoading(true); setError(''); setSuccess('')
    try {
      await registerName(newName.trim())
      setSuccess(`Registered wws:${newName.trim()} successfully.`)
      setNewName('')
      onRefresh?.()
    } catch (err) {
      setError(err.message || 'Registration failed')
    } finally {
      setLoading(false)
    }
  }

  async function handleRenew(name) {
    try { await renewName(name); onRefresh?.() }
    catch (err) { setError(`Renew failed: ${err.message}`) }
  }

  async function handleRelease(name) {
    if (!confirm(`Release wws:${name}? This cannot be undone.`)) return
    try { await releaseName(name); onRefresh?.() }
    catch (err) { setError(`Release failed: ${err.message}`) }
  }

  return (
    <>
      <div className={`panel-overlay ${open ? 'open' : ''}`} onClick={onClose} />
      <div className={`slide-panel ${open ? 'open' : ''}`}>
        <div className="panel-header">
          <span className="panel-title">Name Registry</span>
          <button className="panel-close" onClick={onClose}>✕</button>
        </div>
        <div className="panel-body">
          <div className="section-header" style={{marginBottom:12}}>Registered Names</div>
          {(!names || names.length === 0) && (
            <div className="dim" style={{marginBottom:16}}>No registered names yet.</div>
          )}
          {(names || []).map(n => {
            const ttl = ttlLabel(n.expires_at)
            const warn = n.expires_at && (new Date(n.expires_at) - Date.now()) < 7200_000
            return (
              <div key={n.name} style={{
                display:'flex', alignItems:'center', gap:8,
                padding:'8px 0', borderBottom:'1px solid var(--border)'
              }}>
                <span style={{flex:1, color:'var(--teal)', fontWeight:600}}>{n.name}</span>
                <span className={`mono ${warn ? 'dim' : ''}`} style={{
                  fontSize:11,
                  color: warn ? 'var(--coral)' : 'var(--text-muted)'
                }}>{ttl}</span>
                <button className="btn" style={{padding:'2px 8px'}} onClick={() => handleRenew(n.name)}>↻ Renew</button>
                <button className="btn btn-danger" style={{padding:'2px 8px'}} onClick={() => handleRelease(n.name)}>Release</button>
              </div>
            )
          })}

          <div style={{marginTop:24}}>
            <div className="section-header" style={{marginBottom:12}}>Register New Name</div>
            {error   && <div style={{color:'var(--coral)',marginBottom:8,fontSize:12}}>{error}</div>}
            {success && <div style={{color:'var(--teal)',marginBottom:8,fontSize:12}}>{success}</div>}
            <form onSubmit={handleRegister}>
              <div className="form-row">
                <label className="form-label">wws: name</label>
                <div style={{display:'flex', gap:8}}>
                  <span style={{
                    padding:'7px 10px',
                    background:'var(--surface-2)',
                    border:'1px solid var(--border-2)',
                    borderRadius:'5px 0 0 5px',
                    color:'var(--text-muted)',
                    fontSize:13
                  }}>wws:</span>
                  <input
                    value={newName}
                    onChange={e => setNewName(e.target.value)}
                    placeholder="alice"
                    style={{borderRadius:'0 5px 5px 0', borderLeft:'none'}}
                    required
                  />
                </div>
              </div>
              <button className="btn btn-primary w-full" type="submit" disabled={loading}>
                {loading ? 'Registering…' : 'Register (PoW)'}
              </button>
            </form>
          </div>
        </div>
      </div>
    </>
  )
}
