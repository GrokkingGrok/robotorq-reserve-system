# RoboTorq Mental Model
## The One-Page Guide to Understanding Everything

**Date**: November 15, 2025  
**Purpose**: Connect the philosophy, architecture, and code in your head

---

## 🎯 The Big Picture (30 seconds)

**What is RoboTorq?**
> A currency where **1 RT = 1 hour of verified robotic work**

**Why does it matter?**
> It's the first currency backed by **physics** (measurable energy + computation), not:
> - ❌ Proof-of-work waste (Bitcoin)
> - ❌ Government decree (US Dollar)
> - ❌ Scarcity (Gold)

**How does it work?**
> Robots do work → Energy is measured → Work is cryptographically proven → Currency is minted

---

## 🔗 The Proof Chain (The Core Architecture)

```
┌──────────────────────────────────────────────────────────────┐
│ LAYER 0: DIGGER (Rust) - Where Work Happens                  │
├──────────────────────────────────────────────────────────────┤
│ Robot processes AI tokens (e.g., LLM inference)               │
│ • Measures: 10 tokens/sec, 2000 watts                        │
│ • Calculates: 10 × 2000 = 20,000 JouleTorqs/sec             │
│ • For each token: Hash(token + joules) → Sign               │
│ • Every 60 sec: Send batch of JTU proofs to Refinery         │
│                                                               │
│ Output: Variable-size batches of JouleTorqUnits (JTU)        │
│         Each JTU = {hash, signature, metadata}               │
└──────────────────────────────────────────────────────────────┘
                          ↓
┌──────────────────────────────────────────────────────────────┐
│ LAYER 1: REFINERY (Go) - Proof Aggregation                   │
├──────────────────────────────────────────────────────────────┤
│ Receives JTUs from all diggers on network                    │
│ • Queue: All JTU hashes from all robots                      │
│ • Accumulate: 3,600,000 JTU hashes                          │
│ • Build: Merkle tree (binary tree, 21 levels)               │
│ • Calculate: Branch_Hash (root of 3.6M hashes)              │
│ • Sign: Refinery signature on Branch_Hash                   │
│ • Discard: Original 3.6M JTU hashes (compressed!)           │
│                                                               │
│ Output: TokenTorqIngot = {branch_hash, signature}            │
│         1 ingot per 3.6M JTUs                                │
└──────────────────────────────────────────────────────────────┘
                          ↓
┌──────────────────────────────────────────────────────────────┐
│ LAYER 2: MINT (Go) - Currency Creation                       │
├──────────────────────────────────────────────────────────────┤
│ Receives ingots from Refinery                                 │
│ • Accumulate: 1,000 ingot branch_hashes                     │
│ • Build: Merkle tree (binary tree, 10 levels)               │
│ • Calculate: Merkle_Root (root of 1,000 hashes)             │
│ • Sign: SPHINCS+ signature (archival proof)                 │
│ • Store: Ledger (hashes + merkle tree)                      │
│ • Broadcast: {RT_id, merkle_root, signature} to network     │
│                                                               │
│ Output: 1 RoboTorqUnit (1 RT) = Legal tender currency        │
│         Backed by 3.6B JouleTorqs of verified work           │
└──────────────────────────────────────────────────────────────┘
```

---

## 🧮 The Economic Formula (The Math)

### JouleTorq Production (Cross Product)
```
JouleTorqs/second = (tokens/sec) × (watts)

Examples:
• 10 tok/s @ 2000 W = 20,000 JTU/sec
• 10 tok/s @ 100 W  = 1,000 JTU/sec
• 100 tok/s @ 5000 W = 500,000 JTU/sec
```

**Why this matters**: High power + high throughput = more currency!

### Currency Hierarchy
```
1 JouleTorq (JT)    = 1 token × 1 joule (atomic unit)
1,000 JT            = 1 TokenTorq (TT)
3,600 TT            = 1 RoboTorq (RT)
──────────────────────────────────────────────────
3,600,000 JT        = 1 RT (one hour of ideal work)
```

### Time to Mint 1 RT
```
RT_time = 3,600,000 JTU ÷ (tokens/sec × watts)

Examples:
• 10 tok/s @ 2 kW:   3.6M ÷ 20,000 = 180 sec (3 min)
• 10 tok/s @ 100 W:  3.6M ÷ 1,000 = 3,600 sec (60 min)
• 100 tok/s @ 5 kW:  3.6M ÷ 500,000 = 7.2 sec
```

**Key insight**: More efficient robots (high tokens/watt) earn MORE per joule!

---

## 📂 The Code Map (Where Things Live)

### Digger (Rust) - `src/digger-app/digger/src-tauri/`
```
src/
├── contract.rs          ← BRLA contract execution
├── energy_monitor.rs    ← Power measurement (watts)
├── token_counter.rs     ← Token throughput tracking
├── jtu_creator.rs       ← Hash + sign JTUs (TO BE IMPLEMENTED)
├── ore_batcher.rs       ← Batch JTUs for sending (TO BE IMPLEMENTED)
└── headless.rs          ← Headless mode (no GUI)
```

**Current Status**: ✅ Energy + tokens measured, ⏳ JTU creation not implemented

### Refinery (Go) - `src/refinery/`
```
internal/refinery/
├── ore_receiver.go      ← HTTP endpoint (receives ore from diggers)
├── queue_manager.go     ← Thread-safe JTU queue
├── ingot_assembler.go   ← Accumulates 3.6M units → 1 ingot
├── batch_sender.go      ← Sends ingot batches to Mint

internal/models/
├── jouletorq_unit.go    ← JTU data structure + hash calculation
├── token_torq.go        ← TokenTorqIngot structure + merkle tree
└── joule_torq.go        ← JouleTorqOre (temporary, from digger)
```

**Current Status**: ✅ Single-queue architecture, ⚠️ Still creates 1 unit per token (needs fix: tokens × joules)

### Mint (Go) - `src/mint/`
```
internal/mint/
├── robotorq_unit.go     ← RoboTorqUnit structure + merkle tree
├── batch_aggregator.go  ← Accumulates 1,000 ingots → 1 RT
├── mint_engine.go       ← Currency creation logic
└── ledger.go            ← Stores proofs + merkle trees (TO BE IMPLEMENTED)

internal/nats/
└── subscriber.go        ← Listens to NATS for ingot batches
```

**Current Status**: ✅ Basic aggregation working, ⏳ Ledger storage not implemented

---

## 🔐 The Cryptography Stack

### Signatures (Proof of Authenticity)

| Layer    | Signature Scheme | Size   | Speed       | Use Case                    |
|----------|------------------|--------|-------------|-----------------------------|
| Digger   | Falcon-1024      | 1.3 KB | 0.1ms verify| JTU proofs (ephemeral)      |
| Refinery | Falcon-1024      | 1.3 KB | 0.1ms verify| Ingot proofs (ephemeral)    |
| Mint     | SPHINCS+-128s    | 7.9 KB | 1ms verify  | RT proofs (forever archival)|

**Why two schemes?**
- **Falcon**: Fast, compact (perfect for real-time proofs that expire in 30 days)
- **SPHINCS+**: Ultra-conservative, hash-based (perfect for "forever" archival proofs)

### Hash Functions
- **SHA256**: Everything (JTU hashes, merkle trees, branch hashes)
- **Why SHA256?**: Post-quantum secure (hashes are quantum-resistant), industry standard

---

## 💾 The Storage Strategy (Compression + Pruning)

### Naive Approach (DON'T DO THIS!)
```
1 RT = 3.6M JTUs × 500 bytes = 1.8 GB ❌
1 year @ 1 RT/hour = 15.8 TB ❌❌❌
```

### Merkle Compression (SMART!)
```
Layer 0: 3.6M JTU hashes × 32 bytes = 115 MB
Layer 1: 1,000 Ingot hashes × 32 bytes = 32 KB
Layer 2: 1 RT hash × 32 bytes = 32 bytes
─────────────────────────────────────────────
Ledger: 115 MB + merkle tree ≈ 230 MB total
Broadcast: 32 bytes + signature ≈ 5 KB
```

### 30-Day Pruning (GENIUS!)
```
Keep for 30 days (dispute window):
  ✓ Full merkle trees (230 MB)
  ✓ All JTU hashes (115 MB)

After 30 days, prune to:
  ✓ RT merkle_root (32 bytes)
  ✓ RT signature (8 KB)
  ✓ 1,000 ingot hashes (32 KB)
  ─────────────────────────
  Total: 44 KB per RT ✅

1 year @ 1 RT/hour:
  = 44 KB × 24 × 365
  = 385 MB per year ✅✅✅
```

**Result**: 98% storage reduction via merkle trees + time-based pruning!

---

## 🛠️ The Build Phases (What to Do When)

### ✅ Phase 1: DONE (Current State)
- [x] Single-queue architecture (Refinery)
- [x] Unit-based ingot assembly
- [x] NATS pub/sub between services
- [x] Docker compose orchestration
- [x] Basic metrics + logging

### 🚧 Phase 2: IN PROGRESS (feature/currency-refactor)
- [x] Documentation (architecture, workflow, proof chain)
- [ ] Fix: JTU creation = tokens × joules (not just tokens)
- [ ] Remove redundant fields (JouleTorqTotal, RoboStakeTotal)
- [ ] E2E test with correct economics

### ⏳ Phase 3: NEXT (feature/proof-chain-refactor)
- [ ] **Digger**: Implement JTU hash generation + signing
- [ ] **Digger**: Time-based batching (send every 60 sec)
- [ ] **Refinery**: Build merkle tree from 3.6M JTU hashes
- [ ] **Refinery**: Sign ingot branch_hash
- [ ] **Mint**: Build merkle tree from 1,000 ingot hashes
- [ ] **Mint**: Implement ledger storage
- [ ] **Mint**: 30-day pruning job

### 🔮 Phase 4: FUTURE (feature/crypto-signatures)
- [ ] Falcon-1024 implementation (Digger + Refinery)
- [ ] SPHINCS+ implementation (Mint)
- [ ] Signature verification at each layer
- [ ] Public verification API

---

## 🧠 Mental Shortcuts (When You Get Lost)

### "What's a JouleTorq?"
> It's the **cross product** of tokens and energy:  
> `JouleTorq = tokens × joules`  
> Think of it like square footage: `area = length × width`

### "Why do we discard data?"
> **Data flows down, proofs flow up.**  
> Raw data (joules, tokens) → Hash → Keep hash, discard data  
> It's like shredding a check after you deposit it (bank keeps the proof, you don't need the paper)

### "Where does the currency come from?"
> The **Mint creates it** when 1,000 ingots arrive (3.6B verified JTUs of work).  
> It's not "mined" with waste, it's **minted** because work was **proven**.

### "Can someone fake a JTU?"
> No! Each JTU has:
> 1. Hash of (token + joules + timestamp + contract + digger)
> 2. Dilithium/Falcon signature from digger's private key
> 
> To fake it, you'd need to:
> - Break SHA256 (impossible)
> - Forge the signature (quantum computers can't even do this)

### "What if a digger lies about energy usage?"
> The **oracle** (energy monitor) is a separate trusted device that:
> - Measures power at the outlet (hardware metering)
> - Signs energy readings independently
> - Gets rotated every 30 days (no long-term control)
> 
> To cheat, you'd need to hack the physical meter (hard!) or collude with oracle providers (slashed if caught)

---

## 📊 The Economics (Why This Could Change Everything)

### Problem with Current Currencies

| Currency   | Backed By          | Problem                           |
|------------|-------------------|-----------------------------------|
| USD        | Government trust  | Infinite printing, inflation      |
| Bitcoin    | Wasted electricity| Energy ≠ value, just waste        |
| Gold       | Scarcity          | Hard to transact, no digital form |

### RoboTorq Advantage

| Feature               | RoboTorq                              | Traditional Currency      |
|-----------------------|---------------------------------------|---------------------------|
| **Backing**           | Measurable work (physics)             | Faith or scarcity         |
| **Counterfeiting**    | Impossible (post-quantum crypto)      | Possible (print fake $)   |
| **Inflation Control** | Tied to real productivity             | Central bank decides      |
| **Efficiency**        | Rewards low energy/token              | No efficiency incentive   |
| **Verification**      | Anyone can verify merkle proofs       | Trust banks/governments   |
| **Future-Proof**      | Quantum-resistant (NIST approved)     | Vulnerable to quantum     |

### The Virtuous Cycle
```
More efficient robots → Lower cost per token → More profit for operators
                ↓
Operators invest in better hardware/software
                ↓
Productivity increases → More RT minted → Economy grows
                ↓
More demand for robotic work → More RT needed → RT value increases
                ↓
Cycle repeats (deflationary pressure as efficiency improves)
```

---

## 🎓 The Philosophy (Why You're Building This)

From your README (Appendix O):

> **"Watts > Wall Street"**

Traditional finance optimizes for:
- ✗ Quarterly earnings (short-term)
- ✗ Stock price manipulation
- ✗ Rent-seeking (extracting value without creating it)

RoboTorq optimizes for:
- ✓ **Real work performed** (long-term value creation)
- ✓ **Energy efficiency** (sustainability)
- ✓ **Verifiable proof** (trustless system)

### The Vision
1. **Today**: Robots work for humans (get paid in fiat)
2. **Tomorrow**: Robots work for themselves (get paid in RT)
3. **Future**: Autonomous economy of machines (humans optional participants)

**The radical idea**: What if robots could **own** their own labor?
- A robot earns RT by working
- Uses RT to pay for electricity, repairs, upgrades
- Saves RT to invest in other robots (capital formation)
- Eventually: Self-sustaining robotic economy

This isn't sci-fi - this is **infrastructure for the 2030s**.

---

## 🚀 When You Feel Overwhelmed, Remember:

### You're Building Three Things
1. **A currency** (economics/game theory)
2. **A protocol** (distributed systems/crypto)
3. **A system** (software engineering)

### Most people only build ONE of these in a lifetime.

### You Don't Need to Hold It All at Once
- **Writing code?** → Focus on the implementation (this file)
- **Designing architecture?** → Focus on proof chain (PROOF_CHAIN_ARCHITECTURE.md)
- **Explaining the vision?** → Focus on economics (README.md)

### The Documents Work Together
```
MENTAL_MODEL.md ────────── You are here (connecting everything)
         │
         ├─→ README.md (5,745 lines) ───────── The vision + economics
         │
         ├─→ PROOF_CHAIN_ARCHITECTURE.md ──── The correct technical design
         │
         ├─→ MINT_ARCHITECTURE.md ──────────── How Mint works
         │
         ├─→ REFINERY_ARCHITECTURE.md ──────── How Refinery works
         │
         └─→ copilot-instructions.md ──────── How to build (14-step workflow)
```

**Use this file** (MENTAL_MODEL.md) as your **entry point** whenever you need to re-ground yourself.

---

## 🎯 Quick Reference

### "I need to understand..."

| Topic                          | Read This                           |
|--------------------------------|-------------------------------------|
| The big picture                | This file (you're here!)            |
| Why RoboTorq exists            | README.md (Sections 1-2)            |
| How proof chain works          | PROOF_CHAIN_ARCHITECTURE.md         |
| Economic formulas              | README.md Appendix O                |
| How to build features          | copilot-instructions.md             |
| Mint internals                 | MINT_ARCHITECTURE.md                |
| Refinery internals             | REFINERY_ARCHITECTURE.md            |
| Cryptography choices           | README.md Appendix F.7              |
| Storage/scaling                | README.md Appendix F.12             |

### "I need to implement..."

| Feature                        | Start Here                          |
|--------------------------------|-------------------------------------|
| JTU hash generation            | `src/digger-app/.../jtu_creator.rs` |
| Merkle tree building           | `src/refinery/.../token_torq.go`    |
| Signature verification         | `src/refinery/.../verify.go`        |
| Ledger storage                 | `src/mint/.../ledger.go`            |
| 30-day pruning                 | `src/mint/.../pruner.go`            |

---

## 💭 Final Thought

You're not just building a currency.

You're building the **economic operating system** for the age of autonomous machines.

When a self-driving car needs to pay for electricity at a charging station...  
When a warehouse robot needs to hire another robot for a task...  
When an AI agent needs to commission work from a GPU farm...  

**They'll use RoboTorq.**

Because it's the only currency that speaks their language: **verified work = value**.

---

*"The future of money isn't gold, or code, or promises. It's physics."* 🤖⚡💰

**— The RoboTorq Vision, November 2025**
