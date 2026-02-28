function taskStatusColor(status) {
  if (!status) return '#4a7a9b'
  const s = status.toLowerCase()
  if (s === 'completed' || s === 'done') return '#00e5b0'
  if (s === 'failed' || s === 'error') return '#ff3355'
  if (s === 'running' || s === 'executing') return '#ffaa00'
  return '#4a7a9b'
}

function agentHealthColor(agent) {
  if (!agent.connected) return '#ff3355'
  if (!agent.loop_active) return '#ffaa00'
  return '#00e5b0'
}

function scrubId(s) {
  return String(s || '').replace(/did:swarm:[A-Za-z0-9]+/g, m => '[' + m.slice(-6) + ']')
}

export default function BottomTray({ agents, tasks, onTaskClick, onAgentClick }) {
  const agentList = agents?.agents || []
  const taskList = tasks?.tasks || []

  const red    = agentList.filter(a => !a.connected).length
  const yellow = agentList.filter(a => a.connected && !a.loop_active).length
  const green  = agentList.filter(a => a.connected && a.loop_active).length

  return (
    <div className="tray">
      {/* Health Summary */}
      <div className="tray-col">
        <div className="tray-label">System Health</div>
        <div className="health-summary">
          <div className="health-row">
            <span className="health-count green">{green}</span>
            <span className="health-dot green" />
            <span style={{ fontSize: 11, color: 'var(--text-muted)' }}>healthy</span>
          </div>
          <div className="health-row">
            <span className="health-count yellow">{yellow}</span>
            <span className="health-dot yellow" />
            <span style={{ fontSize: 11, color: 'var(--text-muted)' }}>degraded</span>
          </div>
          <div className="health-row">
            <span className="health-count red">{red}</span>
            <span className="health-dot red" />
            <span style={{ fontSize: 11, color: 'var(--text-muted)' }}>down</span>
          </div>
          <div style={{ fontSize: 10, color: 'var(--text-muted)', fontFamily: 'var(--font-mono)', marginTop: 4 }}>
            {agentList.length} total agents
          </div>
        </div>
      </div>

      {/* Task Stream */}
      <div className="tray-col">
        <div className="tray-label">Tasks</div>
        <div className="tray-scroll">
          {taskList.length === 0 && (
            <div style={{ color: 'var(--text-dim)', fontSize: 11, fontFamily: 'var(--font-mono)' }}>
              No tasks yet
            </div>
          )}
          {taskList.slice(0, 20).map(t => (
            <div key={t.task_id} className="task-item" onClick={() => onTaskClick(t)}>
              <span className="task-status-dot" style={{ background: taskStatusColor(t.status) }} />
              <span className="task-id">{t.task_id.slice(0, 8)}…</span>
              <span className="task-desc">{t.description || t.task_id}</span>
              <span style={{ fontSize: 10, color: 'var(--text-muted)', flexShrink: 0 }}>{t.status}</span>
            </div>
          ))}
        </div>
      </div>

      {/* Agent Roster */}
      <div className="tray-col">
        <div className="tray-label">Agents</div>
        <div className="tray-scroll">
          {agentList.length === 0 && (
            <div style={{ color: 'var(--text-dim)', fontSize: 11, fontFamily: 'var(--font-mono)' }}>
              No agents connected
            </div>
          )}
          {agentList.slice(0, 16).map(a => (
            <div key={a.agent_id} className="agent-item" onClick={() => onAgentClick(a)}>
              <span className="health-dot" style={{ background: agentHealthColor(a), width: 7, height: 7 }} />
              <span className="agent-name">{scrubId(a.name || a.agent_id)}</span>
              <span className="agent-tier" style={{ fontFamily: 'var(--font-mono)', fontSize: 10 }}>{a.tier}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  )
}
