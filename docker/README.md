# Eterra Node — Docker Guide

This guide provides comprehensive instructions for building, running, backing up, and restoring the `solochain-eterra-node` using Docker and Docker Compose. It also covers environment variables, compose configuration, troubleshooting tips, and multi-architecture build instructions.

---

## Table of Contents

- [Prerequisites (Linux)](#prerequisites-linux)
- [Building the Docker Image](#building-the-docker-image)
- [Running the Node](#running-the-node)
- [Backing Up the Node Data](#backing-up-the-node-data)
- [Restoring the Node Data](#restoring-the-node-data)
- [Environment Variables](#environment-variables)
- [Docker Compose Configuration](#docker-compose-configuration)
- [Troubleshooting](#troubleshooting)
- [Multi-Architecture Build](#multi-architecture-build)

---

## Rust Prerequisites (for Building the Node)

If you plan to build the `solochain-eterra-node` binary or rebuild the Docker image from source, ensure your environment has the proper Rust toolchain installed.

### 1. Install Rust and Cargo
```bash
curl https://sh.rustup.rs -sSf | sh
source $HOME/.cargo/env
```

### 2. Verify Installation
```bash
rustc --version
cargo --version
```

### 3. Install the Required Rust Toolchain for Substrate
Substrate-based projects require a nightly Rust toolchain. Use the recommended version for compatibility with this node:
```bash
rustup install nightly-2023-12-07
rustup target add wasm32-unknown-unknown --toolchain nightly-2023-12-07
rustup override set nightly-2023-12-07
```

### 4. Install System Build Dependencies

**For Ubuntu / Debian:**
```bash
sudo apt update
sudo apt install -y build-essential clang pkg-config libssl-dev git cmake
```

**For macOS:**
```bash
brew install llvm cmake openssl
```

After completing these steps, proceed with the [Prerequisites (Linux)](#prerequisites-linux) section to prepare Docker and Docker Compose.

---

## Prerequisites (Linux)

Before building and running the node on Linux, ensure Docker and Docker Compose are installed and running.

1. **Install Docker and Docker Compose:**
   ```bash
   sudo apt update && sudo apt install -y docker.io docker-compose
   sudo systemctl enable --now docker
   ```

2. **Verify installation:**
   ```bash
   docker --version
   docker compose version
   ```

3. **Clone the repository and navigate into it:**
   ```bash
   git clone https://github.com/your-org/polkadot-sdk-solochain-template.git
   cd polkadot-sdk-solochain-template/docker
   ```

Once these prerequisites are complete, continue with the [Building the Docker Image](#building-the-docker-image) section below.

---

## Building the Docker Image

To build the `solochain-eterra-node` Docker image locally, run:

```bash
docker build -t solochain-eterra-node .
```

This will create an image tagged `solochain-eterra-node` based on the Dockerfile in the current directory.

---

## Running the Node

You can run the node using Docker directly:

```bash
docker run -d \
  --name eterra-node \
  -p 9944:9944 \
  -p 9933:9933 \
  -p 9615:9615 \
  -v eterra-node-data:/root/.local/share/eterra-node \
  solochain-eterra-node
```

- Ports exposed:
  - `9944`: WebSocket RPC
  - `9933`: HTTP RPC
  - `9615`: Prometheus metrics

The `eterra-node-data` volume stores the blockchain data persistently.

---

## Backing Up the Node Data

To backup the node's persistent data volume:

```bash
docker run --rm \
  -v eterra-node-data:/data \
  -v $(pwd):/backup \
  alpine \
  tar czf /backup/eterra-node-data-backup.tar.gz -C /data .
```

This command creates a compressed backup archive in your current directory.

---

## Restoring the Node Data

To restore from a backup archive:

```bash
docker run --rm \
  -v eterra-node-data:/data \
  -v $(pwd):/backup \
  alpine \
  sh -c "rm -rf /data/* && tar xzf /backup/eterra-node-data-backup.tar.gz -C /data"
```

Make sure to replace `eterra-node-data-backup.tar.gz` with your backup file name.

---

## Environment Variables

You can configure the node behavior with environment variables when running the container:

| Variable             | Description                                   | Default         |
|----------------------|-----------------------------------------------|-----------------|
| `CHAIN_SPEC`         | Chain specification file or preset name       | `solochain`     |
| `NODE_NAME`          | Custom name for the node                       | `eterra-node`   |
| `RPC_PORT`           | HTTP RPC port                                 | `9933`          |
| `WS_PORT`            | WebSocket RPC port                            | `9944`          |
| `PROMETHEUS_PORT`    | Prometheus metrics port                        | `9615`          |
| `LOG_LEVEL`          | Log verbosity level (`error`, `warn`, `info`, `debug`) | `info`          |

Example usage:

```bash
docker run -d \
  -e NODE_NAME="my-eterra-node" \
  -e LOG_LEVEL="debug" \
  -p 9944:9944 \
  -p 9933:9933 \
  -p 9615:9615 \
  -v eterra-node-data:/root/.local/share/eterra-node \
  solochain-eterra-node
```

---

## Docker Compose Configuration

You can also use Docker Compose to manage the node container. Below is a sample `docker-compose.yml` file:

```yaml
version: '3.8'

services:
  eterra-node:
    image: solochain-eterra-node
    container_name: eterra-node
    ports:
      - "9944:9944"
      - "9933:9933"
      - "9615:9615"
    environment:
      - NODE_NAME=eterra-node
      - LOG_LEVEL=info
    volumes:
      - eterra-node-data:/root/.local/share/eterra-node
    restart: unless-stopped

volumes:
  eterra-node-data:
```

To start the node:

```bash
docker-compose up -d
```

To stop the node:

```bash
docker-compose down
```

---

## Troubleshooting

- **Node fails to start or crashes:**
  - Check container logs with `docker logs eterra-node`.
  - Verify that no other services are using the exposed ports.
- **Data volume issues:**
  - Ensure the volume is mounted correctly.
  - Backup and restore data if corruption is suspected.
- **Networking issues:**
  - Confirm Docker network settings.
  - Use `docker network inspect` to debug.

---

## Multi-Architecture Build

To build images for multiple architectures (e.g., `amd64` and `arm64`), use Docker Buildx:

1. Enable experimental features and create a new builder:

```bash
docker buildx create --use
```

2. Build and push multi-arch images:

```bash
docker buildx build --platform linux/amd64,linux/arm64 \
  -t your-dockerhub-username/solochain-eterra-node:latest \
  --push .
```

Replace `your-dockerhub-username` with your Docker Hub username.

This enables running the node on different hardware platforms with the same image tag.

---

Thank you for using the Eterra Node Docker setup! For further assistance, please refer to the official Eterra documentation or open an issue on the project repository.
