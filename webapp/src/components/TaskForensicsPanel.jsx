import { DataSet, Network } from 'vis-network/standalone'
import { useEffect, useMemo, useRef, useState } from 'react'

export default function TaskForensicsPanel({ taskTrace }) {
  const [index, setIndex] = useState((taskTrace?.timeline || []).length)
  const [playing, setPlaying] = useState(false)
  const dagRef = useRef(null)
  const dagNet = useRef(null)

  const timeline = taskTrace?.timeline || []

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
    const descendants = taskTrace?.descendants || []
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
  }, [taskTrace])

  const replayed = useMemo(() => timeline.slice(0, Math.max(0, Math.min(index, timeline.length))), [index, timeline])

  return (
    <div className="grid">
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
              [{e.timestamp}] {e.stage} {e.detail}
            </div>
          ))}
        </div>
      </div>

      <div className="card">
        <h2>Recursive Decomposition/Assignments</h2>
        <pre className="mono">{JSON.stringify(taskTrace?.descendants || [], null, 2)}</pre>
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
              [{m.timestamp}] {m.topic} {m.method || '-'} {m.outcome}
            </div>
          ))}
        </div>
      </div>

      <div className="card">
        <h2>Root Task + Aggregation State</h2>
        <pre className="mono">{JSON.stringify(taskTrace?.task || {}, null, 2)}</pre>
      </div>
    </div>
  )
}
