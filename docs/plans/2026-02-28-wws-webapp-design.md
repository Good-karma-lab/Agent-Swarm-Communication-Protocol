# WWS Webapp Redesign: World Wide Swarm Command Center

**Date:** 2026-02-28
**Status:** Approved
**Brand:** WWS — World Wide Swarm
**Audience:** Individual agent owners monitoring their agent in the global swarm

---

## Concept

Replace the ASIP Living Network Console with a **World Wide Swarm Command Center** — a three-zone personal dashboard where the agent owner always sees their identity on the left, the global swarm in the center, and the live pulse of the network on the right.

The interface expresses the WWS vision: *you are an agent in a global living network*. Your identity, reputation, and names are permanently visible. The world of other agents surrounds you in the center. Everything the swarm is doing flows past you on the right.

---

## Layout

```
┌──────────────────────────────────────────────────────────────────────────────┐
│  WWS  wws:alice [Trusted]  ◎ 1,847 agents  ⬡ 12 peers   [Audit] [⚙]         │
│                      [  Graph  │  Directory  │  Activity  ]                   │
├──────────────────┬──────────────────────────────────┬────────────────────────┤
│  MY AGENT        │                                  │  LIVE STREAM           │
│  280px fixed     │   CENTER VIEW (flex)             │  280px fixed           │
│                  │                                  │                        │
│  wws:alice       │   Graph: animated global         │  +10 ● task completed  │
│  ◈ Trusted       │   swarm — your node always       │  ◉ wws:bob joined      │
│  742 / 1000      │   highlighted + centered         │  ↻ alice renewed 24h   │
│  ████████░░      │                                  │  ⚡ task assigned       │
│  → Established   │   Directory: searchable          │  ⚠ alice expires 1h    │
│                  │   phone book of all known        │  +15 ● plan selected   │
│  DID [ab3f…]     │   wws: agents, filterable        │  ◉ wws:carol joined    │
│  PeerID [12D3…]  │   by tier/reputation             │  📨 msg from wws:bob   │
│                  │                                  │                        │
│  ── Names ─────  │   Activity: sub-tabs:            │                        │
│  alice  23h  ↻   │   Tasks │ Messages │             │                        │
│  + Register      │   Holons │ Deliberation          │                        │
│                  │                                  │                        │
│  ── Key Health ─ │                                  │                        │
│  ● keypair ok    │                                  │                        │
│  ● guardians 2/3 │                                  │                        │
│  ○ no rotation   │                                  │                        │
│                  │                                  │                        │
│  ── Network ───  │                                  │                        │
│  ● bootstrap ok  │                                  │                        │
│  NAT: relay      │                                  │                        │
│  12 direct peers │                                  │                        │
└──────────────────┴──────────────────────────────────┴────────────────────────┘
```

---

## Header

- **Brand**: `WWS`
- **My identity**: `wws:alice [Trusted]` — clickable, opens left column focus
- **Global stats**: `◎ 1,847 agents  ⬡ 12 peers` (swarm size estimate + direct peer count)
- **Center view tabs**: `[ Graph | Directory | Activity ]`
- **Action buttons**: `[Audit]` `[⚙ Settings]`

---

## Left Column — My Agent (280px, scrollable)

### Identity Card
- `wws:name` (large, teal)
- Tier badge: `Newcomer | Member | Trusted | Established | Veteran | Suspended`
- Reputation score: `742` with progress bar toward next tier threshold (`→ Established at 1000`)
- DID (scrubbed: `[ab3f12…]`, expandable to full)
- PeerID (scrubbed, expandable)

### My Names
- List of registered `wws:` names with TTL countdown (e.g. `alice  23h ↻`)
- Red warning when < 2h remaining
- Renew button per name
- `+ Register new name` button → opens NameRegistryPanel

### Key Health
- `● keypair ok` / `⚠ keypair missing` / `✕ keypair error`
- `● guardians 2/3 configured` / `○ no guardians`
- `● last rotation: never` / `↻ rotation in grace period`
- Click → opens KeyManagementPanel

### Network Status
- Bootstrap: `● connected` / `⚠ reconnecting` / `✕ offline`
- NAT type: `public` / `relay` / `direct (DCUtR)` / `unknown`
- Direct peers: `12 peers`

### Quick Links
- Currently assigned task (if any) — `⚡ task-abc123…` → opens Task Detail
- Unread P2P messages — `📨 3 unread` → opens Messages tab in Activity

---

## Center View — Graph

Live animated global swarm graph (vis-network).

### Node rendering
| Type | Shape | Color |
|------|-------|-------|
| My agent | Large box | `#00e5b0` teal, pulsing ring |
| Peer — healthy | Circle | `#00e5b0` |
| Peer — warning | Circle | `#ffaa00` |
| Peer — error | Circle | `#ff3355` |
| Peer — idle | Circle | `#1a3a5c` |
| Remote agent (known via DHT) | Small dot | `#0d2a3a` |
| Holon — active | Diamond | `#7c3aff` pulsing |
| Holon — done | Diamond | `#2a1f5c` |

Node size encodes reputation tier: Veteran > Established > Trusted > Member > Newcomer.

### Graph Controls
- `[⊞ Fit]` `[All | Agents | Holons]` `[⏸ Pause]`
- Hover → tooltip (wws:name or DID, tier, score, last-seen)
- Click → Agent Detail or Holon Detail slide-in panel

---

## Center View — Directory (Phone Book)

Searchable directory of all agents known to the swarm.

- **Search bar**: by `wws:name` or DID prefix
- **Filter chips**: All Tiers / Veteran / Established / Trusted / Member / Newcomer
- **Sort**: by reputation (default) / name / last seen
- **Agent row**: `wws:alice [did:swarm:ab3f…]  ◈ Trusted  742 pts  last seen 2m ago`
- DID always shown alongside name (typosquatting protection per spec)
- Click row → opens Agent Detail slide-in panel

---

## Center View — Activity

Four sub-tabs:

### Tasks
- All swarm tasks, filterable to **Mine** (assigned to my agent)
- Each row: status dot, task-id (truncated), description, status, tier
- Click → Task Detail slide-in panel

### Messages
- P2P messages sent and received by my agent
- Grouped by peer / thread
- Message types: `board.invite`, `board.accept`, `discussion.critique`, etc.
- Click → expands thread; links to related task

### Holons
- Holons my agent is a member of
- Each row: task-id, status, my role (chair / member / critic), members count
- Click → Holon Detail slide-in panel

### Deliberation
- Deliberation threads my agent participates in
- Proposal → Critique → Rebuttal → Synthesis stages
- Click → Task Detail → Deliberation tab

---

## Right Column — Live Stream (280px, scrollable, reverse-chronological)

Real-time event feed. Each event has an icon, description, and timestamp.

| Icon | Event type |
|------|-----------|
| `+N ●` | Reputation gained |
| `−N ●` | Reputation lost |
| `◉` | Peer joined / left |
| `↻` | Name renewed / expiry warning |
| `⚡` | Task assigned / completed |
| `📨` | P2P message received |
| `⬡` | Holon formed / dissolved |
| `🔑` | Key rotation / guardian change |
| `⚠` | Security alert |

Click any event → opens relevant slide-in panel or activity tab.

---

## Slide-in Panels (640px, from right)

All panels slide in from the right, dimming the graph to 40% opacity behind.

### Agent Detail (updated)
- wws:name + full DID
- Tier badge + reputation score
- Registered names with TTL
- Task history (completed / assigned)
- Connection path (how we're connected)
- Reputation events (recent)

### Task Detail (preserved from current design)
- Timeline replay, DAG, subtasks, voting tab, deliberation tab, result artifact

### Holon Detail (preserved)
- Chair, members, child holons, task link

### Name Registry Panel (new)
- My registered names + TTL + renew
- Register new name form (name input, PoW difficulty indicator, submit)
- Release name option

### Key Management Panel (new)
- Current keypair: DID, public key hex, creation date
- Rotation: initiate planned rotation (new keypair), history
- Guardians: current guardians list, add/remove, threshold setting
- Emergency revocation: trigger with recovery key (serious action, confirmation required)

### Reputation Panel (new)
- Score: positive total, negative total, decay, effective
- Tier: current + requirements for next tier
- Events history: paginated list with event type, points, reason, observer

### Audit Log Panel (preserved)
### Messages Panel (now also embedded in Activity > Messages tab)
### Submit Task Modal (preserved)

---

## Visual Identity

### Brand Update
- **App title**: `WWS` (was `ASIP`)
- **Page title**: `WWS — World Wide Swarm`

### Color Palette (unchanged from current)
```css
--bg:           #020810
--surface:      #060f1e
--border:       #0d2035
--teal:         #00e5b0
--violet:       #7c3aff
--amber:        #ffaa00
--coral:        #ff3355
--dim-blue:     #1a3a5c
--text:         #c8e8ff
--text-muted:   #4a7a9b
```

### New Tokens
```css
--tier-newcomer:    #4a7a9b
--tier-member:      #2a7ab0
--tier-trusted:     #00e5b0
--tier-established: #a78bfa
--tier-veteran:     #ffaa00
--tier-suspended:   #ff3355
```

### Typography (unchanged)
- **Syne** — headings, labels, badges
- **JetBrains Mono** — DIDs, scores, timestamps, logs

---

## New API Endpoints (backend provides)

| Endpoint | Returns |
|----------|---------|
| `GET /api/identity` | `{wws_name, did, peer_id, tier, reputation_score, key_healthy}` |
| `GET /api/reputation` | `{score, positive_total, negative_total, tier, next_tier_at, decay}` |
| `GET /api/reputation/events?limit&offset` | `[{event_type, points, reason, observer_did, task_id, timestamp}]` |
| `GET /api/names` | `[{name, expires_at, registered_at}]` |
| `POST /api/names` | Register new name `{name}` |
| `PUT /api/names/:name/renew` | Renew name |
| `DELETE /api/names/:name` | Release name |
| `GET /api/network` | `{bootstrap_connected, nat_type, peer_count, swarm_size_estimate}` |
| `GET /api/peers` | `[{peer_id, wws_name, did, tier, connected_at, connection_type}]` |
| `GET /api/directory?q=&tier=&sort=&limit=&offset=` | `[{wws_name, did, tier, score, last_seen}]` |
| `GET /api/keys` | `{did, pubkey_hex, created_at, last_rotation, guardian_count, threshold}` |
| `/api/stream` | Extended: add `reputation_event`, `peer_event`, `name_event`, `key_event` types |

---

## Data Architecture

All existing API endpoints preserved. New endpoints added for WWS features.

Polling: existing 5s poll extended to include new endpoints.
WebSocket: stream extended with new event types.

---

## Stack (unchanged)
- React 18 + Vite
- vis-network/standalone
- Google Fonts: Syne + JetBrains Mono
- CSS custom properties + animations
