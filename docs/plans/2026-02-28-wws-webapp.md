# WWS Webapp Redesign Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the current ASIP Living Network Console with the WWS (World Wide Swarm) three-zone Command Center.

**Architecture:** Three-column layout (Left: My Agent 280px fixed | Center: Graph/Directory/Activity flex | Right: Live Stream 280px fixed). Complete replacement of all existing components. New API endpoints for identity, reputation, names, keys, network, and directory. Slide-in panels for detail views.

**Tech Stack:** React 18 + Vite, vis-network/standalone, Google Fonts (Syne + JetBrains Mono), CSS custom properties.

**Design doc:** `docs/plans/2026-02-28-wws-webapp-design.md`

---

## Task 1: Design System — styles.css

**Files:**
- Modify: `webapp/src/styles.css`

**Step 1: Replace the entire file**

Replace all content with the new WWS design system:

```css
/* ===== WWS Design System ===== */
@import url('https://fonts.googleapis.com/css2?family=Syne:wght@400;600;700;800&family=JetBrains+Mono:wght@400;500&display=swap');

*, *::before, *::after { box-sizing: border-box; margin: 0; padding: 0; }

:root {
  --bg:           #020810;
  --surface:      #060f1e;
  --surface-2:    #0a1628;
  --border:       #0d2035;
  --border-2:     #142840;
  --teal:         #00e5b0;
  --teal-dim:     #00a880;
  --violet:       #7c3aff;
  --amber:        #ffaa00;
  --coral:        #ff3355;
  --dim-blue:     #1a3a5c;
  --text:         #c8e8ff;
  --text-muted:   #4a7a9b;
  --text-dim:     #2a5a7c;

  --tier-newcomer:    #4a7a9b;
  --tier-member:      #2a7ab0;
  --tier-trusted:     #00e5b0;
  --tier-established: #a78bfa;
  --tier-veteran:     #ffaa00;
  --tier-suspended:   #ff3355;

  --col-left:  280px;
  --col-right: 280px;
  --header-h:  88px;
}

html, body { height: 100%; overflow: hidden; }

body {
  font-family: 'Syne', sans-serif;
  background: var(--bg);
  color: var(--text);
  font-size: 13px;
  line-height: 1.5;
}

/* ===== APP SHELL ===== */
#root { height: 100vh; display: flex; flex-direction: column; }

.app-header {
  height: var(--header-h);
  min-height: var(--header-h);
  background: var(--surface);
  border-bottom: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  padding: 0 16px;
  flex-shrink: 0;
  z-index: 10;
}

.header-row1 {
  display: flex;
  align-items: center;
  gap: 16px;
  height: 44px;
  border-bottom: 1px solid var(--border);
}

.header-row2 {
  display: flex;
  align-items: center;
  gap: 0;
  height: 44px;
}

.app-body {
  flex: 1;
  display: flex;
  overflow: hidden;
}

/* ===== HEADER ELEMENTS ===== */
.brand {
  font-size: 18px;
  font-weight: 800;
  color: var(--teal);
  letter-spacing: 0.08em;
  margin-right: 4px;
}

.header-identity {
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
  cursor: pointer;
  padding: 3px 8px;
  border-radius: 4px;
  border: 1px solid transparent;
  transition: border-color 0.15s, background 0.15s;
}
.header-identity:hover { border-color: var(--border-2); background: var(--surface-2); }

.header-stats {
  display: flex;
  gap: 16px;
  margin-left: 8px;
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  color: var(--text-muted);
}

.header-spacer { flex: 1; }

.btn {
  padding: 4px 12px;
  border: 1px solid var(--border-2);
  background: var(--surface-2);
  color: var(--text-muted);
  border-radius: 4px;
  cursor: pointer;
  font-family: 'Syne', sans-serif;
  font-size: 12px;
  font-weight: 600;
  transition: border-color 0.15s, color 0.15s, background 0.15s;
}
.btn:hover { border-color: var(--teal); color: var(--teal); }
.btn-primary { border-color: var(--teal); color: var(--teal); }
.btn-primary:hover { background: color-mix(in srgb, var(--teal) 15%, transparent); }
.btn-danger { border-color: var(--coral); color: var(--coral); }
.btn-danger:hover { background: color-mix(in srgb, var(--coral) 15%, transparent); }

.view-tabs {
  display: flex;
  gap: 0;
  height: 100%;
  align-items: stretch;
}

.view-tab {
  padding: 0 20px;
  border: none;
  border-bottom: 2px solid transparent;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  font-family: 'Syne', sans-serif;
  font-size: 13px;
  font-weight: 600;
  transition: color 0.15s, border-color 0.15s;
  display: flex;
  align-items: center;
}
.view-tab:hover { color: var(--text); }
.view-tab.active { color: var(--teal); border-bottom-color: var(--teal); }

/* ===== COLUMNS ===== */
.col-left {
  width: var(--col-left);
  min-width: var(--col-left);
  background: var(--surface);
  border-right: 1px solid var(--border);
  overflow-y: auto;
  overflow-x: hidden;
  display: flex;
  flex-direction: column;
}

.col-center { flex: 1; min-width: 0; position: relative; }

.col-right {
  width: var(--col-right);
  min-width: var(--col-right);
  background: var(--surface);
  border-left: 1px solid var(--border);
  overflow-y: auto;
  overflow-x: hidden;
  display: flex;
  flex-direction: column;
}

/* ===== LEFT COLUMN SECTIONS ===== */
.section {
  border-bottom: 1px solid var(--border);
  padding: 12px 14px;
}

.section-header {
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.12em;
  color: var(--text-dim);
  text-transform: uppercase;
  margin-bottom: 10px;
}

/* Identity card */
.identity-name {
  font-size: 16px;
  font-weight: 700;
  color: var(--teal);
  margin-bottom: 4px;
  word-break: break-all;
}

.tier-badge {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 2px 8px;
  border-radius: 3px;
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  border: 1px solid currentColor;
  margin-bottom: 8px;
}
.tier-badge.newcomer    { color: var(--tier-newcomer); }
.tier-badge.member      { color: var(--tier-member); }
.tier-badge.trusted     { color: var(--tier-trusted); }
.tier-badge.established { color: var(--tier-established); }
.tier-badge.veteran     { color: var(--tier-veteran); }
.tier-badge.suspended   { color: var(--tier-suspended); }

.rep-score {
  font-family: 'JetBrains Mono', monospace;
  font-size: 13px;
  color: var(--text);
  margin-bottom: 4px;
}
.rep-next { font-size: 11px; color: var(--text-muted); margin-bottom: 6px; }

.rep-bar-track {
  height: 4px;
  background: var(--border);
  border-radius: 2px;
  overflow: hidden;
  margin-bottom: 8px;
}
.rep-bar-fill {
  height: 100%;
  background: var(--teal);
  border-radius: 2px;
  transition: width 0.6s ease;
}

.id-field {
  display: flex;
  align-items: center;
  gap: 6px;
  font-family: 'JetBrains Mono', monospace;
  font-size: 10px;
  color: var(--text-muted);
  margin-bottom: 3px;
}
.id-label { color: var(--text-dim); min-width: 42px; }
.id-value { cursor: pointer; }
.id-value:hover { color: var(--text); }

/* Names list */
.name-row {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 0;
  border-bottom: 1px solid var(--border);
}
.name-row:last-of-type { border-bottom: none; }
.name-label {
  font-family: 'JetBrains Mono', monospace;
  font-size: 12px;
  color: var(--teal);
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.name-ttl {
  font-family: 'JetBrains Mono', monospace;
  font-size: 10px;
  color: var(--text-muted);
  white-space: nowrap;
}
.name-ttl.warning { color: var(--coral); }
.name-renew {
  background: none;
  border: none;
  color: var(--teal-dim);
  cursor: pointer;
  font-size: 13px;
  padding: 0 2px;
  line-height: 1;
}
.name-renew:hover { color: var(--teal); }

.add-name-btn {
  margin-top: 8px;
  width: 100%;
  text-align: left;
  background: none;
  border: 1px dashed var(--border-2);
  color: var(--text-muted);
  border-radius: 4px;
  padding: 5px 8px;
  cursor: pointer;
  font-family: 'Syne', sans-serif;
  font-size: 11px;
  font-weight: 600;
  transition: border-color 0.15s, color 0.15s;
}
.add-name-btn:hover { border-color: var(--teal); color: var(--teal); }

/* Status indicators */
.status-row {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 3px 0;
  font-size: 12px;
  cursor: pointer;
}
.status-row:hover .status-label { color: var(--text); }
.status-dot {
  width: 7px; height: 7px;
  border-radius: 50%;
  flex-shrink: 0;
}
.status-dot.ok      { background: var(--teal); }
.status-dot.warn    { background: var(--amber); }
.status-dot.error   { background: var(--coral); }
.status-dot.off     { background: var(--dim-blue); }
.status-label { color: var(--text-muted); flex: 1; }
.status-value {
  font-family: 'JetBrains Mono', monospace;
  font-size: 10px;
  color: var(--text-dim);
}

.quick-link {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 4px 0;
  font-size: 12px;
  color: var(--text-muted);
  cursor: pointer;
  border-radius: 3px;
}
.quick-link:hover { color: var(--teal); }
.quick-link-icon { font-size: 13px; }

/* ===== CENTER — GRAPH ===== */
.graph-container {
  width: 100%; height: 100%;
  position: absolute; inset: 0;
  background: var(--bg);
}

.graph-controls {
  position: absolute;
  top: 12px; left: 12px;
  display: flex;
  gap: 6px;
  z-index: 5;
}

/* ===== CENTER — DIRECTORY ===== */
.directory-container {
  position: absolute; inset: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.directory-toolbar {
  padding: 12px 16px;
  border-bottom: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  gap: 8px;
  flex-shrink: 0;
}

.search-input {
  width: 100%;
  background: var(--surface-2);
  border: 1px solid var(--border-2);
  border-radius: 5px;
  color: var(--text);
  font-family: 'Syne', sans-serif;
  font-size: 13px;
  padding: 7px 12px;
  outline: none;
  transition: border-color 0.15s;
}
.search-input:focus { border-color: var(--teal); }
.search-input::placeholder { color: var(--text-dim); }

.filter-chips {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}

.filter-chip {
  padding: 2px 10px;
  border-radius: 12px;
  border: 1px solid var(--border-2);
  background: none;
  color: var(--text-muted);
  font-family: 'Syne', sans-serif;
  font-size: 11px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s;
}
.filter-chip:hover { border-color: var(--teal); color: var(--teal); }
.filter-chip.active { background: color-mix(in srgb, var(--teal) 15%, transparent); border-color: var(--teal); color: var(--teal); }

.directory-list {
  flex: 1;
  overflow-y: auto;
  padding: 8px 0;
}

.agent-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 16px;
  cursor: pointer;
  border-bottom: 1px solid var(--border);
  transition: background 0.1s;
}
.agent-row:hover { background: var(--surface-2); }
.agent-row-name {
  font-weight: 600;
  font-size: 13px;
  color: var(--teal);
  min-width: 0;
}
.agent-row-did {
  font-family: 'JetBrains Mono', monospace;
  font-size: 10px;
  color: var(--text-dim);
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex: 1;
}
.agent-row-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}
.agent-row-score {
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  color: var(--text-muted);
}
.agent-row-seen {
  font-size: 10px;
  color: var(--text-dim);
}

/* ===== CENTER — ACTIVITY ===== */
.activity-container {
  position: absolute; inset: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.activity-tabs {
  display: flex;
  border-bottom: 1px solid var(--border);
  padding: 0 16px;
  flex-shrink: 0;
}

.activity-tab {
  padding: 10px 14px;
  border: none;
  border-bottom: 2px solid transparent;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  font-family: 'Syne', sans-serif;
  font-size: 12px;
  font-weight: 600;
  transition: color 0.15s, border-color 0.15s;
}
.activity-tab:hover { color: var(--text); }
.activity-tab.active { color: var(--teal); border-bottom-color: var(--teal); }

.activity-content {
  flex: 1;
  overflow-y: auto;
  padding: 0;
}

/* Task / Holon rows reuse .agent-row styles */
.task-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 16px;
  cursor: pointer;
  border-bottom: 1px solid var(--border);
  transition: background 0.1s;
}
.task-row:hover { background: var(--surface-2); }
.task-status-dot {
  width: 7px; height: 7px;
  border-radius: 50%;
  flex-shrink: 0;
}
.task-row-id {
  font-family: 'JetBrains Mono', monospace;
  font-size: 11px;
  color: var(--text-muted);
  white-space: nowrap;
  flex-shrink: 0;
}
.task-row-desc {
  flex: 1;
  font-size: 12px;
  color: var(--text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.task-row-status {
  font-size: 10px;
  color: var(--text-dim);
  flex-shrink: 0;
}

/* Message threads */
.msg-thread {
  border-bottom: 1px solid var(--border);
  padding: 10px 16px;
  cursor: pointer;
  transition: background 0.1s;
}
.msg-thread:hover { background: var(--surface-2); }
.msg-thread-peer { font-size: 12px; font-weight: 600; color: var(--teal); margin-bottom: 3px; }
.msg-thread-preview { font-size: 11px; color: var(--text-muted); }
.msg-thread-meta { font-family: 'JetBrains Mono', monospace; font-size: 10px; color: var(--text-dim); margin-top: 2px; }

/* ===== RIGHT — LIVE STREAM ===== */
.stream-header {
  padding: 10px 14px;
  border-bottom: 1px solid var(--border);
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.12em;
  color: var(--text-dim);
  text-transform: uppercase;
  flex-shrink: 0;
}

.stream-list {
  flex: 1;
  overflow-y: auto;
  padding: 6px 0;
}

.stream-event {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 6px 14px;
  cursor: pointer;
  transition: background 0.1s;
  border-radius: 3px;
  margin: 1px 4px;
}
.stream-event:hover { background: var(--surface-2); }
.stream-icon { font-size: 13px; flex-shrink: 0; line-height: 1.4; }
.stream-body { flex: 1; min-width: 0; }
.stream-text { font-size: 11px; color: var(--text); line-height: 1.4; }
.stream-time { font-family: 'JetBrains Mono', monospace; font-size: 9px; color: var(--text-dim); margin-top: 1px; }

/* ===== SLIDE-IN PANELS ===== */
.panel-overlay {
  position: fixed; inset: 0;
  background: rgba(2,8,16,0.6);
  z-index: 100;
  opacity: 0;
  pointer-events: none;
  transition: opacity 0.25s;
}
.panel-overlay.open { opacity: 1; pointer-events: all; }

.slide-panel {
  position: fixed;
  top: 0; right: 0; bottom: 0;
  width: 640px;
  background: var(--surface);
  border-left: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  transform: translateX(100%);
  transition: transform 0.25s cubic-bezier(0.4,0,0.2,1);
  z-index: 101;
  overflow: hidden;
}
.slide-panel.open { transform: translateX(0); }

.panel-header {
  padding: 16px 20px;
  border-bottom: 1px solid var(--border);
  display: flex;
  align-items: center;
  gap: 12px;
  flex-shrink: 0;
}
.panel-title { font-size: 15px; font-weight: 700; color: var(--text); flex: 1; }
.panel-close {
  background: none; border: none; color: var(--text-muted);
  cursor: pointer; font-size: 18px; line-height: 1; padding: 2px;
}
.panel-close:hover { color: var(--text); }

.panel-body { flex: 1; overflow-y: auto; padding: 16px 20px; }

/* ===== MODALS (preserved) ===== */
.modal-overlay {
  position: fixed; inset: 0;
  background: rgba(2,8,16,0.8);
  display: flex; align-items: center; justify-content: center;
  z-index: 200;
}
.modal {
  background: var(--surface);
  border: 1px solid var(--border-2);
  border-radius: 8px;
  padding: 24px;
  min-width: 480px;
  max-width: 640px;
  width: 90vw;
}
.modal-title { font-size: 16px; font-weight: 700; margin-bottom: 16px; color: var(--text); }

/* ===== FORM ELEMENTS ===== */
input, textarea, select {
  background: var(--surface-2);
  border: 1px solid var(--border-2);
  border-radius: 5px;
  color: var(--text);
  font-family: 'Syne', sans-serif;
  font-size: 13px;
  padding: 7px 12px;
  outline: none;
  width: 100%;
  transition: border-color 0.15s;
}
input:focus, textarea:focus, select:focus { border-color: var(--teal); }
input::placeholder { color: var(--text-dim); }

.form-row { margin-bottom: 12px; }
.form-label { display: block; font-size: 11px; font-weight: 600; color: var(--text-muted); margin-bottom: 4px; }

/* ===== SCROLLBARS ===== */
::-webkit-scrollbar { width: 4px; height: 4px; }
::-webkit-scrollbar-track { background: transparent; }
::-webkit-scrollbar-thumb { background: var(--border-2); border-radius: 2px; }
::-webkit-scrollbar-thumb:hover { background: var(--dim-blue); }

/* ===== UTILITY ===== */
.mono { font-family: 'JetBrains Mono', monospace; }
.teal  { color: var(--teal); }
.muted { color: var(--text-muted); }
.dim   { color: var(--text-dim); }
.flex-row { display: flex; align-items: center; gap: 8px; }
.spacer { flex: 1; }
.mt-4  { margin-top: 4px; }
.mt-8  { margin-top: 8px; }
.mt-12 { margin-top: 12px; }
.w-full { width: 100%; }
```

**Step 2: Verify the dev server still starts**

```bash
cd webapp && npm run dev -- --port 5173
```

Expected: no errors, dev server starts.

**Step 3: Commit**

```bash
git add webapp/src/styles.css
git commit -m "feat(wws): new WWS design system — three-zone layout tokens + component styles"
```

---

## Task 2: API Client — new WWS endpoints

**Files:**
- Modify: `webapp/src/api/client.js`

**Step 1: Read the current file first, then replace with:**

```javascript
const BASE = ''

export async function fetchJson(url, options = {}) {
  const res = await fetch(BASE + url, options)
  const ct = res.headers.get('content-type') || ''
  if (!ct.includes('application/json')) {
    const err = new Error(`HTTP ${res.status}`)
    err.status = res.status
    throw err
  }
  const data = await res.json()
  if (!res.ok) {
    const err = new Error(data.error || `HTTP ${res.status}`)
    err.status = res.status
    throw err
  }
  return data
}

// ── Identity & Reputation ─────────────────────────────────────
export const getIdentity       = () => fetchJson('/api/identity')
export const getReputation     = () => fetchJson('/api/reputation')
export const getRepEvents      = (limit=50, offset=0) =>
  fetchJson(`/api/reputation/events?limit=${limit}&offset=${offset}`)

// ── Names ─────────────────────────────────────────────────────
export const getMyNames        = () => fetchJson('/api/names')
export const registerName      = (name) => fetchJson('/api/names', {
  method: 'POST',
  headers: {'Content-Type':'application/json'},
  body: JSON.stringify({name})
})
export const renewName         = (name) => fetchJson(`/api/names/${encodeURIComponent(name)}/renew`, { method: 'PUT' })
export const releaseName       = (name) => fetchJson(`/api/names/${encodeURIComponent(name)}`, { method: 'DELETE' })

// ── Network & Peers ───────────────────────────────────────────
export const getNetwork        = () => fetchJson('/api/network')
export const getPeers          = () => fetchJson('/api/peers')

// ── Directory ─────────────────────────────────────────────────
export const getDirectory      = (params = {}) => {
  const q = new URLSearchParams()
  if (params.q)      q.set('q', params.q)
  if (params.tier)   q.set('tier', params.tier)
  if (params.sort)   q.set('sort', params.sort || 'reputation')
  if (params.limit)  q.set('limit', params.limit)
  if (params.offset) q.set('offset', params.offset)
  return fetchJson(`/api/directory?${q}`)
}

// ── Keys ──────────────────────────────────────────────────────
export const getKeys           = () => fetchJson('/api/keys')

// ── Existing endpoints (preserved) ───────────────────────────
export const getTasks          = () => fetchJson('/api/tasks')
export const getTask           = (id) => fetchJson(`/api/tasks/${id}`)
export const submitTask        = (payload) => fetchJson('/api/tasks', {
  method: 'POST',
  headers: {'Content-Type':'application/json'},
  body: JSON.stringify(payload)
})
export const getHolons         = () => fetchJson('/api/holons')
export const getHolon          = (taskId) => fetchJson(`/api/holons/${taskId}`)
export const getDeliberation   = (taskId) => fetchJson(`/api/tasks/${taskId}/deliberation`)
export const getBallots        = (taskId) => fetchJson(`/api/tasks/${taskId}/ballots`)
export const getIrvRounds      = (taskId) => fetchJson(`/api/tasks/${taskId}/irv-rounds`)
export const getAuditLog       = () => fetchJson('/api/audit')
export const getGraph          = () => fetchJson('/api/graph')
export const getMessages       = () => fetchJson('/api/messages')
```

**Step 2: Commit**

```bash
git add webapp/src/api/client.js
git commit -m "feat(wws): WWS API client — identity, reputation, names, network, directory, keys"
```

---

## Task 3: Header Component

**Files:**
- Create: `webapp/src/components/Header.jsx`
- (Old Header.jsx or AppHeader.jsx will be deleted in Task 13)

**Step 1: Create the file**

```jsx
import { useState } from 'react'

const TIER_LABELS = {
  newcomer: 'Newcomer', member: 'Member', trusted: 'Trusted',
  established: 'Established', veteran: 'Veteran', suspended: 'Suspended'
}

export default function Header({ identity, network, view, onViewChange, onAudit, onSettings }) {
  const name  = identity?.wws_name  || '—'
  const tier  = identity?.tier       || 'newcomer'
  const agents = network?.swarm_size_estimate ?? '—'
  const peers  = network?.peer_count ?? '—'

  return (
    <header className="app-header">
      <div className="header-row1">
        <span className="brand">WWS</span>
        <button className="header-identity" onClick={onSettings}>
          {name} <span className={`tier-badge ${tier}`}>{TIER_LABELS[tier]}</span>
        </button>
        <div className="header-stats">
          <span>◎ {agents} agents</span>
          <span>⬡ {peers} peers</span>
        </div>
        <div className="header-spacer" />
        <button className="btn" onClick={onAudit}>Audit</button>
        <button className="btn" style={{marginLeft:6}} onClick={onSettings}>⚙</button>
      </div>
      <div className="header-row2">
        <div className="view-tabs">
          {['graph','directory','activity'].map(v => (
            <button
              key={v}
              className={`view-tab ${view === v ? 'active' : ''}`}
              onClick={() => onViewChange(v)}
            >
              {v.charAt(0).toUpperCase() + v.slice(1)}
            </button>
          ))}
        </div>
      </div>
    </header>
  )
}
```

**Step 2: Commit**

```bash
git add webapp/src/components/Header.jsx
git commit -m "feat(wws): Header component — brand row + center-view tab switcher"
```

---

## Task 4: Left Column Component

**Files:**
- Create: `webapp/src/components/LeftColumn.jsx`

**Step 1: Create the file**

```jsx
import { useState } from 'react'

const TIER_LABELS = {
  newcomer: 'Newcomer', member: 'Member', trusted: 'Trusted',
  established: 'Established', veteran: 'Veteran', suspended: 'Suspended'
}

function ttlLabel(expiresAt) {
  if (!expiresAt) return '—'
  const secs = Math.max(0, Math.floor((new Date(expiresAt) - Date.now()) / 1000))
  if (secs < 60)    return `${secs}s`
  if (secs < 3600)  return `${Math.floor(secs/60)}m`
  if (secs < 86400) return `${Math.floor(secs/3600)}h`
  return `${Math.floor(secs/86400)}d`
}

function truncate(s, len=10) {
  return s ? `[${s.slice(0,len)}…]` : '—'
}

export default function LeftColumn({
  identity, reputation, names, keys, network,
  onOpenNameRegistry, onOpenKeyMgmt, onOpenReputation,
  onOpenTask, onOpenMessages
}) {
  const tier  = identity?.tier   || 'newcomer'
  const score = reputation?.score ?? 0
  const nextAt = reputation?.next_tier_at ?? 1000
  const pct   = Math.min(100, Math.round((score / nextAt) * 100))

  return (
    <aside className="col-left">
      {/* Identity */}
      <div className="section">
        <div className="section-header">My Agent</div>
        <div className="identity-name">{identity?.wws_name || '—'}</div>
        <div className={`tier-badge ${tier}`}>{TIER_LABELS[tier]}</div>
        <div className="rep-score"
             style={{cursor:'pointer'}}
             onClick={onOpenReputation}
             title="View reputation detail">
          {score} pts
        </div>
        <div className="rep-next">→ {TIER_LABELS[nextTier(tier)]} at {nextAt}</div>
        <div className="rep-bar-track">
          <div className="rep-bar-fill" style={{width:`${pct}%`}} />
        </div>
        <div className="id-field">
          <span className="id-label">DID</span>
          <span className="id-value mono" title={identity?.did}>
            {truncate(identity?.did?.replace('did:swarm:',''), 8)}
          </span>
        </div>
        <div className="id-field">
          <span className="id-label">PeerID</span>
          <span className="id-value mono" title={identity?.peer_id}>
            {truncate(identity?.peer_id, 8)}
          </span>
        </div>
      </div>

      {/* Names */}
      <div className="section">
        <div className="section-header">Names</div>
        {(!names || names.length === 0) && (
          <div className="dim" style={{fontSize:11}}>No registered names</div>
        )}
        {(names || []).map(n => {
          const ttl = ttlLabel(n.expires_at)
          const warn = n.expires_at && (new Date(n.expires_at) - Date.now()) < 7200_000
          return (
            <div className="name-row" key={n.name}>
              <span className="name-label">{n.name}</span>
              <span className={`name-ttl ${warn ? 'warning' : ''}`}>{ttl}</span>
              <button className="name-renew" title="Renew" onClick={() => {}}>↻</button>
            </div>
          )
        })}
        <button className="add-name-btn" onClick={onOpenNameRegistry}>+ Register name</button>
      </div>

      {/* Key Health */}
      <div className="section">
        <div className="section-header" style={{cursor:'pointer'}} onClick={onOpenKeyMgmt}>
          Key Health ›
        </div>
        <div className="status-row" onClick={onOpenKeyMgmt}>
          <div className={`status-dot ${identity?.key_healthy ? 'ok' : 'error'}`} />
          <span className="status-label">keypair</span>
          <span className="status-value">{identity?.key_healthy ? 'ok' : 'missing'}</span>
        </div>
        <div className="status-row" onClick={onOpenKeyMgmt}>
          <div className={`status-dot ${(keys?.guardian_count ?? 0) > 0 ? 'ok' : 'off'}`} />
          <span className="status-label">guardians</span>
          <span className="status-value">{keys?.guardian_count ?? 0}/{keys?.threshold ?? 0}</span>
        </div>
        <div className="status-row" onClick={onOpenKeyMgmt}>
          <div className={`status-dot ${keys?.last_rotation ? 'ok' : 'off'}`} />
          <span className="status-label">rotation</span>
          <span className="status-value">{keys?.last_rotation ? 'done' : 'never'}</span>
        </div>
      </div>

      {/* Network */}
      <div className="section">
        <div className="section-header">Network</div>
        <div className="status-row">
          <div className={`status-dot ${network?.bootstrap_connected ? 'ok' : 'error'}`} />
          <span className="status-label">bootstrap</span>
          <span className="status-value">{network?.bootstrap_connected ? 'ok' : 'offline'}</span>
        </div>
        <div className="status-row">
          <div className="status-dot ok" />
          <span className="status-label">NAT</span>
          <span className="status-value">{network?.nat_type || '—'}</span>
        </div>
        <div className="status-row">
          <div className={`status-dot ${(network?.peer_count ?? 0) > 0 ? 'ok' : 'off'}`} />
          <span className="status-label">peers</span>
          <span className="status-value">{network?.peer_count ?? 0} direct</span>
        </div>
      </div>

      {/* Quick Links */}
      <div className="section">
        <div className="section-header">Quick Links</div>
        {identity?.assigned_task_id && (
          <div className="quick-link" onClick={() => onOpenTask(identity.assigned_task_id)}>
            <span className="quick-link-icon">⚡</span>
            <span className="mono" style={{fontSize:11}}>{identity.assigned_task_id.slice(0,16)}…</span>
          </div>
        )}
        <div className="quick-link" onClick={onOpenMessages}>
          <span className="quick-link-icon">📨</span>
          <span>Messages</span>
        </div>
      </div>
    </aside>
  )
}

function nextTier(tier) {
  const seq = ['newcomer','member','trusted','established','veteran']
  const idx = seq.indexOf(tier)
  return seq[Math.min(idx + 1, seq.length - 1)]
}
```

**Step 2: Commit**

```bash
git add webapp/src/components/LeftColumn.jsx
git commit -m "feat(wws): LeftColumn — identity, reputation bar, names+TTL, key health, network"
```

---

## Task 5: Center Graph Component

**Files:**
- Create: `webapp/src/components/CenterGraph.jsx`
- (Replaces the old LiveGraph.jsx logic — will be wired in Task 13)

**Step 1: Create the file**

```jsx
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
```

**Step 2: Commit**

```bash
git add webapp/src/components/CenterGraph.jsx
git commit -m "feat(wws): CenterGraph — vis-network with my-node highlight + tier-sized nodes"
```

---

## Task 6: Center Directory Component

**Files:**
- Create: `webapp/src/components/CenterDirectory.jsx`

**Step 1: Create the file**

```jsx
import { useState, useEffect, useCallback } from 'react'
import { getDirectory } from '../api/client.js'

const TIERS = ['all','veteran','established','trusted','member','newcomer']

export default function CenterDirectory({ onSelectAgent }) {
  const [query,   setQuery]   = useState('')
  const [tier,    setTier]    = useState('all')
  const [agents,  setAgents]  = useState([])
  const [loading, setLoading] = useState(false)

  const load = useCallback(async () => {
    setLoading(true)
    try {
      const params = { limit: 100 }
      if (query) params.q = query
      if (tier !== 'all') params.tier = tier
      const data = await getDirectory(params)
      setAgents(Array.isArray(data) ? data : [])
    } catch {
      setAgents([])
    } finally {
      setLoading(false)
    }
  }, [query, tier])

  useEffect(() => {
    const t = setTimeout(load, 250)
    return () => clearTimeout(t)
  }, [load])

  function lastSeenLabel(ts) {
    if (!ts) return '—'
    const s = Math.floor((Date.now() - new Date(ts)) / 1000)
    if (s < 60)  return `${s}s ago`
    if (s < 3600) return `${Math.floor(s/60)}m ago`
    return `${Math.floor(s/3600)}h ago`
  }

  return (
    <div className="directory-container">
      <div className="directory-toolbar">
        <input
          className="search-input"
          placeholder="Search wws:name or DID…"
          value={query}
          onChange={e => setQuery(e.target.value)}
        />
        <div className="filter-chips">
          {TIERS.map(t => (
            <button
              key={t}
              className={`filter-chip ${tier === t ? 'active' : ''}`}
              onClick={() => setTier(t)}
            >
              {t.charAt(0).toUpperCase() + t.slice(1)}
            </button>
          ))}
        </div>
      </div>
      <div className="directory-list">
        {loading && <div style={{padding:'16px',color:'var(--text-muted)',textAlign:'center'}}>Loading…</div>}
        {!loading && agents.length === 0 && (
          <div style={{padding:'16px',color:'var(--text-muted)',textAlign:'center'}}>
            No agents found
          </div>
        )}
        {agents.map(a => (
          <div className="agent-row" key={a.did || a.wws_name} onClick={() => onSelectAgent?.(a)}>
            <div style={{flex:'0 0 auto'}}>
              <div className="agent-row-name">{a.wws_name || '—'}</div>
              <div className="agent-row-did">{a.did}</div>
            </div>
            <div className="agent-row-meta">
              <span className={`tier-badge ${a.tier || 'newcomer'}`} style={{fontSize:9}}>
                {a.tier || 'newcomer'}
              </span>
              <span className="agent-row-score">{a.score ?? '—'} pts</span>
              <span className="agent-row-seen">{lastSeenLabel(a.last_seen)}</span>
            </div>
          </div>
        ))}
      </div>
    </div>
  )
}
```

**Step 2: Commit**

```bash
git add webapp/src/components/CenterDirectory.jsx
git commit -m "feat(wws): CenterDirectory — searchable phone book with tier filters"
```

---

## Task 7: Center Activity Component

**Files:**
- Create: `webapp/src/components/CenterActivity.jsx`

**Step 1: Create the file**

```jsx
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
```

**Step 2: Commit**

```bash
git add webapp/src/components/CenterActivity.jsx
git commit -m "feat(wws): CenterActivity — Tasks/Messages/Holons/Deliberation sub-tabs"
```

---

## Task 8: Right Column — Live Stream

**Files:**
- Create: `webapp/src/components/RightStream.jsx`

**Step 1: Create the file**

```jsx
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
```

**Step 2: Commit**

```bash
git add webapp/src/components/RightStream.jsx
git commit -m "feat(wws): RightStream — live event feed with icons + time-ago labels"
```

---

## Task 9: Name Registry Panel

**Files:**
- Create: `webapp/src/components/NameRegistryPanel.jsx`

**Step 1: Create the file**

```jsx
import { useState } from 'react'
import { registerName, renewName, releaseName } from '../api/client.js'

function ttlLabel(expiresAt) {
  if (!expiresAt) return '—'
  const secs = Math.max(0, Math.floor((new Date(expiresAt) - Date.now()) / 1000))
  if (secs < 60)    return `${secs}s`
  if (secs < 3600)  return `${Math.floor(secs/60)}m`
  if (secs < 86400) return `${Math.floor(secs/3600)}h`
  return `${Math.floor(secs/86400)}d`
}

export default function NameRegistryPanel({ open, names, onClose, onRefresh }) {
  const [newName, setNewName]   = useState('')
  const [loading, setLoading]   = useState(false)
  const [error,   setError]     = useState('')
  const [success, setSuccess]   = useState('')

  async function handleRegister(e) {
    e.preventDefault()
    if (!newName.trim()) return
    setLoading(true); setError(''); setSuccess('')
    try {
      await registerName(newName.trim())
      setSuccess(`Registered wws:${newName.trim()} successfully.`)
      setNewName('')
      onRefresh?.()
    } catch (err) {
      setError(err.message || 'Registration failed')
    } finally {
      setLoading(false)
    }
  }

  async function handleRenew(name) {
    try { await renewName(name); onRefresh?.() }
    catch (err) { setError(`Renew failed: ${err.message}`) }
  }

  async function handleRelease(name) {
    if (!confirm(`Release wws:${name}? This cannot be undone.`)) return
    try { await releaseName(name); onRefresh?.() }
    catch (err) { setError(`Release failed: ${err.message}`) }
  }

  return (
    <>
      <div className={`panel-overlay ${open ? 'open' : ''}`} onClick={onClose} />
      <div className={`slide-panel ${open ? 'open' : ''}`}>
        <div className="panel-header">
          <span className="panel-title">Name Registry</span>
          <button className="panel-close" onClick={onClose}>✕</button>
        </div>
        <div className="panel-body">
          <div className="section-header" style={{marginBottom:12}}>Registered Names</div>
          {(!names || names.length === 0) && (
            <div className="dim" style={{marginBottom:16}}>No registered names yet.</div>
          )}
          {(names || []).map(n => {
            const ttl = ttlLabel(n.expires_at)
            const warn = n.expires_at && (new Date(n.expires_at) - Date.now()) < 7200_000
            return (
              <div key={n.name} style={{
                display:'flex', alignItems:'center', gap:8,
                padding:'8px 0', borderBottom:'1px solid var(--border)'
              }}>
                <span style={{flex:1, color:'var(--teal)', fontWeight:600}}>{n.name}</span>
                <span className={`mono ${warn ? 'dim' : ''}`} style={{
                  fontSize:11,
                  color: warn ? 'var(--coral)' : 'var(--text-muted)'
                }}>{ttl}</span>
                <button className="btn" style={{padding:'2px 8px'}} onClick={() => handleRenew(n.name)}>↻ Renew</button>
                <button className="btn btn-danger" style={{padding:'2px 8px'}} onClick={() => handleRelease(n.name)}>Release</button>
              </div>
            )
          })}

          <div style={{marginTop:24}}>
            <div className="section-header" style={{marginBottom:12}}>Register New Name</div>
            {error   && <div style={{color:'var(--coral)',marginBottom:8,fontSize:12}}>{error}</div>}
            {success && <div style={{color:'var(--teal)',marginBottom:8,fontSize:12}}>{success}</div>}
            <form onSubmit={handleRegister}>
              <div className="form-row">
                <label className="form-label">wws: name</label>
                <div style={{display:'flex', gap:8}}>
                  <span style={{
                    padding:'7px 10px',
                    background:'var(--surface-2)',
                    border:'1px solid var(--border-2)',
                    borderRadius:'5px 0 0 5px',
                    color:'var(--text-muted)',
                    fontSize:13
                  }}>wws:</span>
                  <input
                    value={newName}
                    onChange={e => setNewName(e.target.value)}
                    placeholder="alice"
                    style={{borderRadius:'0 5px 5px 0', borderLeft:'none'}}
                    required
                  />
                </div>
              </div>
              <button className="btn btn-primary w-full" type="submit" disabled={loading}>
                {loading ? 'Registering…' : 'Register (PoW)'}
              </button>
            </form>
          </div>
        </div>
      </div>
    </>
  )
}
```

**Step 2: Commit**

```bash
git add webapp/src/components/NameRegistryPanel.jsx
git commit -m "feat(wws): NameRegistryPanel — name list + TTL + renew/release + register form"
```

---

## Task 10: Key Management Panel

**Files:**
- Create: `webapp/src/components/KeyManagementPanel.jsx`

**Step 1: Create the file**

```jsx
export default function KeyManagementPanel({ open, keys, onClose }) {
  if (!keys) return null

  function fmtDate(ts) {
    if (!ts) return 'never'
    return new Date(ts).toLocaleString()
  }

  return (
    <>
      <div className={`panel-overlay ${open ? 'open' : ''}`} onClick={onClose} />
      <div className={`slide-panel ${open ? 'open' : ''}`}>
        <div className="panel-header">
          <span className="panel-title">Key Management</span>
          <button className="panel-close" onClick={onClose}>✕</button>
        </div>
        <div className="panel-body">
          <div className="section-header" style={{marginBottom:12}}>Current Keypair</div>
          <div style={{marginBottom:16}}>
            <div className="id-field"><span className="id-label">DID</span><span className="mono">{keys.did}</span></div>
            <div className="id-field"><span className="id-label">Pubkey</span><span className="mono">{keys.pubkey_hex}</span></div>
            <div className="id-field"><span className="id-label">Created</span><span className="mono">{fmtDate(keys.created_at)}</span></div>
            <div className="id-field"><span className="id-label">Rotated</span><span className="mono">{fmtDate(keys.last_rotation)}</span></div>
          </div>

          <div className="section-header" style={{marginBottom:12}}>Guardians</div>
          <div style={{marginBottom:16}}>
            <div style={{fontSize:13, color:'var(--text-muted)', marginBottom:8}}>
              {keys.guardian_count || 0} of {keys.threshold || 0} configured
            </div>
            {(keys.guardian_count || 0) === 0 && (
              <div style={{color:'var(--amber)', fontSize:12}}>
                ⚠ No guardians configured. Recovery is not possible without guardians.
              </div>
            )}
          </div>

          <div className="section-header" style={{marginBottom:12}}>Key Rotation</div>
          <div style={{marginBottom:16, fontSize:12, color:'var(--text-muted)', lineHeight:1.6}}>
            Initiating rotation will generate a new keypair. Both keypairs will sign
            messages for a 48-hour grace period. Configure your new key with peers before
            the grace period ends.
          </div>
          <button className="btn w-full" style={{marginBottom:8}}>
            Initiate Planned Rotation
          </button>

          <div style={{marginTop:24}}>
            <div className="section-header" style={{marginBottom:8}}>Emergency Revocation</div>
            <div style={{fontSize:12, color:'var(--text-muted)', marginBottom:12, lineHeight:1.6}}>
              Immediately invalidates the current keypair. Requires recovery key (BIP-39 mnemonic).
              This action is irreversible.
            </div>
            <button className="btn btn-danger w-full">
              Emergency Revocation…
            </button>
          </div>
        </div>
      </div>
    </>
  )
}
```

**Step 2: Commit**

```bash
git add webapp/src/components/KeyManagementPanel.jsx
git commit -m "feat(wws): KeyManagementPanel — keypair info, guardians, rotation, emergency revoke"
```

---

## Task 11: Reputation Panel

**Files:**
- Create: `webapp/src/components/ReputationPanel.jsx`

**Step 1: Create the file**

```jsx
import { useState, useEffect } from 'react'
import { getRepEvents } from '../api/client.js'

const TIER_THRESHOLDS = {
  newcomer: 0, member: 100, trusted: 500, established: 1000, veteran: 2500
}

export default function ReputationPanel({ open, reputation, onClose }) {
  const [events, setEvents] = useState([])
  const [loading, setLoading] = useState(false)

  useEffect(() => {
    if (!open) return
    setLoading(true)
    getRepEvents(50, 0)
      .then(data => setEvents(Array.isArray(data) ? data : []))
      .catch(() => setEvents([]))
      .finally(() => setLoading(false))
  }, [open])

  if (!reputation) return null

  const { score = 0, positive_total = 0, negative_total = 0, tier = 'newcomer', next_tier_at = 100, decay = 0 } = reputation

  return (
    <>
      <div className={`panel-overlay ${open ? 'open' : ''}`} onClick={onClose} />
      <div className={`slide-panel ${open ? 'open' : ''}`}>
        <div className="panel-header">
          <span className="panel-title">Reputation</span>
          <button className="panel-close" onClick={onClose}>✕</button>
        </div>
        <div className="panel-body">
          <div style={{marginBottom:20}}>
            <div style={{fontSize:28, fontWeight:700, color:'var(--teal)', fontFamily:'JetBrains Mono,monospace'}}>
              {score} pts
            </div>
            <div className={`tier-badge ${tier}`} style={{marginTop:6}}>{tier}</div>
          </div>

          <div className="section-header" style={{marginBottom:10}}>Score Breakdown</div>
          <div style={{display:'grid', gridTemplateColumns:'1fr 1fr', gap:8, marginBottom:20}}>
            <div style={{background:'var(--surface-2)', border:'1px solid var(--border)', borderRadius:6, padding:10}}>
              <div style={{fontSize:10, color:'var(--text-muted)', marginBottom:4}}>POSITIVE</div>
              <div style={{fontSize:16, fontWeight:700, color:'var(--teal)', fontFamily:'JetBrains Mono,monospace'}}>+{positive_total}</div>
            </div>
            <div style={{background:'var(--surface-2)', border:'1px solid var(--border)', borderRadius:6, padding:10}}>
              <div style={{fontSize:10, color:'var(--text-muted)', marginBottom:4}}>NEGATIVE</div>
              <div style={{fontSize:16, fontWeight:700, color:'var(--coral)', fontFamily:'JetBrains Mono,monospace'}}>−{Math.abs(negative_total)}</div>
            </div>
            <div style={{background:'var(--surface-2)', border:'1px solid var(--border)', borderRadius:6, padding:10}}>
              <div style={{fontSize:10, color:'var(--text-muted)', marginBottom:4}}>DECAY</div>
              <div style={{fontSize:16, fontWeight:700, color:'var(--amber)', fontFamily:'JetBrains Mono,monospace'}}>−{decay}</div>
            </div>
            <div style={{background:'var(--surface-2)', border:'1px solid var(--border)', borderRadius:6, padding:10}}>
              <div style={{fontSize:10, color:'var(--text-muted)', marginBottom:4}}>NEXT TIER</div>
              <div style={{fontSize:14, fontWeight:700, color:'var(--text)', fontFamily:'JetBrains Mono,monospace'}}>{next_tier_at}</div>
            </div>
          </div>

          <div className="section-header" style={{marginBottom:10}}>Recent Events</div>
          {loading && <div style={{color:'var(--text-muted)', fontSize:12}}>Loading…</div>}
          {events.map((ev, i) => (
            <div key={i} style={{
              display:'flex', alignItems:'center', gap:8,
              padding:'6px 0', borderBottom:'1px solid var(--border)',
              fontSize:12
            }}>
              <span style={{
                color: ev.points > 0 ? 'var(--teal)' : 'var(--coral)',
                fontFamily:'JetBrains Mono,monospace',
                fontWeight:700,
                minWidth:40
              }}>
                {ev.points > 0 ? '+' : ''}{ev.points}
              </span>
              <span style={{flex:1, color:'var(--text-muted)'}}>{ev.reason || ev.event_type}</span>
              <span style={{fontFamily:'JetBrains Mono,monospace', fontSize:10, color:'var(--text-dim)'}}>
                {ev.timestamp ? new Date(ev.timestamp).toLocaleDateString() : '—'}
              </span>
            </div>
          ))}
          {!loading && events.length === 0 && (
            <div style={{color:'var(--text-muted)', fontSize:12}}>No events yet.</div>
          )}
        </div>
      </div>
    </>
  )
}
```

**Step 2: Commit**

```bash
git add webapp/src/components/ReputationPanel.jsx
git commit -m "feat(wws): ReputationPanel — score breakdown + tier progress + event history"
```

---

## Task 12: Agent Detail Panel (update)

**Files:**
- Modify: `webapp/src/components/AgentDetailPanel.jsx` (if it exists, else create)

**Step 1: Check if the file exists, then create/replace with:**

```jsx
export default function AgentDetailPanel({ open, agent, onClose }) {
  if (!agent) return null

  function fmtDate(ts) {
    if (!ts) return '—'
    const s = Math.floor((Date.now() - new Date(ts)) / 1000)
    if (s < 60)   return `${s}s ago`
    if (s < 3600) return `${Math.floor(s/60)}m ago`
    if (s < 86400) return `${Math.floor(s/3600)}h ago`
    return new Date(ts).toLocaleDateString()
  }

  const tier = agent.tier || 'newcomer'

  return (
    <>
      <div className={`panel-overlay ${open ? 'open' : ''}`} onClick={onClose} />
      <div className={`slide-panel ${open ? 'open' : ''}`}>
        <div className="panel-header">
          <span className="panel-title">Agent Detail</span>
          <button className="panel-close" onClick={onClose}>✕</button>
        </div>
        <div className="panel-body">
          <div style={{marginBottom:20}}>
            <div style={{fontSize:18, fontWeight:700, color:'var(--teal)'}}>{agent.wws_name || '—'}</div>
            <div className={`tier-badge ${tier}`} style={{marginTop:6}}>{tier}</div>
            <div style={{marginTop:8, fontFamily:'JetBrains Mono,monospace', fontSize:11, color:'var(--text-muted)', wordBreak:'break-all'}}>
              {agent.did}
            </div>
          </div>

          <div className="section-header" style={{marginBottom:10}}>Reputation</div>
          <div style={{marginBottom:16, fontSize:13, fontFamily:'JetBrains Mono,monospace', color:'var(--text)'}}>
            {agent.score ?? '—'} pts · last seen {fmtDate(agent.last_seen)}
          </div>

          {agent.names && agent.names.length > 0 && (
            <>
              <div className="section-header" style={{marginBottom:10}}>Registered Names</div>
              {agent.names.map(n => (
                <div key={n} style={{color:'var(--teal)', fontWeight:600, padding:'3px 0'}}>{n}</div>
              ))}
              <div style={{marginBottom:16}} />
            </>
          )}

          {agent.connection_type && (
            <>
              <div className="section-header" style={{marginBottom:10}}>Connection</div>
              <div style={{fontSize:12, color:'var(--text-muted)', marginBottom:16}}>
                {agent.connection_type}
              </div>
            </>
          )}

          {agent.task_history && agent.task_history.length > 0 && (
            <>
              <div className="section-header" style={{marginBottom:10}}>Task History</div>
              {agent.task_history.slice(0,10).map(t => (
                <div key={t.id} style={{
                  padding:'4px 0', borderBottom:'1px solid var(--border)',
                  fontSize:12, color:'var(--text-muted)', fontFamily:'JetBrains Mono,monospace'
                }}>
                  {t.id?.slice(0,16)}… <span style={{color:'var(--text-dim)'}}>{t.status}</span>
                </div>
              ))}
            </>
          )}
        </div>
      </div>
    </>
  )
}
```

**Step 2: Commit**

```bash
git add webapp/src/components/AgentDetailPanel.jsx
git commit -m "feat(wws): AgentDetailPanel — wws:name, tier, DID, reputation, names, connection"
```

---

## Task 13: App.jsx — Complete Rewrite + Wire Everything

**Files:**
- Modify: `webapp/src/App.jsx`
- Delete: `webapp/src/components/BottomTray.jsx` (if it exists)

**Step 1: Read current App.jsx to understand imports, then replace entirely with:**

```jsx
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

  // SSE / WebSocket event stream
  useEffect(() => {
    const es = new EventSource('/api/stream')
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
```

**Step 2: Delete BottomTray if it exists**

```bash
rm -f webapp/src/components/BottomTray.jsx
```

**Step 3: Update index.html title**

In `webapp/index.html`, change:
```html
<title>ASIP — Holonic Swarm Console</title>
```
to:
```html
<title>WWS — World Wide Swarm</title>
```

**Step 4: Commit**

```bash
git add webapp/src/App.jsx webapp/index.html
git rm --cached webapp/src/components/BottomTray.jsx 2>/dev/null || true
git commit -m "feat(wws): App.jsx complete rewrite — WWS three-zone command center wired up"
```

---

## Task 14: Update Playwright Tests

**Files:**
- Modify: `tests/playwright/webapp.spec.js`

**Step 1: Replace the test file with:**

```javascript
import { test, expect } from '@playwright/test'

const BASE = 'http://localhost:5173'

test.describe('WWS Webapp', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto(BASE)
    await page.waitForLoadState('networkidle')
  })

  test('shows WWS brand in header', async ({ page }) => {
    await expect(page.locator('.brand')).toContainText('WWS')
  })

  test('shows three center view tabs', async ({ page }) => {
    const tabs = page.locator('.view-tab')
    await expect(tabs).toHaveCount(3)
    await expect(tabs.nth(0)).toContainText('Graph')
    await expect(tabs.nth(1)).toContainText('Directory')
    await expect(tabs.nth(2)).toContainText('Activity')
  })

  test('switches to Directory view', async ({ page }) => {
    await page.click('.view-tab:nth-child(2)')
    await expect(page.locator('.directory-container')).toBeVisible()
    await expect(page.locator('.search-input')).toBeVisible()
  })

  test('switches to Activity view', async ({ page }) => {
    await page.click('.view-tab:nth-child(3)')
    await expect(page.locator('.activity-container')).toBeVisible()
    const actTabs = page.locator('.activity-tab')
    await expect(actTabs).toHaveCount(4)
  })

  test('shows left column with My Agent section', async ({ page }) => {
    await expect(page.locator('.col-left')).toBeVisible()
    await expect(page.locator('.identity-name')).toBeVisible()
  })

  test('shows right column live stream', async ({ page }) => {
    await expect(page.locator('.col-right')).toBeVisible()
    await expect(page.locator('.stream-header')).toContainText('Live Stream')
  })

  test('opens Name Registry panel from left column', async ({ page }) => {
    await page.click('.add-name-btn')
    await expect(page.locator('.slide-panel.open')).toBeVisible()
    await expect(page.locator('.panel-title')).toContainText('Name Registry')
    await page.click('.panel-close')
    await expect(page.locator('.slide-panel.open')).toHaveCount(0)
  })

  test('opens Audit panel from header', async ({ page }) => {
    await page.click('.btn:has-text("Audit")')
    await expect(page.locator('.slide-panel.open')).toBeVisible()
    await page.click('.panel-close')
  })

  test('no console errors on load', async ({ page }) => {
    const errors = []
    page.on('console', msg => { if (msg.type() === 'error') errors.push(msg.text()) })
    await page.goto(BASE)
    await page.waitForTimeout(2000)
    expect(errors).toHaveLength(0)
  })
})
```

**Step 2: Run tests**

```bash
cd tests && npx playwright test webapp.spec.js --reporter=list
```

Expected: All 9 tests pass. (Some may be skipped if connector is not running — that's fine.)

**Step 3: Commit**

```bash
git add tests/playwright/webapp.spec.js
git commit -m "test(wws): update Playwright suite for WWS three-zone layout"
```

---

## Task 15: Final visual verification

**Step 1: Start dev server and inspect**

```bash
cd webapp && npm run dev -- --port 5173
```

**Step 2: Run Playwright screenshot test**

```bash
cd tests && npx playwright test webapp.spec.js --reporter=list
```

**Step 3: Check no regressions in Rust tests**

```bash
~/.cargo/bin/cargo test --workspace --quiet
```

Expected: all tests pass.

**Step 4: Final commit (if any minor fixes needed)**

```bash
git add -p
git commit -m "fix(wws): final visual polish after verification"
```

---

## Summary of Changes

| File | Action |
|------|--------|
| `webapp/src/styles.css` | Complete rewrite — WWS design system |
| `webapp/src/api/client.js` | New endpoints: identity, reputation, names, network, peers, directory, keys |
| `webapp/src/components/Header.jsx` | New — two-row header with view tabs |
| `webapp/src/components/LeftColumn.jsx` | New — identity, reputation, names, key health, network |
| `webapp/src/components/CenterGraph.jsx` | New — vis-network with my-node highlight |
| `webapp/src/components/CenterDirectory.jsx` | New — phone book with search + tier filters |
| `webapp/src/components/CenterActivity.jsx` | New — Tasks/Messages/Holons/Deliberation tabs |
| `webapp/src/components/RightStream.jsx` | New — real-time event stream |
| `webapp/src/components/NameRegistryPanel.jsx` | New — register/renew/release names |
| `webapp/src/components/KeyManagementPanel.jsx` | New — keypair info + rotation + guardians |
| `webapp/src/components/ReputationPanel.jsx` | New — score breakdown + event history |
| `webapp/src/components/AgentDetailPanel.jsx` | New/Updated — agent detail with WWS fields |
| `webapp/src/App.jsx` | Complete rewrite — three-zone layout wired |
| `webapp/index.html` | Update title to "WWS — World Wide Swarm" |
| `webapp/src/components/BottomTray.jsx` | Delete |
| `tests/playwright/webapp.spec.js` | Update for new layout |
