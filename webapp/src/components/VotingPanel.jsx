import { useState } from 'react'

function scrub(s) {
  return String(s || '').replace(/did:swarm:[A-Za-z0-9]+/g, m => '[' + m.slice(-6) + ']')
}

function CriticBar({ value }) {
  const pct = Math.round((value || 0) * 100)
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: 4 }}>
      <div style={{ width: 60, background: '#222', borderRadius: 2, height: 6 }}>
        <div style={{ width: `${pct}%`, background: pct > 60 ? '#00b894' : pct > 30 ? '#fdcb6e' : '#d63031', height: '100%', borderRadius: 2 }} />
      </div>
      <span style={{ fontSize: 10, color: '#aaa' }}>{pct}%</span>
    </div>
  )
}

function IrvRoundView({ rounds }) {
  if (!rounds || rounds.length === 0) return null
  return (
    <div style={{ marginTop: 12 }}>
      <h4 style={{ color: '#aaa', fontSize: 12, margin: '0 0 8px' }}>IRV Round History</h4>
      {rounds.map((r) => (
        <div key={r.round_number} style={{ background: '#0d1117', padding: '6px 10px', borderRadius: 4, marginBottom: 4, fontSize: 11 }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 4 }}>
            <strong style={{ color: '#ccc' }}>Round {r.round_number}</strong>
            {r.eliminated && <span style={{ color: '#d63031' }}>Eliminated: {r.eliminated.slice(0, 8)}…</span>}
          </div>
          <div style={{ display: 'flex', gap: 12, flexWrap: 'wrap' }}>
            {Object.entries(r.tallies || {}).map(([planId, count]) => (
              <span key={planId} style={{ color: r.eliminated === planId ? '#d63031' : '#4a9eff' }}>
                {planId.slice(0, 8)}…: <strong>{count}</strong>
              </span>
            ))}
          </div>
        </div>
      ))}
    </div>
  )
}

function BallotTable({ ballots, planIds }) {
  if (!ballots || ballots.length === 0) return null
  return (
    <div style={{ marginTop: 12, overflowX: 'auto' }}>
      <h4 style={{ color: '#aaa', fontSize: 12, margin: '0 0 8px' }}>Per-Voter Ballots</h4>
      <table style={{ fontSize: 11, width: '100%', borderCollapse: 'collapse' }}>
        <thead>
          <tr style={{ background: '#0d1117' }}>
            <th style={{ textAlign: 'left', padding: '4px 8px', color: '#888' }}>Voter</th>
            <th style={{ textAlign: 'left', padding: '4px 8px', color: '#888' }}>Rankings</th>
            {planIds.slice(0, 3).map(p => (
              <th key={p} style={{ textAlign: 'center', padding: '4px 8px', color: '#888', minWidth: 80 }}>
                {p.slice(0, 8)}… score
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {ballots.map((b, i) => (
            <tr key={i} style={{ borderBottom: '1px solid #1e1e2e' }}>
              <td style={{ padding: '4px 8px', fontFamily: 'monospace', color: '#4a9eff' }}>{scrub(b.voter)}</td>
              <td style={{ padding: '4px 8px', color: '#ccc' }}>{(b.rankings || []).map(r => r.slice(0, 6)).join(' > ')}</td>
              {planIds.slice(0, 3).map(p => (
                <td key={p} style={{ padding: '4px 8px', textAlign: 'center' }}>
                  {b.critic_scores?.[p] ? (
                    <CriticBar value={
                      0.30 * (b.critic_scores[p].feasibility || 0) +
                      0.25 * (b.critic_scores[p].parallelism || 0) +
                      0.30 * (b.critic_scores[p].completeness || 0) +
                      0.15 * (1 - (b.critic_scores[p].risk || 0))
                    } />
                  ) : <span style={{ color: '#444' }}>—</span>}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}

export default function VotingPanel({ voting, taskVoting }) {
  const [showBallots, setShowBallots] = useState({})

  const toggleBallots = (taskId) => {
    setShowBallots(prev => ({ ...prev, [taskId]: !prev[taskId] }))
  }

  // Gather plan IDs for ballot table columns
  const getPlanIds = (item) => (item.plans || []).map(p => p.plan_id)

  return (
    <div className="card">
      <h2>Voting Process</h2>
      <table>
        <thead>
          <tr>
            <th>Task</th>
            <th>Phase</th>
            <th>Commits</th>
            <th>Reveals</th>
            <th>Plans</th>
            <th>Details</th>
          </tr>
        </thead>
        <tbody>
          {(voting?.rfp || []).map((item) => (
            <>
              <tr key={item.task_id}>
                <td className="mono">{item.task_id?.slice(0, 12) + '…'}</td>
                <td>{item.phase}</td>
                <td>{item.commit_count}/{item.expected_proposers || 0}</td>
                <td>{item.reveal_count}/{item.expected_proposers || 0}</td>
                <td>
                  {(item.plans || [])
                    .map((p) => `${p.plan_id?.slice(0,8)}… by ${scrub(p.proposer_name || 'unknown')}`)
                    .join(', ')}
                </td>
                <td>
                  <button
                    onClick={() => toggleBallots(item.task_id)}
                    style={{ background: 'none', border: '1px solid #333', color: '#4a9eff', cursor: 'pointer', borderRadius: 4, padding: '2px 8px', fontSize: 11 }}
                  >
                    {showBallots[item.task_id] ? 'Hide' : 'Ballots'}
                  </button>
                </td>
              </tr>
              {showBallots[item.task_id] && (
                <tr key={item.task_id + '-detail'}>
                  <td colSpan={6} style={{ padding: '8px 16px', background: '#0d1117' }}>
                    <BallotTable
                      ballots={taskVoting?.ballots || []}
                      planIds={getPlanIds(item)}
                    />
                    <IrvRoundView rounds={taskVoting?.irv_rounds || []} />
                  </td>
                </tr>
              )}
            </>
          ))}
        </tbody>
      </table>
    </div>
  )
}
