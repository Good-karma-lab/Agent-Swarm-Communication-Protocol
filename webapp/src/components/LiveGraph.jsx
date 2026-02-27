import { DataSet, Network } from 'vis-network/standalone'
import { useEffect, useRef, useState } from 'react'

const HOLON_COLORS = {
  Forming:      '#636e72',
  Deliberating: '#ffaa00',
  Voting:       '#ff7675',
  Executing:    '#7c3aff',
  Synthesizing: '#a78bfa',
  Done:         '#00e5b0',
}

export default function LiveGraph({ topology, holons, agents, onNodeClick }) {
  const ref = useRef(null)
  const net = useRef(null)
  const [filter, setFilter] = useState('all') // 'all' | 'agents' | 'holons'
  const [paused, setPaused] = useState(false)

  useEffect(() => {
    if (!ref.current) return

    const agentHealthMap = {}
    ;(agents?.agents || []).forEach(a => { agentHealthMap[a.agent_id] = a })

    const nodes = []
    const edges = []

    // Agent nodes from topology
    if (filter !== 'holons') {
      ;(topology?.nodes || []).forEach(n => {
        const agentData = agentHealthMap[n.id]
        const connected = agentData ? agentData.connected : true
        const loopActive = agentData ? agentData.loop_active : true
        let color = '#2a7ab0'
        if (!connected) color = '#ff3355'
        else if (!loopActive) color = '#ffaa00'
        else if (n.tier === 'Root') color = '#00e5b0'
        else if (n.is_self) color = '#7c3aff'

        nodes.push({
          id: n.id,
          label: (n.name || n.id || '').replace('did:swarm:', '').slice(0, 12),
          color: { background: color, border: color, highlight: { background: '#fff', border: color } },
          shape: n.tier === 'Root' ? 'box' : 'dot',
          size: n.tier === 'Root' ? 20 : n.is_self ? 16 : 12,
          font: { color: '#c8e8ff', size: 10, face: 'JetBrains Mono' },
          title: `${n.name || n.id}\nTier: ${n.tier}\nConnected: ${connected}\nLoop: ${loopActive}`,
        })
      })

      // Topology edges
      ;(topology?.edges || []).forEach((e, i) => {
        const isHierarchy = e.kind === 'hierarchy' || e.kind === 'root_hierarchy'
        edges.push({
          id: `topo-${i}`,
          from: e.source,
          to: e.target,
          color: { color: isHierarchy ? '#1a4a6a' : '#0d2a3a', opacity: 0.8 },
          dashes: !isHierarchy,
          width: isHierarchy ? 1 : 0.5,
        })
      })
    }

    // Holon nodes
    if (filter !== 'agents') {
      ;(holons || []).forEach(h => {
        const color = HOLON_COLORS[h.status] || '#636e72'
        nodes.push({
          id: `holon:${h.task_id}`,
          label: h.task_id.slice(0, 10) + '…',
          color: { background: color, border: color, highlight: { background: '#fff', border: color } },
          shape: 'diamond',
          size: 18,
          font: { color: '#c8e8ff', size: 10, face: 'JetBrains Mono' },
          title: `Holon: ${h.task_id}\nStatus: ${h.status}\nDepth: ${h.depth}\nMembers: ${h.members?.length || 0}`,
        })

        // Parent holon edges
        if (h.parent_holon) {
          edges.push({
            id: `holon-parent-${h.task_id}`,
            from: `holon:${h.parent_holon}`,
            to: `holon:${h.task_id}`,
            color: { color: '#3d1d7f', opacity: 0.6 },
            dashes: true,
            width: 1,
          })
        }

        // Membership edges (agent → holon) — only in 'all' filter
        if (filter === 'all') {
          ;(h.members || []).forEach((memberId, mi) => {
            const agentNodeExists = (topology?.nodes || []).some(n => n.id === memberId)
            if (agentNodeExists) {
              edges.push({
                id: `member-${h.task_id}-${mi}`,
                from: memberId,
                to: `holon:${h.task_id}`,
                color: { color: '#3d1d7f', opacity: 0.4 },
                dashes: [4, 4],
                width: 0.8,
                arrows: { to: { enabled: true, scaleFactor: 0.5 } },
              })
            }
          })
        }
      })
    }

    const options = {
      interaction: { hover: true, tooltipDelay: 200 },
      physics: {
        enabled: !paused,
        stabilization: { enabled: true, iterations: 150 },
        barnesHut: { springLength: 140, springConstant: 0.04, damping: 0.2 },
      },
      edges: { smooth: { type: 'continuous' } },
      layout: { improvedLayout: true },
    }

    if (net.current) net.current.destroy()
    net.current = new Network(ref.current, { nodes: new DataSet(nodes), edges: new DataSet(edges) }, options)

    net.current.on('click', (params) => {
      if (params.nodes.length > 0 && onNodeClick) {
        const nodeId = params.nodes[0]
        if (nodeId.startsWith('holon:')) {
          const taskId = nodeId.replace('holon:', '')
          const holon = (holons || []).find(h => h.task_id === taskId)
          if (holon) onNodeClick({ type: 'holon', data: holon })
        } else {
          const agent = (agents?.agents || []).find(a => a.agent_id === nodeId)
          if (agent) onNodeClick({ type: 'agent', data: { agent } })
        }
      }
    })

    return () => { if (net.current) net.current.destroy() }
  }, [topology, holons, agents, filter, paused])

  const fitGraph = () => { if (net.current) net.current.fit({ animation: true }) }

  return (
    <div className="graph-area">
      <div id="live-graph" ref={ref} className="graph-container" />

      {(topology?.nodes || []).length === 0 && (holons || []).length === 0 && (
        <div className="graph-empty">
          Waiting for agents to connect…
        </div>
      )}

      <div className="graph-controls">
        <button className="btn" style={{ fontSize: 11 }} onClick={fitGraph}>⊞ Fit</button>
        <button className={`btn${filter === 'all' ? ' btn-primary' : ''}`} style={{ fontSize: 11 }} onClick={() => setFilter('all')}>All</button>
        <button className={`btn${filter === 'agents' ? ' btn-primary' : ''}`} style={{ fontSize: 11 }} onClick={() => setFilter('agents')}>Agents</button>
        <button className={`btn${filter === 'holons' ? ' btn-primary' : ''}`} style={{ fontSize: 11 }} onClick={() => setFilter('holons')}>Holons</button>
        <button className="btn" style={{ fontSize: 11 }} onClick={() => setPaused(p => !p)}>
          {paused ? '▶ Resume' : '⏸ Pause'}
        </button>
      </div>
    </div>
  )
}
