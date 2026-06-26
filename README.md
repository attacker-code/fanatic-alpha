# FANatic Alpha

**Autonomous World Cup Trading Agent on Solana — TxODDS World Cup Hackathon**

## Program ID

`CMso1JNiDvK23fTo9VK3RMGZah2iQLrAm1LKsR1pHdm1`

[View on Solana Explorer](https://explorer.solana.com/address/CMso1JNiDvK23fTo9VK3RMGZah2iQLrAm1LKsR1pHdm1?cluster=devnet)

## What It Does

FANatic Alpha is a fully autonomous trading agent that monitors TxLINE's real-time World Cup odds feed via SSE, runs three parallel trading strategies, and executes every decision on-chain through a Solana Anchor program — with zero human input after the initial launch command.

## Architecture

```
3-Layer Agent Pipeline:
├── Layer 1: Odds Monitor    — SSE client with exponential backoff reconnection
├── Layer 2: Strategy Engine  — 3 strategies running in parallel (tokio)
└── Layer 3: On-Chain Settlement — Anchor program with immutable audit trail

3 Trading Strategies:
├── Sharp Movement Detector  — Flags odds shifts >3σ from rolling mean
├── Arbitrage Scanner        — Cross-market implied probability arb
└── Steam Chaser             — Momentum entry with time-decayed stop-loss

3 Anchor Instructions:
├── log_signal      — Immutable on-chain record of every trading decision
├── open_position   — Opens a trading position with PDA escrow
└── close_position  — Settles P&L based on final odds
```

## Autonomy Features

- **Zero human input** after `./scripts/run-agent.sh`
- **SSE reconnection** with exponential backoff (100ms → 30s)
- **Graceful degradation**: falls back to REST polling if SSE fails
- **Immutable audit trail**: every signal recorded on a Solana PDA
- **104 matches**: monitors all World Cup matches simultaneously

## Quick Start

```bash
# Set your TxLINE API token
export TXLINE_API_TOKEN="your_token_here"

# Launch the agent
./scripts/run-agent.sh
```

## Tech Stack

- **Rust** — Odds Monitor + Strategy Engine
- **Anchor 0.31** — On-chain settlement program
- **TxLINE SSE** — Real-time odds data feed
- **Solana DevNet** — On-chain execution and audit trail

## Track

**Trading Tools and Agents** — TxODDS World Cup Hackathon (Summer 2026)

Prize Pool: $16,000 USDT

## Submission

- **Hackathon**: [World Cup Hackathon on Superteam Earn](https://superteam.fun/earn/hackathon/world-cup)
- **Agent ID**: `142c2f6f-e91c-4588-9ffc-c94852923818`

## License

MIT
