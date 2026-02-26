import { useCallback, useEffect, useState } from 'react'
import { api } from './api/client'
import { usePolling } from './hooks/usePolling'
import Sidebar from './components/Sidebar'
import OverviewPanel from './components/OverviewPanel'
import HierarchyTree from './components/HierarchyTree'
import VotingPanel from './components/VotingPanel'
import MessagesPanel from './components/MessagesPanel'
import TaskForensicsPanel from './components/TaskForensicsPanel'
import TopologyPanel from './components/TopologyPanel'
import AuditPanel from './components/AuditPanel'
import IdeasPanel from './components/IdeasPanel'

export default function App() {
  const [tab, setTab] = useState('overview')
  const [hierarchy, setHierarchy] = useState({ nodes: [] })
  const [voting, setVoting] = useState({ voting: [], rfp: [] })
  const [messages, setMessages] = useState([])
  const [tasks, setTasks] = useState({ tasks: [] })
  const [agents, setAgents] = useState({ agents: [] })
  const [flow, setFlow] = useState({ counters: {} })
  const [topology, setTopology] = useState({ nodes: [], edges: [] })
  const [taskId, setTaskId] = useState('')
  const [taskTrace, setTaskTrace] = useState({ timeline: [], descendants: [], messages: [] })
  const [taskVoting, setTaskVoting] = useState({ voting: [], rfp: [] })
  const [description, setDescription] = useState('')
  const [recommendations, setRecommendations] = useState({ recommended_features: [] })
  const [audit, setAudit] = useState({ events: [] })
  const [auth, setAuth] = useState({ token_required: false })
  const [operatorToken, setOperatorToken] = useState(localStorage.getItem('openswarm.web.token') || '')
  const [submitError, setSubmitError] = useState('')
  const [live, setLive] = useState({ active_tasks: 0, known_agents: 0, messages: [], events: [] })

  const refresh = useCallback(async () => {
    const [h, v, m, t, ag, f, tp, r, a, au] = await Promise.all([
      api.hierarchy(),
      api.voting(),
      api.messages(),
      api.tasks(),
      api.agents(),
      api.flow(),
      api.topology(),
      api.recommendations(),
      api.audit(),
      api.authStatus()
    ])
    setHierarchy(h)
    setVoting(v)
    setMessages(m)
    setTasks(t)
    setAgents(ag)
    setFlow(f)
    setTopology(tp)
    setRecommendations(r)
    setAudit(a)
    setAuth(au)
  }, [])

  usePolling(refresh, 5000)

  useEffect(() => {
    const proto = location.protocol === 'https:' ? 'wss' : 'ws'
    const ws = new WebSocket(`${proto}://${location.host}/api/stream`)
    ws.onmessage = (event) => {
      try {
        const payload = JSON.parse(event.data)
        if (payload.type === 'snapshot') {
          setLive(payload)
        }
      } catch (_) {
        // ignore
      }
    }
    return () => ws.close()
  }, [])

  const submitTask = async () => {
    if (!description.trim()) return
    localStorage.setItem('openswarm.web.token', operatorToken || '')
    try {
      const res = await api.submitTask(description, operatorToken)
      setSubmitError('')
      setDescription('')
      if (res.task_id) setTaskId(res.task_id)
      await refresh()
    } catch (err) {
      setSubmitError(err.payload?.error || err.message)
    }
  }

  const loadTrace = async (requestedTaskId) => {
    const effectiveTaskId = (requestedTaskId || taskId || '').trim()
    if (!effectiveTaskId) return
    if (requestedTaskId) setTaskId(effectiveTaskId)
    const [trace, votingDetail] = await Promise.all([api.taskTimeline(effectiveTaskId), api.votingTask(effectiveTaskId)])
    setTaskTrace(trace)
    setTaskVoting(votingDetail)
  }

  return (
    <div className="app">
      <Sidebar
        live={live}
        auth={auth}
        operatorToken={operatorToken}
        setOperatorToken={setOperatorToken}
        desc={description}
        setDesc={setDescription}
        submitTask={submitTask}
        submitError={submitError}
        refresh={refresh}
        taskId={taskId}
        setTaskId={setTaskId}
        loadTrace={loadTrace}
        tab={tab}
        setTab={setTab}
      />

      <main className="main">
        {tab === 'overview' ? <OverviewPanel flow={flow} live={live} voting={voting} messages={messages} tasks={tasks} agents={agents} /> : null}
        {tab === 'hierarchy' ? <HierarchyTree nodes={hierarchy.nodes} /> : null}
        {tab === 'voting' ? <VotingPanel voting={voting} /> : null}
        {tab === 'messages' ? <MessagesPanel messages={messages} /> : null}
        {tab === 'task' ? <TaskForensicsPanel taskTrace={taskTrace} tasks={tasks.tasks || []} taskId={taskId} setTaskId={setTaskId} loadTrace={loadTrace} taskVoting={taskVoting} /> : null}
        {tab === 'topology' ? <TopologyPanel topology={topology} /> : null}
        {tab === 'audit' ? <AuditPanel audit={audit} /> : null}
        {tab === 'ideas' ? <IdeasPanel recommendations={recommendations} /> : null}
      </main>
    </div>
  )
}
