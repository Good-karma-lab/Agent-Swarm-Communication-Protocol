export default function AuditPanel({ audit }) {
  const scrub = (s) => String(s || '').replace(/did:swarm:[A-Za-z0-9]+/g, '[agent]')
  return (
    <div className="card">
      <h2>Operator Audit Log</h2>
      <div className="log mono">
        {(audit?.events || []).map((e, i) => (
          <div key={`${e.timestamp}-${i}`}>
            [{e.timestamp}] {scrub(e.message)}
          </div>
        ))}
      </div>
    </div>
  )
}
