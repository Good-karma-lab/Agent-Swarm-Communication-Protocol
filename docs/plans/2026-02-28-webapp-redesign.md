# Webapp Redesign: Living Network Console — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the 10-tab developer-tool UI with a full-viewport animated holonic network graph as the primary interface, with contextual slide-in detail panels for all drill-down content.

**Architecture:** Keep all state management, API calls, polling, and WebSocket logic in `App.jsx`. Replace the tab-based layout with a graph-centric layout: Header → LiveGraph (full viewport) → BottomTray. Clicking graph nodes or tray items opens typed slide-in panels from the right. All 10 existing panels are preserved as content inside these panels.

**Tech Stack:** React 18, Vite, vis-network/standalone (existing), Google Fonts (Syne + JetBrains Mono), CSS custom properties + CSS animations (no new JS animation library).

---

## New file structure

```
webapp/src/
├── App.jsx                          ← major refactor (keep all state/logic, new layout)
├── styles.css                       ← complete rewrite
├── main.jsx                         ← unchanged
├── api/client.js                    ← unchanged
├── hooks/usePolling.js              ← unchanged
└── components/
    ├── Header.jsx                   ← NEW
    ├── LiveGraph.jsx                ← NEW (merges TopologyPanel + HolonTreePanel)
    ├── BottomTray.jsx               ← NEW
    ├── SlidePanel.jsx               ← NEW (generic slide-in wrapper)
    ├── TaskDetailPanel.jsx          ← NEW (merges TaskForensicsPanel + VotingPanel + DeliberationPanel)
    ├── AgentDetailPanel.jsx         ← NEW
    ├── HolonDetailPanel.jsx         ← NEW
    ├── AuditPanel.jsx               ← refactored for slide-in
    └── MessagesPanel.jsx            ← refactored for slide-in
    [DELETE: Sidebar, OverviewPanel, HierarchyTree, VotingPanel, TaskForensicsPanel,
             TopologyPanel, IdeasPanel, HolonTreePanel, DeliberationPanel]
```

## Panel types (used in App state)

```js
// panel state shape — null means no panel open
{ type: 'task',   data: { taskId } }
{ type: 'agent',  data: { agent } }
{ type: 'holon',  data: { holon } }
{ type: 'audit',  data: {} }
{ type: 'messages', data: {} }
```

---

## Task 1: Design System (styles.css)

**Files:**
- Rewrite: `webapp/src/styles.css`

**Step 1: Replace entire styles.css**

```css
@import url('https://fonts.googleapis.com/css2?family=Syne:wght@400;600;700;800&family=JetBrains+Mono:wght@400;600&display=swap');

:root {
  --bg:           #020810;
  --surface:      #060f1e;
  --surface-2:    #0a1628;
  --border:       #0d2035;
  --border-2:     #152a42;

  --teal:         #00e5b0;
  --teal-dim:     #00a07a;
  --violet:       #7c3aff;
  --violet-dim:   #3d1d7f;
  --amber:        #ffaa00;
  --coral:        #ff3355;
  --dim-blue:     #1a3a5c;

  --text:         #c8e8ff;
  --text-muted:   #4a7a9b;
  --text-dim:     #1e3a52;

  --font-display: 'Syne', sans-serif;
  --font-mono:    'JetBrains Mono', monospace;
}

* { box-sizing: border-box; margin: 0; padding: 0; }

html, body, #root {
  height: 100%;
  overflow: hidden;
}

body {
  font-family: var(--font-display);
  background: var(--bg);
  color: var(--text);
  font-size: 14px;
}

/* ── Layout ─────────────────────────────── */
.app {
  display: flex;
  flex-direction: column;
  height: 100vh;
  overflow: hidden;
  position: relative;
}

/* ── Header ─────────────────────────────── */
.header {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 0 20px;
  height: 52px;
  flex-shrink: 0;
  border-bottom: 1px solid var(--border);
  background: rgba(2, 8, 16, 0.92);
  backdrop-filter: blur(8px);
  z-index: 20;
}

.header-brand {
  font-size: 18px;
  font-weight: 800;
  color: var(--teal);
  letter-spacing: -0.5px;
  margin-right: 8px;
}

.header-stats {
  display: flex;
  gap: 12px;
  flex: 1;
}

.header-stat {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--text-muted);
  font-family: var(--font-mono);
}

.header-stat strong {
  color: var(--text);
  font-weight: 600;
}

.health-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  display: inline-block;
}

.health-dot.red    { background: var(--coral); box-shadow: 0 0 6px var(--coral); }
.health-dot.yellow { background: var(--amber); box-shadow: 0 0 6px var(--amber); }
.health-dot.green  { background: var(--teal);  box-shadow: 0 0 6px var(--teal);  }

.header-actions {
  display: flex;
  gap: 8px;
  align-items: center;
}

/* ── Buttons ─────────────────────────────── */
.btn {
  font-family: var(--font-display);
  font-size: 12px;
  font-weight: 600;
  padding: 6px 14px;
  border-radius: 6px;
  border: 1px solid var(--border-2);
  background: var(--surface-2);
  color: var(--text-muted);
  cursor: pointer;
  transition: all 0.15s;
  letter-spacing: 0.3px;
}

.btn:hover {
  color: var(--text);
  border-color: var(--teal-dim);
  background: rgba(0, 229, 176, 0.06);
}

.btn-primary {
  background: var(--teal);
  color: #020810;
  border-color: var(--teal);
  font-weight: 700;
}

.btn-primary:hover {
  background: #00ffcc;
  border-color: #00ffcc;
  box-shadow: 0 0 16px rgba(0, 229, 176, 0.4);
}

.btn-ghost {
  background: transparent;
  border-color: transparent;
  color: var(--text-muted);
}

.btn-ghost:hover {
  color: var(--text);
  background: var(--surface-2);
  border-color: var(--border-2);
}

/* ── Graph area ──────────────────────────── */
.graph-area {
  flex: 1;
  position: relative;
  overflow: hidden;
}

.graph-container {
  width: 100%;
  height: 100%;
  position: absolute;
  inset: 0;
}

.graph-controls {
  position: absolute;
  top: 12px;
  right: 12px;
  display: flex;
  gap: 6px;
  z-index: 10;
}

.graph-empty {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-dim);
  font-family: var(--font-mono);
  font-size: 13px;
  pointer-events: none;
}

/* ── Bottom Tray ─────────────────────────── */
.tray {
  display: grid;
  grid-template-columns: 220px 1fr 280px;
  height: 180px;
  flex-shrink: 0;
  border-top: 1px solid var(--border);
  background: rgba(6, 15, 30, 0.95);
  overflow: hidden;
}

.tray-col {
  padding: 12px 14px;
  border-right: 1px solid var(--border);
  overflow: hidden;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.tray-col:last-child { border-right: none; }

.tray-label {
  font-size: 10px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 1px;
  color: var(--text-muted);
  flex-shrink: 0;
}

.tray-scroll {
  overflow-y: auto;
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.tray-scroll::-webkit-scrollbar { width: 2px; }
.tray-scroll::-webkit-scrollbar-track { background: transparent; }
.tray-scroll::-webkit-scrollbar-thumb { background: var(--border-2); border-radius: 2px; }

/* health tray */
.health-summary {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.health-row {
  display: flex;
  align-items: center;
  gap: 8px;
  font-family: var(--font-mono);
  font-size: 11px;
}

.health-count {
  font-size: 20px;
  font-weight: 700;
  font-family: var(--font-mono);
  line-height: 1;
}

.health-count.red    { color: var(--coral); }
.health-count.yellow { color: var(--amber); }
.health-count.green  { color: var(--teal);  }

/* task stream */
.task-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 5px 8px;
  border-radius: 5px;
  background: var(--surface);
  border: 1px solid var(--border);
  cursor: pointer;
  transition: all 0.12s;
  font-size: 11px;
}

.task-item:hover {
  border-color: var(--teal-dim);
  background: rgba(0, 229, 176, 0.04);
}

.task-status-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  flex-shrink: 0;
}

.task-id {
  font-family: var(--font-mono);
  color: var(--teal);
  font-size: 10px;
  flex-shrink: 0;
}

.task-desc {
  color: var(--text-muted);
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
  flex: 1;
}

/* agent roster */
.agent-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 6px;
  border-radius: 4px;
  cursor: pointer;
  transition: background 0.12s;
  font-size: 11px;
}

.agent-item:hover {
  background: var(--surface);
}

.agent-name {
  font-family: var(--font-mono);
  color: var(--text);
  font-size: 10px;
  flex: 1;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}

.agent-tier {
  color: var(--text-muted);
  font-size: 10px;
}

/* ── Slide Panel ─────────────────────────── */
.slide-overlay {
  position: absolute;
  inset: 0;
  z-index: 50;
  pointer-events: none;
}

.slide-backdrop {
  position: absolute;
  inset: 0;
  background: rgba(2, 8, 16, 0.6);
  backdrop-filter: blur(2px);
  pointer-events: all;
  animation: fadeIn 0.2s ease forwards;
}

.slide-panel {
  position: absolute;
  top: 0;
  right: 0;
  bottom: 0;
  width: 640px;
  background: var(--surface);
  border-left: 1px solid var(--border-2);
  display: flex;
  flex-direction: column;
  pointer-events: all;
  animation: slideIn 0.3s cubic-bezier(0.16, 1, 0.32, 1) forwards;
  overflow: hidden;
}

@keyframes fadeIn {
  from { opacity: 0; }
  to   { opacity: 1; }
}

@keyframes slideIn {
  from { transform: translateX(100%); }
  to   { transform: translateX(0); }
}

.slide-header {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 14px 20px;
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}

.slide-title {
  flex: 1;
  font-size: 14px;
  font-weight: 700;
  color: var(--text);
}

.slide-close {
  background: none;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  font-size: 18px;
  padding: 2px 6px;
  border-radius: 4px;
  line-height: 1;
  transition: color 0.12s;
}

.slide-close:hover { color: var(--text); }

.slide-body {
  flex: 1;
  overflow-y: auto;
  padding: 16px 20px;
}

.slide-body::-webkit-scrollbar { width: 4px; }
.slide-body::-webkit-scrollbar-track { background: transparent; }
.slide-body::-webkit-scrollbar-thumb { background: var(--border-2); border-radius: 2px; }

/* ── Panel tabs ──────────────────────────── */
.panel-tabs {
  display: flex;
  gap: 2px;
  border-bottom: 1px solid var(--border);
  padding: 0 20px;
  flex-shrink: 0;
}

.panel-tab {
  font-family: var(--font-display);
  font-size: 12px;
  font-weight: 600;
  padding: 8px 14px;
  background: none;
  border: none;
  border-bottom: 2px solid transparent;
  color: var(--text-muted);
  cursor: pointer;
  transition: all 0.15s;
  margin-bottom: -1px;
}

.panel-tab:hover { color: var(--text); }

.panel-tab.active {
  color: var(--teal);
  border-bottom-color: var(--teal);
}

/* ── Detail panel internals ──────────────── */
.detail-section {
  margin-bottom: 20px;
}

.detail-section-title {
  font-size: 11px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 1px;
  color: var(--text-muted);
  margin-bottom: 8px;
}

.detail-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  font-size: 12px;
  font-family: var(--font-mono);
  color: var(--text-muted);
  background: var(--surface-2);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 10px 12px;
}

.detail-meta strong { color: var(--text); font-weight: 600; }

.badge {
  display: inline-flex;
  align-items: center;
  padding: 2px 8px;
  border-radius: 4px;
  font-size: 11px;
  font-weight: 700;
  font-family: var(--font-display);
}

.badge-teal    { background: rgba(0, 229, 176, 0.15); color: var(--teal);   border: 1px solid rgba(0, 229, 176, 0.3); }
.badge-violet  { background: rgba(124, 58, 255, 0.15); color: #a78bfa;      border: 1px solid rgba(124, 58, 255, 0.3); }
.badge-amber   { background: rgba(255, 170, 0, 0.15);  color: var(--amber); border: 1px solid rgba(255, 170, 0, 0.3); }
.badge-coral   { background: rgba(255, 51, 85, 0.15);  color: var(--coral); border: 1px solid rgba(255, 51, 85, 0.3); }
.badge-dim     { background: rgba(26, 58, 92, 0.5);    color: var(--text-muted); border: 1px solid var(--border-2); }

/* ── Tables ──────────────────────────────── */
.data-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 11px;
  font-family: var(--font-mono);
}

.data-table th {
  text-align: left;
  padding: 6px 8px;
  color: var(--text-muted);
  font-weight: 600;
  border-bottom: 1px solid var(--border-2);
  font-size: 10px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.data-table td {
  padding: 6px 8px;
  border-bottom: 1px solid var(--border);
  color: var(--text);
  vertical-align: top;
}

.data-table tr:hover td { background: rgba(0, 229, 176, 0.02); }

/* ── Log / mono areas ────────────────────── */
.log-box {
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 10px 12px;
  font-family: var(--font-mono);
  font-size: 11px;
  color: var(--text-muted);
  overflow-y: auto;
  max-height: 240px;
  line-height: 1.6;
}

.log-box::-webkit-scrollbar { width: 3px; }
.log-box::-webkit-scrollbar-track { background: transparent; }
.log-box::-webkit-scrollbar-thumb { background: var(--border-2); }

/* ── Score bars ──────────────────────────── */
.score-bar-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 4px;
}

.score-bar-label {
  width: 96px;
  font-size: 11px;
  font-family: var(--font-mono);
  color: var(--text-muted);
  flex-shrink: 0;
}

.score-bar-track {
  flex: 1;
  height: 6px;
  background: var(--border-2);
  border-radius: 3px;
  overflow: hidden;
}

.score-bar-fill {
  height: 100%;
  border-radius: 3px;
  transition: width 0.4s ease;
}

.score-bar-fill.green  { background: var(--teal); }
.score-bar-fill.yellow { background: var(--amber); }
.score-bar-fill.red    { background: var(--coral); }

.score-bar-value {
  width: 32px;
  text-align: right;
  font-size: 11px;
  font-family: var(--font-mono);
  color: var(--text-muted);
}

/* ── Timeline replay ─────────────────────── */
.timeline-controls {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 8px;
}

.timeline-slider {
  flex: 1;
  appearance: none;
  height: 4px;
  border-radius: 2px;
  background: var(--border-2);
  outline: none;
  cursor: pointer;
}

.timeline-slider::-webkit-slider-thumb {
  appearance: none;
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: var(--teal);
  cursor: pointer;
}

/* ── Deliberation messages ───────────────── */
.deliberation-msg {
  border-radius: 0 6px 6px 0;
  padding: 10px 14px;
  margin-bottom: 10px;
  border-left: 3px solid;
}

.deliberation-msg-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 6px;
}

.deliberation-msg-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 11px;
}

.deliberation-msg-time {
  font-family: var(--font-mono);
  font-size: 11px;
  color: var(--text-muted);
}

.deliberation-msg-content {
  font-size: 12px;
  color: var(--text);
  line-height: 1.5;
  cursor: pointer;
}

.deliberation-msg-content.collapsed {
  max-height: 60px;
  overflow: hidden;
}

/* ── Modal ───────────────────────────────── */
.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(2, 8, 16, 0.8);
  backdrop-filter: blur(4px);
  z-index: 100;
  display: flex;
  align-items: center;
  justify-content: center;
  animation: fadeIn 0.15s ease;
}

.modal {
  background: var(--surface);
  border: 1px solid var(--border-2);
  border-radius: 10px;
  padding: 24px;
  width: 480px;
  max-width: 90vw;
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.modal-title {
  font-size: 16px;
  font-weight: 700;
  color: var(--text);
}

.input {
  font-family: var(--font-mono);
  font-size: 12px;
  padding: 8px 12px;
  border-radius: 6px;
  border: 1px solid var(--border-2);
  background: var(--bg);
  color: var(--text);
  width: 100%;
  resize: vertical;
  transition: border-color 0.15s;
}

.input:focus {
  outline: none;
  border-color: var(--teal-dim);
}

.error-msg {
  color: var(--coral);
  font-size: 12px;
  font-family: var(--font-mono);
}

/* ── Agent detail ────────────────────────── */
.member-chip {
  display: inline-flex;
  align-items: center;
  padding: 3px 8px;
  background: var(--surface-2);
  border: 1px solid var(--border-2);
  border-radius: 4px;
  font-family: var(--font-mono);
  font-size: 10px;
  color: var(--text-muted);
}

/* ── IRV rounds ──────────────────────────── */
.irv-round {
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 8px 12px;
  margin-bottom: 6px;
  font-size: 11px;
  font-family: var(--font-mono);
}

.irv-round-header {
  display: flex;
  justify-content: space-between;
  margin-bottom: 6px;
  font-weight: 600;
}

/* ── Graph node tooltip (vis-network custom) */
#live-graph { width: 100%; height: 100%; }

/* ── DAG graph ───────────────────────────── */
.dag-container {
  height: 320px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--bg);
}

/* ── Scrollbar global ────────────────────── */
::-webkit-scrollbar { width: 4px; }
::-webkit-scrollbar-track { background: transparent; }
::-webkit-scrollbar-thumb { background: var(--border-2); border-radius: 2px; }
```

**Step 2: Verify app loads without errors**

Run: `cd webapp && npm run dev`
Open http://localhost:5173 — the app may break visually (old components + new CSS conflict), that's OK. Confirm no build errors.

**Step 3: Commit**

```bash
cd webapp
git add src/styles.css
git commit -m "feat: new Living Network design system (Syne + JetBrains Mono, teal/violet palette)"
```

---

## Task 2: SlidePanel Component

**Files:**
- Create: `webapp/src/components/SlidePanel.jsx`

**Step 1: Create SlidePanel.jsx**

```jsx
export default function SlidePanel({ title, onClose, tabs, activeTab, onTabChange, children }) {
  // Close on Escape key
  import { useEffect } from 'react'
  useEffect(() => {
    const handler = (e) => { if (e.key === 'Escape') onClose() }
    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [onClose])

  return (
    <div className="slide-overlay">
      <div className="slide-backdrop" onClick={onClose} />
      <div className="slide-panel">
        <div className="slide-header">
          <span className="slide-title">{title}</span>
          <button className="slide-close" onClick={onClose}>✕</button>
        </div>
        {tabs && tabs.length > 0 && (
          <div className="panel-tabs">
            {tabs.map(t => (
              <button
                key={t.id}
                className={`panel-tab${activeTab === t.id ? ' active' : ''}`}
                onClick={() => onTabChange(t.id)}
              >
                {t.label}
              </button>
            ))}
          </div>
        )}
        <div className="slide-body">
          {children}
        </div>
      </div>
    </div>
  )
}
```

> Note: Fix the import placement — move `import { useEffect } from 'react'` to the top of the file, outside the function:

```jsx
import { useEffect } from 'react'

export default function SlidePanel({ title, onClose, tabs, activeTab, onTabChange, children }) {
  useEffect(() => {
    const handler = (e) => { if (e.key === 'Escape') onClose() }
    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [onClose])

  return (
    <div className="slide-overlay">
      <div className="slide-backdrop" onClick={onClose} />
      <div className="slide-panel">
        <div className="slide-header">
          <span className="slide-title">{title}</span>
          <button className="slide-close" onClick={onClose}>✕</button>
        </div>
        {tabs && tabs.length > 0 && (
          <div className="panel-tabs">
            {tabs.map(t => (
              <button
                key={t.id}
                className={`panel-tab${activeTab === t.id ? ' active' : ''}`}
                onClick={() => onTabChange(t.id)}
              >
                {t.label}
              </button>
            ))}
          </div>
        )}
        <div className="slide-body">
          {children}
        </div>
      </div>
    </div>
  )
}
```

**Step 2: Commit**

```bash
git add webapp/src/components/SlidePanel.jsx
git commit -m "feat: add SlidePanel generic slide-in container"
```

---

## Task 3: Header Component

**Files:**
- Create: `webapp/src/components/Header.jsx`

**Step 1: Create Header.jsx**

```jsx
export default function Header({ agents, tasks, live, onSubmitClick, onAuditClick, onMessagesClick }) {
  const agentList = agents?.agents || []
  const taskList = tasks?.tasks || []

  const red    = agentList.filter(a => !a.connected).length
  const yellow = agentList.filter(a => a.connected && !a.loop_active).length
  const green  = agentList.filter(a => a.connected && a.loop_active).length

  return (
    <header className="header">
      <span className="header-brand">ASIP</span>

      <div className="header-stats">
        <div className="header-stat">
          <span className="health-dot green" />
          <strong>{green}</strong> healthy
        </div>
        {yellow > 0 && (
          <div className="header-stat">
            <span className="health-dot yellow" />
            <strong>{yellow}</strong> degraded
          </div>
        )}
        {red > 0 && (
          <div className="header-stat">
            <span className="health-dot red" />
            <strong>{red}</strong> down
          </div>
        )}
        <div className="header-stat">
          <strong>{agentList.length}</strong> agents
        </div>
        <div className="header-stat">
          <strong>{taskList.length}</strong> tasks
        </div>
        {live?.active_tasks > 0 && (
          <div className="header-stat">
            <span className="health-dot green" style={{ animation: 'pulse 1.5s infinite' }} />
            <strong>{live.active_tasks}</strong> active
          </div>
        )}
      </div>

      <div className="header-actions">
        <button className="btn btn-ghost" onClick={onMessagesClick}>Messages</button>
        <button className="btn btn-ghost" onClick={onAuditClick}>Audit</button>
        <button className="btn btn-primary" onClick={onSubmitClick}>+ Submit Task</button>
      </div>
    </header>
  )
}
```

Add the pulse animation to `styles.css` (append at end):

```css
@keyframes pulse {
  0%, 100% { opacity: 1; transform: scale(1); }
  50% { opacity: 0.5; transform: scale(0.8); }
}
```

**Step 2: Commit**

```bash
git add webapp/src/components/Header.jsx webapp/src/styles.css
git commit -m "feat: add Header component with live health indicators"
```

---

## Task 4: BottomTray Component

**Files:**
- Create: `webapp/src/components/BottomTray.jsx`

**Step 1: Create BottomTray.jsx**

```jsx
function taskStatusColor(status) {
  if (!status) return '#4a7a9b'
  const s = status.toLowerCase()
  if (s === 'completed' || s === 'done') return '#00e5b0'
  if (s === 'failed' || s === 'error') return '#ff3355'
  if (s === 'running' || s === 'executing') return '#ffaa00'
  return '#4a7a9b'
}

function agentHealthColor(agent) {
  if (!agent.connected) return '#ff3355'
  if (!agent.loop_active) return '#ffaa00'
  return '#00e5b0'
}

function scrubId(s) {
  return String(s || '').replace(/did:swarm:[A-Za-z0-9]+/g, m => '[' + m.slice(-6) + ']')
}

export default function BottomTray({ agents, tasks, onTaskClick, onAgentClick }) {
  const agentList = agents?.agents || []
  const taskList = tasks?.tasks || []

  const red    = agentList.filter(a => !a.connected).length
  const yellow = agentList.filter(a => a.connected && !a.loop_active).length
  const green  = agentList.filter(a => a.connected && a.loop_active).length

  return (
    <div className="tray">
      {/* Health Summary */}
      <div className="tray-col">
        <div className="tray-label">System Health</div>
        <div className="health-summary">
          <div className="health-row">
            <span className="health-count green">{green}</span>
            <span className="health-dot green" />
            <span style={{ fontSize: 11, color: 'var(--text-muted)' }}>healthy</span>
          </div>
          <div className="health-row">
            <span className="health-count yellow">{yellow}</span>
            <span className="health-dot yellow" />
            <span style={{ fontSize: 11, color: 'var(--text-muted)' }}>degraded</span>
          </div>
          <div className="health-row">
            <span className="health-count red">{red}</span>
            <span className="health-dot red" />
            <span style={{ fontSize: 11, color: 'var(--text-muted)' }}>down</span>
          </div>
          <div style={{ fontSize: 10, color: 'var(--text-muted)', fontFamily: 'var(--font-mono)', marginTop: 4 }}>
            {agentList.length} total agents
          </div>
        </div>
      </div>

      {/* Task Stream */}
      <div className="tray-col">
        <div className="tray-label">Tasks</div>
        <div className="tray-scroll">
          {taskList.length === 0 && (
            <div style={{ color: 'var(--text-dim)', fontSize: 11, fontFamily: 'var(--font-mono)' }}>
              No tasks yet
            </div>
          )}
          {taskList.slice(0, 20).map(t => (
            <div key={t.task_id} className="task-item" onClick={() => onTaskClick(t)}>
              <span className="task-status-dot" style={{ background: taskStatusColor(t.status) }} />
              <span className="task-id">{t.task_id.slice(0, 8)}…</span>
              <span className="task-desc">{t.description || t.task_id}</span>
              <span style={{ fontSize: 10, color: 'var(--text-muted)', flexShrink: 0 }}>{t.status}</span>
            </div>
          ))}
        </div>
      </div>

      {/* Agent Roster */}
      <div className="tray-col">
        <div className="tray-label">Agents</div>
        <div className="tray-scroll">
          {agentList.length === 0 && (
            <div style={{ color: 'var(--text-dim)', fontSize: 11, fontFamily: 'var(--font-mono)' }}>
              No agents connected
            </div>
          )}
          {agentList.slice(0, 16).map(a => (
            <div key={a.agent_id} className="agent-item" onClick={() => onAgentClick(a)}>
              <span className="health-dot" style={{ background: agentHealthColor(a), width: 7, height: 7 }} />
              <span className="agent-name">{scrubId(a.name || a.agent_id)}</span>
              <span className="agent-tier" style={{ fontFamily: 'var(--font-mono)', fontSize: 10 }}>{a.tier}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  )
}
```

**Step 2: Commit**

```bash
git add webapp/src/components/BottomTray.jsx
git commit -m "feat: add BottomTray with health summary, task stream, agent roster"
```

---

## Task 5: LiveGraph Component

**Files:**
- Create: `webapp/src/components/LiveGraph.jsx`

**Step 1: Create LiveGraph.jsx**

The graph combines `topology` (agent nodes/edges) and `holons` (holon nodes + membership edges).
- Agent nodes: `dot` shape, colored by health
- Holon nodes: `diamond` shape, colored by status, larger
- Edges: hierarchy (teal), membership (violet dotted), task-assignment (animated)

```jsx
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

function agentColor(node) {
  // node from topology: { id, name, tier, is_self, connected, loop_active }
  if (node.connected === false) return '#ff3355'
  if (node.loop_active === false) return '#ffaa00'
  if (node.tier === 'Root') return '#00e5b0'
  if (node.is_self) return '#7c3aff'
  return '#2a7ab0'
}

export default function LiveGraph({ topology, holons, agents, onNodeClick }) {
  const ref = useRef(null)
  const net = useRef(null)
  const [filter, setFilter] = useState('all') // 'all' | 'agents' | 'holons'
  const [paused, setPaused] = useState(false)

  // Build agent health map from agents list for richer coloring
  const agentHealthMap = {}
  ;(agents?.agents || []).forEach(a => { agentHealthMap[a.agent_id] = a })

  useEffect(() => {
    if (!ref.current) return

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
        if (h.parent_holon && filter !== 'agents') {
          edges.push({
            id: `holon-parent-${h.task_id}`,
            from: `holon:${h.parent_holon}`,
            to: `holon:${h.task_id}`,
            color: { color: '#3d1d7f', opacity: 0.6 },
            dashes: true,
            width: 1,
          })
        }

        // Membership edges (agent → holon)
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

    const nodeDataSet = new DataSet(nodes)
    const edgeDataSet = new DataSet(edges)

    const options = {
      interaction: { hover: true, tooltipDelay: 200 },
      physics: {
        enabled: !paused,
        stabilization: { enabled: true, iterations: 150 },
        barnesHut: { springLength: 140, springConstant: 0.04, damping: 0.2 },
      },
      edges: { smooth: { type: 'continuous' } },
      layout: { improvedLayout: true },
      background: { color: 'transparent' },
    }

    if (net.current) net.current.destroy()
    net.current = new Network(ref.current, { nodes: nodeDataSet, edges: edgeDataSet }, options)

    net.current.on('click', (params) => {
      if (params.nodes.length > 0) {
        const nodeId = params.nodes[0]
        if (nodeId.startsWith('holon:')) {
          const taskId = nodeId.replace('holon:', '')
          const holon = (holons || []).find(h => h.task_id === taskId)
          if (holon && onNodeClick) onNodeClick({ type: 'holon', data: holon })
        } else {
          const agent = (agents?.agents || []).find(a => a.agent_id === nodeId)
          if (agent && onNodeClick) onNodeClick({ type: 'agent', data: { agent } })
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
```

**Step 2: Commit**

```bash
git add webapp/src/components/LiveGraph.jsx
git commit -m "feat: add LiveGraph component (unified topology + holons vis-network)"
```

---

## Task 6: SubmitTaskModal Component

**Files:**
- Create: `webapp/src/components/SubmitTaskModal.jsx`

**Step 1: Create SubmitTaskModal.jsx**

```jsx
import { useEffect } from 'react'

export default function SubmitTaskModal({ description, setDescription, operatorToken, setOperatorToken, auth, onSubmit, onClose, submitError }) {
  useEffect(() => {
    const handler = (e) => { if (e.key === 'Escape') onClose() }
    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [onClose])

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={e => e.stopPropagation()}>
        <div className="modal-title">Submit Task</div>

        <textarea
          className="input"
          rows={4}
          placeholder="Describe the task…"
          value={description}
          onChange={e => setDescription(e.target.value)}
          autoFocus
        />

        {auth?.token_required && (
          <input
            className="input"
            placeholder="Operator token"
            type="password"
            value={operatorToken}
            onChange={e => setOperatorToken(e.target.value)}
          />
        )}

        {submitError && <div className="error-msg">{submitError}</div>}

        <div style={{ display: 'flex', gap: 8, justifyContent: 'flex-end' }}>
          <button className="btn" onClick={onClose}>Cancel</button>
          <button className="btn btn-primary" onClick={onSubmit} disabled={!description.trim()}>
            Submit
          </button>
        </div>
      </div>
    </div>
  )
}
```

**Step 2: Commit**

```bash
git add webapp/src/components/SubmitTaskModal.jsx
git commit -m "feat: add SubmitTaskModal"
```

---

## Task 7: AuditPanel and MessagesPanel (refactored)

**Files:**
- Rewrite: `webapp/src/components/AuditPanel.jsx`
- Rewrite: `webapp/src/components/MessagesPanel.jsx`

**Step 1: Rewrite AuditPanel.jsx** (no card wrapper — renders inside SlidePanel)

```jsx
export default function AuditPanel({ audit }) {
  const events = audit?.events || []
  return (
    <div>
      <div className="detail-section-title">Operator Audit Log</div>
      <div className="log-box" style={{ maxHeight: '70vh' }}>
        {events.length === 0 && <div style={{ color: 'var(--text-dim)' }}>No audit events yet.</div>}
        {events.map((e, i) => (
          <div key={i} style={{ marginBottom: 2 }}>
            <span style={{ color: 'var(--text-muted)' }}>[{e.timestamp}]</span>{' '}
            {e.message}
          </div>
        ))}
      </div>
    </div>
  )
}
```

**Step 2: Rewrite MessagesPanel.jsx**

The existing MessagesPanel filters to business messages. Keep that logic:

```jsx
const SKIP_METHODS = new Set(['keepalive', 'ping', 'pong', 'peer_discovery', 'peer_announce', 'swarm_join', 'swarm_leave'])
const SKIP_TOPICS  = new Set(['_keepalive', '_discovery', '_internal'])

function scrub(s) {
  return String(s || '').replace(/did:swarm:[A-Za-z0-9]+/g, m => '[' + m.slice(-6) + ']')
}

export default function MessagesPanel({ messages }) {
  const filtered = (messages || []).filter(m => {
    if (SKIP_METHODS.has(m.method)) return false
    if (SKIP_TOPICS.has(m.topic))   return false
    return true
  })

  return (
    <div>
      <div className="detail-section-title">P2P Business Messages ({filtered.length})</div>
      <div className="log-box" style={{ maxHeight: '70vh' }}>
        {filtered.length === 0 && <div style={{ color: 'var(--text-dim)' }}>No messages yet.</div>}
        {filtered.map((m, i) => (
          <div key={i} style={{ marginBottom: 2 }}>
            <span style={{ color: 'var(--text-muted)' }}>[{m.timestamp}]</span>{' '}
            <span style={{ color: 'var(--teal, #00e5b0)' }}>{m.topic}</span>{' '}
            {m.method && <span style={{ color: '#a78bfa' }}>{m.method}</span>}{' '}
            {scrub(m.outcome || '')}
          </div>
        ))}
      </div>
    </div>
  )
}
```

**Step 3: Commit**

```bash
git add webapp/src/components/AuditPanel.jsx webapp/src/components/MessagesPanel.jsx
git commit -m "feat: refactor AuditPanel + MessagesPanel for slide-in use"
```

---

## Task 8: AgentDetailPanel Component

**Files:**
- Create: `webapp/src/components/AgentDetailPanel.jsx`

**Step 1: Create AgentDetailPanel.jsx**

```jsx
function scrub(s) {
  return String(s || '').replace(/did:swarm:[A-Za-z0-9]+/g, m => '[' + m.slice(-6) + ']')
}

function healthLabel(a) {
  if (!a.connected) return { text: 'DOWN', cls: 'badge-coral' }
  if (!a.loop_active) return { text: 'DEGRADED', cls: 'badge-amber' }
  return { text: 'HEALTHY', cls: 'badge-teal' }
}

export default function AgentDetailPanel({ agent, tasks, onTaskClick }) {
  const health = healthLabel(agent)
  const taskList = (tasks?.tasks || []).filter(t => t.assigned_to === agent.agent_id || t.assigned_to_name === agent.name)

  return (
    <div>
      {/* Header meta */}
      <div className="detail-meta" style={{ marginBottom: 20 }}>
        <span>ID: <strong>{scrub(agent.agent_id)}</strong></span>
        <span>Name: <strong>{scrub(agent.name)}</strong></span>
        <span>Tier: <strong>{agent.tier}</strong></span>
        <span className={`badge ${health.cls}`}>{health.text}</span>
      </div>

      {/* Stats */}
      <div className="detail-section">
        <div className="detail-section-title">Activity</div>
        <table className="data-table">
          <thead>
            <tr>
              <th>Metric</th>
              <th>Value</th>
            </tr>
          </thead>
          <tbody>
            <tr><td>Connected</td><td>{agent.connected ? 'yes' : 'no'}</td></tr>
            <tr><td>Loop active</td><td>{agent.loop_active ? 'yes' : 'no'}</td></tr>
            <tr><td>Tasks assigned</td><td>{agent.tasks_assigned_count ?? 0}</td></tr>
            <tr><td>Tasks processed</td><td>{agent.tasks_processed_count ?? 0}</td></tr>
            <tr><td>Plans proposed</td><td>{agent.plans_proposed_count ?? 0}</td></tr>
            <tr><td>Plans revealed</td><td>{agent.plans_revealed_count ?? 0}</td></tr>
            <tr><td>Votes cast</td><td>{agent.votes_cast_count ?? 0}</td></tr>
            <tr><td>Last poll (s)</td><td>{agent.last_task_poll_secs ?? '—'}</td></tr>
            <tr><td>Last result (s)</td><td>{agent.last_result_secs ?? '—'}</td></tr>
          </tbody>
        </table>
      </div>

      {/* Assigned tasks */}
      {taskList.length > 0 && (
        <div className="detail-section">
          <div className="detail-section-title">Assigned Tasks</div>
          {taskList.map(t => (
            <div
              key={t.task_id}
              onClick={() => onTaskClick && onTaskClick(t)}
              style={{
                padding: '6px 10px',
                background: 'var(--surface-2)',
                border: '1px solid var(--border)',
                borderRadius: 5,
                marginBottom: 4,
                cursor: 'pointer',
                fontFamily: 'var(--font-mono)',
                fontSize: 11,
              }}
            >
              <span style={{ color: 'var(--teal)' }}>{t.task_id.slice(0, 12)}…</span>
              {' '}
              <span style={{ color: 'var(--text-muted)' }}>{t.status}</span>
              {' '}
              <span>{t.description?.slice(0, 60) || ''}</span>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}
```

**Step 2: Commit**

```bash
git add webapp/src/components/AgentDetailPanel.jsx
git commit -m "feat: add AgentDetailPanel"
```

---

## Task 9: HolonDetailPanel Component

**Files:**
- Create: `webapp/src/components/HolonDetailPanel.jsx`

**Step 1: Create HolonDetailPanel.jsx**

```jsx
function scrub(s) {
  return String(s || '').replace(/did:swarm:[A-Za-z0-9]+/g, m => '[' + m.slice(-6) + ']')
}

const STATUS_BADGE = {
  Forming:      'badge-dim',
  Deliberating: 'badge-amber',
  Voting:       'badge-coral',
  Executing:    'badge-violet',
  Synthesizing: 'badge-violet',
  Done:         'badge-teal',
}

export default function HolonDetailPanel({ holon, holons, onTaskClick, onHolonClick }) {
  const allHolons = holons || []
  const children = allHolons.filter(h => h.parent_holon === holon.task_id)
  const badgeCls = STATUS_BADGE[holon.status] || 'badge-dim'

  return (
    <div>
      {/* Header meta */}
      <div className="detail-meta" style={{ marginBottom: 20 }}>
        <span>Task: <strong style={{ fontFamily: 'var(--font-mono)' }}>{holon.task_id.slice(0, 20)}…</strong></span>
        <span className={`badge ${badgeCls}`}>{holon.status}</span>
        <span className="badge badge-dim">Depth {holon.depth}</span>
      </div>

      {/* Holon info */}
      <div className="detail-section">
        <div className="detail-section-title">Composition</div>
        <div className="detail-meta">
          <span>Chair: <strong style={{ color: 'var(--teal)' }}>{scrub(holon.chair)}</strong></span>
          {holon.adversarial_critic && (
            <span>⚔️ Critic: <strong style={{ color: 'var(--coral)' }}>{scrub(holon.adversarial_critic)}</strong></span>
          )}
          <span>Members: <strong>{holon.members?.length || 0}</strong></span>
          {holon.parent_holon && (
            <span>Parent: <strong style={{ fontFamily: 'var(--font-mono)', fontSize: 10 }}>{holon.parent_holon.slice(0, 16)}…</strong></span>
          )}
        </div>
      </div>

      {/* Member list */}
      {holon.members?.length > 0 && (
        <div className="detail-section">
          <div className="detail-section-title">Members</div>
          <div style={{ display: 'flex', flexWrap: 'wrap', gap: 6 }}>
            {holon.members.map((m, i) => (
              <span key={i} className="member-chip">{scrub(m)}</span>
            ))}
          </div>
        </div>
      )}

      {/* Child holons */}
      {children.length > 0 && (
        <div className="detail-section">
          <div className="detail-section-title">Child Holons ({children.length})</div>
          {children.map(child => (
            <div
              key={child.task_id}
              onClick={() => onHolonClick && onHolonClick(child)}
              style={{
                padding: '6px 10px',
                background: 'var(--surface-2)',
                border: '1px solid var(--border)',
                borderRadius: 5,
                marginBottom: 4,
                cursor: 'pointer',
                display: 'flex',
                gap: 10,
                alignItems: 'center',
                fontSize: 11,
                fontFamily: 'var(--font-mono)',
              }}
            >
              <span className={`badge ${STATUS_BADGE[child.status] || 'badge-dim'}`} style={{ fontSize: 10 }}>{child.status}</span>
              <span style={{ color: 'var(--teal)' }}>{child.task_id.slice(0, 14)}…</span>
              <span style={{ color: 'var(--text-muted)' }}>d{child.depth}</span>
            </div>
          ))}
        </div>
      )}

      {/* Link to task */}
      <div style={{ display: 'flex', gap: 8 }}>
        <button
          className="btn"
          onClick={() => onTaskClick && onTaskClick({ task_id: holon.task_id })}
        >
          View Task Detail →
        </button>
      </div>
    </div>
  )
}
```

**Step 2: Commit**

```bash
git add webapp/src/components/HolonDetailPanel.jsx
git commit -m "feat: add HolonDetailPanel"
```

---

## Task 10: TaskDetailPanel Component

This is the largest component. It merges TaskForensicsPanel + VotingPanel + DeliberationPanel into one panel with tabs.

**Files:**
- Create: `webapp/src/components/TaskDetailPanel.jsx`

**Step 1: Create TaskDetailPanel.jsx**

```jsx
import { DataSet, Network } from 'vis-network/standalone'
import { useEffect, useRef, useState } from 'react'
import { api } from '../api/client'

function scrub(s) {
  return String(s || '').replace(/did:swarm:[A-Za-z0-9]+/g, '[agent]')
}

function scrubId(s) {
  return String(s || '').replace(/did:swarm:[A-Za-z0-9]+/g, m => '[' + m.slice(-6) + ']')
}

// ── Score bar ─────────────────────────────
function ScoreBar({ label, value }) {
  const pct = Math.round((value || 0) * 100)
  const cls = pct > 60 ? 'green' : pct > 30 ? 'yellow' : 'red'
  return (
    <div className="score-bar-row">
      <span className="score-bar-label">{label}</span>
      <div className="score-bar-track">
        <div className={`score-bar-fill ${cls}`} style={{ width: `${pct}%` }} />
      </div>
      <span className="score-bar-value">{pct}%</span>
    </div>
  )
}

// ── IRV rounds ────────────────────────────
function IrvRounds({ rounds }) {
  if (!rounds?.length) return null
  return (
    <div className="detail-section">
      <div className="detail-section-title">IRV Round History</div>
      {rounds.map(r => (
        <div key={r.round_number} className="irv-round">
          <div className="irv-round-header">
            <span>Round {r.round_number}</span>
            {r.eliminated && <span style={{ color: 'var(--coral)' }}>Eliminated: {r.eliminated.slice(0, 8)}…</span>}
          </div>
          <div style={{ display: 'flex', gap: 12, flexWrap: 'wrap' }}>
            {Object.entries(r.tallies || {}).map(([planId, count]) => (
              <span key={planId} style={{ color: r.eliminated === planId ? 'var(--coral)' : 'var(--teal)' }}>
                {planId.slice(0, 8)}…: <strong>{count}</strong>
              </span>
            ))}
          </div>
        </div>
      ))}
    </div>
  )
}

// ── Voting tab ────────────────────────────
function VotingTab({ taskVoting, taskBallots }) {
  const rfp = taskVoting?.rfp?.[0]
  const ballots = taskBallots?.ballots || []
  const irvRounds = taskBallots?.irv_rounds || []
  const planIds = (rfp?.plans || []).map(p => p.plan_id)

  if (!rfp) return <div style={{ color: 'var(--text-muted)', fontSize: 12 }}>No voting data for this task.</div>

  return (
    <div>
      <div className="detail-section">
        <div className="detail-section-title">RFP Status</div>
        <div className="detail-meta">
          <span>Phase: <strong>{rfp.phase}</strong></span>
          <span>Commits: <strong>{rfp.commit_count}/{rfp.expected_proposers || 0}</strong></span>
          <span>Reveals: <strong>{rfp.reveal_count}/{rfp.expected_proposers || 0}</strong></span>
        </div>
      </div>

      {planIds.length > 0 && (
        <div className="detail-section">
          <div className="detail-section-title">Plans</div>
          <table className="data-table">
            <thead>
              <tr>
                <th>Plan ID</th>
                <th>Proposer</th>
                <th>Subtasks</th>
              </tr>
            </thead>
            <tbody>
              {(rfp.plans || []).map(p => (
                <tr key={p.plan_id}>
                  <td>{p.plan_id?.slice(0, 12)}…</td>
                  <td>{scrubId(p.proposer_name || 'unknown')}</td>
                  <td>{p.subtask_count}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {ballots.length > 0 && (
        <div className="detail-section">
          <div className="detail-section-title">Per-Voter Ballots</div>
          {ballots.map((b, i) => (
            <div key={i} style={{ background: 'var(--bg)', border: '1px solid var(--border)', borderRadius: 6, padding: '8px 12px', marginBottom: 8 }}>
              <div style={{ fontFamily: 'var(--font-mono)', fontSize: 11, color: 'var(--teal)', marginBottom: 6 }}>
                {scrubId(b.voter)}
              </div>
              <div style={{ fontSize: 11, color: 'var(--text-muted)', marginBottom: 6, fontFamily: 'var(--font-mono)' }}>
                Rankings: {(b.rankings || []).map(r => r.slice(0, 6)).join(' › ')}
              </div>
              {planIds.slice(0, 3).map(p => (
                b.critic_scores?.[p] ? (
                  <div key={p} style={{ marginBottom: 8 }}>
                    <div style={{ fontSize: 10, color: 'var(--text-muted)', marginBottom: 4, fontFamily: 'var(--font-mono)' }}>
                      Plan {p.slice(0, 8)}…
                    </div>
                    <ScoreBar label="Feasibility"  value={b.critic_scores[p].feasibility} />
                    <ScoreBar label="Parallelism"  value={b.critic_scores[p].parallelism} />
                    <ScoreBar label="Completeness" value={b.critic_scores[p].completeness} />
                    <ScoreBar label="Risk (inv.)"  value={1 - (b.critic_scores[p].risk || 0)} />
                  </div>
                ) : null
              ))}
            </div>
          ))}
        </div>
      )}

      <IrvRounds rounds={irvRounds} />
    </div>
  )
}

// ── Deliberation tab ──────────────────────
const TYPE_COLOR = {
  ProposalSubmission: '#2a7ab0',
  CritiqueFeedback:   '#ffaa00',
  Rebuttal:           '#ff3355',
  SynthesisResult:    '#00e5b0',
}
const TYPE_ICON = {
  ProposalSubmission: '📋',
  CritiqueFeedback:   '🔍',
  Rebuttal:           '↩️',
  SynthesisResult:    '🔗',
}

function DelibMsg({ msg, adversarialId }) {
  const [expanded, setExpanded] = useState(false)
  const color = TYPE_COLOR[msg.message_type] || '#4a7a9b'
  const icon = TYPE_ICON[msg.message_type] || '💬'
  const isAdversarial = adversarialId && msg.speaker === adversarialId

  return (
    <div className="deliberation-msg" style={{ borderLeftColor: color, background: 'var(--surface-2)' }}>
      <div className="deliberation-msg-header">
        <div className="deliberation-msg-meta">
          <span>{icon}</span>
          {isAdversarial && <span title="Adversarial critic">⚔️</span>}
          <span style={{ fontFamily: 'var(--font-mono)', color: 'var(--teal)', fontSize: 11 }}>{scrubId(msg.speaker)}</span>
          <span className="badge" style={{ background: `${color}22`, color, border: `1px solid ${color}44`, fontSize: 10 }}>
            {msg.message_type} R{msg.round}
          </span>
        </div>
        <span className="deliberation-msg-time">{new Date(msg.timestamp).toLocaleTimeString()}</span>
      </div>

      <div
        className={`deliberation-msg-content${!expanded && msg.content.length > 200 ? ' collapsed' : ''}`}
        onClick={() => setExpanded(e => !e)}
      >
        {msg.content}
      </div>
      {msg.content.length > 200 && (
        <button onClick={() => setExpanded(e => !e)} style={{ background: 'none', border: 'none', color, cursor: 'pointer', fontSize: 11, padding: '2px 0' }}>
          {expanded ? '▲ Less' : '▼ More'}
        </button>
      )}

      {msg.critic_scores && Object.keys(msg.critic_scores).length > 0 && (
        <div style={{ marginTop: 8, borderTop: '1px solid var(--border)', paddingTop: 8 }}>
          <div style={{ fontSize: 11, color: 'var(--text-muted)', marginBottom: 6 }}>Critic scores per plan:</div>
          {Object.entries(msg.critic_scores).map(([planId, scores]) => (
            <div key={planId} style={{ marginBottom: 8 }}>
              <div style={{ fontSize: 10, color: 'var(--text-muted)', marginBottom: 4, fontFamily: 'var(--font-mono)' }}>
                Plan {planId.slice(0, 8)}…
              </div>
              <ScoreBar label="Feasibility"  value={scores.feasibility} />
              <ScoreBar label="Parallelism"  value={scores.parallelism} />
              <ScoreBar label="Completeness" value={scores.completeness} />
              <ScoreBar label="Risk (inv.)"  value={1 - (scores.risk || 0)} />
            </div>
          ))}
        </div>
      )}
    </div>
  )
}

function DeliberationTab({ taskId }) {
  const [msgs, setMsgs] = useState([])
  const [holonInfo, setHolonInfo] = useState(null)
  const [loading, setLoading] = useState(false)

  useEffect(() => {
    if (!taskId) return
    setLoading(true)
    Promise.all([
      api.taskDeliberation(taskId),
      api.holonDetail(taskId).catch(() => null),
    ]).then(([d, h]) => {
      setMsgs(d.messages || [])
      setHolonInfo(h)
      setLoading(false)
    }).catch(() => setLoading(false))
  }, [taskId])

  if (loading) return <div style={{ color: 'var(--text-muted)', fontSize: 12 }}>Loading…</div>
  if (!msgs.length) return <div style={{ color: 'var(--text-muted)', fontSize: 12 }}>No deliberation messages yet.</div>

  return (
    <div>
      {holonInfo && (
        <div className="detail-meta" style={{ marginBottom: 16 }}>
          <span>Chair: <strong style={{ color: 'var(--teal)' }}>{scrubId(holonInfo.chair)}</strong></span>
          <span>Members: <strong>{holonInfo.members?.length || 0}</strong></span>
          <span>Depth: <strong>{holonInfo.depth}</strong></span>
          <span>Status: <strong>{holonInfo.status}</strong></span>
          {holonInfo.adversarial_critic && <span>⚔️ Adversarial critic assigned</span>}
        </div>
      )}
      {msgs.map(msg => (
        <DelibMsg key={msg.id} msg={msg} adversarialId={holonInfo?.adversarial_critic} />
      ))}
    </div>
  )
}

// ── Overview tab (forensics) ──────────────
function OverviewTab({ taskTrace, taskVoting }) {
  const dagRef = useRef(null)
  const dagNet = useRef(null)
  const [index, setIndex] = useState(0)
  const [playing, setPlaying] = useState(false)

  const timeline = taskTrace?.timeline || []
  const descendants = taskTrace?.descendants || []
  const rfp = taskVoting?.rfp?.[0]

  useEffect(() => {
    setIndex(timeline.length)
    setPlaying(false)
  }, [taskTrace])

  useEffect(() => {
    if (!playing) return
    const timer = setInterval(() => {
      setIndex(prev => {
        if (prev >= timeline.length) { setPlaying(false); return prev }
        return prev + 1
      })
    }, 700)
    return () => clearInterval(timer)
  }, [playing, timeline.length])

  useEffect(() => {
    if (!dagRef.current) return
    const root = taskTrace?.task
    const nodes = []
    const edges = []
    if (root?.task_id) {
      nodes.push({ id: root.task_id, label: `ROOT\n${(root.description || '').slice(0, 40)}`, color: '#00e5b0', shape: 'box', font: { color: '#020810', size: 10 } })
    }
    descendants.forEach(t => {
      nodes.push({ id: t.task_id, label: `${t.task_id.slice(0,10)}\n${(t.description || '').slice(0, 28)}`, color: '#2a7ab0', shape: 'box', font: { color: '#c8e8ff', size: 10 } })
      if (t.parent_task_id) edges.push({ from: t.parent_task_id, to: t.task_id, color: '#1a4a6a' })
    })
    if (dagNet.current) dagNet.current.destroy()
    dagNet.current = new Network(
      dagRef.current,
      { nodes: new DataSet(nodes), edges: new DataSet(edges) },
      {
        layout: { hierarchical: { enabled: true, direction: 'UD', sortMethod: 'directed' } },
        physics: false,
        edges: { smooth: true },
        nodes: { margin: 8 },
      }
    )
    return () => { if (dagNet.current) dagNet.current.destroy() }
  }, [taskTrace, descendants])

  const replayed = timeline.slice(0, Math.max(0, Math.min(index, timeline.length)))

  return (
    <div>
      {/* Task meta */}
      <div className="detail-meta" style={{ marginBottom: 16 }}>
        <span>Status: <strong>{taskTrace?.task?.status || '—'}</strong></span>
        <span>Tier: <strong>{taskTrace?.task?.tier_level ?? '—'}</strong></span>
        <span>Assigned: <strong>{scrub(taskTrace?.task?.assigned_to_name || 'unassigned')}</strong></span>
        <span>Subtasks: <strong>{(taskTrace?.task?.subtasks || []).length}</strong></span>
      </div>

      {taskTrace?.task?.description && (
        <div className="detail-section">
          <div className="detail-section-title">Description</div>
          <div style={{ fontSize: 13, color: 'var(--text)', lineHeight: 1.5, padding: '8px 12px', background: 'var(--surface-2)', borderRadius: 6, border: '1px solid var(--border)' }}>
            {taskTrace.task.description}
          </div>
        </div>
      )}

      {/* Timeline replay */}
      {timeline.length > 0 && (
        <div className="detail-section">
          <div className="detail-section-title">Timeline Replay</div>
          <div className="timeline-controls">
            <button className="btn" style={{ fontSize: 11 }} onClick={() => setPlaying(p => !p)}>
              {playing ? '⏸' : '▶'}
            </button>
            <button className="btn" style={{ fontSize: 11 }} onClick={() => { setPlaying(false); setIndex(0) }}>⏮</button>
            <input
              type="range" className="timeline-slider"
              min="0" max={Math.max(0, timeline.length)}
              value={Math.min(index, timeline.length)}
              onChange={e => { setPlaying(false); setIndex(Number(e.target.value)) }}
            />
            <span style={{ fontSize: 11, fontFamily: 'var(--font-mono)', color: 'var(--text-muted)', flexShrink: 0 }}>
              {Math.min(index, timeline.length)}/{timeline.length}
            </span>
          </div>
          <div className="log-box">
            {replayed.map((e, i) => (
              <div key={`${e.timestamp}-${i}`}>
                <span style={{ color: 'var(--text-muted)' }}>[{e.timestamp}]</span>{' '}
                <span style={{ color: 'var(--teal)' }}>{e.stage}</span>{' '}
                {scrub(e.detail)}
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Subtask table */}
      {descendants.length > 0 && (
        <div className="detail-section">
          <div className="detail-section-title">Subtasks</div>
          <table className="data-table">
            <thead>
              <tr>
                <th>ID</th><th>Status</th><th>Assignee</th><th>Result</th>
              </tr>
            </thead>
            <tbody>
              {descendants.map(t => (
                <tr key={t.task_id}>
                  <td>{t.task_id.slice(0, 10)}…</td>
                  <td>{t.status}</td>
                  <td>{scrub(t.assigned_to_name || 'unassigned')}</td>
                  <td>{t.result_text || (t.has_result ? 'captured' : '—')}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {/* Task DAG */}
      {(descendants.length > 0 || taskTrace?.task) && (
        <div className="detail-section">
          <div className="detail-section-title">Task DAG</div>
          <div className="dag-container" ref={dagRef} />
        </div>
      )}

      {/* Related messages */}
      {(taskTrace?.messages || []).length > 0 && (
        <div className="detail-section">
          <div className="detail-section-title">Propagation Messages</div>
          <div className="log-box">
            {taskTrace.messages.map((m, i) => (
              <div key={i}>
                <span style={{ color: 'var(--text-muted)' }}>[{m.timestamp}]</span>{' '}
                <span style={{ color: 'var(--teal)' }}>{m.topic}</span>{' '}
                {m.method || ''}{' '}{scrub(m.outcome || '')}
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Result artifact */}
      {taskTrace?.result_artifact && (
        <div className="detail-section">
          <div className="detail-section-title">Result Artifact</div>
          <div className="log-box">
            <div>artifact_id: {taskTrace.result_artifact.artifact_id || '—'}</div>
            <div>content_type: {taskTrace.result_artifact.content_type || '—'}</div>
            <div>size_bytes: {taskTrace.result_artifact.size_bytes ?? '—'}</div>
            <div>created_at: {taskTrace.result_artifact.created_at || '—'}</div>
            {taskTrace.result_text && <div style={{ marginTop: 8, color: 'var(--text)' }}>result: {taskTrace.result_text}</div>}
          </div>
        </div>
      )}
    </div>
  )
}

// ── Main export ───────────────────────────
const TABS = [
  { id: 'overview',     label: 'Overview' },
  { id: 'voting',       label: 'Voting' },
  { id: 'deliberation', label: 'Deliberation' },
]

export default function TaskDetailPanel({ taskId, taskTrace, taskVoting, taskBallots, onTabChange, activeTab: propTab }) {
  const [activeTab, setActiveTab] = useState(propTab || 'overview')

  const handleTabChange = (id) => {
    setActiveTab(id)
    onTabChange && onTabChange(id)
  }

  return (
    <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
      <div style={{ padding: '0 0 12px' }}>
        <div style={{ fontFamily: 'var(--font-mono)', fontSize: 11, color: 'var(--text-muted)', marginBottom: 4 }}>Task ID</div>
        <div style={{ fontFamily: 'var(--font-mono)', fontSize: 13, color: 'var(--teal)' }}>{taskId}</div>
      </div>

      {/* Inline tabs (rendered by parent SlidePanel via tabs prop, but we handle content here) */}
      <div style={{ display: 'flex', gap: 2, borderBottom: '1px solid var(--border)', marginBottom: 16, marginLeft: -20, marginRight: -20, paddingLeft: 20 }}>
        {TABS.map(t => (
          <button
            key={t.id}
            className={`panel-tab${activeTab === t.id ? ' active' : ''}`}
            onClick={() => handleTabChange(t.id)}
          >
            {t.label}
          </button>
        ))}
      </div>

      {activeTab === 'overview' && (
        <OverviewTab taskTrace={taskTrace} taskVoting={taskVoting} />
      )}
      {activeTab === 'voting' && (
        <VotingTab taskVoting={taskVoting} taskBallots={taskBallots} />
      )}
      {activeTab === 'deliberation' && (
        <DeliberationTab taskId={taskId} />
      )}
    </div>
  )
}
```

**Step 2: Commit**

```bash
git add webapp/src/components/TaskDetailPanel.jsx
git commit -m "feat: add TaskDetailPanel with overview/voting/deliberation tabs"
```

---

## Task 11: App.jsx Refactor

This is the final wiring task. Keep all state/polling/WebSocket/submitTask/loadTrace logic. Replace the layout.

**Files:**
- Rewrite: `webapp/src/App.jsx`

**Step 1: Rewrite App.jsx**

```jsx
import { useCallback, useEffect, useState } from 'react'
import { api } from './api/client'
import { usePolling } from './hooks/usePolling'
import Header from './components/Header'
import LiveGraph from './components/LiveGraph'
import BottomTray from './components/BottomTray'
import SlidePanel from './components/SlidePanel'
import TaskDetailPanel from './components/TaskDetailPanel'
import AgentDetailPanel from './components/AgentDetailPanel'
import HolonDetailPanel from './components/HolonDetailPanel'
import AuditPanel from './components/AuditPanel'
import MessagesPanel from './components/MessagesPanel'
import SubmitTaskModal from './components/SubmitTaskModal'

export default function App() {
  // ── Data state ─────────────────────────
  const [hierarchy, setHierarchy]       = useState({ nodes: [] })
  const [voting, setVoting]             = useState({ voting: [], rfp: [] })
  const [messages, setMessages]         = useState([])
  const [tasks, setTasks]               = useState({ tasks: [] })
  const [agents, setAgents]             = useState({ agents: [] })
  const [flow, setFlow]                 = useState({ counters: {} })
  const [topology, setTopology]         = useState({ nodes: [], edges: [] })
  const [audit, setAudit]               = useState({ events: [] })
  const [auth, setAuth]                 = useState({ token_required: false })
  const [holons, setHolons]             = useState([])
  const [live, setLive]                 = useState({ active_tasks: 0, known_agents: 0, messages: [], events: [] })

  // ── Task detail state ──────────────────
  const [taskId, setTaskId]             = useState('')
  const [taskTrace, setTaskTrace]       = useState({ timeline: [], descendants: [], messages: [] })
  const [taskVoting, setTaskVoting]     = useState({ voting: [], rfp: [] })
  const [taskBallots, setTaskBallots]   = useState({ ballots: [], irv_rounds: [] })

  // ── UI state ───────────────────────────
  const [panel, setPanel]               = useState(null) // { type, data }
  const [showSubmit, setShowSubmit]     = useState(false)
  const [description, setDescription]   = useState('')
  const [operatorToken, setOperatorToken] = useState(localStorage.getItem('openswarm.web.token') || '')
  const [submitError, setSubmitError]   = useState('')

  // ── Polling ────────────────────────────
  const refresh = useCallback(async () => {
    const [h, v, m, t, ag, f, tp, a, au, hl] = await Promise.all([
      api.hierarchy(),
      api.voting(),
      api.messages(),
      api.tasks(),
      api.agents(),
      api.flow(),
      api.topology(),
      api.audit(),
      api.authStatus(),
      api.holons().catch(() => []),
    ])
    setHierarchy(h)
    setVoting(v)
    setMessages(m)
    setTasks(t)
    setAgents(ag)
    setFlow(f)
    setTopology(tp)
    setAudit(a)
    setAuth(au)
    setHolons(hl)
  }, [])

  usePolling(refresh, 5000)

  // ── WebSocket ──────────────────────────
  useEffect(() => {
    const proto = location.protocol === 'https:' ? 'wss' : 'ws'
    const ws = new WebSocket(`${proto}://${location.host}/api/stream`)
    ws.onmessage = (event) => {
      try {
        const payload = JSON.parse(event.data)
        if (payload.type === 'snapshot') setLive(payload)
      } catch (_) {}
    }
    return () => ws.close()
  }, [])

  // ── Task submission ────────────────────
  const submitTask = async () => {
    if (!description.trim()) return
    localStorage.setItem('openswarm.web.token', operatorToken || '')
    try {
      const res = await api.submitTask(description, operatorToken)
      setSubmitError('')
      setDescription('')
      setShowSubmit(false)
      if (res.task_id) loadTrace(res.task_id)
      await refresh()
    } catch (err) {
      setSubmitError(err.payload?.error || err.message)
    }
  }

  // ── Task trace loading ─────────────────
  const loadTrace = useCallback(async (requestedTaskId) => {
    const effectiveTaskId = (requestedTaskId || taskId || '').trim()
    if (!effectiveTaskId) return
    setTaskId(effectiveTaskId)
    const [trace, votingDetail, ballots] = await Promise.all([
      api.taskTimeline(effectiveTaskId),
      api.votingTask(effectiveTaskId),
      api.taskBallots(effectiveTaskId).catch(() => ({ ballots: [], irv_rounds: [] })),
    ])
    setTaskTrace(trace)
    setTaskVoting(votingDetail)
    setTaskBallots(ballots)
  }, [taskId])

  // ── Panel open helpers ─────────────────
  const openTaskPanel = (task) => {
    loadTrace(task.task_id)
    setPanel({ type: 'task', data: { taskId: task.task_id } })
  }

  const openAgentPanel = (agent) => {
    setPanel({ type: 'agent', data: { agent } })
  }

  const openHolonPanel = (holon) => {
    setPanel({ type: 'holon', data: { holon } })
  }

  const handleGraphNodeClick = ({ type, data }) => {
    if (type === 'agent')  openAgentPanel(data.agent)
    if (type === 'holon')  openHolonPanel(data)
  }

  const closePanel = () => setPanel(null)

  // ── Render ─────────────────────────────
  return (
    <div className="app">
      <Header
        agents={agents}
        tasks={tasks}
        live={live}
        onSubmitClick={() => setShowSubmit(true)}
        onAuditClick={() => setPanel({ type: 'audit', data: {} })}
        onMessagesClick={() => setPanel({ type: 'messages', data: {} })}
      />

      <LiveGraph
        topology={topology}
        holons={holons}
        agents={agents}
        onNodeClick={handleGraphNodeClick}
      />

      <BottomTray
        agents={agents}
        tasks={tasks}
        onTaskClick={openTaskPanel}
        onAgentClick={openAgentPanel}
      />

      {/* Slide-in panels */}
      {panel?.type === 'task' && (
        <SlidePanel title={`Task: ${panel.data.taskId.slice(0, 16)}…`} onClose={closePanel}>
          <TaskDetailPanel
            taskId={panel.data.taskId}
            taskTrace={taskTrace}
            taskVoting={taskVoting}
            taskBallots={taskBallots}
          />
        </SlidePanel>
      )}

      {panel?.type === 'agent' && (
        <SlidePanel title={`Agent: ${(panel.data.agent.name || panel.data.agent.agent_id || '').slice(0, 20)}`} onClose={closePanel}>
          <AgentDetailPanel
            agent={panel.data.agent}
            tasks={tasks}
            onTaskClick={openTaskPanel}
          />
        </SlidePanel>
      )}

      {panel?.type === 'holon' && (
        <SlidePanel title={`Holon: ${panel.data.holon.task_id.slice(0, 16)}…`} onClose={closePanel}>
          <HolonDetailPanel
            holon={panel.data.holon}
            holons={holons}
            onTaskClick={openTaskPanel}
            onHolonClick={openHolonPanel}
          />
        </SlidePanel>
      )}

      {panel?.type === 'audit' && (
        <SlidePanel title="Audit Log" onClose={closePanel}>
          <AuditPanel audit={audit} />
        </SlidePanel>
      )}

      {panel?.type === 'messages' && (
        <SlidePanel title="P2P Messages" onClose={closePanel}>
          <MessagesPanel messages={messages} />
        </SlidePanel>
      )}

      {/* Submit task modal */}
      {showSubmit && (
        <SubmitTaskModal
          description={description}
          setDescription={setDescription}
          operatorToken={operatorToken}
          setOperatorToken={setOperatorToken}
          auth={auth}
          onSubmit={submitTask}
          onClose={() => { setShowSubmit(false); setSubmitError('') }}
          submitError={submitError}
        />
      )}
    </div>
  )
}
```

**Step 2: Verify build**

Run: `cd webapp && npm run dev`
Open http://localhost:5173 — you should see the new layout.
Expected: Header bar, graph area (may be empty if no agents), bottom tray, dark space theme.

Fix any import errors before committing.

**Step 3: Commit**

```bash
git add webapp/src/App.jsx
git commit -m "feat: wire up Living Network Console layout in App.jsx"
```

---

## Task 12: Delete Old Components

**Files to delete:**
- `webapp/src/components/Sidebar.jsx`
- `webapp/src/components/OverviewPanel.jsx`
- `webapp/src/components/HierarchyTree.jsx`
- `webapp/src/components/VotingPanel.jsx`
- `webapp/src/components/TaskForensicsPanel.jsx`
- `webapp/src/components/TopologyPanel.jsx`
- `webapp/src/components/IdeasPanel.jsx`
- `webapp/src/components/HolonTreePanel.jsx`
- `webapp/src/components/DeliberationPanel.jsx`

**Step 1: Delete old files**

```bash
cd webapp/src/components
rm Sidebar.jsx OverviewPanel.jsx HierarchyTree.jsx VotingPanel.jsx TaskForensicsPanel.jsx TopologyPanel.jsx IdeasPanel.jsx HolonTreePanel.jsx DeliberationPanel.jsx
```

**Step 2: Verify build is clean**

Run: `cd webapp && npm run build`
Expected: Clean build, no errors or unused import warnings.

If there are errors, track them down to missing imports or references and fix them.

**Step 3: Commit**

```bash
git add -A
git commit -m "chore: delete old tab-based components (replaced by slide-in panels)"
```

---

## Task 13: Visual Polish Pass

After the app is working, do a visual polish pass:

**Step 1: Add grain texture overlay to graph area**

In `styles.css`, add after `.graph-area`:

```css
.graph-area::before {
  content: '';
  position: absolute;
  inset: 0;
  background-image: url("data:image/svg+xml,%3Csvg viewBox='0 0 256 256' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noise'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='4' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noise)' opacity='0.03'/%3E%3C/svg%3E");
  pointer-events: none;
  z-index: 1;
  opacity: 0.4;
}

.graph-container { z-index: 2; }
.graph-controls { z-index: 10; }
```

**Step 2: Add radial glow behind graph**

In `styles.css`, update `.graph-area`:

```css
.graph-area {
  flex: 1;
  position: relative;
  overflow: hidden;
  background:
    radial-gradient(ellipse 60% 40% at 50% 30%, rgba(0, 229, 176, 0.04) 0%, transparent 70%),
    radial-gradient(ellipse 40% 30% at 70% 60%, rgba(124, 58, 255, 0.04) 0%, transparent 70%),
    var(--bg);
}
```

**Step 3: Final build and verify**

```bash
cd webapp && npm run build
```

Expected: Clean build.

**Step 4: Commit**

```bash
git add webapp/src/styles.css
git commit -m "feat: add graph background glow and grain texture"
```

---

## Build & Run Reference

```bash
# Dev server (hot reload)
cd /Users/aostapenko/Work/OpenSwarm/webapp
npm run dev
# → http://localhost:5173

# Production build
npm run build

# The Rust connector serves the built webapp
# Rebuild webapp then restart connector:
npm run build
cd ..
~/.cargo/bin/cargo run --bin openswarm-connector
```

---

## Checklist

- [ ] Task 1: styles.css design system
- [ ] Task 2: SlidePanel component
- [ ] Task 3: Header component
- [ ] Task 4: BottomTray component
- [ ] Task 5: LiveGraph component
- [ ] Task 6: SubmitTaskModal component
- [ ] Task 7: AuditPanel + MessagesPanel refactored
- [ ] Task 8: AgentDetailPanel
- [ ] Task 9: HolonDetailPanel
- [ ] Task 10: TaskDetailPanel
- [ ] Task 11: App.jsx refactor
- [ ] Task 12: Delete old components
- [ ] Task 13: Visual polish pass
