# Webapp Redesign: Living Network Console

**Date:** 2026-02-28
**Status:** Approved
**Audience:** Operators / clients monitoring production runs

---

## Concept

Replace the current 10-tab developer-tool UI with a **Living Network Console** — a full-viewport animated graph that IS the primary interface. The topology and holon network is always live in the background; all detail content slides in as contextual panels without navigating away.

The visual metaphor matches the product: a self-organizing holonic swarm rendered as a breathing, flowing organism.

---

## Layout

### Primary View
```
┌────────────────────────────────────────────────────────────────┐
│  ASIP  ●●● 14 agents  4 tasks  [+ Submit Task]   [Audit] [⚙]  │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│              LIVE HOLON / TOPOLOGY GRAPH                       │
│         (animated nodes, flowing edge particles)               │
│              click any node → side panel slides in             │
│                                                                │
├─────────────────┬──────────────────┬───────────────────────────┤
│   HEALTH TRAY   │   TASK STREAM    │     AGENT ROSTER          │
│  RED 1 YLW 2    │  ▶ task-abc …   │  agent-001 ● tier-1       │
│  GRN 11         │  ▶ task-def …   │  agent-002 ● tier-2       │
└─────────────────┴──────────────────┴───────────────────────────┘
```

- **Header**: brand, global health indicators (RED/YELLOW/GREEN counts), active task + agent counts, Submit Task button, Audit link, settings
- **Graph** (center): full remaining viewport height, live animated
- **Bottom tray** (fixed ~180px): three columns — Health Summary, Task Stream, Agent Roster

---

## Graph Specification

### Node Types
| Type | Shape | Color |
|------|-------|-------|
| Agent — healthy | Circle | `#00e5b0` teal |
| Agent — warning | Circle | `#ffaa00` amber |
| Agent — error | Circle | `#ff3355` coral |
| Agent — idle | Circle | `#1a3a5c` dim blue |
| Holon — active | Hexagon | `#7c3aff` violet (pulsing) |
| Holon — done | Hexagon | `#2a1f5c` dim violet |

- Node size: root agent = largest; tier scales down; sub-holons visually nest
- Agents in a holon get a colored ring matching holon status color

### Edge Types
| Type | Style |
|------|-------|
| Hierarchy | Solid faint teal |
| Holon membership | Dotted violet |
| Task assignment (active) | Animated particle flow, teal |

### Interactions
- **Hover** → tooltip (name, status, last-seen, task count)
- **Click** → slide-in detail panel from right
- **Scroll/pinch** → zoom
- **Drag** → pan
- **Double-click empty space** → reset zoom to fit

### Live Behavior
- Nodes animate in on appearance (fade + scale from 0)
- Disconnected nodes fade out before removal
- Executing tasks: pulse wave along edge from source to destination
- Graph re-layouts smoothly when topology changes
- Active holon nodes: subtle breathing scale pulse

### Graph Controls (top-right corner)
- `[⊞ Fit]` — fit all nodes
- `[⬡ Holons]` / `[○ Agents]` / `[All]` — filter toggle
- `[❚❚ Pause]` — freeze physics layout (data still updates)

---

## Detail Panels

All panels slide in from the right (~640px wide). Graph dims to 40% opacity behind them. Dismissed with `✕` button or `Esc`.

### Task Detail
1. Header: task ID, status badge, tier, assigned agent
2. Timeline Replay — play/pause/scrub, stage transitions
3. Task DAG — vis-network subtask graph
4. Subtask table — status, assignee, result
5. **Voting tab**: commit/reveal counts, per-voter ballots, critic score bars, IRV rounds
6. **Deliberation tab**: Proposal→Critique→Rebuttal→Synthesis thread, adversarial critic markers, score bars
7. Result Artifact — content type, size, text output
8. Related Messages — P2P messages scoped to task

### Agent Detail
1. Header: scrubbed agent ID, tier, health badge, connectivity
2. Stats: tasks assigned/processed/proposed/revealed/voted
3. Last poll / last result timestamps
4. Loop active status
5. Currently assigned task (link opens Task Detail)

### Holon Detail
1. Header: task ID, holon status badge, depth
2. Chair agent, adversarial critic
3. Member list (scrubbed IDs)
4. Child holons list (each links to own Holon Detail)
5. Link → Task Detail for this holon's task
6. Link → open Deliberation tab directly

### Audit Log Panel
- Full timestamped operator audit event log, monospace, scrollable
- Accessible from header `[Audit]` button

### Messages Panel
- P2P debug log filtered to business messages, monospace
- Accessible from header

### Task Submission
- Modal overlay triggered by `[+ Submit Task]`
- Textarea for description, optional operator token, submit/cancel

---

## Visual Identity

### Color Palette
```css
--bg:           #020810   /* near-black deep space */
--surface:      #060f1e   /* panel surface */
--border:       #0d2035   /* barely visible dark blue */

--teal:         #00e5b0   /* primary accent, healthy nodes */
--violet:       #7c3aff   /* holon nodes */
--amber:        #ffaa00   /* warning */
--coral:        #ff3355   /* error */
--dim-blue:     #1a3a5c   /* idle */

--text:         #c8e8ff   /* cool ice white */
--text-muted:   #4a7a9b   /* steel blue-grey */
```

### Typography
- **Syne** (Google Fonts) — headings, labels, navigation, status badges
- **JetBrains Mono** — all data values, IDs, timestamps, logs, scores

### Motion
- Detail panel slide-in: 300ms `cubic-bezier(0.16, 1, 0.32, 1)`
- Health status color transitions: 800ms
- Node appear: fade + scale, 400ms
- Node disappear: fade out + shrink, 600ms
- Active holon pulse: 2s breathing cycle
- Edge particles: continuous flow animation
- Task submission: ripple effect from button through graph

---

## Data Architecture (unchanged)

All existing API endpoints and WebSocket stream are preserved. The redesign is purely frontend — same polling logic, same data models, same backend.

**Panels → new locations:**

| Old Tab | New Location |
|---------|-------------|
| Overview | Bottom tray + header health indicators |
| Hierarchy | Agent Detail panel (tier/parent info) |
| Voting | Task Detail → Voting tab |
| Messages | Header → Messages panel |
| Task Forensics | Task Detail panel |
| Topology | Main graph (merged) |
| Audit | Header → Audit panel |
| Ideas | Removed (operator-facing console) |
| Holons | Main graph (merged) + Holon Detail panel |
| Deliberation | Task Detail → Deliberation tab |

---

## Stack
- React 18 + Vite (required)
- vis-network (existing, for DAG and graph)
- Google Fonts: Syne + JetBrains Mono
- CSS custom properties + CSS animations (no extra animation library needed)
- Existing API client unchanged
