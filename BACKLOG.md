# OpenSwarm Backlog

## Completed in this change

- [x] Console task injection now publishes to swarm network topics.
- [x] Zeroclaw integration updated with OpenRouter backend support.
- [x] Added automated Zeroclaw updater script (`scripts/update-zeroclaw.sh`).
- [x] Added end-to-end test suite scaffolding under `tests/e2e`.
- [x] Added live ZeroClaw + OpenRouter E2E test script (opt-in, API-key gated).

## Remaining high-priority items

- [x] Add explicit RPC APIs for vote submission/inspection (for deterministic external voting tests).
- [x] Add fault-injection E2E tests (leader failover, message loss/reorder, reconnect storms).
- [x] Add long-running soak tests with memory and task-store growth assertions.
- [x] Add CI matrix split: fast deterministic E2E on PRs, heavy scale/live-LLM tests on scheduled runs.
- [x] Investigate live swarm convergence and improve active member visibility + bootstrap/connect reliability.
- [x] Investigate live agent behavior drift and add coordinator/execution fallbacks for end-to-end flow continuity.
