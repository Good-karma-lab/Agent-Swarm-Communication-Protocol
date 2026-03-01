import { useEffect, useRef } from 'react'

export default function CenterGraph({ graphData, myPeerId, onSelectNode }) {
  const containerRef = useRef(null)
  const networkRef   = useRef(null)

  useEffect(() => {
    if (!containerRef.current) return
    import('vis-network/standalone').then(({ Network, DataSet }) => {
      const nodes = new DataSet()
      const edges = new DataSet()

      const options = {
        nodes: {
          shape: 'dot', size: 8,
          font: { color: '#4a7a9b', size: 11, face: 'JetBrains Mono' },
          borderWidth: 1,
        },
        edges: {
          color: { color: '#0d2035', highlight: '#00e5b0' },
          width: 1,
          smooth: { type: 'continuous' }
        },
        physics: {
          solver: 'forceAtlas2Based',
          forceAtlas2Based: { gravitationalConstant: -30, springLength: 100 },
          stabilization: { iterations: 100 }
        },
        interaction: { hover: true, tooltipDelay: 200 },
        background: { color: 'transparent' }
      }

      networkRef.current = new Network(containerRef.current, { nodes, edges }, options)

      networkRef.current.on('click', (params) => {
        if (params.nodes.length > 0 && onSelectNode) {
          onSelectNode(params.nodes[0])
        }
      })
    })

    return () => { networkRef.current?.destroy(); networkRef.current = null }
  }, [])

  useEffect(() => {
    if (!networkRef.current || !graphData) return
    import('vis-network/standalone').then(({ DataSet }) => {
      const net = networkRef.current
      const nodesDs = net.body.data.nodes
      const edgesDs = net.body.data.edges

      const newNodes = (graphData.nodes || []).map(n => {
        const isMe = n.id === myPeerId
        return {
          id: n.id,
          label: n.wws_name || n.id.slice(0, 10),
          color: {
            background: isMe ? '#00e5b0' : nodeColor(n),
            border:     isMe ? '#ffffff' : '#0d2035',
            highlight: { background: '#00e5b0', border: '#ffffff' }
          },
          size: isMe ? 18 : nodeSize(n.tier),
          shape: n.type === 'holon' ? 'diamond' : (isMe ? 'box' : 'dot'),
          borderWidth: isMe ? 2 : 1,
          title: tooltipHtml(n),
        }
      })
      const newEdges = (graphData.edges || []).map(e => ({
        id: `${e.from}-${e.to}`,
        from: e.from, to: e.to
      }))

      nodesDs.clear(); nodesDs.add(newNodes)
      edgesDs.clear(); edgesDs.add(newEdges)

      if (myPeerId) {
        try { net.focus(myPeerId, { scale: 1, animation: { duration: 500 } }) }
        catch (_) {}
      }
    })
  }, [graphData, myPeerId])

  return (
    <div className="graph-container">
      <div ref={containerRef} style={{ width:'100%', height:'100%' }} />
      <div className="graph-controls">
        <button className="btn" onClick={() => networkRef.current?.fit()}>⊞ Fit</button>
        <button className="btn" onClick={() => networkRef.current?.stabilize()}>↺</button>
      </div>
    </div>
  )
}

function nodeColor(n) {
  if (n.type === 'holon') return n.active ? '#7c3aff' : '#2a1f5c'
  switch (n.status) {
    case 'healthy': return '#00e5b0'
    case 'warning': return '#ffaa00'
    case 'error':   return '#ff3355'
    default:        return '#1a3a5c'
  }
}

function nodeSize(tier) {
  const sizes = { veteran: 14, established: 12, trusted: 10, member: 9, newcomer: 8 }
  return sizes[tier] || 8
}

function tooltipHtml(n) {
  const lines = []
  if (n.wws_name) lines.push(`<b>${n.wws_name}</b>`)
  if (n.did)      lines.push(`<span style="font-family:monospace;font-size:10px">${n.did.slice(0,24)}…</span>`)
  if (n.tier)     lines.push(n.tier)
  if (n.score != null) lines.push(`${n.score} pts`)
  return lines.join('<br/>')
}
