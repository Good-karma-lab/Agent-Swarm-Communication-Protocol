export async function fetchJson(url, options) {
  const res = await fetch(url, options)
  const data = await res.json()
  if (!res.ok) {
    const err = new Error(data.error || 'request_failed')
    err.status = res.status
    err.payload = data
    throw err
  }
  return data
}

export const api = {
  health: () => fetchJson('/api/health'),
  authStatus: () => fetchJson('/api/auth-status'),
  hierarchy: () => fetchJson('/api/hierarchy'),
  voting: () => fetchJson('/api/voting'),
  messages: () => fetchJson('/api/messages'),
  tasks: () => fetchJson('/api/tasks'),
  flow: () => fetchJson('/api/flow'),
  topology: () => fetchJson('/api/topology'),
  recommendations: () => fetchJson('/api/ui-recommendations'),
  audit: () => fetchJson('/api/audit'),
  taskTimeline: (taskId) => fetchJson(`/api/tasks/${taskId}/timeline`),
  submitTask: (description, token) =>
    fetchJson('/api/tasks', {
      method: 'POST',
      headers: {
        'content-type': 'application/json',
        'x-ops-token': token || ''
      },
      body: JSON.stringify({ description })
    })
}
