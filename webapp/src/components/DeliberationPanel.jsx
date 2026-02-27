import { useState, useEffect } from 'react'
import { api } from '../api/client'

const TYPE_ICON = {
  ProposalSubmission: '📋',
  CritiqueFeedback: '🔍',
  Rebuttal: '↩️',
  SynthesisResult: '🔗',
}

const TYPE_COLOR = {
  ProposalSubmission: '#4a9eff',
  CritiqueFeedback: '#ff9f43',
  Rebuttal: '#ee5a24',
  SynthesisResult: '#00d2d3',
}

function scrubId(s) {
  return String(s || '').replace(/did:swarm:[A-Za-z0-9]+/g, m => '[' + m.slice(-6) + ']')
}

function CriticScoreBar({ label, value }) {
  const pct = Math.round((value || 0) * 100)
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginBottom: 2 }}>
      <span style={{ width: 100, fontSize: 11, color: '#aaa' }}>{label}</span>
      <div style={{ flex: 1, background: '#222', borderRadius: 3, height: 8 }}>
        <div style={{ width: `${pct}%`, background: pct > 60 ? '#00b894' : pct > 30 ? '#fdcb6e' : '#d63031', height: '100%', borderRadius: 3 }} />
      </div>
      <span style={{ fontSize: 11, color: '#ccc', width: 30 }}>{pct}%</span>
    </div>
  )
}

function DeliberationMessage({ msg, isAdversarial }) {
  const [expanded, setExpanded] = useState(false)
  const color = TYPE_COLOR[msg.message_type] || '#888'
  const icon = TYPE_ICON[msg.message_type] || '💬'

  return (
    <div style={{ borderLeft: `3px solid ${color}`, paddingLeft: 12, marginBottom: 12, background: '#1a1a2e', borderRadius: '0 6px 6px 0', padding: '8px 12px' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 4 }}>
        <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
          <span style={{ fontSize: 16 }}>{icon}</span>
          {isAdversarial && <span title="Adversarial critic">⚔️</span>}
          <span style={{ color: '#ccc', fontSize: 12, fontFamily: 'monospace' }}>{scrubId(msg.speaker)}</span>
          <span style={{ background: color, color: '#000', fontSize: 10, padding: '1px 6px', borderRadius: 10, fontWeight: 700 }}>
            {msg.message_type} R{msg.round}
          </span>
        </div>
        <span style={{ color: '#666', fontSize: 11 }}>{new Date(msg.timestamp).toLocaleTimeString()}</span>
      </div>

      <div
        style={{ color: '#ddd', fontSize: 13, cursor: 'pointer', maxHeight: expanded ? 'none' : 60, overflow: 'hidden' }}
        onClick={() => setExpanded(e => !e)}
      >
        {msg.content}
      </div>
      {msg.content.length > 200 && (
        <button
          onClick={() => setExpanded(e => !e)}
          style={{ background: 'none', border: 'none', color: color, cursor: 'pointer', fontSize: 11, padding: '2px 0' }}
        >
          {expanded ? '▲ Collapse' : '▼ Expand'}
        </button>
      )}

      {msg.critic_scores && Object.keys(msg.critic_scores).length > 0 && (
        <div style={{ marginTop: 8, borderTop: '1px solid #333', paddingTop: 8 }}>
          <div style={{ color: '#888', fontSize: 11, marginBottom: 6 }}>Critic scores per plan:</div>
          {Object.entries(msg.critic_scores).map(([planId, scores]) => (
            <div key={planId} style={{ marginBottom: 8 }}>
              <div style={{ color: '#aaa', fontSize: 11, marginBottom: 4 }}>Plan {planId.slice(0, 8)}...</div>
              <CriticScoreBar label="Feasibility" value={scores.feasibility} />
              <CriticScoreBar label="Parallelism" value={scores.parallelism} />
              <CriticScoreBar label="Completeness" value={scores.completeness} />
              <CriticScoreBar label="Risk (inv.)" value={1 - (scores.risk || 0)} />
            </div>
          ))}
        </div>
      )}
    </div>
  )
}

export default function DeliberationPanel({ taskId: propTaskId }) {
  const [taskId, setTaskId] = useState(propTaskId || '')
  const [deliberation, setDeliberation] = useState([])
  const [holonInfo, setHolonInfo] = useState(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')

  const load = async (id) => {
    const tid = (id || taskId).trim()
    if (!tid) return
    setLoading(true)
    setError('')
    try {
      const [d, h] = await Promise.all([
        api.taskDeliberation(tid),
        api.holonDetail(tid).catch(() => null)
      ])
      setDeliberation(d.messages || [])
      setHolonInfo(h)
    } catch (e) {
      setError(e.message || 'Failed to load deliberation')
    }
    setLoading(false)
  }

  useEffect(() => { if (propTaskId) { setTaskId(propTaskId); load(propTaskId) } }, [propTaskId])

  const adversarialId = holonInfo?.adversarial_critic || null

  return (
    <div className="card">
      <h2>Deliberation Thread</h2>
      <div style={{ display: 'flex', gap: 8, marginBottom: 16 }}>
        <input
          className="input"
          value={taskId}
          onChange={e => setTaskId(e.target.value)}
          placeholder="Task ID"
          style={{ flex: 1 }}
        />
        <button className="btn" onClick={() => load()}>Load</button>
      </div>

      {holonInfo && (
        <div style={{ background: '#0d1117', padding: 12, borderRadius: 6, marginBottom: 16, fontSize: 12 }}>
          <div style={{ display: 'flex', gap: 16, flexWrap: 'wrap' }}>
            <span>Chair: <strong style={{ color: '#4a9eff' }}>{scrubId(holonInfo.chair)}</strong></span>
            <span>Members: <strong>{holonInfo.members?.length || 0}</strong></span>
            <span>Depth: <strong>{holonInfo.depth}</strong></span>
            <span>Status: <strong style={{ color: '#00b894' }}>{holonInfo.status}</strong></span>
            {holonInfo.adversarial_critic && <span>⚔️ Adversarial critic assigned</span>}
          </div>
        </div>
      )}

      {loading && <div style={{ color: '#aaa', padding: 16 }}>Loading deliberation...</div>}
      {error && <div style={{ color: '#d63031', padding: 8 }}>{error}</div>}

      {deliberation.length === 0 && !loading && (
        <div style={{ color: '#666', padding: 16 }}>No deliberation messages yet. Enter a task ID above.</div>
      )}

      <div style={{ display: 'flex', flexDirection: 'column', gap: 0 }}>
        {deliberation.map((msg) => (
          <DeliberationMessage
            key={msg.id}
            msg={msg}
            isAdversarial={adversarialId && msg.speaker === adversarialId}
          />
        ))}
      </div>
    </div>
  )
}
