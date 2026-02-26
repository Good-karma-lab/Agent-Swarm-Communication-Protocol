import { DataSet, Network } from 'vis-network/standalone'
import { useEffect, useRef } from 'react'

export default function TopologyPanel({ topology }) {
  const ref = useRef(null)
  const net = useRef(null)

  useEffect(() => {
    if (!ref.current) return
    const data = {
      nodes: new DataSet(
        (topology?.nodes || []).map((n) => ({
          id: n.id,
          label: (n.id || '').replace('did:swarm:', ''),
          color: n.is_self ? '#4fd18b' : n.tier === 'Tier1' ? '#ffca63' : '#7fb8ff',
          shape: 'dot',
          size: n.is_self ? 20 : 14
        }))
      ),
      edges: new DataSet(
        (topology?.edges || []).map((e) => ({
          from: e.source,
          to: e.target,
          color: e.kind === 'hierarchy' ? '#4d7fb0' : '#37516d',
          dashes: e.kind !== 'hierarchy'
        }))
      )
    }
    const options = {
      interaction: { hover: true },
      physics: { enabled: true, stabilization: false, barnesHut: { springLength: 120 } },
      edges: { smooth: true },
      nodes: { font: { color: '#dce9f7', size: 11 } },
      layout: { improvedLayout: true }
    }
    if (net.current) net.current.destroy()
    net.current = new Network(ref.current, data, options)
    return () => {
      if (net.current) net.current.destroy()
    }
  }, [topology])

  return (
    <div className="card">
      <h2>Interactive Topology</h2>
      <div id="topologyGraph" ref={ref} />
    </div>
  )
}
