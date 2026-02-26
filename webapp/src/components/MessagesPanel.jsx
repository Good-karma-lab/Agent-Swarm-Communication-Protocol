export default function MessagesPanel({ messages }) {
  return (
    <div className="card">
      <h2>Peer-to-Peer Debug Logs</h2>
      <div className="log mono">
        {(messages || []).map((m, idx) => (
          <div key={`${m.timestamp}-${idx}`}>
            [{m.timestamp}] {m.direction} {m.topic} {m.method || '-'} peer={m.peer || '-'} task={m.task_id || '-'} {m.outcome}
          </div>
        ))}
      </div>
    </div>
  )
}
