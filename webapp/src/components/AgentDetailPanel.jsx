function scrub(s) {
  return String(s || '').replace(/did:swarm:[A-Za-z0-9]+/g, m => '[' + m.slice(-6) + ']')
}

function healthLabel(a) {
  if (!a.connected) return { text: 'DOWN', cls: 'badge-coral' }
  if (!a.loop_active) return { text: 'DEGRADED', cls: 'badge-amber' }
  return { text: 'HEALTHY', cls: 'badge-teal' }
}

export default function AgentDetailPanel({ agent, tasks, onTaskClick }) {
  const health = healthLabel(agent)
  const taskList = (tasks?.tasks || []).filter(t =>
    t.assigned_to === agent.agent_id || t.assigned_to_name === agent.name
  )

  return (
    <div>
      {/* Header meta */}
      <div className="detail-meta" style={{ marginBottom: 20 }}>
        <span>ID: <strong>{scrub(agent.agent_id)}</strong></span>
        <span>Name: <strong>{scrub(agent.name)}</strong></span>
        <span>Tier: <strong>{agent.tier}</strong></span>
        <span className={`badge ${health.cls}`}>{health.text}</span>
      </div>

      {/* Stats */}
      <div className="detail-section">
        <div className="detail-section-title">Activity</div>
        <table className="data-table">
          <thead>
            <tr>
              <th>Metric</th>
              <th>Value</th>
            </tr>
          </thead>
          <tbody>
            <tr><td>Connected</td><td>{agent.connected ? 'yes' : 'no'}</td></tr>
            <tr><td>Loop active</td><td>{agent.loop_active ? 'yes' : 'no'}</td></tr>
            <tr><td>Tasks assigned</td><td>{agent.tasks_assigned_count ?? 0}</td></tr>
            <tr><td>Tasks processed</td><td>{agent.tasks_processed_count ?? 0}</td></tr>
            <tr><td>Plans proposed</td><td>{agent.plans_proposed_count ?? 0}</td></tr>
            <tr><td>Plans revealed</td><td>{agent.plans_revealed_count ?? 0}</td></tr>
            <tr><td>Votes cast</td><td>{agent.votes_cast_count ?? 0}</td></tr>
            <tr><td>Last poll (s)</td><td>{agent.last_task_poll_secs ?? '—'}</td></tr>
            <tr><td>Last result (s)</td><td>{agent.last_result_secs ?? '—'}</td></tr>
          </tbody>
        </table>
      </div>

      {/* Assigned tasks */}
      {taskList.length > 0 && (
        <div className="detail-section">
          <div className="detail-section-title">Assigned Tasks</div>
          {taskList.map(t => (
            <div
              key={t.task_id}
              onClick={() => onTaskClick && onTaskClick(t)}
              style={{
                padding: '6px 10px',
                background: 'var(--surface-2)',
                border: '1px solid var(--border)',
                borderRadius: 5,
                marginBottom: 4,
                cursor: 'pointer',
                fontFamily: 'var(--font-mono)',
                fontSize: 11,
              }}
            >
              <span style={{ color: 'var(--teal)' }}>{t.task_id.slice(0, 12)}…</span>
              {' '}
              <span style={{ color: 'var(--text-muted)' }}>{t.status}</span>
              {' '}
              <span>{t.description?.slice(0, 60) || ''}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
