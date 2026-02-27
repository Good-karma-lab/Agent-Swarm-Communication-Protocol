import { useMemo, useState } from 'react'

export default function OverviewPanel({ flow, live, voting, messages, agents }) {
  const scrub = (s) => String(s || '').replace(/did:swarm:[A-Za-z0-9]+/g, '[agent]')
  const [agentFilter, setAgentFilter] = useState('all')

  const statusRank = (a) => {
    if (!a.connected) return 0
    if (!a.loop_active) return 1
    return 2
  }

  const healthLabel = (a) => {
    if (!a.connected) return 'RED'
    if (!a.loop_active) return 'YELLOW'
    return 'GREEN'
  }

  const filteredAgents = useMemo(() => {
    const arr = [...(agents?.agents || [])]
      .sort((a, b) => {
        const sr = statusRank(a) - statusRank(b)
        if (sr !== 0) return sr
        return (b.last_task_poll_secs ?? -1) - (a.last_task_poll_secs ?? -1)
      })

    if (agentFilter === 'unhealthy') return arr.filter((a) => !a.connected || !a.loop_active)
    if (agentFilter === 'no-loop') return arr.filter((a) => a.connected && !a.loop_active)
    if (agentFilter === 'disconnected') return arr.filter((a) => !a.connected)
    return arr
  }, [agents, agentFilter])

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
        <div className="row" style={{ marginBottom: 8 }}>
          <button className={agentFilter === 'all' ? 'active' : ''} onClick={() => setAgentFilter('all')}>all</button>
          <button className={agentFilter === 'unhealthy' ? 'active' : ''} onClick={() => setAgentFilter('unhealthy')}>unhealthy</button>
          <button className={agentFilter === 'no-loop' ? 'active' : ''} onClick={() => setAgentFilter('no-loop')}>no-loop</button>
          <button className={agentFilter === 'disconnected' ? 'active' : ''} onClick={() => setAgentFilter('disconnected')}>disconnected</button>
        </div>
        <table>
          <thead>
            <tr>
              <th>Health</th>
              <th>Name</th>
              <th>Tier</th>
              <th>Connected</th>
              <th>Loop</th>
              <th>Assigned</th>
              <th>Processed</th>
              <th>Proposed</th>
              <th>Revealed</th>
              <th>Voted</th>
              <th>Poll(s)</th>
              <th>Result(s)</th>
            </tr>
          </thead>
          <tbody>
            {filteredAgents.map((a) => (
              <tr key={a.agent_id}>
                <td>{healthLabel(a)}</td>
                <td>{a.name}</td>
                <td>{a.tier}</td>
                <td>{a.connected ? 'yes' : 'no'}</td>
                <td>{a.loop_active ? 'yes' : 'no'}</td>
                <td>{a.tasks_assigned_count ?? 0}</td>
                <td>{a.tasks_processed_count ?? 0}</td>
                <td>{a.plans_proposed_count ?? 0}</td>
                <td>{a.plans_revealed_count ?? 0}</td>
                <td>{a.votes_cast_count ?? 0}</td>
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
