const EVENT_ICONS = {
  reputation_gained:  '+● ',
  reputation_lost:    '−● ',
  peer_joined:        '◉',
  peer_left:          '◌',
  name_renewed:       '↻',
  name_expiry:        '⚠',
  task_assigned:      '⚡',
  task_completed:     '⚡',
  message_received:   '📨',
  holon_formed:       '⬡',
  holon_dissolved:    '⬡',
  key_rotated:        '🔑',
  key_event:          '🔑',
  security_alert:     '⚠',
}

function timeAgo(ts) {
  if (!ts) return ''
  const s = Math.floor((Date.now() - new Date(ts)) / 1000)
  if (s < 5)    return 'just now'
  if (s < 60)   return `${s}s`
  if (s < 3600) return `${Math.floor(s/60)}m`
  return `${Math.floor(s/3600)}h`
}

export default function RightStream({ events, onSelectEvent }) {
  return (
    <aside className="col-right">
      <div className="stream-header">Live Stream</div>
      <div className="stream-list">
        {(!events || events.length === 0) && (
          <div style={{padding:'16px',color:'var(--text-muted)',textAlign:'center',fontSize:11}}>
            Waiting for events…
          </div>
        )}
        {(events || []).map((ev, i) => (
          <div
            className="stream-event"
            key={ev.id || i}
            onClick={() => onSelectEvent?.(ev)}
          >
            <span className="stream-icon">{EVENT_ICONS[ev.type] || '·'}</span>
            <div className="stream-body">
              <div className="stream-text">{ev.description || ev.type}</div>
              <div className="stream-time">{timeAgo(ev.timestamp)}</div>
            </div>
          </div>
        ))}
      </div>
    </aside>
  )
}
