# End-to-End Tests

This directory contains process-level E2E coverage for the connector, console, networking, and live agent workflows.

## Test suites

- `connector_scenarios.sh`
  - Connection establishment
  - Local autodiscovery smoke (mDNS)
  - Internet-style autodiscovery (bootstrap + dial)
  - Voting/decomposition/distribution/result timeline validation
  - Peer-to-peer messaging and scaling via ignored network integration tests

- `connector_console_e2e.sh`
  - Operator console command flow
  - Console task injection propagation to another node

- `zeroclaw_openrouter_live.sh` (opt-in)
  - Full live flow with ZeroClaw + OpenRouter (`minimax/minimax-m2.5` by default)
  - Requires `OPENROUTER_API_KEY`

- `run_all.sh`
  - Runs deterministic suites by default
  - Runs live suite only if `E2E_LIVE_LLM=1`

## Usage

```bash
bash tests/e2e/run_all.sh
```

Live LLM run:

```bash
export E2E_LIVE_LLM=1
export OPENROUTER_API_KEY="<your-key>"
export MODEL_NAME="minimax/minimax-m2.5"
bash tests/e2e/run_all.sh
```
