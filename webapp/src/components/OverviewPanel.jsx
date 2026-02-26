export default function OverviewPanel({ flow, live, voting, messages, agents }) {
  const scrub = (s) => String(s || '').replace(/did:swarm:[A-Za-z0-9]+/g, '[agent]')
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
                [{e.timestamp}] {e.category}: {scrub(e.message)}
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

      <div className="card">
        <h2>Agent Health</h2>
        <table>
          <thead>
            <tr>
              <th>Name</th>
              <th>Tier</th>
              <th>Connected</th>
              <th>Loop</th>
              <th>Poll(s)</th>
              <th>Result(s)</th>
            </tr>
          </thead>
          <tbody>
            {(agents?.agents || []).map((a) => (
              <tr key={a.agent_id}>
                <td>{a.name}</td>
                <td>{a.tier}</td>
                <td>{a.connected ? 'yes' : 'no'}</td>
                <td>{a.loop_active ? 'yes' : 'no'}</td>
                <td>{a.last_task_poll_secs ?? '-'}</td>
                <td>{a.last_result_secs ?? '-'}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  )
}
