import { DataSet, Network } from 'vis-network/standalone'
import { useEffect, useMemo, useRef, useState } from 'react'

export default function TaskForensicsPanel({ taskTrace, tasks, taskId, setTaskId, loadTrace, taskVoting }) {
  const scrub = (s) => String(s || '').replace(/did:swarm:[A-Za-z0-9]+/g, '[agent]')
  const [index, setIndex] = useState((taskTrace?.timeline || []).length)
  const [playing, setPlaying] = useState(false)
  const dagRef = useRef(null)
  const dagNet = useRef(null)

  const timeline = taskTrace?.timeline || []
  const descendants = taskTrace?.descendants || []
  const rfp = taskVoting?.rfp?.[0]

  useEffect(() => {
    setIndex(timeline.length)
    setPlaying(false)
  }, [taskTrace])

  useEffect(() => {
    if (!playing) return
    const timer = setInterval(() => {
      setIndex((prev) => {
        if (prev >= timeline.length) {
          setPlaying(false)
          return prev
        }
        return prev + 1
      })
    }, 700)
    return () => clearInterval(timer)
  }, [playing, timeline.length])

  useEffect(() => {
    if (!dagRef.current) return
    const root = taskTrace?.task
    const nodes = []
    const edges = []
    if (root?.task_id) {
      nodes.push({ id: root.task_id, label: `ROOT\n${(root.description || '').slice(0, 48)}`, color: '#4fd18b', shape: 'box' })
    }
    descendants.forEach((t) => {
      nodes.push({ id: t.task_id, label: `${t.task_id}\n${(t.description || '').slice(0, 32)}`, color: '#7fb8ff', shape: 'box' })
      if (t.parent_task_id) {
        edges.push({ from: t.parent_task_id, to: t.task_id, color: '#4d7fb0' })
      }
    })
    if (dagNet.current) dagNet.current.destroy()
    dagNet.current = new Network(
      dagRef.current,
      {
        nodes: new DataSet(nodes),
        edges: new DataSet(edges)
      },
      {
        layout: { hierarchical: { enabled: true, direction: 'UD', sortMethod: 'directed' } },
        physics: false,
        edges: { smooth: true },
        nodes: { font: { color: '#dce9f7', size: 11 }, margin: 8 }
      }
    )
    return () => {
      if (dagNet.current) dagNet.current.destroy()
    }
  }, [taskTrace, descendants])

  const replayed = useMemo(() => timeline.slice(0, Math.max(0, Math.min(index, timeline.length))), [index, timeline])

  return (
    <div className="grid">
      <div className="card">
        <h2>All Tasks</h2>
        <div className="log mono">
          {(tasks || []).map((t) => (
            <div key={t.task_id}>
              <button
                onClick={() => {
                  loadTrace(t.task_id)
                }}
              >
                Load
              </button>{' '}
              {t.task_id} | {t.status} | tier={t.tier_level} | assigned={scrub(t.assigned_to_name || 'unassigned')}
            </div>
          ))}
        </div>
      </div>

      <div className="card">
        <h2>Task Details</h2>
        <div className="row" style={{ marginBottom: 8 }}>
          <input value={taskId} onChange={(e) => setTaskId(e.target.value)} placeholder="task id" style={{ width: '100%' }} />
          <button onClick={() => loadTrace(taskId)}>Load</button>
        </div>
        <div className="log mono">
          <div>description: {taskTrace?.task?.description || '-'}</div>
          <div>status: {taskTrace?.task?.status || '-'}</div>
          <div>tier: {taskTrace?.task?.tier_level ?? '-'}</div>
          <div>assigned: {scrub(taskTrace?.task?.assigned_to_name || 'unassigned')}</div>
          <div>subtasks: {(taskTrace?.task?.subtasks || []).length}</div>
        </div>
      </div>

      <div className="card">
        <h2>Plans + Voting</h2>
        <div className="muted">phase={rfp?.phase || 'n/a'} commits={rfp?.commit_count || 0} reveals={rfp?.reveal_count || 0}</div>
        <table>
          <thead>
            <tr>
              <th>Plan</th>
              <th>Proposer</th>
              <th>Subtasks</th>
            </tr>
          </thead>
          <tbody>
            {(rfp?.plans || []).map((p) => (
              <tr key={p.plan_id}>
                <td className="mono">{p.plan_id}</td>
                <td>{scrub(p.proposer_name || 'unknown')}</td>
                <td>{p.subtask_count}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <div className="card">
        <h2>Task Timeline Replay</h2>
        <div className="row" style={{ marginBottom: 8 }}>
          <button onClick={() => setPlaying((p) => !p)}>{playing ? 'Pause' : 'Play'}</button>
          <button
            onClick={() => {
              setPlaying(false)
              setIndex(0)
            }}
          >
            Reset
          </button>
          <span className="muted">
            {Math.min(index, timeline.length)}/{timeline.length}
          </span>
        </div>
        <input
          type="range"
          min="0"
          max={Math.max(0, timeline.length)}
          value={Math.min(index, timeline.length)}
          onChange={(e) => {
            setPlaying(false)
            setIndex(Number(e.target.value))
          }}
          style={{ width: '100%' }}
        />
        <div className="log mono">
          {replayed.map((e, i) => (
            <div key={`${e.timestamp}-${i}`}>
              [{e.timestamp}] {e.stage} {scrub(e.detail)}
            </div>
          ))}
        </div>
      </div>

      <div className="card">
        <h2>Recursive Decomposition/Assignments</h2>
        <table>
          <thead>
            <tr>
              <th>Subtask</th>
              <th>Status</th>
              <th>Assignee</th>
              <th>Result</th>
            </tr>
          </thead>
          <tbody>
            {descendants.map((t) => (
              <tr key={t.task_id}>
                <td className="mono">{t.task_id}</td>
                <td>{t.status}</td>
                <td>{scrub(t.assigned_to_name || 'unassigned')}</td>
                <td>{t.result_text || (t.has_result ? 'result captured' : 'no')}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      <div className="card">
        <h2>Task DAG</h2>
        <div id="taskDagGraph" ref={dagRef} />
      </div>

      <div className="card">
        <h2>Task Propagation Messages</h2>
        <div className="log mono">
          {(taskTrace?.messages || []).map((m, i) => (
            <div key={`${m.timestamp}-${i}`}>
              [{m.timestamp}] {m.topic} {m.method || '-'} {scrub(m.outcome)}
            </div>
          ))}
        </div>
      </div>

      <div className="card">
        <h2>Result Artifact</h2>
        <div className="log mono">
          <div>artifact_id: {taskTrace?.result_artifact?.artifact_id || '-'}</div>
          <div>content_type: {taskTrace?.result_artifact?.content_type || '-'}</div>
          <div>size_bytes: {taskTrace?.result_artifact?.size_bytes ?? '-'}</div>
          <div>created_at: {taskTrace?.result_artifact?.created_at || '-'}</div>
          <div>text_result: {taskTrace?.result_text || '-'}</div>
        </div>
      </div>
    </div>
  )
}
