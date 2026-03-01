const BASE = ''

export async function fetchJson(url, options = {}) {
  const res = await fetch(BASE + url, options)
  const ct = res.headers.get('content-type') || ''
  if (!ct.includes('application/json')) {
    const err = new Error(`HTTP ${res.status}`)
    err.status = res.status
    throw err
  }
  const data = await res.json()
  if (!res.ok) {
    const err = new Error(data.error || `HTTP ${res.status}`)
    err.status = res.status
    throw err
  }
  return data
}

// ── Identity & Reputation ─────────────────────────────────────
export const getIdentity       = () => fetchJson('/api/identity')
export const getReputation     = () => fetchJson('/api/reputation')
export const getRepEvents      = (limit=50, offset=0) =>
  fetchJson(`/api/reputation/events?limit=${limit}&offset=${offset}`)

// ── Names ─────────────────────────────────────────────────────
export const getMyNames        = () => fetchJson('/api/names')
export const registerName      = (name) => fetchJson('/api/names', {
  method: 'POST',
  headers: {'Content-Type':'application/json'},
  body: JSON.stringify({name})
})
export const renewName         = (name) => fetchJson(`/api/names/${encodeURIComponent(name)}/renew`, { method: 'PUT' })
export const releaseName       = (name) => fetchJson(`/api/names/${encodeURIComponent(name)}`, { method: 'DELETE' })

// ── Network & Peers ───────────────────────────────────────────
export const getNetwork        = () => fetchJson('/api/network')
export const getPeers          = () => fetchJson('/api/peers')

// ── Directory ─────────────────────────────────────────────────
export const getDirectory      = (params = {}) => {
  const q = new URLSearchParams()
  if (params.q)      q.set('q', params.q)
  if (params.tier)   q.set('tier', params.tier)
  if (params.sort)   q.set('sort', params.sort || 'reputation')
  if (params.limit)  q.set('limit', params.limit)
  if (params.offset) q.set('offset', params.offset)
  return fetchJson(`/api/directory?${q}`)
}

// ── Keys ──────────────────────────────────────────────────────
export const getKeys           = () => fetchJson('/api/keys')

// ── Existing endpoints (preserved) ───────────────────────────
export const getTasks          = () => fetchJson('/api/tasks')
export const getTask           = (id) => fetchJson(`/api/tasks/${id}`)
export const submitTask        = (payload) => fetchJson('/api/tasks', {
  method: 'POST',
  headers: {'Content-Type':'application/json'},
  body: JSON.stringify(payload)
})
export const getHolons         = () => fetchJson('/api/holons')
export const getHolon          = (taskId) => fetchJson(`/api/holons/${taskId}`)
export const getDeliberation   = (taskId) => fetchJson(`/api/tasks/${taskId}/deliberation`)
export const getBallots        = (taskId) => fetchJson(`/api/tasks/${taskId}/ballots`)
export const getIrvRounds      = (taskId) => fetchJson(`/api/tasks/${taskId}/irv-rounds`)
export const getAuditLog       = () => fetchJson('/api/audit')
export const getGraph          = () => fetchJson('/api/graph')
export const getMessages       = () => fetchJson('/api/messages')
