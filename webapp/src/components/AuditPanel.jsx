export default function AuditPanel({ audit }) {
  return (
    <div className="card">
      <h2>Operator Audit Log</h2>
      <div className="log mono">
        {(audit?.events || []).map((e, i) => (
          <div key={`${e.timestamp}-${i}`}>
            [{e.timestamp}] {e.message}
          </div>
        ))}
      </div>
    </div>
  )
}
