export default function OverviewPanel({ flow, live, voting, messages }) {
  return (
    <div className="grid">
      <div className="card">
        <h2>Flow Counters</h2>
        <pre className="mono">{JSON.stringify(flow?.counters || {}, null, 2)}</pre>
      </div>
      <div className="card">
        <h2>Recent Event Log</h2>
        <div className="log mono">
          {(live?.events || []).map((e, i) => (
            <div key={`${e.timestamp}-${i}`}>
              [{e.timestamp}] {e.category}: {e.message}
            </div>
          ))}
        </div>
      </div>
      <div className="card">
        <h2>Voting</h2>
        <div className="muted">engines={voting?.voting?.length || 0} rfp={voting?.rfp?.length || 0}</div>
      </div>
      <div className="card">
        <h2>P2P Messages</h2>
        <div className="muted">trace items={(messages || []).length}</div>
      </div>
    </div>
  )
}
