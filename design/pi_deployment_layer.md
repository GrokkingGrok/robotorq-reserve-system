# 🧱 Pi Deployment Layer — RoboTorq v0 LocalNet

This guide describes how to deploy and connect **Mint**, **Oracle**, and **Member Wallet** nodes across multiple Raspberry Pis to simulate the RoboTorq Economy’s three-node network.

---

## ⚙️ 1. Hardware & Base Setup

### Recommended

| Component | Requirement | Notes |
|------------|--------------|-------|
| Raspberry Pi | 4B or 5 (4GB+ RAM) | ARM64 recommended |
| Storage | SSD (USB 3.0) or fast SD | Mint Node benefits from SSD |
| OS | Raspberry Pi OS 64-bit (Lite) | No GUI needed |
| Network | Same LAN or Tailscale mesh | Enables peer RPCs |
| Power | UPS or stable power supply | Prevent data corruption |

---

### Initial Setup (run on each Pi)

```bash
sudo apt update && sudo apt upgrade -y
sudo apt install -y git curl docker.io docker-compose net-tools htop
sudo usermod -aG docker $USER
sudo reboot
```

---

## 🧩 2. Node Roles & Hostnames

| Node Type   | Hostname        | Example Role                   | Notes              |
| ----------- | --------------- | ------------------------------ | ------------------ |
| Mint Node   | `mint.local`    | Contract host, UBD distributor | Requires Postgres  |
| Oracle Node | `oracle1.local` | Verifies proofs (PoWv)         | CPU-light          |
| Wallet Node | `wallet.local`  | Member access, UI, bid + fund  | Runs Redis, NATS   |
| Optional    | `metrics.local` | Monitoring stack               | Prometheus/Grafana |


💡 Use raspi-config or /etc/hostname to rename each Pi.

---

## 📦 3. Directory Layout (per Pi)

```bash
/opt/roboTorq/
├─ mint-node/
│  ├─ docker-compose.yml
│  ├─ config.yml
│  └─ certs/
├─ oracle-node/
│  ├─ docker-compose.yml
│  ├─ oracle.conf
│  └─ certs/
└─ wallet-node/
   ├─ docker-compose.yml
   ├─ api/
   ├─ web/
   └─ certs/
```

Each node shares .proto files for gRPC message consistency.

---

## 🔐 4. Networking & Security
Generate TLS/mTLS Certificates

On the Mint Node:
```bash
mkdir -p /opt/roboTorq/certs && cd /opt/roboTorq/certs
openssl genrsa -out ca.key 4096
openssl req -x509 -new -nodes -key ca.key -sha256 -days 365 -out ca.crt -subj "/CN=RoboTorqCA"
```

Generate node certs (example for Oracle):
```bash
openssl genrsa -out oracle.key 2048
openssl req -new -key oracle.key -out oracle.csr -subj "/CN=oracle1.local"
openssl x509 -req -in oracle.csr -CA ca.crt -CAkey ca.key -CAcreateserial -out oracle.crt -days 365 -sha256
```

Copy ca.crt to all nodes into /opt/roboTorq/certs/.

---

## 🔗 5. Networking Layer
NATS Message Bus (Wallet Node)

docker-compose.yml
```yaml
version: "3.8"
services:
  nats:
    image: nats:alpine
    container_name: nats
    ports:
      - "4222:4222"
      - "8222:8222"
    command: "--js --auth token123"
    restart: always

```

Mint and Oracle nodes will connect via nats://wallet.local:4222.

```yaml
# /opt/roboTorq/mint-node/docker-compose.yml
services:
  mint:
    image: ghcr.io/roboTorq/mint-node:latest
    restart: always
    volumes:
      - ./certs:/certs
    environment:
      - NATS_URL=nats://wallet.local:4222
```

---

## 🧠 6. gRPC & Protobuf Setup
Install Protobuf Compiler
```bash
sudo apt install -y protobuf-compiler
```
Shared schema file (example):

```proto
syntax = "proto3";

message BRLA {
  string id = 1;
  string builder = 2;
  uint64 hours_requested = 3;
  double rate = 4;
}

message Proof {
  string type = 1;
  string hash = 2;
  string oracle_id = 3;
}

service MintService {
  rpc SubmitProof(Proof) returns (Ack);
  rpc GetUBDStatus(Empty) returns (UBDStatus);
}

```
Compile to Go or Rust depending on your node language.

---

## 💾 7. Storage Layer
Mint Node (Postgres)
```bash
docker run -d \
  --name torq-postgres \
  -e POSTGRES_USER=torq \
  -e POSTGRES_PASSWORD=torqpass \
  -e POSTGRES_DB=roboTorq \
  -p 5432:5432 \
  -v /opt/roboTorq/db:/var/lib/postgresql/data \
  postgres:16-alpine
```

Redis Cache (Wallet Node)
```bash
docker run -d \
  --name redis-cache \
  -p 6379:6379 \
  redis:alpine
```

---

## 🧮 8. Monitoring Layer (Optional)
Prometheus + Grafana Node (metrics.local)
```bash
docker-compose up -d
```

Prometheus scrapes /metrics endpoints from each Pi; Grafana visualizes:

UBD/sec

PoWv verification lag

Mint throughput

---

## 🚀 9. Deployment Order

| Step | Node   | Action                                          |
| ---- | ------ | ----------------------------------------------- |
| 1    | Wallet | Start NATS + Redis                              |
| 2    | Mint   | Start Postgres + Mint Node container            |
| 3    | Oracle | Start Oracle Node; connect via gRPC + NATS      |
| 4    | All    | Verify connectivity via `nats-sub` / `nats-pub` |
| 5    | Wallet | Launch web dashboard + APIs                     |

---

## 🧩 10. Quick Connectivity Test

Run from Mint Node:
```bash
nats pub mint.hello "Mint online"
```

Run from Oracle Node:
```bash
nats sub mint.hello
```

✅ If you see “Mint online” — nodes are successfully connected.

---

## 🌀 11. Example Mint Docker Compose (simplified)

```yaml
version: "3.8"
services:
  mint:
    image: ghcr.io/roboTorq/mint-node:latest
    container_name: mint
    environment:
      - NATS_URL=nats://wallet.local:4222
      - POSTGRES_URL=postgres://torq:torqpass@mint.local:5432/roboTorq
    volumes:
      - ./certs:/certs
    ports:
      - "8080:8080"
    restart: always
```

---

## ✅ 12. Validation Checklist

| Test               | Command                        | Expected             |
| ------------------ | ------------------------------ | -------------------- |
| Ping between nodes | `ping oracle1.local`           | ✅ Reachable          |
| NATS pub/sub       | `nats pub/sub`                 | ✅ Real-time          |
| gRPC ping          | `grpcurl mint.local:8080 list` | ✅ Returns services   |
| Mint writes        | Connect to Postgres            | ✅ BRLA records saved |
| Redis cache        | `redis-cli ping`               | ✅ PONG               |

| systemd active | sudo systemctl is-active mint.service |
| Tailscale up | tailscale status |

## 🧠 Next Step

Once connectivity and base services are live:

Deploy mock BRLA auctions between Wallet + Mint.

Send dummy proofs from Oracle via gRPC.

Simulate consensus ≥3 signatures → trigger mock mint.

Congratulations — you’ll have your first RoboTorq v0 LocalNet running on real hardware.
That’s your prototype “energy → token → UBD” loop, end-to-end.

## 🧩 Suggested Extension

For later (v0.5+):

Add Prometheus exporters for each node (/metrics endpoint).

Use ZeroTier or Tailscale for remote access.

Add auto-restart scripts with systemd or Docker healthchecks.


## Summary

Goal: 3 Raspberry Pis form a testnet running Mint, Oracle, and Wallet nodes
Outcome: Physical peer network verifying BRLAs, minting mock RoboTorq, and streaming test UBD
Result: You can visualize and test the entire RoboTorq flow before writing full consensus logic











