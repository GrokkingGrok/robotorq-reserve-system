# 🧭 Learning Path to RoboTorq v0
> Goal: Build a minimal 3-node RoboTorq network (Mint ↔ Oracle ↔ Wallet)  
> Milestone: Achieve Proof-of-Coordination — BRLA → Verified Proof → Minted UBD Stream

---

## Stage 0: Repo Setup (5 min)

- `docker-compose.yml` for local 3-node
- **+ GitHub Actions CI/CD**

## 🏗️ Stage 1: Core Infrastructure — “Run Anything Anywhere”
**Goal:** Learn to spin up and connect multiple isolated services locally.

| Status | Skill | Why You Need It | Key Targets | Tasks |
|:------:|-------|-----------------|--------------|-------|
| ⬜ | **Docker** | Run Mint/Oracle/Wallet in isolated containers | Dockerfiles, networking, env vars | • Write a Dockerfile for a basic Go or Rust node<br>• Run 3 containers and make them ping each other<br>• Mount `.env` files and shared volumes |

🕒 **Est. Time:** 2 hours  
🏁 **Outcome:** You can run 3 connected “Hello Node” containers on your laptop.

---

## 🔗 Stage 2: Communication Layer — “Make the Nodes Talk”
**Goal:** Get Mint ↔ Oracle nodes exchanging structured messages.

| Status | Skill | Why You Need It | Key Targets | Tasks |
|:------:|-------|-----------------|--------------|-------|
| ⬜ | **gRPC** | Define and serve Mint↔Oracle RPC endpoints | RPC schemas + stubs | • Create `.proto` file for `MintRequest`, `ProofResponse`<br>• Generate gRPC server/client code<br>• Send test messages between containers |
| ⬜ | **Protobuf** | Define BRLA, Proof, Bid message types | Message schema design | • Define fields: `id`, `builder_id`, `oracle_signatures`, etc.<br>• Validate serialization/deserialization roundtrip |
| ⬜ | **NATS (Pub/Sub)** | Async message bus for Mint → Oracle broadcasts | Event-driven communication | • Run NATS in Docker<br>• Publish and subscribe events like `"proof.request"` and `"proof.response"` |

🕒 **Est. Time:** 8–10 hours  
🏁 **Outcome:** Mint node can send a “verify” message and receive a signed Oracle response.

---

## 🧠 Stage 3: Persistence & State — “Memory That Survives”
**Goal:** Store and query BRLAs, bids, and proofs.

| Status | Skill | Why You Need It | Key Targets | Tasks |
|:------:|-------|-----------------|--------------|-------|
| ⬜ | **Postgres** | Persistent data layer for BRLA, bids, proofs | Tables + migrations | • Run Postgres via Docker<br>• Create tables for `brla`, `proofs`<br>• Practice SQL CRUD ops |
| ⬜ | **Redis** | Cache for UBD/sec streams & ephemeral data | Speed + TTL control | • Run Redis via Docker<br>• Store `UBD_RATE` key<br>• Test expiry using TTLs |

🕒 **Est. Time:** 6 hours  
🏁 **Outcome:** Mint node stores proof data and streams `UBD/sec` metrics from cache.

---

## 🔒 Stage 4: Security Layer — “Trust the Transport”
**Goal:** Secure traffic and authenticate node connections.

| Status | Skill | Why You Need It | Key Targets | Tasks |
|:------:|-------|-----------------|--------------|-------|
| ⬜ | **TLS / mTLS** | Encrypt all node-to-node traffic | Certificates + mutual auth | • Generate self-signed certs with `mkcert` or OpenSSL<br>• Configure gRPC and NATS to require TLS |

🕒 **Est. Time:** 3–4 hours  
🏁 **Outcome:** Only verified nodes can communicate across the network.

---

## 📊 Stage 5: Observability — “See the UBD Flow”
**Goal:** Monitor performance and UBD/sec in real-time.

| Status | Skill | Why You Need It | Key Targets | Tasks |
|:------:|-------|-----------------|--------------|-------|
| ⬜ | **Prometheus + Grafana** | Visualize proof throughput and UBD streams | Metrics endpoints + dashboards | • Expose `/metrics` on each node<br>• Build Grafana dashboard for UBD/sec per Mint node |

🕒 **Est. Time:** 4 hours  
🏁 **Outcome:** You can watch real-time minting and verification metrics.

---

## 🧪 Stage 6: Reliability — “Make It Hard to Break”
**Goal:** Prevent and detect silent errors during node coordination.

| Status | Skill | Why You Need It | Key Targets | Tasks |
|:------:|-------|-----------------|--------------|-------|
| ⬜ | **Testing (Go: testify / Rust: cargo test)** | Catch integration errors early | Unit + integration tests | • Write tests for Mint↔Oracle message flow<br>• Simulate Oracle timeout and retries<br>• Test database inserts and retrievals |

🕒 **Est. Time:** 3 hours  
🏁 **Outcome:** Each core message path has at least one automated test.

---

## 🌐 Stage 7: Deployment — “Mini-RoboTorq in the Wild”
**Goal:** Run your simulated 3-node network on real infrastructure.

| Status | Skill | Why You Need It | Key Targets | Tasks |
|:------:|-------|-----------------|--------------|-------|
| ⬜ | **Fly.io / Render / VPS** | Deploy Mint, Oracle, Wallet remotely | Container orchestration | • Push Docker images<br>• Run each node on a separate instance<br>• Measure latency + uptime |



🕒 **Est. Time:** 2 hours  
🏁 **Outcome:** A live 3-node RoboTorq network exchanging proofs and minting mock UBD.

---

## 🚀 Final Milestone: **RoboTorq v0 — Proof of Coordination**
✅ You’ll have a minimal working simulation that:
1. Accepts a BRLA  
2. Sends a Proof Verification Request  
3. Receives ≥3 Oracle responses  
4. Mints mock RoboTorq  
5. Streams UBD/sec to wallet nodes  

Everything beyond this — staking, energy pricing, trust reputation, etc. — layers neatly on top.

---

### 🧩 Pro Tip: Track Your Progress
You can mark off each section in the table above using:
- ⬜ → 🟡 when learning
- 🟡 → ✅ when you can demo it

> 💬 *“Don’t try to learn everything. Just learn what you need to make the nodes talk.”*  
> — _RoboTorq Development Principle #1_
