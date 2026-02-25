//! HTTP server for onboarding docs + operator web dashboard.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, State};
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use tokio::sync::RwLock;

use openswarm_protocol::{ProtocolMethod, SwarmMessage, SwarmTopics, TaskInjectionParams, Tier};

use crate::connector::{ConnectorState, MessageTraceEvent};

struct EmbeddedDocs {
    skill_md: &'static str,
    heartbeat_md: &'static str,
    messaging_md: &'static str,
}

static DOCS: EmbeddedDocs = EmbeddedDocs {
    skill_md: include_str!("../../../docs/SKILL.md"),
    heartbeat_md: include_str!("../../../docs/HEARTBEAT.md"),
    messaging_md: include_str!("../../../docs/MESSAGING.md"),
};

#[derive(Clone)]
struct WebState {
    state: Arc<RwLock<ConnectorState>>,
    network_handle: openswarm_network::SwarmHandle,
}

pub struct FileServer {
    bind_addr: String,
    state: Arc<RwLock<ConnectorState>>,
    network_handle: openswarm_network::SwarmHandle,
}

impl FileServer {
    pub fn new(
        bind_addr: String,
        state: Arc<RwLock<ConnectorState>>,
        network_handle: openswarm_network::SwarmHandle,
    ) -> Self {
        Self {
            bind_addr,
            state,
            network_handle,
        }
    }

    pub async fn run(self) -> Result<(), anyhow::Error> {
        let web_state = WebState {
            state: self.state,
            network_handle: self.network_handle,
        };

        let app = Router::new()
            .route("/", get(index))
            .route("/dashboard", get(index))
            .route("/SKILL.md", get(skill_md))
            .route("/HEARTBEAT.md", get(heartbeat_md))
            .route("/MESSAGING.md", get(messaging_md))
            .route("/agent-onboarding.json", get(onboarding))
            .route("/api/health", get(api_health))
            .route("/api/hierarchy", get(api_hierarchy))
            .route("/api/voting", get(api_voting))
            .route("/api/voting/:task_id", get(api_voting_task))
            .route("/api/messages", get(api_messages))
            .route("/api/messages/:task_id", get(api_messages_task))
            .route("/api/tasks", get(api_tasks).post(api_submit_task))
            .route("/api/tasks/:task_id/timeline", get(api_task_timeline))
            .route("/api/topology", get(api_topology))
            .route("/api/flow", get(api_flow))
            .route("/api/ui-recommendations", get(api_ui_recommendations))
            .route("/api/stream", get(api_stream))
            .with_state(web_state);

        let listener = tokio::net::TcpListener::bind(&self.bind_addr).await?;
        tracing::info!(addr = %self.bind_addr, "HTTP web dashboard listening");
        axum::serve(listener, app).await?;
        Ok(())
    }
}

async fn index() -> Html<&'static str> {
    Html(dashboard_html())
}

async fn skill_md() -> impl IntoResponse {
    ([("content-type", "text/markdown; charset=utf-8")], DOCS.skill_md)
}

async fn heartbeat_md() -> impl IntoResponse {
    (
        [("content-type", "text/markdown; charset=utf-8")],
        DOCS.heartbeat_md,
    )
}

async fn messaging_md() -> impl IntoResponse {
    (
        [("content-type", "text/markdown; charset=utf-8")],
        DOCS.messaging_md,
    )
}

async fn onboarding() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "name": "OpenSwarm Connector",
        "version": env!("CARGO_PKG_VERSION"),
        "protocol": "JSON-RPC 2.0",
        "rpc_default_port": 9370,
        "files_default_port": 9371,
        "dashboard": "/",
        "methods": [
            "swarm.get_status",
            "swarm.receive_task",
            "swarm.get_task",
            "swarm.get_task_timeline",
            "swarm.register_agent",
            "swarm.propose_plan",
            "swarm.submit_vote",
            "swarm.get_voting_state",
            "swarm.submit_result",
            "swarm.connect",
            "swarm.get_network_stats",
            "swarm.inject_task",
            "swarm.get_hierarchy",
            "swarm.list_swarms",
            "swarm.create_swarm",
            "swarm.join_swarm"
        ]
    }))
}

async fn api_health() -> Json<serde_json::Value> {
    Json(serde_json::json!({"ok": true, "service": "openswarm-web"}))
}

async fn api_hierarchy(State(web): State<WebState>) -> Json<serde_json::Value> {
    let s = web.state.read().await;
    let active = s.active_member_ids(Duration::from_secs(180));

    let mut nodes = Vec::new();
    for agent_id in active {
        let tier = s
            .agent_tiers
            .get(&agent_id)
            .cloned()
            .unwrap_or(if agent_id == s.agent_id.to_string() {
                s.my_tier
            } else {
                Tier::Executor
            });
        let parent_id = s.agent_parents.get(&agent_id).cloned();
        let task_count = s
            .task_details
            .values()
            .filter(|t| t.assigned_to.as_ref().map(|a| a.to_string()) == Some(agent_id.clone()))
            .count();

        let last_seen_secs = s.member_last_seen.get(&agent_id).map(|ts| {
            chrono::Utc::now()
                .signed_duration_since(*ts)
                .num_seconds()
                .max(0)
        });

        nodes.push(serde_json::json!({
            "agent_id": agent_id,
            "tier": format!("{:?}", tier),
            "parent_id": parent_id,
            "task_count": task_count,
            "last_seen_secs": last_seen_secs,
            "is_self": agent_id == s.agent_id.to_string(),
        }));
    }

    Json(serde_json::json!({
        "generated_at": chrono::Utc::now(),
        "self_agent": s.agent_id.to_string(),
        "nodes": nodes,
    }))
}

async fn api_voting(State(web): State<WebState>) -> Json<serde_json::Value> {
    Json(voting_payload(&web.state, None).await)
}

async fn api_voting_task(
    State(web): State<WebState>,
    Path(task_id): Path<String>,
) -> Json<serde_json::Value> {
    Json(voting_payload(&web.state, Some(task_id)).await)
}

async fn voting_payload(
    state: &Arc<RwLock<ConnectorState>>,
    task_filter: Option<String>,
) -> serde_json::Value {
    let s = state.read().await;

    let voting = s
        .voting_engines
        .iter()
        .filter(|(task_id, _)| task_filter.as_ref().map(|t| t == *task_id).unwrap_or(true))
        .map(|(task_id, v)| {
            serde_json::json!({
                "task_id": task_id,
                "proposal_count": v.proposal_count(),
                "ballot_count": v.ballot_count(),
                "finalized": v.is_finalized(),
            })
        })
        .collect::<Vec<_>>();

    let rfp = s
        .rfp_coordinators
        .iter()
        .filter(|(task_id, _)| task_filter.as_ref().map(|t| t == *task_id).unwrap_or(true))
        .map(|(task_id, r)| {
            let plans = r
                .reveals
                .values()
                .map(|p| {
                    serde_json::json!({
                        "proposer": p.proposer.to_string(),
                        "plan_id": p.plan.plan_id,
                        "plan_hash": p.plan_hash,
                        "rationale": p.plan.rationale,
                        "subtask_count": p.plan.subtasks.len(),
                    })
                })
                .collect::<Vec<_>>();
            serde_json::json!({
                "task_id": task_id,
                "phase": format!("{:?}", r.phase()),
                "commit_count": r.commit_count(),
                "reveal_count": r.reveal_count(),
                "commits": r.commits_for_debug(),
                "plans": plans,
            })
        })
        .collect::<Vec<_>>();

    serde_json::json!({ "voting": voting, "rfp": rfp })
}

async fn api_messages(State(web): State<WebState>) -> Json<serde_json::Value> {
    Json(messages_payload(&web.state, None).await)
}

async fn api_messages_task(
    State(web): State<WebState>,
    Path(task_id): Path<String>,
) -> Json<serde_json::Value> {
    Json(messages_payload(&web.state, Some(task_id)).await)
}

async fn messages_payload(
    state: &Arc<RwLock<ConnectorState>>,
    task_filter: Option<String>,
) -> serde_json::Value {
    let s = state.read().await;
    let items: Vec<&MessageTraceEvent> = s
        .message_trace
        .iter()
        .rev()
        .filter(|m| {
            task_filter
                .as_ref()
                .map(|t| m.task_id.as_ref().map(|id| id == t).unwrap_or(false))
                .unwrap_or(true)
        })
        .take(1000)
        .collect();
    serde_json::to_value(items).unwrap_or_else(|_| serde_json::json!([]))
}

async fn api_tasks(State(web): State<WebState>) -> Json<serde_json::Value> {
    let s = web.state.read().await;
    let tasks = s
        .task_set
        .elements()
        .into_iter()
        .map(|task_id| {
            let task = s.task_details.get(&task_id);
            serde_json::json!({
                "task_id": task_id,
                "description": task.map(|t| t.description.clone()).unwrap_or_default(),
                "status": task.map(|t| format!("{:?}", t.status)).unwrap_or_else(|| "Unknown".to_string()),
                "tier_level": task.map(|t| t.tier_level).unwrap_or(0),
                "assigned_to": task.and_then(|t| t.assigned_to.as_ref().map(|a| a.to_string())),
                "subtasks": task.map(|t| t.subtasks.clone()).unwrap_or_default(),
            })
        })
        .collect::<Vec<_>>();
    Json(serde_json::json!({"tasks": tasks}))
}

#[derive(Deserialize)]
struct TaskSubmitRequest {
    description: String,
}

async fn api_submit_task(
    State(web): State<WebState>,
    Json(req): Json<TaskSubmitRequest>,
) -> Json<serde_json::Value> {
    if req.description.trim().is_empty() {
        return Json(serde_json::json!({"ok": false, "error": "missing_description"}));
    }

    let mut state_guard = web.state.write().await;
    let epoch = state_guard.epoch_manager.current_epoch();
    let task = openswarm_protocol::Task::new(req.description.clone(), 1, epoch);
    let task_id = task.task_id.clone();
    let originator = state_guard.agent_id.clone();
    let actor = state_guard.agent_id.to_string();
    let swarm_id = state_guard.current_swarm_id.as_str().to_string();

    state_guard.task_set.add(task_id.clone());
    state_guard.task_details.insert(task_id.clone(), task.clone());
    state_guard.push_task_timeline_event(
        &task_id,
        "injected",
        format!("Task injected via web dashboard: {}", req.description),
        Some(actor),
    );
    state_guard.push_log(
        crate::tui::LogCategory::Task,
        format!("Task injected via web UI: {} ({})", task_id, req.description),
    );
    drop(state_guard);

    let inject_params = TaskInjectionParams { task, originator };
    let msg = SwarmMessage::new(
        ProtocolMethod::TaskInjection.as_str(),
        serde_json::to_value(&inject_params).unwrap_or_default(),
        String::new(),
    );

    if let Ok(data) = serde_json::to_vec(&msg) {
        let topic = SwarmTopics::tasks_for(&swarm_id, 1);
        let _ = web.network_handle.publish(&topic, data).await;
    }

    Json(serde_json::json!({
        "ok": true,
        "task_id": task_id,
        "description": req.description
    }))
}

async fn api_task_timeline(
    State(web): State<WebState>,
    Path(task_id): Path<String>,
) -> Json<serde_json::Value> {
    let s = web.state.read().await;

    let timeline = s.task_timelines.get(&task_id).cloned().unwrap_or_default();
    let task = s.task_details.get(&task_id).cloned();
    let messages = s
        .message_trace
        .iter()
        .filter(|m| m.task_id.as_ref().map(|id| id == &task_id).unwrap_or(false))
        .cloned()
        .collect::<Vec<_>>();

    let descendants = collect_task_descendants(&task_id, &s.task_details);

    Json(serde_json::json!({
        "task": task,
        "timeline": timeline,
        "descendants": descendants,
        "messages": messages,
    }))
}

fn collect_task_descendants(
    root: &str,
    details: &HashMap<String, openswarm_protocol::Task>,
) -> Vec<openswarm_protocol::Task> {
    let mut out = Vec::new();
    let mut frontier = vec![root.to_string()];
    while let Some(parent) = frontier.pop() {
        for task in details.values() {
            if task.parent_task_id.as_deref() == Some(parent.as_str()) {
                out.push(task.clone());
                frontier.push(task.task_id.clone());
            }
        }
    }
    out
}

async fn api_topology(State(web): State<WebState>) -> Json<serde_json::Value> {
    let s = web.state.read().await;
    let members = s.active_member_ids(Duration::from_secs(180));

    let nodes = members
        .iter()
        .map(|id| {
            serde_json::json!({
                "id": id,
                "tier": format!("{:?}", s.agent_tiers.get(id).copied().unwrap_or(Tier::Executor)),
                "is_self": *id == s.agent_id.to_string(),
            })
        })
        .collect::<Vec<_>>();

    let mut edges = s
        .agent_parents
        .iter()
        .map(|(child, parent)| {
            serde_json::json!({"source": parent, "target": child, "kind": "hierarchy"})
        })
        .collect::<Vec<_>>();

    for peer in s.agent_set.elements() {
        edges.push(serde_json::json!({
            "source": s.agent_id.to_string(),
            "target": format!("did:swarm:{}", peer),
            "kind": "peer_link"
        }));
    }

    Json(serde_json::json!({"nodes": nodes, "edges": edges}))
}

async fn api_flow(State(web): State<WebState>) -> Json<serde_json::Value> {
    let s = web.state.read().await;
    let mut counters: HashMap<String, usize> = HashMap::new();
    for events in s.task_timelines.values() {
        for event in events {
            *counters.entry(event.stage.clone()).or_insert(0) += 1;
        }
    }

    Json(serde_json::json!({
        "counters": counters,
        "active_tasks": s.task_set.len(),
        "voting_engines": s.voting_engines.len(),
        "rfp_rounds": s.rfp_coordinators.len(),
        "message_trace_size": s.message_trace.len(),
    }))
}

async fn api_ui_recommendations() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "recommended_features": [
            "Task SLA panel (stuck task detector + age heatmap)",
            "Election/succession timeline and incident replay",
            "Agent throughput and reliability leaderboard",
            "Topology drift alerts (partition/churn detection)",
            "Task graph playback over time",
            "Exportable forensic bundle per task (plans, votes, logs, artifacts)",
            "Role-based access control and audit log for operator actions"
        ]
    }))
}

async fn api_stream(
    ws: WebSocketUpgrade,
    State(web): State<WebState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| stream_loop(socket, web.state))
}

async fn stream_loop(mut socket: WebSocket, state: Arc<RwLock<ConnectorState>>) {
    let mut interval = tokio::time::interval(Duration::from_secs(2));
    loop {
        interval.tick().await;
        let payload = {
            let s = state.read().await;
            let recent_messages = s
                .message_trace
                .iter()
                .rev()
                .take(40)
                .cloned()
                .collect::<Vec<_>>();
            let recent_events = s
                .event_log
                .iter()
                .rev()
                .take(40)
                .cloned()
                .collect::<Vec<_>>();
            serde_json::json!({
                "type": "snapshot",
                "time": chrono::Utc::now(),
                "active_tasks": s.task_set.len(),
                "known_agents": s.active_member_count(Duration::from_secs(180)),
                "messages": recent_messages,
                "events": recent_events,
            })
            .to_string()
        };

        if socket.send(Message::Text(payload.into())).await.is_err() {
            break;
        }
    }
}

fn dashboard_html() -> &'static str {
    r#"<!doctype html>
<html>
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>OpenSwarm Web Console</title>
  <script crossorigin src="https://unpkg.com/react@18/umd/react.development.js"></script>
  <script crossorigin src="https://unpkg.com/react-dom@18/umd/react-dom.development.js"></script>
  <script src="https://unpkg.com/@babel/standalone/babel.min.js"></script>
  <script src="https://unpkg.com/vis-network@9.1.9/dist/vis-network.min.js"></script>
  <style>
    :root { --bg:#0b1016; --panel:#121a24; --muted:#9bb0c8; --text:#e8f1fb; --accent:#2ea6ff; --ok:#4fd18b; }
    *{box-sizing:border-box} body{margin:0;font-family:ui-sans-serif,system-ui,-apple-system,Segoe UI,Roboto,Ubuntu;background:radial-gradient(1200px 600px at 85% -10%, #18304e 0%, var(--bg) 55%);color:var(--text)}
    .app{display:grid;grid-template-columns:320px 1fr;min-height:100vh} .side{padding:16px;border-right:1px solid #243243;background:linear-gradient(180deg,#111925,#0d141d)} .main{padding:16px}
    .card{background:var(--panel);border:1px solid #223141;border-radius:12px;padding:12px;margin-bottom:12px}
    h1,h2{margin:6px 0 10px} h1{font-size:20px} h2{font-size:16px;color:#d6e5f8}
    input,button,textarea{background:#0d141d;color:var(--text);border:1px solid #2a3a4d;border-radius:8px;padding:8px}
    button{cursor:pointer} button.primary{background:var(--accent);color:#02121f;border-color:#1e8edc;font-weight:700}
    .row{display:flex;gap:8px;align-items:center;flex-wrap:wrap} .muted{color:var(--muted)}
    .grid{display:grid;grid-template-columns:repeat(2,minmax(280px,1fr));gap:12px} .mono{font-family:ui-monospace,Menlo,Consolas,monospace;font-size:12px}
    .log{max-height:280px;overflow:auto;padding:8px;background:#0a1119;border-radius:8px;border:1px solid #263646}
    .tree ul{list-style:none;padding-left:16px} .tree li{margin:4px 0}
    .pill{display:inline-block;padding:2px 8px;border-radius:999px;background:#163149;color:#8dc9ff;font-size:12px}
    .tabs button{background:transparent;border:1px solid #2b3c4f}.tabs button.active{background:#17324a;border-color:#2ea6ff}
    table{width:100%;border-collapse:collapse} th,td{padding:6px;border-bottom:1px solid #243445;text-align:left;font-size:12px}
    #topologyGraph{height:520px;border:1px solid #2c3f53;border-radius:10px;background:#0b121a}
  </style>
</head>
<body>
  <div id="root"></div>
  <script type="text/babel">
    const {useEffect,useMemo,useRef,useState} = React;
    const fetchJson = (u,o)=>fetch(u,o).then(r=>r.json());

    function App(){
      const [tab,setTab]=useState('overview');
      const [hier,setHier]=useState({nodes:[]});
      const [voting,setVoting]=useState({voting:[],rfp:[]});
      const [messages,setMessages]=useState([]);
      const [tasks,setTasks]=useState({tasks:[]});
      const [flow,setFlow]=useState({counters:{}});
      const [topo,setTopo]=useState({nodes:[],edges:[]});
      const [taskId,setTaskId]=useState('');
      const [taskTrace,setTaskTrace]=useState({timeline:[],descendants:[],messages:[]});
      const [desc,setDesc]=useState('');
      const [uiRec,setUiRec]=useState({recommended_features:[]});
      const [live,setLive]=useState({active_tasks:0, known_agents:0, messages:[], events:[]});
      const graphRef = useRef(null);
      const graphNet = useRef(null);

      const refresh = async ()=>{
        const [h,v,m,t,f,tp,r] = await Promise.all([
          fetchJson('/api/hierarchy'), fetchJson('/api/voting'), fetchJson('/api/messages'),
          fetchJson('/api/tasks'), fetchJson('/api/flow'), fetchJson('/api/topology'), fetchJson('/api/ui-recommendations')
        ]);
        setHier(h); setVoting(v); setMessages(m); setTasks(t); setFlow(f); setTopo(tp); setUiRec(r);
      };

      useEffect(()=>{ refresh(); const x=setInterval(refresh, 5000); return ()=>clearInterval(x); },[]);

      useEffect(()=>{
        const proto = location.protocol === 'https:' ? 'wss' : 'ws';
        const ws = new WebSocket(`${proto}://${location.host}/api/stream`);
        ws.onmessage = (e)=>{ try { const d = JSON.parse(e.data); if(d.type==='snapshot') setLive(d);} catch(_){} };
        return ()=>ws.close();
      },[]);

      useEffect(()=>{
        if (tab !== 'topology' || !graphRef.current || !window.vis) return;
        const data = {
          nodes: new window.vis.DataSet((topo.nodes||[]).map(n=>({
            id:n.id,
            label:(n.id||'').replace('did:swarm:',''),
            color:n.is_self ? '#4fd18b' : (n.tier==='Tier1' ? '#ffca63' : '#7fb8ff'),
            shape:'dot',
            size:n.is_self ? 20 : 14
          }))),
          edges: new window.vis.DataSet((topo.edges||[]).map(e=>({
            from:e.source,to:e.target,
            color:e.kind==='hierarchy' ? '#4d7fb0' : '#37516d',
            dashes:e.kind!=='hierarchy'
          })))
        };
        const options = {
          interaction:{hover:true},
          physics:{enabled:true, stabilization:false, barnesHut:{springLength:120}},
          edges:{smooth:true},
          nodes:{font:{color:'#dce9f7',size:11}},
          layout:{improvedLayout:true}
        };
        if (graphNet.current) graphNet.current.destroy();
        graphNet.current = new window.vis.Network(graphRef.current, data, options);
      }, [tab, topo]);

      const submitTask = async ()=>{
        if(!desc.trim()) return;
        const r = await fetchJson('/api/tasks',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({description:desc})});
        setDesc('');
        if(r.task_id){ setTaskId(r.task_id); }
        refresh();
      };

      const loadTrace = async ()=>{
        if(!taskId.trim()) return;
        const t = await fetchJson(`/api/tasks/${taskId}/timeline`);
        setTaskTrace(t);
      };

      const grouped = useMemo(()=>{
        const map = new Map();
        (hier.nodes||[]).forEach(n=>map.set(n.agent_id,{...n,children:[]}));
        const roots=[];
        (hier.nodes||[]).forEach(n=>{ if(n.parent_id && map.has(n.parent_id)) map.get(n.parent_id).children.push(map.get(n.agent_id)); else roots.push(map.get(n.agent_id)); });
        return roots;
      },[hier]);

      const Tree = ({node}) => {
        const [open,setOpen]=useState(true);
        return <li>
          <div className="row mono">
            <button onClick={()=>setOpen(!open)}>{node.children?.length? (open?'-':'+') : '·'}</button>
            <span className="pill">{node.tier}</span>
            <span>{node.agent_id}</span>
            <span className="muted">tasks={node.task_count||0}</span>
            <span className="muted">seen={node.last_seen_secs ?? 'n/a'}s</span>
          </div>
          {open && node.children?.length>0 && <ul>{node.children.map(c=><Tree key={c.agent_id} node={c} />)}</ul>}
        </li>;
      };

      return <div className="app">
        <aside className="side">
          <h1>OpenSwarm Web Console</h1>
          <div className="card">
            <h2>Live Status</h2>
            <div className="muted">agents={live.known_agents} active_tasks={live.active_tasks}</div>
          </div>
          <div className="card">
            <h2>Submit Task</h2>
            <textarea rows="3" value={desc} onChange={e=>setDesc(e.target.value)} placeholder="Submit a root task" style={{width:'100%'}} />
            <div className="row" style={{marginTop:8}}>
              <button className="primary" onClick={submitTask}>Submit</button>
              <button onClick={refresh}>Refresh</button>
            </div>
          </div>
          <div className="card">
            <h2>Task Forensics</h2>
            <input value={taskId} onChange={e=>setTaskId(e.target.value)} placeholder="task id" style={{width:'100%'}} />
            <div className="row" style={{marginTop:8}}><button onClick={loadTrace}>Load Timeline</button></div>
          </div>
          <div className="card">
            <div className="tabs row">
              {['overview','hierarchy','voting','messages','task','topology','ideas'].map(t=><button key={t} className={tab===t?'active':''} onClick={()=>setTab(t)}>{t}</button>)}
            </div>
          </div>
        </aside>
        <main className="main">
          {tab==='overview' && <div className="grid">
            <div className="card"><h2>Flow Counters</h2><pre className="mono">{JSON.stringify(flow.counters||{},null,2)}</pre></div>
            <div className="card"><h2>Recent Event Log</h2><div className="log mono">{(live.events||[]).map((e,i)=><div key={i}>[{e.timestamp}] {e.category}: {e.message}</div>)}</div></div>
            <div className="card"><h2>Voting</h2><div className="muted">engines={voting.voting?.length||0} rfp={voting.rfp?.length||0}</div></div>
            <div className="card"><h2>P2P Messages</h2><div className="muted">trace items={(messages||[]).length}</div></div>
          </div>}

          {tab==='hierarchy' && <div className="card tree"><h2>Expandable Hierarchy</h2><ul>{grouped.map(n=><Tree key={n.agent_id} node={n} />)}</ul></div>}

          {tab==='voting' && <div className="card"><h2>Voting Process Logs</h2>
            <table><thead><tr><th>Task</th><th>Phase</th><th>Commits</th><th>Reveals</th><th>Plans</th></tr></thead><tbody>
              {(voting.rfp||[]).map(v=><tr key={v.task_id}><td className="mono">{v.task_id}</td><td>{v.phase}</td><td>{v.commit_count}</td><td>{v.reveal_count}</td><td>{(v.plans||[]).map(p=>p.plan_id).join(', ')}</td></tr>)}
            </tbody></table>
          </div>}

          {tab==='messages' && <div className="card"><h2>Peer-to-Peer Debug Logs</h2><div className="log mono">
            {(messages||[]).map((m,i)=><div key={i}>[{m.timestamp}] {m.direction} {m.topic} {m.method||'-'} peer={m.peer||'-'} task={m.task_id||'-'} {m.outcome}</div>)}
          </div></div>}

          {tab==='task' && <div className="grid">
            <div className="card"><h2>Task Timeline</h2><div className="log mono">{(taskTrace.timeline||[]).map((e,i)=><div key={i}>[{e.timestamp}] {e.stage} {e.detail}</div>)}</div></div>
            <div className="card"><h2>Recursive Decomposition/Assignments</h2><pre className="mono">{JSON.stringify(taskTrace.descendants||[],null,2)}</pre></div>
            <div className="card"><h2>Task Propagation Messages</h2><div className="log mono">{(taskTrace.messages||[]).map((m,i)=><div key={i}>[{m.timestamp}] {m.topic} {m.method||'-'} {m.outcome}</div>)}</div></div>
            <div className="card"><h2>Root Task + Aggregation State</h2><pre className="mono">{JSON.stringify(taskTrace.task||{},null,2)}</pre></div>
          </div>}

          {tab==='topology' && <div className="card"><h2>Interactive Topology</h2><div id="topologyGraph" ref={graphRef}></div></div>}

          {tab==='ideas' && <div className="card"><h2>Proposed Next UI Features</h2><ul>{(uiRec.recommended_features||[]).map((x,i)=><li key={i}>{x}</li>)}</ul></div>}
        </main>
      </div>
    }

    ReactDOM.createRoot(document.getElementById('root')).render(<App />);
  </script>
</body>
</html>"#
}
