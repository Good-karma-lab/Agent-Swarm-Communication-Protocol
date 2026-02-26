function isBusinessMessage(m) {
  const method = (m?.method || '').toLowerCase()
  const topic = (m?.topic || '').toLowerCase()

  if (!method && !topic) return false

  if (
    method.includes('keepalive') ||
    method === 'swarm.announce' ||
    method === 'swarm.join' ||
    method === 'swarm.join_response' ||
    method === 'swarm.leave' ||
    method === 'hierarchy.assign_tier'
  ) {
    return false
  }

  if (
    topic.includes('/keepalive') ||
    topic.includes('/swarm/discovery') ||
    topic.includes('/swarm/public/announce')
  ) {
    return false
  }

  return true
}

export default function MessagesPanel({ messages }) {
  const filtered = (messages || []).filter(isBusinessMessage)

  return (
    <div className="card">
      <h2>Peer-to-Peer Debug Logs</h2>
      <div className="log mono">
        {filtered.map((m, idx) => (
          <div key={`${m.timestamp}-${idx}`}>
            [{m.timestamp}] {m.direction} {m.topic} {m.method || '-'} peer={m.peer || '-'} task={m.task_id || '-'} {m.outcome}
          </div>
        ))}
      </div>
    </div>
  )
}
