import { DataSet, Network } from 'vis-network/standalone'
import { useEffect, useRef, useState } from 'react'
import { api } from '../api/client'

function scrub(s) {
  return String(s || '').replace(/did:swarm:[A-Za-z0-9]+/g, '[agent]')
}

function scrubId(s) {
  return String(s || '').replace(/did:swarm:[A-Za-z0-9]+/g, m => '[' + m.slice(-6) + ']')
}

// ── Score bar ─────────────────────────────
function ScoreBar({ label, value }) {
  const pct = Math.round((value || 0) * 100)
  const cls = pct > 60 ? 'green' : pct > 30 ? 'yellow' : 'red'
  return (
    <div className="score-bar-row">
      <span className="score-bar-label">{label}</span>
      <div className="score-bar-track">
        <div className={`score-bar-fill ${cls}`} style={{ width: `${pct}%` }} />
      </div>
      <span className="score-bar-value">{pct}%</span>
    </div>
  )
}

// ── IRV rounds ────────────────────────────
function IrvRounds({ rounds }) {
  if (!rounds?.length) return null
  return (
    <div className="detail-section">
      <div className="detail-section-title">IRV Round History</div>
      {rounds.map(r => (
        <div key={r.round_number} className="irv-round">
          <div className="irv-round-header">
            <span>Round {r.round_number}</span>
            {r.eliminated && <span style={{ color: 'var(--coral)' }}>Eliminated: {r.eliminated.slice(0, 8)}…</span>}
          </div>
          <div style={{ display: 'flex', gap: 12, flexWrap: 'wrap' }}>
            {Object.entries(r.tallies || {}).map(([planId, count]) => (
              <span key={planId} style={{ color: r.eliminated === planId ? 'var(--coral)' : 'var(--teal)' }}>
                {planId.slice(0, 8)}…: <strong>{count}</strong>
              </span>
            ))}
          </div>
        </div>
      ))}
    </div>
  )
}

// ── Voting tab ────────────────────────────
function VotingTab({ taskVoting, taskBallots }) {
  const rfp = taskVoting?.rfp?.[0]
  const ballots = taskBallots?.ballots || []
  const irvRounds = taskBallots?.irv_rounds || []
  const planIds = (rfp?.plans || []).map(p => p.plan_id)

  if (!rfp) return <div style={{ color: 'var(--text-muted)', fontSize: 12 }}>No voting data for this task.</div>

  return (
    <div>
      <div className="detail-section">
        <div className="detail-section-title">RFP Status</div>
        <div className="detail-meta">
          <span>Phase: <strong>{rfp.phase}</strong></span>
          <span>Commits: <strong>{rfp.commit_count}/{rfp.expected_proposers || 0}</strong></span>
          <span>Reveals: <strong>{rfp.reveal_count}/{rfp.expected_proposers || 0}</strong></span>
        </div>
      </div>

      {planIds.length > 0 && (
        <div className="detail-section">
          <div className="detail-section-title">Plans</div>
          <table className="data-table">
            <thead>
              <tr>
                <th>Plan ID</th>
                <th>Proposer</th>
                <th>Subtasks</th>
              </tr>
            </thead>
            <tbody>
              {(rfp.plans || []).map(p => (
                <tr key={p.plan_id}>
                  <td>{p.plan_id?.slice(0, 12)}…</td>
                  <td>{scrubId(p.proposer_name || 'unknown')}</td>
                  <td>{p.subtask_count}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {ballots.length > 0 && (
        <div className="detail-section">
          <div className="detail-section-title">Per-Voter Ballots</div>
          {ballots.map((b, i) => (
            <div key={i} style={{ background: 'var(--bg)', border: '1px solid var(--border)', borderRadius: 6, padding: '8px 12px', marginBottom: 8 }}>
              <div style={{ fontFamily: 'var(--font-mono)', fontSize: 11, color: 'var(--teal)', marginBottom: 6 }}>
                {scrubId(b.voter)}
              </div>
              <div style={{ fontSize: 11, color: 'var(--text-muted)', marginBottom: 6, fontFamily: 'var(--font-mono)' }}>
                Rankings: {(b.rankings || []).map(r => r.slice(0, 6)).join(' › ')}
              </div>
              {planIds.slice(0, 3).map(p => (
                b.critic_scores?.[p] ? (
                  <div key={p} style={{ marginBottom: 8 }}>
                    <div style={{ fontSize: 10, color: 'var(--text-muted)', marginBottom: 4, fontFamily: 'var(--font-mono)' }}>
                      Plan {p.slice(0, 8)}…
                    </div>
                    <ScoreBar label="Feasibility"  value={b.critic_scores[p].feasibility} />
                    <ScoreBar label="Parallelism"  value={b.critic_scores[p].parallelism} />
                    <ScoreBar label="Completeness" value={b.critic_scores[p].completeness} />
                    <ScoreBar label="Risk (inv.)"  value={1 - (b.critic_scores[p].risk || 0)} />
                  </div>
                ) : null
              ))}
            </div>
          ))}
        </div>
      )}

      <IrvRounds rounds={irvRounds} />
    </div>
  )
}

// ── Deliberation tab ──────────────────────
const TYPE_COLOR = {
  ProposalSubmission: '#2a7ab0',
  CritiqueFeedback:   '#ffaa00',
  Rebuttal:           '#ff3355',
  SynthesisResult:    '#00e5b0',
}
const TYPE_ICON = {
  ProposalSubmission: '📋',
  CritiqueFeedback:   '🔍',
  Rebuttal:           '↩️',
  SynthesisResult:    '🔗',
}

function DelibMsg({ msg, adversarialId }) {
  const [expanded, setExpanded] = useState(false)
  const color = TYPE_COLOR[msg.message_type] || '#4a7a9b'
  const icon = TYPE_ICON[msg.message_type] || '💬'
  const isAdversarial = adversarialId && msg.speaker === adversarialId

  return (
    <div className="deliberation-msg" style={{ borderLeftColor: color, background: 'var(--surface-2)' }}>
      <div className="deliberation-msg-header">
        <div className="deliberation-msg-meta">
          <span>{icon}</span>
          {isAdversarial && <span title="Adversarial critic">⚔️</span>}
          <span style={{ fontFamily: 'var(--font-mono)', color: 'var(--teal)', fontSize: 11 }}>{scrubId(msg.speaker)}</span>
          <span className="badge" style={{ background: `${color}22`, color, border: `1px solid ${color}44`, fontSize: 10 }}>
            {msg.message_type} R{msg.round}
          </span>
        </div>
        <span className="deliberation-msg-time">{new Date(msg.timestamp).toLocaleTimeString()}</span>
      </div>

      <div
        className={`deliberation-msg-content${!expanded && msg.content.length > 200 ? ' collapsed' : ''}`}
        onClick={() => setExpanded(e => !e)}
      >
        {msg.content}
      </div>
      {msg.content.length > 200 && (
        <button onClick={() => setExpanded(e => !e)} style={{ background: 'none', border: 'none', color, cursor: 'pointer', fontSize: 11, padding: '2px 0' }}>
          {expanded ? '▲ Less' : '▼ More'}
        </button>
      )}

      {msg.critic_scores && Object.keys(msg.critic_scores).length > 0 && (
        <div style={{ marginTop: 8, borderTop: '1px solid var(--border)', paddingTop: 8 }}>
          <div style={{ fontSize: 11, color: 'var(--text-muted)', marginBottom: 6 }}>Critic scores per plan:</div>
          {Object.entries(msg.critic_scores).map(([planId, scores]) => (
            <div key={planId} style={{ marginBottom: 8 }}>
              <div style={{ fontSize: 10, color: 'var(--text-muted)', marginBottom: 4, fontFamily: 'var(--font-mono)' }}>
                Plan {planId.slice(0, 8)}…
              </div>
              <ScoreBar label="Feasibility"  value={scores.feasibility} />
              <ScoreBar label="Parallelism"  value={scores.parallelism} />
              <ScoreBar label="Completeness" value={scores.completeness} />
              <ScoreBar label="Risk (inv.)"  value={1 - (scores.risk || 0)} />
            </div>
          ))}
        </div>
      )}
    </div>
  )
}

function DeliberationTab({ taskId }) {
  const [msgs, setMsgs] = useState([])
  const [holonInfo, setHolonInfo] = useState(null)
  const [loading, setLoading] = useState(false)

  useEffect(() => {
    if (!taskId) return
    setLoading(true)
    Promise.all([
      api.taskDeliberation(taskId),
      api.holonDetail(taskId).catch(() => null),
    ]).then(([d, h]) => {
      setMsgs(d.messages || [])
      setHolonInfo(h)
      setLoading(false)
    }).catch(() => setLoading(false))
  }, [taskId])

  if (loading) return <div style={{ color: 'var(--text-muted)', fontSize: 12 }}>Loading…</div>
  if (!msgs.length) return <div style={{ color: 'var(--text-muted)', fontSize: 12 }}>No deliberation messages yet.</div>

  return (
    <div>
      {holonInfo && (
        <div className="detail-meta" style={{ marginBottom: 16 }}>
          <span>Chair: <strong style={{ color: 'var(--teal)' }}>{scrubId(holonInfo.chair)}</strong></span>
          <span>Members: <strong>{holonInfo.members?.length || 0}</strong></span>
          <span>Depth: <strong>{holonInfo.depth}</strong></span>
          <span>Status: <strong>{holonInfo.status}</strong></span>
          {holonInfo.adversarial_critic && <span>Adversarial critic assigned</span>}
        </div>
      )}
      {msgs.map(msg => (
        <DelibMsg key={msg.id} msg={msg} adversarialId={holonInfo?.adversarial_critic} />
      ))}
    </div>
  )
}

// ── Overview tab ──────────────────────────
function OverviewTab({ taskTrace, taskVoting }) {
  const dagRef = useRef(null)
  const dagNet = useRef(null)
  const [index, setIndex] = useState(0)
  const [playing, setPlaying] = useState(false)

  const timeline = taskTrace?.timeline || []
  const descendants = taskTrace?.descendants || []

  useEffect(() => {
    setIndex(timeline.length)
    setPlaying(false)
  }, [taskTrace])

  useEffect(() => {
    if (!playing) return
    const timer = setInterval(() => {
      setIndex(prev => {
        if (prev >= timeline.length) { setPlaying(false); return prev }
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
      nodes.push({ id: root.task_id, label: `ROOT\n${(root.description || '').slice(0, 40)}`, color: '#00e5b0', shape: 'box', font: { color: '#020810', size: 10 } })
    }
    descendants.forEach(t => {
      nodes.push({ id: t.task_id, label: `${t.task_id.slice(0,10)}\n${(t.description || '').slice(0, 28)}`, color: '#2a7ab0', shape: 'box', font: { color: '#c8e8ff', size: 10 } })
      if (t.parent_task_id) edges.push({ from: t.parent_task_id, to: t.task_id, color: '#1a4a6a' })
    })
    if (dagNet.current) dagNet.current.destroy()
    dagNet.current = new Network(
      dagRef.current,
      { nodes: new DataSet(nodes), edges: new DataSet(edges) },
      {
        layout: { hierarchical: { enabled: true, direction: 'UD', sortMethod: 'directed' } },
        physics: false,
        edges: { smooth: true },
        nodes: { margin: 8 },
      }
    )
    return () => { if (dagNet.current) dagNet.current.destroy() }
  }, [taskTrace, descendants])

  const replayed = timeline.slice(0, Math.max(0, Math.min(index, timeline.length)))

  return (
    <div>
      {/* Task meta */}
      <div className="detail-meta" style={{ marginBottom: 16 }}>
        <span>Status: <strong>{taskTrace?.task?.status || '—'}</strong></span>
        <span>Tier: <strong>{taskTrace?.task?.tier_level ?? '—'}</strong></span>
        <span>Assigned: <strong>{scrub(taskTrace?.task?.assigned_to_name || 'unassigned')}</strong></span>
        <span>Subtasks: <strong>{(taskTrace?.task?.subtasks || []).length}</strong></span>
      </div>

      {taskTrace?.task?.description && (
        <div className="detail-section">
          <div className="detail-section-title">Description</div>
          <div style={{ fontSize: 13, color: 'var(--text)', lineHeight: 1.5, padding: '8px 12px', background: 'var(--surface-2)', borderRadius: 6, border: '1px solid var(--border)' }}>
            {taskTrace.task.description}
          </div>
        </div>
      )}

      {/* Timeline replay */}
      {timeline.length > 0 && (
        <div className="detail-section">
          <div className="detail-section-title">Timeline Replay</div>
          <div className="timeline-controls">
            <button className="btn" style={{ fontSize: 11 }} onClick={() => setPlaying(p => !p)}>
              {playing ? '⏸' : '▶'}
            </button>
            <button className="btn" style={{ fontSize: 11 }} onClick={() => { setPlaying(false); setIndex(0) }}>⏮</button>
            <input
              type="range" className="timeline-slider"
              min="0" max={Math.max(0, timeline.length)}
              value={Math.min(index, timeline.length)}
              onChange={e => { setPlaying(false); setIndex(Number(e.target.value)) }}
            />
            <span style={{ fontSize: 11, fontFamily: 'var(--font-mono)', color: 'var(--text-muted)', flexShrink: 0 }}>
              {Math.min(index, timeline.length)}/{timeline.length}
            </span>
          </div>
          <div className="log-box">
            {replayed.map((e, i) => (
              <div key={`${e.timestamp}-${i}`}>
                <span style={{ color: 'var(--text-muted)' }}>[{e.timestamp}]</span>{' '}
                <span style={{ color: 'var(--teal)' }}>{e.stage}</span>{' '}
                {scrub(e.detail)}
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Subtask table */}
      {descendants.length > 0 && (
        <div className="detail-section">
          <div className="detail-section-title">Subtasks</div>
          <table className="data-table">
            <thead>
              <tr>
                <th>ID</th><th>Status</th><th>Assignee</th><th>Result</th>
              </tr>
            </thead>
            <tbody>
              {descendants.map(t => (
                <tr key={t.task_id}>
                  <td>{t.task_id.slice(0, 10)}…</td>
                  <td>{t.status}</td>
                  <td>{scrub(t.assigned_to_name || 'unassigned')}</td>
                  <td>{t.result_text || (t.has_result ? 'captured' : '—')}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {/* Task DAG */}
      {(descendants.length > 0 || taskTrace?.task) && (
        <div className="detail-section">
          <div className="detail-section-title">Task DAG</div>
          <div className="dag-container" ref={dagRef} />
        </div>
      )}

      {/* Related messages */}
      {(taskTrace?.messages || []).length > 0 && (
        <div className="detail-section">
          <div className="detail-section-title">Propagation Messages</div>
          <div className="log-box">
            {taskTrace.messages.map((m, i) => (
              <div key={i}>
                <span style={{ color: 'var(--text-muted)' }}>[{m.timestamp}]</span>{' '}
                <span style={{ color: 'var(--teal)' }}>{m.topic}</span>{' '}
                {m.method || ''}{' '}{scrub(m.outcome || '')}
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Result artifact */}
      {taskTrace?.result_artifact && (
        <div className="detail-section">
          <div className="detail-section-title">Result Artifact</div>
          <div className="log-box">
            <div>artifact_id: {taskTrace.result_artifact.artifact_id || '—'}</div>
            <div>content_type: {taskTrace.result_artifact.content_type || '—'}</div>
            <div>size_bytes: {taskTrace.result_artifact.size_bytes ?? '—'}</div>
            <div>created_at: {taskTrace.result_artifact.created_at || '—'}</div>
            {taskTrace.result_text && <div style={{ marginTop: 8, color: 'var(--text)' }}>result: {taskTrace.result_text}</div>}
          </div>
        </div>
      )}
    </div>
  )
}

// ── Main export ───────────────────────────
export default function TaskDetailPanel({ taskId, taskTrace, taskVoting, taskBallots }) {
  const [activeTab, setActiveTab] = useState('overview')

  return (
    <div>
      <div style={{ padding: '0 0 12px' }}>
        <div style={{ fontFamily: 'var(--font-mono)', fontSize: 11, color: 'var(--text-muted)', marginBottom: 4 }}>Task ID</div>
        <div style={{ fontFamily: 'var(--font-mono)', fontSize: 13, color: 'var(--teal)', wordBreak: 'break-all' }}>{taskId}</div>
      </div>

      {/* Inline tabs */}
      <div style={{ display: 'flex', gap: 2, borderBottom: '1px solid var(--border)', marginBottom: 16, marginLeft: -20, marginRight: -20, paddingLeft: 20 }}>
        {['overview', 'voting', 'deliberation'].map(tab => (
          <button
            key={tab}
            className={`panel-tab${activeTab === tab ? ' active' : ''}`}
            onClick={() => setActiveTab(tab)}
          >
            {tab.charAt(0).toUpperCase() + tab.slice(1)}
          </button>
        ))}
      </div>

      {activeTab === 'overview' && (
        <OverviewTab taskTrace={taskTrace} taskVoting={taskVoting} />
      )}
      {activeTab === 'voting' && (
        <VotingTab taskVoting={taskVoting} taskBallots={taskBallots} />
      )}
      {activeTab === 'deliberation' && (
        <DeliberationTab taskId={taskId} />
      )}
    </div>
  )
}
