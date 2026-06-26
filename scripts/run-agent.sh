#!/usr/bin/env bash
set -euo pipefail

echo "============================================"
echo " FANatic Alpha — Autonomous Trading Agent"
echo " World Cup 2026 — TxODDS Hackathon"
echo "============================================"
echo ""

# Check env vars
if [ -z "${TXLINE_API_TOKEN:-}" ]; then
    echo "ERROR: TXLINE_API_TOKEN environment variable is required"
    echo "Get your free token at: https://txline.txodds.com/documentation/worldcup"
    exit 1
fi

if [ -z "${SOLANA_KEYPAIR_PATH:-}" ]; then
    export SOLANA_KEYPAIR_PATH="$HOME/.config/solana/id.json"
    echo "Using default Solana keypair: $SOLANA_KEYPAIR_PATH"
fi

echo "Starting odds monitor..."
cargo run --release --bin odds-monitor
