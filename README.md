# Eterra Node (`solochain-eterra-node`)

**Eterra Node** is the blockchain core for the Eterra ecosystem — a custom Substrate-based network designed for persistent test and production environments. It currently powers **EterraPocket**, the companion collectible card game, and future role-playing and arena games under the Eterra IP.

This node is based on the Substrate Solochain Template, customized to include our Eterra-specific pallets (such as `eterraFaucet` and future `eterraRewards`) and runtime configurations for the **Eterra Testnet** — a `ChainType::Live` network with defined authorities and a treasury account.

---

## 🧱 Getting Started

### 1. Build the Node

Build the node binary in release mode:

```bash
cargo build -p solochain-eterra-node --release
```

After the build completes, your binary will be available at:

```
./target/release/solochain-eterra-node
```

---

## 🌐 Starting the Eterra Testnet Node

### Start a Persistent Validator (Alice)

```bash
BASE=/var/lib/eterra-testnet/alice

./target/release/solochain-eterra-node \
  --chain chain-specs/testnet-raw.json \
  --base-path "$BASE" \
  --validator --alice \
  --force-authoring \
  --port 30333 --rpc-port 9944 \
  --public-addr /ip4/127.0.0.1/tcp/30333 \
  --unsafe-rpc-external --rpc-cors all
```

This starts a persistent node with:
- Database path: `/var/lib/eterra-testnet/alice`
- Chain spec: `Eterra Testnet`
- Role: Authority validator (Alice)
- RPC endpoint: `ws://127.0.0.1:9944`

You should see:
```
👤 Role: AUTHORITY
🔨 Starting block production on slot ...
```

If not, check that:
- The AURA and GRANDPA keys are inserted (`key insert` commands).
- The chain spec has correct authorities.
- You used `--force-authoring` to allow solo block production.

---

## 🔑 Inserting Keys

If your base path is new, insert validator keys before running:

```bash
BASE=/var/lib/eterra-testnet/alice

# AURA
./target/release/solochain-eterra-node key insert \
  --base-path "$BASE" \
  --chain chain-specs/testnet-raw.json \
  --key-type aura \
  --scheme Sr25519 \
  --suri //Alice

# GRANDPA
./target/release/solochain-eterra-node key insert \
  --base-path "$BASE" \
  --chain chain-specs/testnet-raw.json \
  --key-type gran \
  --scheme Ed25519 \
  --suri //Alice
```

---

## 🧹 Purging Chain State

To remove all local data and start fresh:

```bash
BASE=/var/lib/eterra-testnet/alice

./target/release/solochain-eterra-node purge-chain \
  --chain chain-specs/testnet-raw.json \
  --base-path "$BASE" -y
```

---

## ♻️ Updating Runtime After Adding or Modifying a Pallet

When you add or update a pallet in the runtime, you must rebuild the runtime, regenerate the chain specs, and ensure your node runs with the updated runtime. Follow these steps:

1. Build the runtime and node binaries:

```bash
cargo build -r -p solochain-template-runtime -p solochain-eterra-node
```

2. Rebuild the plain and raw chain specs:

```bash
./target/release/solochain-eterra-node build-spec \
  --chain chain-specs/testnet.json > chain-specs/testnet-plain.json
```

```bash
./target/release/solochain-eterra-node build-spec \
  --chain chain-specs/testnet-plain.json --raw > chain-specs/testnet-raw.json
```

3. Purge the existing chain database to avoid runtime version conflicts:

```bash
BASE=/var/lib/eterra-testnet/alice

./target/release/solochain-eterra-node purge-chain --chain chain-specs/testnet-raw.json --base-path "$BASE" -y
```

4. Restart the node with the updated chain spec and base path:

```bash
BASE=/var/lib/eterra-testnet/alice

./target/release/solochain-eterra-node \
  --chain chain-specs/testnet-raw.json \
  --base-path "$BASE" \
  --validator --alice \
  --force-authoring \
  --port 30333 --rpc-port 9944 \
  --public-addr /ip4/127.0.0.1/tcp/30333 \
  --unsafe-rpc-external --rpc-cors all
```

**Troubleshooting:**  
If you do not see your new or updated pallets in Polkadot.js Apps, verify that the runtime WASM hash matches the chain spec, and clear the Polkadot.js metadata cache (in the Apps portal, click the settings gear and select "Clear metadata cache") before reconnecting.

Tip: To verify genesis config like initialServers landed, run: `jq '.genesis | .. | objects | select(has("initialServers"))' chain-specs/testnet-plain.json`

---

## 💾 Backing Up Chain State

To back up your local node’s database and keystore safely:

```bash
BASE=/var/lib/eterra-testnet/alice
BACKUP=~/eterra-backups/alice-$(date +%Y%m%d%H%M).tar.gz

tar -czf "$BACKUP" -C "$BASE" chains keystore
echo "Backup saved to $BACKUP"
```

To restore from a backup:

```bash
BASE=/var/lib/eterra-testnet/alice
BACKUP=/path/to/alice-backup.tar.gz

tar -xzf "$BACKUP" -C "$BASE"
```

Restoring from a backup will replace the current chain database and keystore with the archived versions.  
This is useful when migrating to a new machine, recovering from accidental data loss, or rolling back to a previous known good state.  
Always ensure the node is stopped before restoring, and verify permissions on `/var/lib/eterra-testnet` afterward to avoid startup errors.

### 🧩 Restore Instructions (for Terminal)

```bash
# 1. Stop the running node process
# (If you started it manually, press Ctrl+C in its terminal window)
# If running as a systemd service, you can stop it using:
# sudo systemctl stop eterra-node
```

```bash
# 2. Define your base path and backup file
BASE=/var/lib/eterra-testnet/alice
BACKUP=/path/to/alice-backup.tar.gz
```

```bash
# 3. Verify the backup file exists
ls -lh "$BACKUP"
```

```bash
# 4. Create a safety snapshot of your current data (optional but recommended)
tar -czf ~/eterra-backups/pre-restore-$(date +%Y%m%d%H%M).tar.gz -C "$BASE" chains keystore
echo "Safety snapshot created."
```

```bash
# 5. Purge current state before restoring (optional if the node is already clean)
./target/release/solochain-eterra-node purge-chain \
  --chain chain-specs/testnet-raw.json \
  --base-path "$BASE" -y
```

```bash
# 6. Extract the backup archive into your base path
tar -xzf "$BACKUP" -C "$BASE"
```

```bash
# 7. Verify restored files
ls -R "$BASE"/chains | head
ls -R "$BASE"/keystore | head
```

```bash
# 8. Fix permissions (important for macOS or after copying from another system)
sudo chown -R $USER:staff "$BASE"
```

---

## 🧠 Notes

- The Eterra Testnet is configured as a `ChainType::Live` network with **no default bootnodes** injected.
- Only whitelisted nodes are allowed to connect (via `pallet-node-authorization`).
- The treasury account is derived from the pallet ID `py/trsry`.
- The faucet pallet (`eterraFaucet`) is active for test networks, distributing small test token amounts.

---

## 🧩 Project Structure

- **`node/`** — Core node logic: consensus setup, RPC, and service management.
- **`runtime/`** — The Eterra runtime configuration, including all pallets.
- **`pallets/`** — Custom Eterra pallets (e.g., Faucet, Rewards, etc.).
- **`chain-specs/`** — Stored chain spec JSON files for dev and testnet environments.

---

## 🔗 Connect with Polkadot-JS Apps

Once your node is running, open the [Polkadot/Substrate Apps Portal](https://polkadot.js.org/apps/#/explorer?rpc=ws://127.0.0.1:9944) and connect to:

```
ws://127.0.0.1:9944
```

You should see **Eterra Testnet** in the upper left corner.

---

## 🧭 Troubleshooting

| Symptom | Likely Cause | Fix |
|----------|---------------|-----|
| Node not producing blocks | Missing session keys | Insert AURA/GRANDPA keys |
| Bootnode mismatch warning | Default 127.0.0.1 bootnode injected | Use `ChainType::Live` in `chain_spec.rs` |
| Permission denied under `/var/lib` | User permissions | `sudo chown -R $USER:staff /var/lib/eterra-testnet` |
| Node reset but old data remains | Chain ID changed | Purge and remove old chain folder manually |

---

**Maintainer:** Eterra Development Team  
**License:** Apache 2.0
