import { useState, useEffect, useRef, useCallback } from 'react'
import Header           from './components/Header.jsx'
import LeftColumn       from './components/LeftColumn.jsx'
import CenterGraph      from './components/CenterGraph.jsx'
import CenterDirectory  from './components/CenterDirectory.jsx'
import CenterActivity   from './components/CenterActivity.jsx'
import RightStream      from './components/RightStream.jsx'
import NameRegistryPanel    from './components/NameRegistryPanel.jsx'
import KeyManagementPanel   from './components/KeyManagementPanel.jsx'
import ReputationPanel      from './components/ReputationPanel.jsx'
import AgentDetailPanel     from './components/AgentDetailPanel.jsx'
import TaskDetailPanel      from './components/TaskDetailPanel.jsx'
import HolonDetailPanel     from './components/HolonDetailPanel.jsx'
import AuditPanel           from './components/AuditPanel.jsx'
import SubmitTaskModal      from './components/SubmitTaskModal.jsx'

import {
  getIdentity, getReputation, getMyNames, getNetwork, getPeers,
  getGraph, getTasks, getMessages, getHolons, getKeys
} from './api/client.js'

const POLL_MS = 5000

export default function App() {
  // Data state
  const [identity,    setIdentity]    = useState(null)
  const [reputation,  setReputation]  = useState(null)
  const [names,       setNames]       = useState([])
  const [network,     setNetwork]     = useState(null)
  const [graphData,   setGraphData]   = useState(null)
  const [tasks,       setTasks]       = useState([])
  const [messages,    setMessages]    = useState([])
  const [holons,      setHolons]      = useState([])
  const [keys,        setKeys]        = useState(null)
  const [streamEvents, setStreamEvents] = useState([])

  // UI state
  const [view,       setView]       = useState('graph')
  const [panels,     setPanels]     = useState({
    nameRegistry: false,
    keyMgmt:      false,
    reputation:   false,
    agentDetail:  false,
    taskDetail:   false,
    holonDetail:  false,
    audit:        false,
  })
  const [selectedAgent, setSelectedAgent] = useState(null)
  const [selectedTaskId, setSelectedTaskId] = useState(null)
  const [selectedHolonId, setSelectedHolonId] = useState(null)
  const [showSubmit, setShowSubmit] = useState(false)

  function openPanel(name)  { setPanels(p => ({...p, [name]: true})) }
  function closePanel(name) { setPanels(p => ({...p, [name]: false})) }

  function handleSelectAgent(agent) {
    setSelectedAgent(agent)
    openPanel('agentDetail')
  }
  function handleSelectTask(id) {
    setSelectedTaskId(id)
    openPanel('taskDetail')
  }
  function handleSelectHolon(id) {
    setSelectedHolonId(id)
    openPanel('holonDetail')
  }

  // SSE event stream
  useEffect(() => {
    const es = new EventSource('/api/events')
    es.onmessage = (e) => {
      try {
        const ev = JSON.parse(e.data)
        setStreamEvents(prev => [ev, ...prev].slice(0, 200))
      } catch {}
    }
    es.onerror = () => {}
    return () => es.close()
  }, [])

  // Polling
  const refresh = useCallback(async () => {
    try {
      const [id, rep, nm, net, gr, ts, ms, hs, ks] = await Promise.allSettled([
        getIdentity(), getReputation(), getMyNames(), getNetwork(),
        getGraph(), getTasks(), getMessages(), getHolons(), getKeys()
      ])
      if (id.status  === 'fulfilled') setIdentity(id.value)
      if (rep.status === 'fulfilled') setReputation(rep.value)
      if (nm.status  === 'fulfilled') setNames(nm.value || [])
      if (net.status === 'fulfilled') setNetwork(net.value)
      if (gr.status  === 'fulfilled') setGraphData(gr.value)
      if (ts.status  === 'fulfilled') setTasks(ts.value || [])
      if (ms.status  === 'fulfilled') setMessages(ms.value || [])
      if (hs.status  === 'fulfilled') setHolons(hs.value || [])
      if (ks.status  === 'fulfilled') setKeys(ks.value)
    } catch {}
  }, [])

  useEffect(() => {
    refresh()
    const t = setInterval(refresh, POLL_MS)
    return () => clearInterval(t)
  }, [refresh])

  return (
    <div id="root">
      <Header
        identity={identity}
        network={network}
        view={view}
        onViewChange={setView}
        onAudit={() => openPanel('audit')}
        onSettings={() => openPanel('keyMgmt')}
      />
      <div className="app-body">
        <LeftColumn
          identity={identity}
          reputation={reputation}
          names={names}
          keys={keys}
          network={network}
          onOpenNameRegistry={() => openPanel('nameRegistry')}
          onOpenKeyMgmt={() => openPanel('keyMgmt')}
          onOpenReputation={() => openPanel('reputation')}
          onOpenTask={handleSelectTask}
          onOpenMessages={() => setView('activity')}
        />

        <div className="col-center">
          {view === 'graph' && (
            <CenterGraph
              graphData={graphData}
              myPeerId={identity?.peer_id}
              onSelectNode={(id) => {
                const peer = (graphData?.nodes || []).find(n => n.id === id)
                if (peer) handleSelectAgent(peer)
              }}
            />
          )}
          {view === 'directory' && (
            <CenterDirectory onSelectAgent={handleSelectAgent} />
          )}
          {view === 'activity' && (
            <CenterActivity
              tasks={tasks}
              messages={messages}
              holons={holons}
              myDid={identity?.did}
              onSelectTask={handleSelectTask}
              onSelectHolon={handleSelectHolon}
            />
          )}
        </div>

        <RightStream
          events={streamEvents}
          onSelectEvent={(ev) => {
            if (ev.task_id)  handleSelectTask(ev.task_id)
            if (ev.agent_did) handleSelectAgent({ did: ev.agent_did, wws_name: ev.agent_name })
          }}
        />
      </div>

      {/* Panels */}
      <NameRegistryPanel
        open={panels.nameRegistry}
        names={names}
        onClose={() => closePanel('nameRegistry')}
        onRefresh={refresh}
      />
      <KeyManagementPanel
        open={panels.keyMgmt}
        keys={keys}
        onClose={() => closePanel('keyMgmt')}
      />
      <ReputationPanel
        open={panels.reputation}
        reputation={reputation}
        onClose={() => closePanel('reputation')}
      />
      <AgentDetailPanel
        open={panels.agentDetail}
        agent={selectedAgent}
        onClose={() => closePanel('agentDetail')}
      />
      <TaskDetailPanel
        open={panels.taskDetail}
        taskId={selectedTaskId}
        onClose={() => closePanel('taskDetail')}
      />
      <HolonDetailPanel
        open={panels.holonDetail}
        taskId={selectedHolonId}
        onClose={() => closePanel('holonDetail')}
      />
      <AuditPanel
        open={panels.audit}
        onClose={() => closePanel('audit')}
      />
      {showSubmit && (
        <SubmitTaskModal onClose={() => setShowSubmit(false)} onSubmit={refresh} />
      )}
    </div>
  )
}
