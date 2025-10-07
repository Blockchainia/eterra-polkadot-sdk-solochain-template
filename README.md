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

---

## 🔍 Useful Commands

### Build a Human-Readable Chain Spec
```bash
./target/release/solochain-eterra-node build-spec \
  --chain local --disable-default-bootnode > chain-specs/testnet.json
```

### Convert to a Raw Chain Spec (for validators)
```bash
./target/release/solochain-eterra-node build-spec \
  --chain chain-specs/testnet.json \
  --raw --disable-default-bootnode > chain-specs/testnet-raw.json
```

### Verify Bootnodes (should be empty for Eterra Testnet)
```bash
jq '.bootNodes' chain-specs/testnet-raw.json
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

## ⚙️ Example: Recreate Testnet Genesis

```bash
./target/release/solochain-eterra-node build-spec \
  --chain local --disable-default-bootnode > chain-specs/testnet.json

# Manually adjust authorities, balances, faucet, and treasury as needed.
# Then:
./target/release/solochain-eterra-node build-spec \
  --chain chain-specs/testnet.json --raw > chain-specs/testnet-raw.json
```

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

