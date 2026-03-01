import { useState } from 'react'

const TASK_STATUS_COLORS = {
  pending:    '#4a7a9b',
  assigned:   '#00e5b0',
  inprogress: '#7c3aff',
  completed:  '#00e5b0',
  failed:     '#ff3355',
}

export default function CenterActivity({
  tasks, messages, holons, myDid,
  onSelectTask, onSelectHolon
}) {
  const [tab, setTab] = useState('tasks')
  const [mineOnly, setMineOnly] = useState(false)

  const displayedTasks = mineOnly
    ? (tasks || []).filter(t => t.assignee_did === myDid)
    : (tasks || [])

  function truncateId(id, len=16) {
    return id ? id.slice(0,len)+'…' : '—'
  }

  function msgPeerLabel(msg) {
    return msg.from_did === myDid
      ? (msg.to_wws_name || truncateId(msg.to_did, 12))
      : (msg.from_wws_name || truncateId(msg.from_did, 12))
  }

  // Group messages by peer
  const msgGroups = {}
  ;(messages || []).forEach(m => {
    const key = m.from_did === myDid ? m.to_did : m.from_did
    if (!msgGroups[key]) msgGroups[key] = { peer: msgPeerLabel(m), msgs: [] }
    msgGroups[key].msgs.push(m)
  })

  return (
    <div className="activity-container">
      <div className="activity-tabs">
        {['tasks','messages','holons','deliberation'].map(t => (
          <button
            key={t}
            className={`activity-tab ${tab === t ? 'active' : ''}`}
            onClick={() => setTab(t)}
          >
            {t.charAt(0).toUpperCase() + t.slice(1)}
          </button>
        ))}
        {tab === 'tasks' && (
          <button
            className={`filter-chip ${mineOnly ? 'active' : ''}`}
            style={{marginLeft:'auto',alignSelf:'center',marginRight:8}}
            onClick={() => setMineOnly(v => !v)}
          >
            Mine only
          </button>
        )}
      </div>
      <div className="activity-content">
        {tab === 'tasks' && displayedTasks.map(t => (
          <div className="task-row" key={t.id} onClick={() => onSelectTask?.(t.id)}>
            <div className="task-status-dot" style={{background: TASK_STATUS_COLORS[t.status] || '#4a7a9b'}} />
            <span className="task-row-id">{truncateId(t.id)}</span>
            <span className="task-row-desc">{t.description || '—'}</span>
            <span className="task-row-status">{t.status}</span>
          </div>
        ))}
        {tab === 'tasks' && displayedTasks.length === 0 && (
          <div style={{padding:'24px',color:'var(--text-muted)',textAlign:'center'}}>No tasks</div>
        )}

        {tab === 'messages' && Object.entries(msgGroups).map(([did, g]) => (
          <div className="msg-thread" key={did}>
            <div className="msg-thread-peer">{g.peer}</div>
            <div className="msg-thread-preview">
              {g.msgs[0]?.body?.slice(0, 80) || g.msgs[0]?.message_type || '—'}
            </div>
            <div className="msg-thread-meta">{g.msgs.length} message{g.msgs.length !== 1 ? 's' : ''}</div>
          </div>
        ))}
        {tab === 'messages' && Object.keys(msgGroups).length === 0 && (
          <div style={{padding:'24px',color:'var(--text-muted)',textAlign:'center'}}>No messages</div>
        )}

        {tab === 'holons' && (holons || []).map(h => (
          <div className="task-row" key={h.task_id} onClick={() => onSelectHolon?.(h.task_id)}>
            <div className="task-status-dot" style={{background: h.active ? '#7c3aff' : '#2a1f5c'}} />
            <span className="task-row-id">{truncateId(h.task_id)}</span>
            <span className="task-row-desc">{h.my_role || '—'} · {h.member_count || 0} members</span>
            <span className="task-row-status">{h.active ? 'active' : 'done'}</span>
          </div>
        ))}
        {tab === 'holons' && (holons || []).length === 0 && (
          <div style={{padding:'24px',color:'var(--text-muted)',textAlign:'center'}}>No holons</div>
        )}

        {tab === 'deliberation' && (
          <div style={{padding:'24px',color:'var(--text-muted)',textAlign:'center'}}>
            Click a task to view its deliberation thread.
          </div>
        )}
      </div>
    </div>
  )
}
