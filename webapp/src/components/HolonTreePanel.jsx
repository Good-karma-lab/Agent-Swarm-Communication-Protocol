import { useState } from 'react'

const STATUS_COLORS = {
  Forming: '#636e72',
  Deliberating: '#fdcb6e',
  Voting: '#e17055',
  Executing: '#0984e3',
  Synthesizing: '#a29bfe',
  Done: '#00b894',
}

function scrubId(s) {
  return String(s || '').replace(/did:swarm:[A-Za-z0-9]+/g, m => '[' + m.slice(-6) + ']')
}

function HolonNode({ holon, holons, depth = 0, onSelect }) {
  const [collapsed, setCollapsed] = useState(false)
  const color = STATUS_COLORS[holon.status] || '#888'
  const children = holons.filter(h => h.parent_holon === holon.task_id)

  return (
    <div style={{ marginLeft: depth * 24 }}>
      <div
        onClick={() => onSelect(holon)}
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 8,
          padding: '6px 10px',
          marginBottom: 4,
          background: '#1a1a2e',
          borderRadius: 6,
          cursor: 'pointer',
          borderLeft: `4px solid ${color}`,
          transition: 'background 0.15s',
        }}
        onMouseEnter={e => e.currentTarget.style.background = '#252540'}
        onMouseLeave={e => e.currentTarget.style.background = '#1a1a2e'}
      >
        <span style={{
          background: color,
          color: '#000',
          borderRadius: 12,
          padding: '2px 8px',
          fontSize: 11,
          fontWeight: 700,
          whiteSpace: 'nowrap',
        }}>
          {holon.status}
        </span>
        <span style={{ color: '#4a9eff', fontFamily: 'monospace', fontSize: 12 }}>
          d{holon.depth}
        </span>
        <span style={{ color: '#aaa', fontSize: 12, flex: 1 }} title={holon.task_id}>
          {holon.task_id.slice(0, 16)}…
        </span>
        <span style={{ color: '#888', fontSize: 11 }}>
          {holon.members?.length || 0} members
        </span>
        {children.length > 0 && (
          <button
            onClick={e => { e.stopPropagation(); setCollapsed(c => !c) }}
            style={{ background: 'none', border: 'none', color: '#aaa', cursor: 'pointer', fontSize: 14 }}
          >
            {collapsed ? '▶' : '▼'}
          </button>
        )}
      </div>
      {!collapsed && children.map(child => (
        <HolonNode key={child.task_id} holon={child} holons={holons} depth={depth + 1} onSelect={onSelect} />
      ))}
    </div>
  )
}

function HolonDetail({ holon }) {
  if (!holon) return null
  const color = STATUS_COLORS[holon.status] || '#888'
  return (
    <div style={{ background: '#0d1117', padding: 16, borderRadius: 8, borderLeft: `4px solid ${color}` }}>
      <h3 style={{ margin: '0 0 12px', color }}>{holon.status} — Depth {holon.depth}</h3>
      <div style={{ fontSize: 12, color: '#aaa', marginBottom: 8 }}>
        Task: <span style={{ fontFamily: 'monospace', color: '#ccc' }}>{holon.task_id}</span>
      </div>
      {holon.parent_holon && (
        <div style={{ fontSize: 12, color: '#aaa', marginBottom: 8 }}>
          Parent: <span style={{ fontFamily: 'monospace', color: '#ccc' }}>{holon.parent_holon.slice(0, 16)}…</span>
        </div>
      )}
      <div style={{ fontSize: 12, color: '#aaa', marginBottom: 8 }}>
        Chair: <span style={{ color: '#4a9eff' }}>
          {String(holon.chair || '').replace(/did:swarm:[A-Za-z0-9]+/g, m => '[' + m.slice(-6) + ']')}
        </span>
      </div>
      {holon.adversarial_critic && (
        <div style={{ fontSize: 12, color: '#aaa', marginBottom: 8 }}>
          Adversarial critic: <span style={{ color: '#e17055' }}>
            {String(holon.adversarial_critic).replace(/did:swarm:[A-Za-z0-9]+/g, m => '[' + m.slice(-6) + ']')}
          </span>
        </div>
      )}
      <div style={{ fontSize: 12, color: '#aaa', marginBottom: 8 }}>
        Members ({holon.members?.length || 0}):
        <div style={{ display: 'flex', flexWrap: 'wrap', gap: 4, marginTop: 4 }}>
          {(holon.members || []).map(m => (
            <span key={m} style={{ background: '#1a1a2e', padding: '2px 8px', borderRadius: 4, fontFamily: 'monospace', fontSize: 11 }}>
              {String(m).replace(/did:swarm:[A-Za-z0-9]+/g, x => '[' + x.slice(-6) + ']')}
            </span>
          ))}
        </div>
      </div>
      {holon.child_holons?.length > 0 && (
        <div style={{ fontSize: 12, color: '#aaa' }}>
          Child holons: {holon.child_holons.length}
        </div>
      )}
    </div>
  )
}

export default function HolonTreePanel({ holons: propHolons }) {
  const [selected, setSelected] = useState(null)
  const holons = propHolons || []

  // Root holons are those with no parent
  const roots = holons.filter(h => !h.parent_holon)

  const statusCounts = holons.reduce((acc, h) => {
    acc[h.status] = (acc[h.status] || 0) + 1
    return acc
  }, {})

  return (
    <div className="card">
      <h2>Holon Tree</h2>

      <div style={{ display: 'flex', gap: 12, marginBottom: 16, flexWrap: 'wrap' }}>
        {Object.entries(STATUS_COLORS).map(([status, color]) => (
          <div key={status} style={{ display: 'flex', alignItems: 'center', gap: 4, fontSize: 12 }}>
            <div style={{ width: 10, height: 10, borderRadius: '50%', background: color }} />
            <span style={{ color: '#aaa' }}>{status}</span>
            {statusCounts[status] ? <span style={{ color: '#fff', fontWeight: 700 }}>({statusCounts[status]})</span> : null}
          </div>
        ))}
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: selected ? '1fr 1fr' : '1fr', gap: 16 }}>
        <div>
          {holons.length === 0 && (
            <div style={{ color: '#666', padding: 16 }}>No active holons. Inject a task to see the holon tree.</div>
          )}
          {roots.map(holon => (
            <HolonNode
              key={holon.task_id}
              holon={holon}
              holons={holons}
              onSelect={setSelected}
            />
          ))}
        </div>
        {selected && (
          <div>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 8 }}>
              <h3 style={{ margin: 0, fontSize: 14, color: '#aaa' }}>Selected Holon</h3>
              <button
                onClick={() => setSelected(null)}
                style={{ background: 'none', border: 'none', color: '#888', cursor: 'pointer', fontSize: 18 }}
              >x</button>
            </div>
            <HolonDetail holon={selected} />
          </div>
        )}
      </div>
    </div>
  )
}
