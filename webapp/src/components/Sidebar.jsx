export default function Sidebar({
  live,
  auth,
  operatorToken,
  setOperatorToken,
  desc,
  setDesc,
  submitTask,
  submitError,
  refresh,
  taskId,
  setTaskId,
  loadTrace,
  tab,
  setTab
}) {
  const tabs = ['overview', 'hierarchy', 'voting', 'messages', 'task', 'topology', 'audit', 'ideas']
  return (
    <aside className="side">
      <h1>OpenSwarm Web Console</h1>

      <div className="card">
        <h2>Live Status</h2>
        <div className="muted">agents={live?.known_agents || 0} active_tasks={live?.active_tasks || 0}</div>
      </div>

      <div className="card">
        <h2>Submit Task</h2>
        {auth?.token_required ? (
          <>
            <div className="muted" style={{ marginBottom: 6 }}>
              Operator token required
            </div>
            <input
              value={operatorToken}
              onChange={(e) => setOperatorToken(e.target.value)}
              placeholder="x-ops-token"
              style={{ width: '100%', marginBottom: 8 }}
            />
          </>
        ) : null}
        <textarea rows="3" value={desc} onChange={(e) => setDesc(e.target.value)} placeholder="Submit a root task" style={{ width: '100%' }} />
        <div className="row" style={{ marginTop: 8 }}>
          <button className="primary" onClick={submitTask}>
            Submit
          </button>
          <button onClick={refresh}>Refresh</button>
        </div>
        {submitError ? (
          <div className="muted" style={{ color: '#ff7d7d', marginTop: 8 }}>
            submit error: {submitError}
          </div>
        ) : null}
      </div>

      <div className="card">
        <h2>Task Forensics</h2>
        <input value={taskId} onChange={(e) => setTaskId(e.target.value)} placeholder="task id" style={{ width: '100%' }} />
        <div className="row" style={{ marginTop: 8 }}>
          <button onClick={loadTrace}>Load Timeline</button>
        </div>
      </div>

      <div className="card">
        <div className="tabs row">
          {tabs.map((t) => (
            <button key={t} className={tab === t ? 'active' : ''} onClick={() => setTab(t)}>
              {t}
            </button>
          ))}
        </div>
      </div>
    </aside>
  )
}
