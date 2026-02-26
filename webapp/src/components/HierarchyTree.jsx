import { useMemo, useState } from 'react'

function TreeNode({ node }) {
  const [open, setOpen] = useState(true)
  return (
    <li>
      <div className="row mono">
        <button onClick={() => setOpen((v) => !v)}>{node.children?.length ? (open ? '-' : '+') : '·'}</button>
        <span className="pill">{node.tier}</span>
        <span>{node.agent_name || node.agent_id}</span>
        <span className="muted">tasks={node.task_count || 0}</span>
        <span className="muted">seen={node.last_seen_secs ?? 'n/a'}s</span>
      </div>
      {open && node.children?.length > 0 ? (
        <ul>
          {node.children.map((child) => (
            <TreeNode key={child.agent_id} node={child} />
          ))}
        </ul>
      ) : null}
    </li>
  )
}

export default function HierarchyTree({ nodes }) {
  const roots = useMemo(() => {
    const map = new Map()
    ;(nodes || []).forEach((n) => map.set(n.agent_id, { ...n, children: [] }))
    const result = []
    ;(nodes || []).forEach((n) => {
      if (n.parent_id && map.has(n.parent_id)) {
        map.get(n.parent_id).children.push(map.get(n.agent_id))
      } else {
        result.push(map.get(n.agent_id))
      }
    })
    return result
  }, [nodes])

  return (
    <div className="card tree">
      <h2>Expandable Hierarchy</h2>
      <ul>
        {roots.map((node) => (
          <TreeNode key={node.agent_id} node={node} />
        ))}
      </ul>
    </div>
  )
}
