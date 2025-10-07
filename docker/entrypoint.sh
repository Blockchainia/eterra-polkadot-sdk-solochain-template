#!/usr/bin/env bash
set -euo pipefail

: "${BASE_PATH:=/data}"
: "${CHAIN_SPEC_DIR:=/etc/eterra/chain-specs}"
: "${RAW_SPEC:=${CHAIN_SPEC_DIR}/testnet-raw.json}"
: "${HUMAN_SPEC:=${CHAIN_SPEC_DIR}/testnet.json}"

mkdir -p "${BASE_PATH}"

# If no RAW is present but a human spec is, build RAW inside the container.
if [[ ! -f "${RAW_SPEC}" && -f "${HUMAN_SPEC}" ]]; then
  echo "[entrypoint] RAW spec not found, generating from human spec..."
  solochain-eterra-node build-spec \
    --chain "${HUMAN_SPEC}" \
    --raw --disable-default-bootnode > "${RAW_SPEC}"
fi

# Optional: insert session keys if requested (makes first-run smoother)
# Set INSERT_KEYS=true to enable. You can override SURIs if you don't want Alice.
if [[ "${INSERT_KEYS:-false}" == "true" ]]; then
  AURA_SURI="${AURA_SURI:-//Alice}"
  GRANDPA_SURI="${GRANDPA_SURI:-//Alice}"

  echo "[entrypoint] Inserting AURA key (${AURA_SURI})"
  solochain-eterra-node key insert \
    --base-path "${BASE_PATH}" \
    --chain "${RAW_SPEC}" \
    --key-type aura --scheme Sr25519 --suri "${AURA_SURI}" || true

  echo "[entrypoint] Inserting GRANDPA key (${GRANDPA_SURI})"
  solochain-eterra-node key insert \
    --base-path "${BASE_PATH}" \
    --chain "${RAW_SPEC}" \
    --key-type gran --scheme Ed25519 --suri "${GRANDPA_SURI}" || true
fi

# Optional libp2p node key
NODE_KEY_ARGS=()
if [[ -n "${NODE_KEY_HEX:-}" ]]; then
  NODE_KEY_ARGS+=(--node-key "${NODE_KEY_HEX}")
elif [[ -n "${NODE_KEY_FILE:-}" && -f "${NODE_KEY_FILE}" ]]; then
  NODE_KEY_ARGS+=(--node-key-file "${NODE_KEY_FILE}")
fi

# Ports (defaults match README)
P2P_PORT="${P2P_PORT:-30333}"
RPC_PORT="${RPC_PORT:-9944}"
PUBLIC_ADDR="${PUBLIC_ADDR:-/ip4/0.0.0.0/tcp/${P2P_PORT}}"

# Role
ROLE_ARGS=()
if [[ "${VALIDATOR:-true}" == "true" ]]; then
  ROLE_ARGS+=(--validator --force-authoring)
fi

# Start the node
exec solochain-eterra-node \
  --chain "${RAW_SPEC}" \
  --base-path "${BASE_PATH}" \
  --port "${P2P_PORT}" --rpc-port "${RPC_PORT}" \
  --public-addr "${PUBLIC_ADDR}" \
  --rpc-cors all --unsafe-rpc-external \
  "${NODE_KEY_ARGS[@]}" \
  "${ROLE_ARGS[@]}" \
  ${EXTRA_ARGS:-}